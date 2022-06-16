use ethers_core::types::{Address, Bytes, U64};
use serde::{Deserialize, Serialize};

use crate::FeeToken;

/// A Gelato ForwardCall
///
/// <https://docs.gelato.network/developer-products/gelato-relay-sdk/request-types#forwardcall>
///
/// `ForwardCall` is designed to handle payments of type `Synchronous`, as it
/// requires no signatures.
///
/// Because payment is of type `Synchronous`, the target contract MUST
/// pay for its gas in `params.fee_token` during call forwarding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForwardCall {
    /// Chain ID
    pub chain_id: u64,
    /// The contract to call
    #[serde(serialize_with = "crate::ser::serialize_checksum_addr")]
    pub target: Address,
    /// The payload to pass to that contrct
    pub data: Bytes,
    /// The token in which fees will be paid
    pub fee_token: FeeToken,
    /// The gas limit for execution
    #[serde(with = "crate::ser::decimal_u64_ser")]
    pub gas: U64,
}
