use ethers_core::{
    abi::{self, Token},
    types::{
        transaction::eip712::{EIP712Domain, Eip712},
        Address, Bytes, U64,
    },
    utils::keccak256,
};

use serde::{Deserialize, Serialize};

use crate::{ser::RsvSignature, utils::get_meta_box, FeeToken, PaymentType};

const META_TX_TYPE: &str = "MetaTxRequest(uint256 chainId,address target,bytes data,address feeToken,uint256 paymentType,uint256 maxFee,uint256 gas,address user,address sponsor,uint256 sponsorChainId,uint256 nonce,uint256 deadline)";

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MetaTxRequest {
    /// Chain id
    pub chain_id: u64,
    /// Address of dApp's smart contract to call.
    pub target: Address,
    /// Payload for `target`.
    pub data: Bytes,
    /// paymentToken for Gelato Executors
    pub fee_token: FeeToken,
    ///Type identifier for Gelato's payment. Can be 1, 2 or 3.
    pub payment_type: PaymentType, // 1 = gas tank
    /// Maximum fee sponsor is willing to pay Gelato Executors
    pub max_fee: U64,
    /// Gas limit
    pub gas: U64,
    /// EOA of dapp's user
    pub user: Address,
    /// EOA address that pays Gelato Executors.
    pub sponsor: Option<Address>,
    /// Chain ID of where sponsor holds a Gas Tank balance with Gelato
    /// Usually the same as `chain_id`
    /// relevant for payment type 1: AsyncGasTank`
    pub sponsor_chain_id: Option<u64>,
    /// Smart contract nonce for sponsor to sign.
    /// Can be 0 if enforceSponsorNonce is always false.
    pub nonce: usize,
    /// Deadline for executing this MetaTxRequest. If set to 0, no deadline is
    /// enforced
    pub deadline: Option<u64>,
}

/// MetaTxRequest error
#[derive(Debug, thiserror::Error)]
pub enum MetaTxRequestError {
    /// Unknown metabox
    #[error("MetaBox contract unknown for chain id: {0}")]
    UnknownMetaBox(u64),
}

impl Eip712 for MetaTxRequest {
    type Error = MetaTxRequestError;

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        let verifying_contract =
            get_meta_box(self.chain_id).ok_or(MetaTxRequestError::UnknownMetaBox(self.chain_id))?;

        Ok(EIP712Domain {
            name: "GelatoMetaBox".to_owned(),
            version: "V1".to_owned(),
            chain_id: self.chain_id.into(),
            verifying_contract,
            salt: None,
        })
    }

    fn type_hash() -> Result<[u8; 32], Self::Error> {
        Ok(keccak256(META_TX_TYPE))
    }

    fn struct_hash(&self) -> Result<[u8; 32], Self::Error> {
        let encoded_request = abi::encode(&[
            Token::FixedBytes(Self::type_hash()?.to_vec()),
            Token::Uint(self.chain_id.into()),
            Token::Address(self.target),
            Token::FixedBytes(keccak256(&self.data).to_vec()),
            Token::Address(*self.fee_token),
            Token::Uint((self.payment_type as u8).into()),
            Token::Uint(self.max_fee.as_u64().into()),
            Token::Uint(self.gas.as_u64().into()),
            Token::Address(self.user),
            Token::Address(self.sponsor.unwrap_or_default()),
            Token::Uint(self.sponsor_chain_id.unwrap_or_default().into()),
            Token::Uint(self.nonce.into()),
            Token::Uint(self.deadline.unwrap_or_default().into()),
        ]);
        Ok(keccak256(encoded_request))
    }
}

/// Signed Gelato relay MetaTxRequest
///
/// <https://docs.gelato.network/developer-products/gelato-relay-sdk/request-types#metatxrequest>
///
/// MetaTxRequest is designed to handle payments of type AsyncGasTank,
/// SyncGasTank and SyncPullFee, in cases where the target contract does not
/// have any meta-transaction nor replay protection logic. In this case, the
/// appropriate Gelato Relay's smart contract already verifies user and sponsor
/// signatures. user is the EOA address that wants to interact with the dApp,
/// while sponsor is the account that pays fees.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignedMetaTxRequest {
    /// Metatx request
    #[serde(flatten)]
    req: MetaTxRequest,

    /// EIP-712 signature over the meta-tx request
    user_signature: RsvSignature,

    /// EIP-712 signature over the meta-tx request
    sponsor_signature: Option<RsvSignature>,
}
