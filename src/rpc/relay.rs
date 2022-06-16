use serde::{Deserialize, Serialize};

use ethers_core::types::{Address, Bytes, U64};

use crate::FeeToken;

/// A Relay Request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayRequest {
    /// The address of the contract to be called
    #[serde(serialize_with = "crate::ser::serialize_checksum_addr")]
    pub dest: Address,
    /// The calldata
    pub data: Bytes,
    /// The fee token
    pub token: FeeToken,
    /// The amount of fee
    #[serde(with = "crate::ser::decimal_u64_ser")]
    pub relayer_fee: U64,
}
