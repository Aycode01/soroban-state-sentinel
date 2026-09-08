//! The workspace root package exists to host the repo-level integration tests in
//! `tests/` (`cargo test --workspace` runs them). It contains no runtime code —
//! the integration tests spawn the compiled `soroban-state-sentinel` binary and
//! drive it against a local mock RPC server.