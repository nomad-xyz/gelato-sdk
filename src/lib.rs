//! A Gelato relay SDK in rust

#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
#![forbid(where_clauses_object_safety)]

/// Gelato Types
pub mod types;

use forward::SignedForwardRequest;
pub use types::*;

/// lib utils
pub(crate) mod utils;

/// Forward Request
pub mod forward;

/// Re-export reqwest for convenience
pub use reqwest;
use reqwest::{IntoUrl, Url};

use ethers_core::types::{Bytes, H160, H256, U64};
use once_cell::sync::Lazy;
use std::str::FromStr;

static DEFAULT_URL: Lazy<reqwest::Url> =
    Lazy::new(|| "https://relay.gelato.digital/".parse().unwrap());

/// A Gelato Relay Client
#[derive(Debug, Clone)]
pub struct GelatoClient {
    url: reqwest::Url,
    client: reqwest::Client,
}

impl Default for GelatoClient {
    fn default() -> Self {
        Self {
            url: DEFAULT_URL.clone(),
            client: Default::default(),
        }
    }
}

impl GelatoClient {
    /// Instantiate a new client with a specific URL
    ///
    /// # Errors
    ///
    /// If the url param cannot be parsed as a URL
    pub fn new<S>(url: S) -> Result<Self, reqwest::Error>
    where
        S: IntoUrl,
    {
        Ok(Self {
            url: url.into_url()?,
            ..Default::default()
        })
    }

    async fn get(&self, url: Url) -> Result<reqwest::Response, reqwest::Error> {
        self.client.get(url).send().await
    }

    /// Instantiate a new client with a specific URL and a reqwest Client
    ///
    /// # Errors
    ///
    /// If the url param cannot be parsed as a URL
    pub fn new_with_client<S>(
        url: S,
        client: reqwest::Client,
    ) -> Result<Self, <reqwest::Url as FromStr>::Err>
    where
        S: AsRef<str>,
    {
        Ok(Self {
            url: url.as_ref().parse()?,
            client,
        })
    }

    /// Send a transaction over the relay
    pub async fn send_relay_transaction(
        &self,
        chain_id: usize,
        dest: H160,
        data: Bytes,
        fee_token: FeeToken,
        relayer_fee: U64,
    ) -> Result<RelayResponse, reqwest::Error> {
        let params = RelayRequest {
            dest,
            data,
            token: fee_token,
            relayer_fee,
        };

        let url = format!("{}/relays/{}", &self.url, chain_id);
        let res = reqwest::Client::new()
            .post(url)
            .json(&params)
            .send()
            .await?;

        res.json().await
    }

    fn send_forward_request_url(&self, chain_id: u64) -> Url {
        self.url
            .join("metabox-relays/")
            .unwrap()
            .join(&format!("{}", chain_id))
            .unwrap()
    }

    /// Send a transaction forward request
    ///
    /// <https://docs.gelato.network/developer-products/gelato-relay-sdk/request-types#forwardrequest>
    ///
    /// ForwardRequest is designed to handle payments of type 1, 2 and 3, in
    /// cases where all meta-transaction related logic (or other kinds of
    /// replay protection mechanisms such as hash based commitments) is already
    /// implemented inside target smart contract. The sponsor is still required
    /// to EIP-712 sign this request, in order to ensure the integrity of
    /// payments. Optionally, nonce may or may not be enforced, by setting
    /// enforceSponsorNonce. Some dApps may not need to rely on a nonce for
    /// ForwardRequest if they already implement strong forms of replay
    /// protection.
    pub async fn send_forward_request(
        &self,
        params: &SignedForwardRequest,
    ) -> Result<RelayResponse, reqwest::Error> {
        self.client
            .post(self.send_forward_request_url(params.chain_id))
            .json(&params)
            .send()
            .await?
            .json()
            .await
    }

    /// Check if a chain id is supported by Gelato API
    pub async fn is_chain_supported(&self, chain_id: usize) -> Result<bool, reqwest::Error> {
        Ok(self.get_gelato_relay_chains().await?.contains(&chain_id))
    }

    fn relay_chains_url(&self) -> reqwest::Url {
        self.url.join("relays/").unwrap()
    }

    /// Get a list of supported chains
    pub async fn get_gelato_relay_chains(&self) -> Result<Vec<usize>, reqwest::Error> {
        let res = self.client.get(self.relay_chains_url()).send().await?;
        Ok(res.json::<RelayChainsResponse>().await?.relays().collect())
    }

    fn get_estimated_fee_url(
        &self,
        chain_id: usize,
        payment_token: FeeToken,
        gas_limit: usize,
        is_high_priority: bool,
    ) -> Url {
        let mut url = self
            .url
            .join("oracles/")
            .unwrap()
            .join(&format!("{}/", chain_id))
            .unwrap()
            .join("estimate")
            .unwrap();

        let payment_token = format!("{:?}", *payment_token);

        url.query_pairs_mut()
            .append_pair("paymentToken", &payment_token)
            .append_pair("gasLimit", &gas_limit.to_string())
            .append_pair("isHighPriority", &is_high_priority.to_string());
        url
    }

    /// Get the estimated fee for a specific amount of gas on a specific chain,
    /// denominated in a specific payment token./
    ///
    ///
    pub async fn get_estimated_fee(
        &self,
        chain_id: usize,
        payment_token: impl Into<FeeToken>,
        gas_limit: usize,
        is_high_priority: bool,
    ) -> Result<usize, reqwest::Error> {
        let url =
            self.get_estimated_fee_url(chain_id, payment_token.into(), gas_limit, is_high_priority);

        Ok(reqwest::get(url)
            .await?
            .json::<EstimatedFeeResponse>()
            .await?
            .estimated_fee())
    }

    fn get_task_status_url(&self, task_id: H256) -> Url {
        self.url
            .join("/tasks/GelatoMetaBox/")
            .unwrap()
            .join(&format!("{:?}/", task_id))
            .unwrap()
    }

    /// Fetch the status of a task
    pub async fn get_task_status(
        &self,
        task_id: H256,
    ) -> Result<Option<TransactionStatus>, reqwest::Error> {
        Ok(self
            .get(self.get_task_status_url(task_id))
            .await?
            .json::<TaskStatusResponse>()
            .await?
            .into_iter()
            .next())
    }
}
