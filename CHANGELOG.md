# Changelog

# Unreleased

- fix: serialize U64s as decimal strings
- feature: re-sponsor signed forward requests
- fix: export `get_forwarder` and `get_meta_box`
- feature: support MetaTxRequest (user-driven, sponsor-allowed)
- feature: support ForwardCall (type 0 payments)
- refactor: move all rpc requests into modules
- refactor: Rename the dispatch-ready forward req to `Signed___`.
- feat: Move the unsigned forward req into this crate
- refactor: Clean up typing
- refactor: Start moving towards `Address` over `H160`
- fix: use PaymentType type for payment types

# v0.1.0-alpha

- adds bindings getting estimated relayer fee
- adds bindings for sending relay txs, getting supported chains, and fetching task status

- an SDK :)
