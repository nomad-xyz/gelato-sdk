use ethers_core::types::{Address, Bytes, U64};
use serde::{Deserialize, Serialize};

use crate::FeeToken;

/// A Gelato ForwardCall
///
/// <https://docs.gelato.network/developer-products/gelato-relay-sdk/request-types#forwardcall>
///
/// `ForwardCall` is designed to handle payments of type 0, as it requires no
/// signatures. The target contract MUST implement payment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForwardCall {
    /// Chain ID
    pub chain_id: u64,
    /// The contract to call
    pub target: Address,
    /// The payload to pass to that contrct
    pub data: Bytes,
    /// The token in which fees will be paid
    pub fee_token: FeeToken,
    /// The gas limit for execution
    pub gas: U64,
}
