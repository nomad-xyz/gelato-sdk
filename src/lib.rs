//! A Gelato relay SDK in rust

#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
#![forbid(where_clauses_object_safety)]

/// Gelato Types
pub mod types;
pub use types::*;

/// serialization convenience types
pub(crate) mod ser;
/// lib utils
pub(crate) mod utils;
pub use utils::{get_forwarder, get_meta_box};

mod client;
pub use client::*;

/// Forward Request
pub mod rpc;

/// Builders for complex request types
pub mod builders;
pub use builders::*;

/// Re-export reqwest for convenience
pub use reqwest;
