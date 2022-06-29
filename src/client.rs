use futures_timer::Delay;
use futures_util::ready;
use pin_project::pin_project;

use reqwest::{IntoUrl, Url};

use ethers_core::types::{H256, U64};
use once_cell::sync::Lazy;
use std::{
    future::Future,
    pin::Pin,
    str::FromStr,
    task::{Context, Poll},
    time::Duration,
};

use crate::{
    rpc::{self, Check, CheckOrDate, Execution},
    FeeToken,
};

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
    pub async fn send_forward_call(
        &self,
        params: &rpc::ForwardCall,
    ) -> Result<rpc::RelayResponse, reqwest::Error> {
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
    pub async fn send_meta_tx_request(
        &self,
        params: &rpc::SignedMetaTxRequest,
    ) -> Result<rpc::RelayResponse, reqwest::Error> {
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

/// Gelato Task error
#[derive(Debug, thiserror::Error)]
pub enum TaskError {
    /// reqwest
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    /// cancelled by backend
    #[error("Cancelled by backend")]
    Cancelled {
        /// Cancellation message
        message: Option<String>,
        /// Cancellation reason
        reason: Option<String>,
    },
    /// Reverted
    #[error("Execution Reverted")]
    Reverted {
        /// execution
        execution: Execution,
        /// last check
        last_check: Box<Check>,
    },
    /// BlackListed by backend
    #[error("BlackListed by backend")]
    BlackListed {
        /// Cancellation message
        message: Option<String>,
        /// Cancellation reason
        reason: Option<String>,
    },
    /// Not found
    #[error("Dropped by backend")]
    NotFound,
    /// Too many retries
    #[error("Backend returned too many error responses")]
    TooManyRetries,
}

// convenience
type PinBoxFut<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A pending Gelato task
///
/// Retries are decremented when the server returns "undefined", indicating a
/// potentially recoverable backend error. Unrecoverable backend errors (e.g.
/// deserialization errors or HTTP 500-series statuses are not retried.
#[pin_project(project = TaskProj)]
pub struct Task<'a> {
    /// Task Id
    id: H256,
    /// Client
    client: &'a GelatoClient,
    /// task state
    state: TaskState<'a>,
    /// retries
    retries: usize,
    /// delay between requests
    delay: Duration,
}

const DEFAULT_RETRIES: usize = 5;
const DEFAULT_DELAY: u64 = 15;

enum TaskState<'a> {
    /// Initial delay to ensure the GettingTx loop doesn't immediately fail
    Delaying(Pin<Box<Delay>>),
    ///
    Requesting(PinBoxFut<'a, Result<Option<rpc::TransactionStatus>, reqwest::Error>>),
}

impl<'a> std::fmt::Debug for Task<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Task").field("id", &self.id).finish()
    }
}

impl<'a> Task<'a> {
    /// Instantiate a Task
    pub fn new(id: H256, client: &'a GelatoClient) -> Self {
        let delay = Duration::from_secs(DEFAULT_DELAY);
        Self {
            id,
            client,
            state: TaskState::Delaying(Box::pin(Delay::new(delay))),
            retries: DEFAULT_RETRIES,
            delay,
        }
    }

    /// Set the number of retries. Retries are decremented when the server
    /// returns "undefined", indicating a potentially recoverable backend error.
    /// Unrecoverable backend errors (e.g. deserialization errors or HTTP
    /// 500-series statuses are not retried
    #[must_use]
    pub fn retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    /// Sets the polling delay (the time between poll attempts)
    #[must_use]
    pub fn polling_interval<T: Into<Duration>>(mut self, duration: T) -> Self {
        self.delay = duration.into();

        if matches!(self.state, TaskState::Delaying(_)) {
            self.state = TaskState::Delaying(Box::pin(Delay::new(self.delay)))
        }

        self
    }
}

macro_rules! make_request {
    ($cx:ident, $this:ident) => {
        *$this.state = TaskState::Requesting(Box::pin($this.client.get_task_status(*$this.id)));
        $cx.waker().wake_by_ref();
        return Poll::Pending
    };
}

macro_rules! delay_it {
    ($cx:ident, $this:ident) => {
        *$this.state = TaskState::Delaying(Box::pin(Delay::new(*$this.delay)));
        $cx.waker().wake_by_ref();
        return Poll::Pending
    };
}

impl<'a> Future for Task<'a> {
    type Output = Result<Execution, TaskError>;

    #[tracing::instrument(skip(self), fields(task_id = ?self.id, retries_remaining = self.retries))]
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this: TaskProj = self.project();

        let status_fut = match this.state {
            // early returns only :)
            TaskState::Delaying(delay) => {
                ready!(delay.as_mut().poll(cx));
                make_request!(cx, this);
            }
            // just unpack the future
            TaskState::Requesting(fut) => fut,
        };

        let status = ready!(status_fut.as_mut().poll(cx));

        if let Ok(None) = status {
            tracing::warn!("Undefined status while polling task");
            if *this.retries == 0 {
                return Poll::Ready(Err(TaskError::TooManyRetries));
            }
            *this.retries -= 1;
        }

        if let Err(e) = status {
            return Poll::Ready(Err(TaskError::ReqwestError(e)));
        }

        let rpc::TransactionStatus {
            last_check,
            execution,
            ..
        } = status.expect("checked").expect("checked");

        if last_check.is_none() {
            delay_it!(cx, this);
        }

        let last_check = last_check.expect("checked");
        let last_check = match last_check {
            CheckOrDate::Date(_) => {
                delay_it!(cx, this);
            }
            CheckOrDate::Check(last_check) => last_check,
        };

        match last_check.task_state {
            rpc::TaskState::ExecSuccess => {
                Poll::Ready(Ok(execution.expect("exists if status is sucess")))
            }
            rpc::TaskState::ExecReverted => Poll::Ready(Err(TaskError::Reverted {
                execution: execution.expect("exists if status is reverted"),
                last_check: Box::new(last_check),
            })),
            rpc::TaskState::Blacklisted => Poll::Ready(Err(TaskError::BlackListed {
                message: last_check.message,
                reason: last_check.reason,
            })),
            rpc::TaskState::Cancelled => Poll::Ready(Err(TaskError::Cancelled {
                message: last_check.message,
                reason: last_check.reason,
            })),
            rpc::TaskState::NotFound => Poll::Ready(Err(TaskError::NotFound)),
            _ => {
                delay_it!(cx, this);
            }
        }
    }
}
