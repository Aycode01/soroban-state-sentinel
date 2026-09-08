#![allow(clippy::result_large_err)] // error enums wrap large source errors (reqwest::Error); acceptable for a CLI

//! # sentinel-rpc-client
//!
//! A minimal, typed JSON-RPC client for [Soroban RPC](https://developers.stellar.org/docs/data/apis/rpc).
//!
//! Implements the read-only methods the sentinel needs:
//!
//! - [`RpcClient::get_ledger_entries`] — batched `getLedgerEntries` (max 200 keys
//!   per request, per the RPC API reference), returning typed XDR values.
//! - [`RpcClient::get_latest_ledger`] — `getLatestLedger` (sequence, protocol version,
//!   close time).
//! - [`RpcClient::get_network`] — `getNetwork` (network passphrase, protocol version).
//! - [`RpcClient::fetch_network_config`] — live network configuration (fee rates,
//!   rent denominators, TTL bounds, resource limits) read from the `CONFIG_SETTING`
//!   ledger entries.
//!
//! No signing, no private-key handling, no transaction submission — this crate is
//! read-only by construction.
//!
//! ## Verification notes
//!
//! - `getLedgerEntries` accepts at most **200 ledger keys per request**
//!   (verified against the Soroban RPC API reference, protocol 28 era). The client
//!   batches larger key sets transparently.
//! - RPC params must be passed *by name* (a JSON object), not positionally; the
//!   testnet RPC rejects positional arrays with `-32602 invalid parameters`.

pub mod client;
pub mod error;
pub mod fee_rates;
pub mod network_config;
pub mod types;

pub use client::RpcClient;
pub use error::RpcError;
pub use fee_rates::FeeRates;
pub use network_config::NetworkConfig;
pub use types::{GetLedgerEntriesResponse, LatestLedger, LedgerEntryInfo, NetworkInfo};
