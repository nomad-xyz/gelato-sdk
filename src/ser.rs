use ethers_core::types::Signature;
use serde::{Deserialize, Serialize};

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

impl Serialize for RsvSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("0x{}", self.0))
    }
}

#[cfg(test)]
mod test {
    use ethers_signers::{LocalWallet, Signer};

    use super::*;

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
