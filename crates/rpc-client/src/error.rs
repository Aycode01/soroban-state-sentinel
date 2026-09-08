//! Typed errors for the Soroban RPC client.
//!
//! The three failure classes required by the sentinel design are kept distinct so
//! callers can react appropriately:
//!
//! - [`RpcError::Transport`] — the network itself failed (DNS, connect, timeout,
//!   TLS, …). Retryable.
//! - [`RpcError::Malformed`] — we got bytes back but they were not a valid
//!   JSON-RPC response or not valid XDR. Not retryable without fixing the request.
//! - [`RpcError::Rpc`] — the RPC server answered with a JSON-RPC `error` object.
//! - [`RpcError::EntryNotFound`] — a convenience wrapper for "the specific key you
//!   asked for via [`crate::RpcClient::get_entry`] was not in the response".
//!   `getLedgerEntries` itself never errors for missing keys; it just omits them.

use std::fmt;

/// Errors produced by the Soroban RPC client.
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    /// The HTTP transport failed (connect, DNS, timeout, TLS, …).
    #[error("transport error talking to Soroban RPC at {url}: {source}")]
    Transport {
        /// The RPC endpoint that failed.
        url: String,
        /// The underlying transport error.
        #[source]
        source: reqwest::Error,
    },

    /// The response was not a valid JSON-RPC payload, or contained XDR that could
    /// not be decoded.
    #[error("malformed response from Soroban RPC: {0}")]
    Malformed(String),

    /// The RPC server returned a JSON-RPC `error` object.
    #[error("RPC server error {code}: {message}")]
    Rpc {
        /// JSON-RPC error code.
        code: i64,
        /// Human-readable error message from the server.
        message: String,
    },

    /// The specific key requested was not present in the response.
    #[error("entry not found: {0}")]
    EntryNotFound(String),

    /// A required piece of live network configuration is missing or unreadable.
    #[error("network configuration error: {0}")]
    NetworkConfig(String),
}

/// Small helper for turning a `serde_json::Error` into [`RpcError::Malformed`]
/// with context.
pub(crate) fn malformed<E: fmt::Display>(context: &str, err: E) -> RpcError {
    RpcError::Malformed(format!("{context}: {err}"))
}
