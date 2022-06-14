use serde::{Deserialize, Serialize};

use ethers_core::types::U64;

use crate::FeeToken;

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

/// Response to estimated fee request. Contains the estimated fee
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EstimatedFeeResponse {
    /// The oracle-recommended fee, as a decimal string
    estimated_fee: String,
}

impl EstimatedFeeResponse {
    /// Return the estimated fee as a number
    pub(crate) fn estimated_fee(&self) -> U64 {
        U64::from_dec_str(&self.estimated_fee).unwrap()
    }
}
