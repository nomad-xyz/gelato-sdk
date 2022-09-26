use ethers_core::types::{
    transaction::eip2718::TypedTransaction, Address, Bytes, NameOrAddress, TransactionRequest, U64,
};

use crate::{
    rpc::{MetaTxRequest, SignedMetaTxRequest},
    FeeToken, PaymentType,
};

/// Builder for a [`MetaTxRequest`]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetaTxRequestBuilder {
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
    pub payment_type: Option<PaymentType>,
    /// Maximum fee sponsor is willing to pay Gelato Executors. Required
    pub max_fee: Option<U64>,
    /// Gas limit. Required
    pub gas: Option<U64>,
    /// EOA of dapp's user. Required
    pub user: Option<Address>,
    /// EOA address that pays Gelato Executors.
    /// Optional. User pays if unset.
    pub sponsor: Option<Address>,
    /// Chain ID of where sponsor holds a Gas Tank balance with Gelato
    /// Usually the same as `chain_id`
    /// relevant for payment type 1: `AsyncGasTank`
    /// Required. May be set automatically by the sponsor signer
    pub sponsor_chain_id: Option<u64>,
    /// Smart contract nonce for sponsor to sign.
    pub nonce: Option<usize>,
    /// Deadline for executing this MetaTxRequest. If set to 0, no deadline is
    /// enforced
    pub deadline: Option<u64>,
}

impl From<&TransactionRequest> for MetaTxRequestBuilder {
    fn from(tx: &TransactionRequest) -> Self {
        let mut builder = MetaTxRequestBuilder::default();

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
            builder = builder.user_address(from);
        }

        builder
    }
}

impl From<&TypedTransaction> for MetaTxRequestBuilder {
    fn from(tx: &TypedTransaction) -> Self {
        let mut builder = MetaTxRequestBuilder::default();

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
            builder = builder.user_address(*from);
        }

        builder
    }
}

impl MetaTxRequestBuilder {
    /// Which keys need to be populated
    pub fn missing_keys(&self) -> Vec<&'static str> {
        let mut missing = vec![];
        if self.target.is_none() {
            missing.push("target");
        }
        if self.max_fee.is_none() {
            missing.push("max_fee");
        }
        if self.gas.is_none() {
            missing.push("gas");
        }
        if self.user.is_none() {
            missing.push("user");
        }
        if self.nonce.is_none() {
            missing.push("nonce");
        }
        missing
    }

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

    /// Set `user`. Required. May be set automatically by `user_signer`
    pub fn user_address(mut self, val: Address) -> Self {
        self.user = Some(val);
        self
    }

    /// Set a signer that will sign the request. Note that this will override
    /// the existing user with the address of that of the signer
    pub fn with_user<S>(mut self, user: &S) -> MetaTxRequestBuilderWithUser<S>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        self.user = Some(user.address());
        MetaTxRequestBuilderWithUser {
            builder: self,
            user,
        }
    }

    /// Set the sponsor address. Note that this will be overridden if
    /// `sponsored_by` is also called. Required.
    pub fn sponsor_address(mut self, sponsor: Address) -> Self {
        self.sponsor = Some(sponsor);
        self
    }

    /// Sponsor the request with a specific signer. Note that this will
    /// override the existing sponsor address with that of the signer
    pub fn sponsored_by<S>(mut self, sponsor: &S) -> MetaTxRequestBuilderWithSponsor<S>
    where
        S: ethers_signers::Signer,
        S::Error: 'static,
    {
        self.sponsor = Some(sponsor.address());
        self.chain_id = Some(sponsor.chain_id());
        MetaTxRequestBuilderWithSponsor {
            builder: self,
            sponsor,
        }
    }

    /// Set `sponsor_chain_id`. Defaults to 1 (ethereum) if a sponsor is set. May be set
    /// automatically if `sponsored_by` is called
    pub fn sponsor_chain_id(mut self, val: u64) -> Self {
        self.sponsor_chain_id = Some(val);
        self
    }

    /// Set `nonce`. Required
    pub fn nonce(mut self, val: usize) -> Self {
        self.nonce = Some(val);
        self
    }

    /// Set `deadline`. If set to 0, no deadline is
    /// enforced
    pub fn deadline(mut self, val: u64) -> Self {
        self.deadline = Some(val);
        self
    }
    /// Build this request
    pub fn build(self) -> eyre::Result<MetaTxRequest> {
        let missing = self.missing_keys();
        eyre::ensure!(
            missing.is_empty(),
            "Missing required values in build: {}",
            missing.join(", ")
        );

        // default value IF there's a sponsor set
        let sponsor_chain_id = self.sponsor.map(|_| self.sponsor_chain_id.unwrap_or(1));

        Ok(MetaTxRequest {
            chain_id: self.chain_id.unwrap_or(1),
            target: self.target.unwrap(),
            data: self.data.unwrap_or_default(),
            fee_token: self.fee_token.unwrap_or_default(),
            payment_type: self.payment_type.unwrap_or(PaymentType::AsyncGasTank),
            max_fee: self.max_fee.unwrap(),
            gas: self.gas.unwrap(),
            user: self.user.unwrap(),
            sponsor: self.sponsor,
            sponsor_chain_id,
            nonce: self.nonce.unwrap_or_default(),
            deadline: self.deadline,
        })
    }
}

/// Builder for a [`SignedMetaTxRequest`] with sponsor but no user yet set
pub struct MetaTxRequestBuilderWithSponsor<'a, S> {
    builder: MetaTxRequestBuilder,
    sponsor: &'a S,
}

impl<'a, S> MetaTxRequestBuilderWithSponsor<'a, S> {
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

    /// Set `user`. Required. May be set automatically by `user_signer`
    pub fn user_address(mut self, val: Address) -> Self {
        self.builder.user = Some(val);
        self
    }

    /// Set a signer that will sign the request. Note that this will override
    /// the existing user with the address of that of the signer
    pub fn with_user<'b, T>(
        mut self,
        user: &'b T,
    ) -> MetaTxRequestBuilderWithUserAndSponsor<'b, 'a, T, S>
    where
        T: ethers_signers::Signer,
        T::Error: 'static,
    {
        self.builder.user = Some(user.address());
        MetaTxRequestBuilderWithUserAndSponsor::<'b, 'a, T, S> {
            builder: self.builder,
            sponsor: self.sponsor,
            user,
        }
    }

    /// Set `sponsor_address` unsetting the existing sponsor signer
    pub fn sponsor_address(mut self, address: Address) -> MetaTxRequestBuilder {
        self.builder.sponsor = Some(address);
        self.builder
    }

    /// Sponsor the request with a specific signer. Note that this will
    /// override the existing sponsor address with that of the signer
    pub fn sponsored_by<T>(mut self, sponsor: &T) -> MetaTxRequestBuilderWithSponsor<T>
    where
        T: ethers_signers::Signer,
        T::Error: 'static,
    {
        self.builder.sponsor = Some(sponsor.address());
        MetaTxRequestBuilderWithSponsor {
            builder: self.builder,
            sponsor,
        }
    }

    /// Set `sponsor_chain_id`. Defaults to 1 (ethereum) if a sponsor is set
    pub fn sponsor_chain_id(mut self, val: u64) -> Self {
        self.builder.sponsor_chain_id = Some(val);
        self
    }

    /// Set `nonce`. Required
    pub fn nonce(mut self, val: usize) -> Self {
        self.builder.nonce = Some(val);
        self
    }

    /// Set `deadline`. If set to 0, no deadline is
    /// enforced
    pub fn deadline(mut self, val: u64) -> Self {
        self.builder.deadline = Some(val);
        self
    }

    /// Build this request
    pub fn build(self) -> eyre::Result<MetaTxRequest> {
        self.builder.build()
    }
}

/// Builder for a [`SignedMetaTxRequest`] with no sponsor
pub struct MetaTxRequestBuilderWithUser<'a, S> {
    builder: MetaTxRequestBuilder,
    user: &'a S,
}

impl<'a, S> MetaTxRequestBuilderWithUser<'a, S>
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

    /// Set `user_address`. Note that this will unset the existing signer
    pub fn user_address(mut self, val: Address) -> MetaTxRequestBuilder {
        self.builder.user = Some(val);
        self.builder
    }

    /// Set a signer that will sign the request. Note that this will override
    /// the existing user with the address of that of the signer
    pub fn with_user<T>(mut self, user: &T) -> MetaTxRequestBuilderWithUser<T>
    where
        T: ethers_signers::Signer,
        T::Error: 'static,
    {
        self.builder.user = Some(user.address());
        MetaTxRequestBuilderWithUser {
            builder: self.builder,
            user,
        }
    }

    /// Set `sponsor_address` unsetting the existing sponsor signer
    pub fn sponsor_address(mut self, address: Address) -> Self {
        self.builder.sponsor = Some(address);
        self
    }

    /// Sponsor the request with a specific signer. Note that this will
    /// override the existing sponsor address with that of the signer
    pub fn sponsored_by<'b, T>(
        mut self,
        sponsor: &'b T,
    ) -> MetaTxRequestBuilderWithUserAndSponsor<'a, 'b, S, T>
    where
        T: ethers_signers::Signer,
        T::Error: 'static,
    {
        self.builder.sponsor = Some(sponsor.address());
        MetaTxRequestBuilderWithUserAndSponsor {
            builder: self.builder,
            user: self.user,
            sponsor,
        }
    }

    /// Set `sponsor_chain_id`. Defaults to 1 (ethereum) if a sponsor is set
    pub fn sponsor_chain_id(mut self, val: u64) -> Self {
        self.builder.sponsor_chain_id = Some(val);
        self
    }

    /// Set `nonce`. Required
    pub fn nonce(mut self, val: usize) -> Self {
        self.builder.nonce = Some(val);
        self
    }

    /// Set `deadline`. If set to 0, no deadline is
    /// enforced
    pub fn deadline(mut self, val: u64) -> Self {
        self.builder.deadline = Some(val);
        self
    }

    /// Build this request
    pub async fn build(self) -> eyre::Result<SignedMetaTxRequest> {
        Ok(self.builder.build()?.sign(self.user).await?)
    }
}

/// Builder for a [`SignedMetaTxRequest`] with user and sponsor
pub struct MetaTxRequestBuilderWithUserAndSponsor<'a, 'b, S, T> {
    builder: MetaTxRequestBuilder,
    user: &'a S,
    sponsor: &'b T,
}

impl<'a, 'b, S, T> MetaTxRequestBuilderWithUserAndSponsor<'a, 'b, S, T>
where
    S: ethers_signers::Signer,
    S::Error: 'static,
    T: ethers_signers::Signer,
    T::Error: 'static,
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

    /// Set `user_address`. Note that this will unset the existing signer
    pub fn user_address(mut self, val: Address) -> MetaTxRequestBuilderWithSponsor<'b, T> {
        self.builder.user = Some(val);
        MetaTxRequestBuilderWithSponsor {
            builder: self.builder,
            sponsor: self.sponsor,
        }
    }

    /// Set a signer that will sign the request. Note that this will override
    /// the existing user with the address of that of the signer
    pub fn with_user<'c, U>(
        mut self,
        user: &'c U,
    ) -> MetaTxRequestBuilderWithUserAndSponsor<'c, 'b, U, T>
    where
        U: ethers_signers::Signer,
        U::Error: 'static,
    {
        self.builder.user = Some(user.address());
        MetaTxRequestBuilderWithUserAndSponsor::<'c, 'b, U, T> {
            builder: self.builder,
            user,
            sponsor: self.sponsor,
        }
    }

    /// Set `sponsor_address` unsetting the existing sponsor signer
    pub fn sponsor_address(mut self, address: Address) -> MetaTxRequestBuilderWithUser<'a, S> {
        self.builder.sponsor = Some(address);
        MetaTxRequestBuilderWithUser {
            builder: self.builder,
            user: self.user,
        }
    }

    /// Sponsor the request with a specific signer. Note that this will
    /// override the existing sponsor address with that of the signer
    pub fn sponsored_by<'c, U>(
        mut self,
        sponsor: &'c U,
    ) -> MetaTxRequestBuilderWithUserAndSponsor<'a, 'c, S, U>
    where
        U: ethers_signers::Signer,
        U::Error: 'static,
    {
        self.builder.sponsor = Some(sponsor.address());
        MetaTxRequestBuilderWithUserAndSponsor::<'a, 'c, S, U> {
            builder: self.builder,
            user: self.user,
            sponsor,
        }
    }

    /// Set `sponsor_chain_id`. Defaults to 1 (ethereum) if a sponsor is set
    pub fn sponsor_chain_id(mut self, val: u64) -> Self {
        self.builder.sponsor_chain_id = Some(val);
        self
    }

    /// Set `nonce`. Required
    pub fn nonce(mut self, val: usize) -> Self {
        self.builder.nonce = Some(val);
        self
    }

    /// Set `deadline`. If set to 0, no deadline is
    /// enforced
    pub fn deadline(mut self, val: u64) -> Self {
        self.builder.deadline = Some(val);
        self
    }

    /// Build this request
    pub async fn build(self) -> eyre::Result<SignedMetaTxRequest> {
        Ok(self
            .builder
            .build()?
            .sign_with_sponsor(self.user, self.sponsor)
            .await?)
    }
}
