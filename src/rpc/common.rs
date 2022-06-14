use serde::{Deserialize, Serialize};

use ethers_core::types::H256;

/// Response to relay request, contains an ID for the task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayResponse {
    /// The task ID
    task_id: H256,
}

impl RelayResponse {
    /// The task ID
    pub fn task_id(&self) -> H256 {
        self.task_id
    }
}
