use ethers_core::types::{Signature, H160};
use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Wrapper around a signature that ensures it serializes/deserializes
/// as a 0x-prepended hex representation of RSV
pub(crate) struct RsvSignature(Signature);

impl std::fmt::Display for RsvSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::ops::Deref for RsvSignature {
    type Target = Signature;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Signature> for RsvSignature {
    fn from(s: Signature) -> Self {
        Self(s)
    }
}

impl From<RsvSignature> for Signature {
    fn from(s: RsvSignature) -> Self {
        s.0
    }
}

impl Serialize for RsvSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("0x{}", self.0))
    }
}

impl<'de> Deserialize<'de> for RsvSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        s.parse()
            .map(RsvSignature)
            .map_err(serde::de::Error::custom)
    }
}

pub(crate) fn serialize_checksum_addr<S>(val: &H160, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&ethers_core::utils::to_checksum(val, None))
}

pub(crate) mod decimal_u64_ser {
    use ethers_core::types::U64;
    use serde::{Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(val: &U64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&val.to_string())
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<U64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        U64::from_dec_str(&s).map_err(serde::de::Error::custom)
    }
}

pub(crate) mod json_u256_ser {
    use ethers_core::types::U256;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Debug, Copy, Clone, Serialize, Deserialize)]
    struct JsonU256<'a> {
        hex: U256,
        #[serde(rename = "type")]
        t: &'a str,
    }

    pub(crate) fn serialize<S>(val: &U256, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (JsonU256 {
            hex: *val,
            t: "BigNumber",
        })
        .serialize(serializer)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        JsonU256::<'de>::deserialize(deserializer).map(|val| val.hex)
    }
}

#[cfg(test)]
mod test {
    use ethers::prelude::U64;
    use ethers_signers::{LocalWallet, Signer};

    use super::*;

    #[test]
    fn u64_ser() {
        #[derive(Serialize, Deserialize, Debug)]
        struct TestU64(#[serde(with = "super::decimal_u64_ser")] U64);

        assert_eq!(
            "382345198",
            ethers_core::types::U64::from(382345198).to_string()
        );
    }

    #[tokio::test]
    async fn sig_serialization() {
        let signer: LocalWallet = "11".repeat(32).parse().unwrap();
        let signature: RsvSignature = signer.sign_message(Vec::new()).await.unwrap().into();

        let hex_sig = format!("0x{}", signature);
        assert_eq!(
            serde_json::to_value(&signature).unwrap(),
            serde_json::Value::String(hex_sig),
        )
    }
}
