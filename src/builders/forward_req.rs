use ethers_core::types::{Address, Bytes, NameOrAddress, U64, transaction::eip2718::TypedTransaction, TransactionRequest};

use crate::{
    rpc::{ForwardRequest, SignedForwardRequest},
    FeeToken, PaymentType,
};

/// Builder for a [`ForwardRequest`]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ForwardRequestBuilder {
    /// Chain id. Defaults to 1 (ethereum).
    pub chain_id: Option<u64>,
    /// Address of dApp's smart contract to call. Required
    pub target: Option<Address>,
    /// Payload for `target`. Defaults to empty bytes: `0x`
    pub data: Option<Bytes>,
    /// paymentToken for Gelato Executors. Defaults to chain-native asset (eth)
    pub fee_token: Option<FeeToken>,
    /// Type identifier for Gelato's payment. Can be 1, 2 or 3.
    /// Defaults to 1: `AsyncGasTank`
    pub payment_type: Option<PaymentType>, // 1 = gas tank
    /// Maximum fee sponsor is willing to pay Gelato Executors. Required
    pub max_fee: Option<U64>,
    /// Gas limit
    pub gas: Option<U64>,
    /// EOA address that pays Gelato Executors.
    pub sponsor: Option<Address>,
    /// Chain ID of where sponsor holds a Gas Tank balance with Gelato
    /// Usually the same as `chain_id`
    /// relevant for payment type 1: `AsyncGasTank`
    pub sponsor_chain_id: Option<u64>,
    /// Smart contract nonce for sponsor to sign.
    /// Can be 0 if enforceSponsorNonce is always false.
    pub nonce: Option<usize>,
    /// Whether or not to enforce replay protection using sponsor's nonce.
    /// Defaults to false, as repla
    pub enforce_sponsor_nonce: Option<bool>,
    /// Whether or not ordering matters for concurrently submitted transactions.
    /// Defaults to `true` if not provided.
    pub enforce_sponsor_nonce_ordering: Option<bool>,
}

impl From<&TransactionRequest> for ForwardRequestBuilder {
    fn from(tx: &TransactionRequest) -> Self {
        let mut builder = ForwardRequestBuilder::default();

        if let Some(NameOrAddress::Address(target)) = tx.to {
            builder = builder.target(target);
        }
        if let Some(gas) = tx.gas {
            builder = builder.gas(gas.as_u64());
        }
        if let Some(data) = &tx.data {
            builder = builder.data(data.clone());
        }
        if let Some(nonce) = tx.nonce {
            builder = builder.nonce(nonce.as_usize());
        }
        if let Some(from) = tx.from {
            builder = builder.sponsor_address(from);
        }

        builder
    }
}

impl From<&TypedTransaction> for ForwardRequestBuilder {
    fn from(tx: &TypedTransaction) -> Self {
        let mut builder = ForwardRequestBuilder::default();

        if let Some(NameOrAddress::Address(target)) = tx.to() {
            builder = builder.target(*target);
        }
        if let Some(gas) = tx.gas() {
            builder = builder.gas(gas.as_u64());
        }
        if let Some(data) = tx.data() {
            builder = builder.data(data.clone());
        }
        if let Some(nonce) = tx.nonce() {
            builder = builder.nonce(nonce.as_usize());
        }
        if let Some(from) = tx.from() {
            builder = builder.sponsor_address(*from);
        }

        builder    }
}

impl ForwardRequestBuilder {
    /// Set `chain_id`. Defaults to 1 (ethereum)
    pub fn chain_id(mut self, val: u64) -> Self {
        self.chain_id = Some(val);
        self
    }

    /// Set `target`. Required.
    pub fn target(mut self, val: Address) -> Self {
        self.target = Some(val);
        self
    }

    /// Set `data`. Defaults to empty bytes: `0x`
    pub fn data(mut self, val: Bytes) -> Self {
        self.data = Some(val);
        self
    }

    /// Set `fee_token`. Defaults to chain-native asset (eth)
    pub fn fee_token(mut self, val: impl Into<FeeToken>) -> Self {
        self.fee_token = Some(val.into());
        self
    }

    /// Set `payment_type`. Defaults to 1: `AsyncGasTank`
    pub fn payment_type(mut self, val: impl Into<PaymentType>) -> Self {
        self.payment_type = Some(val.into());
        self
    }

    /// Set `max_fee`. Required
    pub fn max_fee(mut self, val: impl Into<U64>) -> Self {
        self.max_fee = Some(val.into());
        self
    }

    /// Set `gas`. Required
    pub fn gas(mut self, val: impl Into<U64>) -> Self {
        self.gas = Some(val.into());
        self
    }

    /// Set the sponsor address. Note that this will be overridden if
    /// `sponsored_by` is also called. Required.
    pub fn sponsor_address(mut self, sponsor: Address) -> Self {
        self.sponsor = Some(sponsor);
        self
    }

    /// Sponsor the request with a specific signer. Note taht this will
    /// override the existing sponsor address with that of the signer. Required
    pub fn sponsored_by<S>(mut self, sponsor: &S) -> SponsoredForwardRequestBuilder<S>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        self.sponsor = Some(sponsor.address());
        self.chain_id = Some(sponsor.chain_id());
        SponsoredForwardRequestBuilder {
            builder: self,
            sponsor,
        }
    }

    /// Set `sponsor_chain_id`. Defaults to 1 (ethereum)
    pub fn sponsor_chain_id(mut self, val: u64) -> Self {
        self.sponsor_chain_id = Some(val);
        self
    }

    /// Set `nonce`. Required
    pub fn nonce(mut self, val: usize) -> Self {
        self.nonce = Some(val);
        self
    }

    /// Set `enforce_sponsor_nonce`. Defaults to `true`
    pub fn enforce_sponsor_nonce(mut self, val: bool) -> Self {
        self.enforce_sponsor_nonce = Some(val);
        self
    }

    /// Set `enforce_sponsor_nonce_ordering`. Defaults to `false` if not provided
    pub fn enforce_sponsor_nonce_ordering(mut self, val: bool) -> Self {
        self.enforce_sponsor_nonce_ordering = Some(val);
        self
    }

    /// Build this request
    pub fn build(self) -> eyre::Result<ForwardRequest> {
        let mut missing = vec![];
        if self.target.is_none() { missing.push("target"); }
        if self.max_fee.is_none() { missing.push("max_fee"); }
        if self.gas.is_none() { missing.push("gas"); }
        if self.sponsor.is_none() { missing.push("sponsor"); }
        if self.nonce.is_none() { missing.push("nonce"); }

        eyre::ensure!(
            missing.is_empty(),
            "Missing required values in build: {}",
            missing.join(", ")
        );

        Ok(ForwardRequest {
            chain_id: self.chain_id.unwrap_or(1),
            target: self.target.unwrap(),
            data: self.data.unwrap_or_default(),
            fee_token: self.fee_token.unwrap_or_default(),
            payment_type: self.payment_type.unwrap_or(PaymentType::AsyncGasTank),
            max_fee: self.max_fee.unwrap(),
            gas: self.gas.unwrap(),
            sponsor: self.sponsor.unwrap(),
            sponsor_chain_id: self.sponsor_chain_id.unwrap_or(1),
            nonce: self.nonce.unwrap(),
            enforce_sponsor_nonce: self.enforce_sponsor_nonce.unwrap_or(true),
            enforce_sponsor_nonce_ordering: self.enforce_sponsor_nonce_ordering,
        })

    }
}

/// Builder for a [`SignedForwardRequest`]
pub struct SponsoredForwardRequestBuilder<'a, S> {
    builder: ForwardRequestBuilder,
    sponsor: &'a S,
}

impl<'a, S> SponsoredForwardRequestBuilder<'a, S>
where
    S: ethers_signers::Signer,
    S::Error: 'static,
{
    /// Set `chain_id`. Defaults to 1 (ethereum)
    pub fn chain_id(mut self, val: u64) -> Self {
        self.builder.chain_id = Some(val);
        self
    }

    /// Set `target`. Required.
    pub fn target(mut self, val: Address) -> Self {
        self.builder.target = Some(val);
        self
    }

    /// Set `data`. Defaults to empty bytes: `0x`
    pub fn data(mut self, val: Bytes) -> Self {
        self.builder.data = Some(val);
        self
    }

    /// Set `fee_token`. Defaults to chain-native asset (eth)
    pub fn fee_token(mut self, val: impl Into<FeeToken>) -> Self {
        self.builder.fee_token = Some(val.into());
        self
    }

    /// Set `payment_type`. Defaults to 1: `AsyncGasTank`
    pub fn payment_type(mut self, val: impl Into<PaymentType>) -> Self {
        self.builder.payment_type = Some(val.into());
        self
    }

    /// Set `max_fee`. Required
    pub fn max_fee(mut self, val: impl Into<U64>) -> Self {
        self.builder.max_fee = Some(val.into());
        self
    }

    /// Set `gas`. Required
    pub fn gas(mut self, val: impl Into<U64>) -> Self {
        self.builder.gas = Some(val.into());
        self
    }

    /// Set `sponsor_address` unsetting the existing sponsor signer
    pub fn sponsor_address(mut self, address: Address) -> ForwardRequestBuilder {
        self.builder.sponsor = Some(address);
        self.builder
    }

    /// Sponsor the request with a specific signer
    pub fn sponsored_by<T>(self, sponsor: &T) -> SponsoredForwardRequestBuilder<T>
    where
        T: ethers_signers::Signer,
        T::Error: 'static,
    {
        SponsoredForwardRequestBuilder {
            builder: self.builder,
            sponsor,
        }
    }

    /// Set `sponsor_chain_id`. Defaults to 1 (ethereum)
    pub fn sponsor_chain_id(mut self, val: u64) -> Self {
        self.builder.sponsor_chain_id = Some(val);
        self
    }

    /// Set `nonce`. Required
    pub fn nonce(mut self, val: usize) -> Self {
        self.builder.nonce = Some(val);
        self
    }

    /// Set `enforce_sponsor_nonce`. Defaults to `true`
    pub fn enforce_sponsor_nonce(mut self, val: bool) -> Self {
        self.builder.enforce_sponsor_nonce = Some(val);
        self
    }

    /// Set `enforce_sponsor_nonce_ordering`. Defaults to `false` if not provided
    pub fn enforce_sponsor_nonce_ordering(mut self, val: bool) -> Self {
        self.builder.enforce_sponsor_nonce_ordering = Some(val);
        self
    }

    /// Build this request
    pub async fn build(self) -> eyre::Result<SignedForwardRequest> {
        Ok(self.builder.build()?.sponsor(self.sponsor).await?)
    }
}
