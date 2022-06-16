use serde::{Deserialize, Serialize};

use ethers_core::types::{Address, Bytes, H256, U256};

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
    #[serde(serialize_with = "crate::ser::serialize_checksum_addr")]
    pub to: Address,
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
    #[serde(with = "crate::ser::json_u256_ser")]
    pub gas_price: U256,
    /// Max fee per gas
    #[serde(with = "crate::ser::json_u256_ser")]
    pub max_fee_per_gas: U256,
    /// Max priority fee per gas
    #[serde(with = "crate::ser::json_u256_ser")]
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
