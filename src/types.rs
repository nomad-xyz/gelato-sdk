use ethers_core::types::{Bytes, H160, H256, U256, U64};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Magic value used to specify the chain-native token
static NATIVE_TOKEN: Lazy<FeeToken> = Lazy::new(|| {
    FeeToken(
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
            .parse()
            .unwrap(),
    )
});

/// Gelato payment type
///
/// <https://docs.gelato.network/developer-products/gelato-relay-sdk/payment-types>
#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum PaymentType {
    /// The target smart contract will pay Gelato Relay's smart contract as the
    /// call is forwarded. Payment can be done in feeToken, where it is
    /// expected to be a whitelisted payment token.
    Synchronous = 0,
    /// The sponsor must hold a balance in one of Gelato's Gas Tank smart
    /// contracts. The balance could even be held on a different chainId than
    /// the one the transaction is being relayed on (as defined by
    /// sponsorChainId).
    ///
    /// An event is emitted to tell Gelato how much to charge in the future,
    /// which shall be acknowledged in an off-chain accounting system. A
    /// sponsor signature is expected in order to ensure that the sponsor
    /// agrees on being charged up to a maxFee amount
    AsyncGasTank = 1,
    /// Similar to Type 1, but sponsor is expected to hold a balance with
    /// Gelato on the same chainId where the transaction is executed. Fee
    /// deduction happens during the transaction. A sponsor signature is
    /// expected in order to ensure that the sponsor agrees on being charged up
    /// to a maxFee amount.
    SyncGasTank = 2,
    /// In this scenario a sponsor pre-approves the appropriate Gelato Relay's
    /// smart contract to spend tokens up so some maximum allowance value.
    /// During execution of the transaction, Gelato will credit due fees using
    /// `IERC20(feeToken).transferFrom(...)` in order to pull fees from his/her
    /// account. A sponsor signature is expected in order to ensure that the
    /// sponsor agrees on being charged up to a maxFee amount.
    SyncPullFee = 3,
}

/// A gelato fee token is an ERC20 address, which defaults to `0xee..ee`. This
/// magic value indicates "eth" or the native asset of the chain. This FeeToken
/// must be allowlisted by Gelato validators
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeeToken(H160);

impl std::ops::Deref for FeeToken {
    type Target = H160;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for FeeToken {
    type Err = <H160 as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Default for FeeToken {
    fn default() -> Self {
        *NATIVE_TOKEN
    }
}

impl From<H160> for FeeToken {
    fn from(token: H160) -> Self {
        Self(token)
    }
}

/// A Relay Request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayRequest {
    /// The address of the contract to be called
    pub dest: H160,
    /// The calldata
    pub data: Bytes,
    /// The fee token
    pub token: FeeToken,
    /// The amount of fee
    pub relayer_fee: U64,
}

/// An Estimated Fee Request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EstimatedFeeRequest {
    /// Payment token
    pub payment_token: FeeToken,
    /// Gas limit
    pub gas_limit: U64,
    /// Whether this is high priority
    pub is_high_priority: bool,
}

/// Response to relay request, contains an ID for the task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayResponse {
    /// The task ID
    pub task_id: H256,
}

/// Response to estimated fee request. Contains the estimated fee
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EstimatedFeeResponse {
    /// The oracle-recommended fee, as a decimal string
    estimated_fee: String,
}

impl EstimatedFeeResponse {
    pub(crate) fn estimated_fee(&self) -> usize {
        self.estimated_fee.parse().unwrap()
    }
}

/// Response to Relay chains request. Contains a list of chain ids supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelayChainsResponse {
    /// The supported chain ids
    relays: Vec<String>,
}

impl RelayChainsResponse {
    pub(crate) fn relays(&self) -> impl Iterator<Item = usize> + '_ {
        self.relays.iter().map(|s| s.parse().unwrap())
    }
}

/// Response to the GetTaskStatus api call. Contains an array of task statuses
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusResponse {
    data: Vec<TransactionStatus>,
}

impl std::ops::Deref for TaskStatusResponse {
    type Target = Vec<TransactionStatus>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl IntoIterator for TaskStatusResponse {
    type Item = TransactionStatus;

    type IntoIter = <Vec<TransactionStatus> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

/// A TransactionStatus object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatus {
    /// Service name
    pub service: String,
    /// Chain name
    pub chain: String,
    /// Task id
    pub task_id: H256,
    /// Task state
    pub task_state: TaskState,
    /// Created at date/time string
    #[serde(rename = "created_at")]
    pub created_at: String, // date
    /// Info from last check
    pub last_check: Option<CheckOrDate>,
    /// Execution info
    pub execution: Option<Execution>,
    /// Last execution date/time string
    pub last_execution: String, // date
}

/// Execution details
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Execution {
    /// Transaction status
    pub status: String,
    /// Transaction hash
    pub transaction_hash: H256,
    /// Block number
    pub block_number: usize,
    /// Creation date/time string
    #[serde(rename = "created_at")]
    pub created_at: String,
}

/// Either check details, or a date/time string
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged, rename_all = "camelCase")]
pub enum CheckOrDate {
    /// Date
    Date(String),
    /// Check
    Check(Check),
}

/// Check info for a
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    /// Task state at this check
    pub task_state: TaskState,
    /// Message string
    pub message: Option<String>,
    /// Creation date/time string
    #[serde(rename = "created_at")]
    pub created_at: Option<String>,
}

/// Transaction payload information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    /// Transaction target
    pub to: H160,
    /// Transaction input data
    pub data: Bytes,
    /// Fee data
    pub fee_data: FeeData,
}

/// eip1559 fee data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FeeData {
    /// Gas Price
    pub gas_price: U256,
    /// Max fee per gas
    pub max_fee_per_gas: U256,
    /// Max priority fee per gas
    pub max_priority_fee_per_gas: U256,
}

/// Task states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskState {
    /// CheckPending
    CheckPending,
    /// ExecPending
    ExecPending,
    /// ExecSuccess
    ExecSuccess,
    /// ExecReverted
    ExecReverted,
    /// WaitingForConfirmation
    WaitingForConfirmation,
    /// Blacklisted
    Blacklisted,
    /// Cancelled
    Cancelled,
    /// NotFound
    NotFound,
}
