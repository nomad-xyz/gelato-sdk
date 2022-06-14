/// Re-export reqwest for convenience
pub use reqwest;
use reqwest::{IntoUrl, Url};

use ethers_core::types::{H256, U64};
use once_cell::sync::Lazy;
use std::str::FromStr;

use crate::{rpc, FeeToken};

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

    fn send_relay_transaction_url(&self, chain_id: u64) -> reqwest::Url {
        let path = format!("relays/{}", chain_id);
        let mut url = self.url.clone();
        url.set_path(&path);
        url
    }

    /// Send a transaction over the relay
    pub async fn send_relay_transaction(
        &self,
        params: &rpc::RelayRequest,
        chain_id: u64,
    ) -> Result<rpc::RelayResponse, reqwest::Error> {
        let url = self.send_relay_transaction_url(chain_id);
        let res = reqwest::Client::new().post(url).json(params).send().await?;

        res.json().await
    }

    fn send_forward_request_url(&self, chain_id: u64) -> Url {
        self.url
            .join("metabox-relays/")
            .unwrap()
            .join(&format!("{}", chain_id))
            .unwrap()
    }

    /// Send a transaction forward call
    ///
    /// <https://docs.gelato.network/developer-products/gelato-relay-sdk/request-types#forwardcall>
    ///
    /// `ForwardCall` is designed to handle payments of type `Synchronous`, as
    /// it requires no signatures.
    ///
    /// Because payment is of type `Synchronous`, the target contract MUST
    /// pay for its gas in `params.fee_token` during call forwarding.
    pub async fn send_forward_call(&self, params: &rpc::ForwardCall) -> Result<rpc::RelayResponse, reqwest::Error> {
        self.client
            .post(self.send_forward_request_url(params.chain_id))
            .json(&params)
            .send()
            .await?
            .json()
            .await
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
        params: &rpc::SignedForwardRequest,
    ) -> Result<rpc::RelayResponse, reqwest::Error> {
        self.client
            .post(self.send_forward_request_url(params.chain_id))
            .json(&params)
            .send()
            .await?
            .json()
            .await
    }

    /// Gelato relay MetaTxRequest
    ///
    /// <https://docs.gelato.network/developer-products/gelato-relay-sdk/request-types#metatxrequest>
    ///
    /// MetaTxRequest is designed to handle payments of type AsyncGasTank,
    /// SyncGasTank and SyncPullFee, in cases where the target contract does not
    /// have any meta-transaction nor replay protection logic. In this case, the
    /// appropriate Gelato Relay's smart contract already verifies user and sponsor
    /// signatures. user is the EOA address that wants to interact with the dApp,
    /// while sponsor is the account that pays fees.
    pub async fn send_meta_tx_request(&self, params: &rpc::SignedMetaTxRequest) -> Result<rpc::RelayResponse, reqwest::Error> {
        self.client
        .post(self.send_forward_request_url(params.chain_id))
        .json(&params)
        .send()
        .await?
        .json()
        .await
    }

    /// Check if a chain id is supported by Gelato API
    pub async fn is_chain_supported(&self, chain_id: u64) -> Result<bool, reqwest::Error> {
        Ok(self.get_gelato_relay_chains().await?.contains(&chain_id))
    }

    fn relay_chains_url(&self) -> reqwest::Url {
        self.url.join("relays/").unwrap()
    }

    /// Get a list of supported chains
    pub async fn get_gelato_relay_chains(&self) -> Result<Vec<u64>, reqwest::Error> {
        let res = self.client.get(self.relay_chains_url()).send().await?;
        Ok(res.json::<rpc::RelayChainsResponse>().await?.relays())
    }

    fn estimated_fee_url(
        &self,
        chain_id: u64,
        payment_token: FeeToken,
        gas_limit: U64,
        is_high_priority: bool,
    ) -> Url {
        let path = format!("oracles/{}/estimate", chain_id);
        let mut url = self.url.clone();
        url.set_path(&path);

        let payment_token = format!("{:?}", *payment_token);
        url.query_pairs_mut()
            .append_pair("paymentToken", &payment_token)
            .append_pair("gasLimit", &gas_limit.as_u64().to_string())
            .append_pair("isHighPriority", &is_high_priority.to_string());
        url
    }

    /// Get the estimated fee for a specific amount of gas on a specific chain,
    /// denominated in a specific payment token./
    ///
    ///
    pub async fn get_estimated_fee(
        &self,
        chain_id: u64,
        payment_token: impl Into<FeeToken>,
        gas_limit: U64,
        is_high_priority: bool,
    ) -> Result<U64, reqwest::Error> {
        let url =
            self.estimated_fee_url(chain_id, payment_token.into(), gas_limit, is_high_priority);

        Ok(reqwest::get(url)
            .await?
            .json::<rpc::EstimatedFeeResponse>()
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
    ) -> Result<Option<rpc::TransactionStatus>, reqwest::Error> {
        Ok(self
            .get(self.get_task_status_url(task_id))
            .await?
            .json::<rpc::TaskStatusResponse>()
            .await?
            .into_iter()
            .next())
    }
}
