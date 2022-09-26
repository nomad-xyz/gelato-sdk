use ethers_core::types::Address;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Magic value used to specify the chain-native token
static NATIVE_TOKEN: Lazy<FeeToken> = Lazy::new(|| {
    FeeToken(
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
            .parse()
            .unwrap(),
    )
});

/// Gelato payment type
///
/// <https://docs.gelato.network/developer-products/gelato-relay-sdk/payment-types>
#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum PaymentType {
    /// The target smart contract will pay Gelato Relay's smart contract as the
    /// call is forwarded. Payment can be done in feeToken, where it is
    /// expected to be a whitelisted payment token.
    Synchronous = 0,
    /// The sponsor must hold a balance in one of Gelato's Gas Tank smart
    /// contracts. The balance could even be held on a different chainId than
    /// the one the transaction is being relayed on (as defined by
    /// sponsorChainId).
    ///
    /// An event is emitted to tell Gelato how much to charge in the future,
    /// which shall be acknowledged in an off-chain accounting system. A
    /// sponsor signature is expected in order to ensure that the sponsor
    /// agrees on being charged up to a maxFee amount
    AsyncGasTank = 1,
    /// Similar to Type 1, but sponsor is expected to hold a balance with
    /// Gelato on the same chainId where the transaction is executed. Fee
    /// deduction happens during the transaction. A sponsor signature is
    /// expected in order to ensure that the sponsor agrees on being charged up
    /// to a maxFee amount.
    SyncGasTank = 2,
    /// In this scenario a sponsor pre-approves the appropriate Gelato Relay's
    /// smart contract to spend tokens up so some maximum allowance value.
    /// During execution of the transaction, Gelato will credit due fees using
    /// `IERC20(feeToken).transferFrom(...)` in order to pull fees from his/her
    /// account. A sponsor signature is expected in order to ensure that the
    /// sponsor agrees on being charged up to a maxFee amount.
    SyncPullFee = 3,
}

/// A gelato fee token is an ERC20 address, which defaults to `0xee..ee`. This
/// magic value indicates "eth" or the native asset of the chain. This FeeToken
/// must be allowlisted by Gelato validators
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeToken(#[serde(serialize_with = "crate::ser::serialize_checksum_addr")] Address);

impl std::ops::Deref for FeeToken {
    type Target = Address;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::str::FromStr for FeeToken {
    type Err = <Address as std::str::FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(s.parse()?))
    }
}

impl Default for FeeToken {
    fn default() -> Self {
        *NATIVE_TOKEN
    }
}

impl From<Address> for FeeToken {
    fn from(token: Address) -> Self {
        Self(token)
    }
}
