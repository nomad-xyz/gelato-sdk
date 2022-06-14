use serde::{Deserialize, Serialize};

/// Response to Relay chains request. Contains a list of chain ids supported
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RelayChainsResponse {
    /// The supported chain ids
    relays: Vec<String>,
}

impl RelayChainsResponse {
    pub(crate) fn relays_iter(&self) -> impl Iterator<Item = u64> + '_ {
        self.relays.iter().map(|s| s.parse().unwrap())
    }

    pub(crate) fn relays(&self) -> Vec<u64> {
        self.relays_iter().collect()
    }
}
