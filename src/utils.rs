use std::collections::HashMap;

use ethers_core::types::Address;
use once_cell::sync::Lazy;

pub static CHAIN_ID_TO_FORWARDER: Lazy<HashMap<u64, Address>> = Lazy::new(|| {
    HashMap::from([
        // Ethereum
        (
            1,
            "0x5ca448e53e77499222741DcB6B3c959Fa829dAf2"
                .parse()
                .expect("!forwarder proxy"),
        ),
        // Kovan
        (
            42,
            "0x4F36f93F58d36DcbC1E60b9bdBE213482285C482"
                .parse()
                .expect("!forwarder proxy"),
        ),
        // Goerli
        (
            5,
            "0x61BF11e6641C289d4DA1D59dC3E03E15D2BA971c"
                .parse()
                .expect("!forwarder proxy"),
        ),
        // Rinkeby
        (
            4,
            "0x9B79b798563e538cc326D03696B3Be38b971D282"
                .parse()
                .expect("!forwarder proxy"),
        ),
        // Evmos
        (
            9001,
            "0x9561aCdf04C2B639dFfeCB357438e7B3eD979C5C"
                .parse()
                .expect("!forwarder proxy"),
        ),
        // BSC
        (
            56,
            "0xeeea839E2435873adA11d5dD4CAE6032742C0445"
                .parse()
                .expect("!forwarder proxy"),
        ),
        // Polygon
        (
            137,
            "0xc2336e796F77E4E57b6630b6dEdb01f5EE82383e"
                .parse()
                .expect("!forwarder proxy"),
        ),
    ])
});

/// Get the forwarder for a chain id
pub fn get_forwarder(chain_id: u64) -> Option<Address> {
    CHAIN_ID_TO_FORWARDER.get(&chain_id).copied()
}

/// Todo: Populate
pub static CHAIN_ID_TO_META_BOX: Lazy<HashMap<u64, Address>> = Lazy::new(Default::default);

/// Get the metabox for a chain id
pub fn get_meta_box(chain_id: u64) -> Option<Address> {
    CHAIN_ID_TO_META_BOX.get(&chain_id).copied()
}
