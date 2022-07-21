use reqwest::{IntoUrl, Url};

use ethers_core::types::{H256, U64};
use once_cell::sync::Lazy;

use crate::{
    json_get, json_post,
    rpc::{self},
    task::GelatoTask,
    FeeToken,
};

static DEFAULT_URL: Lazy<reqwest::Url> =
    Lazy::new(|| "https://relay.gelato.digital/".parse().unwrap());

/// Gelato Client Errors
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Reqwest Error
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    /// Url Parsing Error
    #[error("{0}")]
    UrlParse(#[from] url::ParseError),
    /// Serde Json deser Error
    #[error("{0}")]
    SerdeError(#[from] serde_json::Error),
    /// Other Error
    #[error("{0}")]
    Other(String),
}

/// Gelato Client Results
pub type ClientResult<T> = Result<T, ClientError>;

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
    pub fn new<S>(url: S) -> ClientResult<Self>
    where
        S: IntoUrl,
    {
        Ok(Self {
            url: url.into_url()?,
            ..Default::default()
        })
    }

    /// Instantiate a new client with a specific URL and a reqwest Client
    ///
    /// # Errors
    ///
    /// If the url param cannot be parsed as a URL
    pub fn new_with_client<S>(url: S, client: reqwest::Client) -> ClientResult<Self>
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
    ) -> ClientResult<rpc::RelayResponse> {
        json_post!(
            self.client,
            self.send_relay_transaction_url(chain_id),
            params,
        )
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
    pub async fn send_forward_call(
        &self,
        params: &rpc::ForwardCall,
    ) -> ClientResult<rpc::RelayResponse> {
        json_post!(
            self.client,
            self.send_forward_request_url(params.chain_id),
            params
        )
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
    ) -> ClientResult<rpc::RelayResponse> {
        json_post!(
            self.client,
            self.send_forward_request_url(params.chain_id),
            params,
        )
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
    pub async fn send_meta_tx_request(
        &self,
        params: &rpc::SignedMetaTxRequest,
    ) -> ClientResult<rpc::RelayResponse> {
        json_post!(
            self.client,
            self.send_forward_request_url(params.chain_id),
            params,
        )
    }

    /// Check if a chain id is supported by Gelato API
    pub async fn is_chain_supported(&self, chain_id: u64) -> ClientResult<bool> {
        Ok(self.get_gelato_relay_chains().await?.contains(&chain_id))
    }

    fn relay_chains_url(&self) -> reqwest::Url {
        self.url.join("relays/").unwrap()
    }

    /// Get a list of supported chains
    pub async fn get_gelato_relay_chains(&self) -> ClientResult<Vec<u64>> {
        Ok(json_get!(
            self.client,
            self.relay_chains_url(),
            rpc::RelayChainsResponse
        )?
        .relays())
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
    ) -> ClientResult<U64> {
        Ok(json_get!(
            self.client,
            self.estimated_fee_url(chain_id, payment_token.into(), gas_limit, is_high_priority),
            rpc::EstimatedFeeResponse
        )?
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
    pub async fn get_task_status(&self, task_id: H256) -> ClientResult<rpc::TransactionStatus> {
        let resp = json_get!(
            self.client,
            self.get_task_status_url(task_id),
            rpc::TaskStatusResponse,
        )?;

        match resp {
            rpc::TaskStatusResponse::Data { data } => Ok(data
                .into_iter()
                .next()
                .expect("Will be error if no status is returned")),
            rpc::TaskStatusResponse::Error { message } => Err(ClientError::Other(message)),
        }
    }

    /// Create a future that will track the status of a task
    pub fn track_task<P>(&self, task_id: H256, payload: P) -> GelatoTask<P> {
        GelatoTask::new(task_id, self, payload)
    }

    /// Dispatch a forward request. Get a future tracking its status
    pub async fn forward_request(
        &self,
        params: &rpc::SignedForwardRequest,
    ) -> ClientResult<GelatoTask<'_, rpc::SignedForwardRequest>> {
        let resp = self.send_forward_request(params).await?;
        Ok(self.track_task(resp.task_id(), params.clone()))
    }

    /// Dispatch a meta tx request. Get a future tracking its status
    pub async fn meta_tx_request(
        &self,

        params: &rpc::SignedMetaTxRequest,
    ) -> ClientResult<GelatoTask<'_, rpc::SignedMetaTxRequest>> {
        let resp = self.send_meta_tx_request(params).await?;
        Ok(self.track_task(resp.task_id(), params.clone()))
    }
}
