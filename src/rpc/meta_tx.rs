use ethers_core::{
    abi::{self, Token},
    types::{
        transaction::eip712::{EIP712Domain, Eip712},
        Address, Bytes, Signature, U64,
    },
    utils::keccak256,
};

use ethers_signers::Signer;
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
    /// Wrong Signer
    #[error(
        "Wrong signer. Expected {expected:?}. Attempted to sign with key belonging to: {actual:?}"
    )]
    WrongSigner {
        /// Sponsor in the struct
        expected: Address,
        /// Address belonging to the signer
        actual: Address,
    },
    /// Signer errored
    #[error("{0}")]
    SignerError(Box<dyn std::error::Error + Send + Sync + 'static>),
    /// InappropriatePaymentType
    #[error("Payment type Synchronous may not be used with this request")]
    InappropriatePaymentType,
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

impl MetaTxRequest {
    /// Fill MetaTxRequest with user & sponsor signatures and return signed
    /// request struct
    fn add_signatures(
        self,
        user_signature: Signature,
        sponsor_signature: Option<Signature>,
    ) -> SignedMetaTxRequest {
        SignedMetaTxRequest {
            type_id: "MetaTxRequest",
            req: self,
            user_signature: user_signature.into(),
            sponsor_signature: sponsor_signature.map(Into::into),
        }
    }

    async fn get_signature<S>(&self, signer: &S) -> Result<Signature, MetaTxRequestError>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        signer
            .sign_typed_data(self)
            .await
            .map_err(Box::new)
            .map_err(|e| MetaTxRequestError::SignerError(e))
    }

    /// Sign the request with the specified signer
    ///
    /// Errors if the signer does not match the user in the struct
    pub async fn user_sign<S>(&self, signer: &S) -> Result<Signature, MetaTxRequestError>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        let signer_addr = signer.address();
        if signer_addr != self.user {
            return Err(MetaTxRequestError::WrongSigner {
                expected: self.user,
                actual: signer_addr,
            });
        }
        if self.payment_type == PaymentType::Synchronous {
            return Err(MetaTxRequestError::InappropriatePaymentType);
        }

        self.get_signature(signer).await
    }

    /// Sponsor the request with the specified signer
    ///
    /// Overwrites sponsor if sponsor is None
    ///
    /// If this is called after `user_sign`, the tx may need to be re-signed by
    /// the user
    pub async fn sponsor_sign<S>(&mut self, sponsor: &S) -> Result<Signature, MetaTxRequestError>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        let signer_addr = sponsor.address();
        if self.sponsor.is_none() {
            self.sponsor = Some(signer_addr);
        }

        // unwraps are checked by setting it immediately above
        if signer_addr != self.sponsor.unwrap() {
            return Err(MetaTxRequestError::WrongSigner {
                expected: self.sponsor.unwrap(),
                actual: signer_addr,
            });
        }
        if self.payment_type == PaymentType::Synchronous {
            return Err(MetaTxRequestError::InappropriatePaymentType);
        }
        self.get_signature(sponsor).await
    }

    /// Sign the tx request with a user and (optionally) with a sponsor
    pub async fn sign<S, T>(
        mut self,
        user: &S,
        sponsor: Option<&T>,
    ) -> Result<SignedMetaTxRequest, MetaTxRequestError>
    where
        S: Signer,
        S::Error: 'static,
        T: Signer,
        T::Error: 'static,
    {
        let mut sponsor_signature = None;
        if let Some(sponsor) = sponsor {
            sponsor_signature = Some(self.sponsor_sign(sponsor).await?);
        }

        let user_signature = self.user_sign(user).await?;

        Ok(self.add_signatures(user_signature, sponsor_signature))
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
    /// Just guessing here :)
    type_id: &'static str,

    /// Metatx request
    #[serde(flatten)]
    req: MetaTxRequest,

    /// EIP-712 signature over the meta-tx request
    user_signature: RsvSignature,

    /// EIP-712 signature over the meta-tx request
    sponsor_signature: Option<RsvSignature>,
}

impl SignedMetaTxRequest {
    /// Get the attached sponsor signature (if any)
    pub fn sponsor_signature(&self) -> Option<Signature> {
        self.sponsor_signature.map(Into::into)
    }

    /// Get the attached user signature
    pub fn user_signature(&self) -> Signature {
        *self.user_signature
    }
}

impl std::ops::Deref for SignedMetaTxRequest {
    type Target = MetaTxRequest;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}
