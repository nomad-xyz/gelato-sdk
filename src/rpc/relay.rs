use serde::{Deserialize, Serialize};

use ethers_core::types::{Address, Bytes, U64};

use crate::FeeToken;

/// A Relay Request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayRequest {
    /// The address of the contract to be called
    pub dest: Address,
    /// The calldata
    pub data: Bytes,
    /// The fee token
    pub token: FeeToken,
    /// The amount of fee
    pub relayer_fee: U64,
}
