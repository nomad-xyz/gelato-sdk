use ethers_core::{
    abi::{self, Token},
    types::{
        transaction::eip712::{EIP712Domain, Eip712},
        Address, Bytes, Signature, U64,
    },
    utils::keccak256,
};

use serde::{Deserialize, Serialize};

use crate::{utils::get_forwarder, FeeToken, PaymentType};

const FORWARD_REQUEST_TYPE: &str = "ForwardRequest(uint256 chainId,address target,bytes data,address feeToken,uint256 paymentType,uint256 maxFee,uint256 gas,address sponsor,uint256 sponsorChainId,uint256 nonce,bool enforceSponsorNonce,bool enforceSponsorNonceOrdering)";

/// Unfilled Gelato forward request. This request is signed and filled according
/// to EIP-712 then sent to Gelato. Gelato executes the provided tx `data` on
/// the `target` contract address.
///
/// ForwardRequest is designed to handle payments of type , in cases
/// where all meta-transaction related logic (or other kinds of replay
/// protection mechanisms such as hash based commitments) is already
/// implemented inside target smart contract. The sponsor is still required to
/// EIP-712 sign this request, in order to ensure the integrity of payments.
/// Optionally, nonce may or may not be enforced, by setting
/// `enforceSponsorNonce`. Some dApps may not need to rely on a nonce for
/// ForwardRequest if they already implement strong forms of replay protection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForwardRequest {
    /// Chain id
    pub chain_id: usize,
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
    /// EOA address that pays Gelato Executors.
    pub sponsor: Address,
    /// Chain ID of where sponsor holds a Gas Tank balance with Gelato
    /// Usually the same as `
    pub sponsor_chain_id: usize,
    /// Smart contract nonce for sponsor to sign.
    /// Can be 0 if enforceSponsorNonce is always false.
    pub nonce: usize,
    /// Whether or not to enforce replay protection using sponsor's nonce.
    /// Defaults to false, as repla
    pub enforce_sponsor_nonce: bool,
    /// Whether or not ordering matters for concurrently submitted transactions.
    /// Defaults to `true` if not provided.
    pub enforce_sponsor_nonce_ordering: Option<bool>,
}

/// ForwardRequest error
#[derive(Debug, thiserror::Error)]
pub enum ForwardRequestError {
    /// Unknown forwarder
    #[error("Forwarder contract unknown for domain: {0}")]
    UnknownForwarderError(usize),
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

impl Eip712 for ForwardRequest {
    type Error = ForwardRequestError;

    fn domain(&self) -> Result<EIP712Domain, Self::Error> {
        let verifying_contract = get_forwarder(self.chain_id)
            .ok_or(ForwardRequestError::UnknownForwarderError(self.chain_id))?;

        Ok(EIP712Domain {
            name: "GelatoRelayForwarder".to_owned(),
            version: "V1".to_owned(),
            chain_id: self.chain_id.into(),
            verifying_contract,
            salt: None,
        })
    }

    fn type_hash() -> Result<[u8; 32], Self::Error> {
        Ok(keccak256(FORWARD_REQUEST_TYPE))
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
            Token::Address(self.sponsor),
            Token::Uint(self.sponsor_chain_id.into()),
            Token::Uint(self.nonce.into()),
            Token::Bool(self.enforce_sponsor_nonce),
            Token::Bool(self.enforce_sponsor_nonce_ordering.unwrap_or(true)),
        ]);

        Ok(keccak256(encoded_request))
    }
}

impl ForwardRequest {
    /// Fill ForwardRequest with sponsor signature and return full request struct
    pub fn add_signature(
        self,
        sponsor_signature: ethers_core::types::Signature,
    ) -> SignedForwardRequest {
        SignedForwardRequest {
            type_id: "ForwardRequest",
            req: self,
            sponsor_signature,
        }
    }

    /// Sign the request with the specified signer
    pub async fn sign<S>(&self, signer: S) -> Result<SignedForwardRequest, ForwardRequestError>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        let signer_addr = signer.address();
        if signer_addr != self.sponsor {
            return Err(ForwardRequestError::WrongSigner {
                expected: self.sponsor,
                actual: signer_addr,
            });
        }
        if self.payment_type == PaymentType::Synchronous {
            return Err(ForwardRequestError::InappropriatePaymentType);
        }

        let signature = signer
            .sign_typed_data(self)
            .await
            .map_err(Box::new)
            .map_err(|e| ForwardRequestError::SignerError(e))?;
        Ok(self.clone().add_signature(signature))
    }
}

/// Request for forwarding tx to gas-tank based relay service. Signed and ready
/// for dispatch
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SignedForwardRequest {
    /// must be exactly "ForwardRequest"
    type_id: &'static str,

    /// Forward Request Details
    #[serde(flatten)]
    req: ForwardRequest,

    /// EIP-712 signature over the forward request
    pub sponsor_signature: Signature,
}

impl std::ops::Deref for SignedForwardRequest {
    type Target = ForwardRequest;

    fn deref(&self) -> &Self::Target {
        &self.req
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ethers::signers::LocalWallet;
    use ethers::signers::Signer;
    use ethers::types::transaction::eip712::Eip712;
    use once_cell::sync::Lazy;

    const DOMAIN_SEPARATOR: &str =
        "0x1b927f522830945610cf8f0521ef8b3f69352936e1b0920968dcad9cf1e30762";
    const DUMMY_SPONSOR_KEY: &str =
        "9cb3a530d61728e337290409d967db069f5219279f89e5ddb5ae4af76a8da5f4";
    const DUMMY_SPONSOR_ADDRESS: &str = "0x4e4f0d95bc1a4275b748a63221796080b1aa5c10";
    const SPONSOR_SIGNATURE: &str = "0x23c272c0cba2b897de0fd8fe87d419f0f273c82ef10917520b733da889688b1c6fec89412c6f121fccbc30ce89b20a3de2f405018f1ac1249b9ff705fdb62a521b";

    static REQUEST: Lazy<ForwardRequest> = Lazy::new(|| ForwardRequest {
        chain_id: 42,
        target: "0x61bBe925A5D646cE074369A6335e5095Ea7abB7A"
            .parse()
            .unwrap(),
        data: "4b327067000000000000000000000000eeeeeeeeeeeeeeeeeeeeeeeeaeeeeeeeeeeeeeeeee"
            .parse()
            .unwrap(),
        fee_token: "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE"
            .parse()
            .unwrap(),
        payment_type: PaymentType::AsyncGasTank,
        max_fee: 10000000000000000000u64.into(),
        gas: 200000u64.into(),
        sponsor: DUMMY_SPONSOR_ADDRESS.parse().unwrap(),
        sponsor_chain_id: 42,
        nonce: 0,
        enforce_sponsor_nonce: false,
        enforce_sponsor_nonce_ordering: Some(false),
    });

    #[test]
    fn it_computes_domain_separator() {
        let domain_separator = (&*REQUEST).domain_separator().unwrap();

        assert_eq!(
            format!("0x{}", hex::encode(domain_separator)),
            DOMAIN_SEPARATOR,
        );
    }

    #[tokio::test]
    async fn it_computes_and_signs_digest() {
        let sponsor: LocalWallet = DUMMY_SPONSOR_KEY.parse().unwrap();
        assert_eq!(DUMMY_SPONSOR_ADDRESS, format!("{:#x}", sponsor.address()));

        let signature = sponsor.sign_typed_data(&*REQUEST).await.unwrap().to_vec();

        let hex_sig = format!("0x{}", hex::encode(signature));
        assert_eq!(SPONSOR_SIGNATURE, hex_sig);
    }
}
