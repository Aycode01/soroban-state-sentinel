//! The Soroban RPC JSON-RPC/HTTP client.
//!
//! Protocol notes (verified against the Soroban RPC API reference and the live
//! testnet RPC, protocol 28):
//!
//! - Requests are `POST` with a `Content-Type: application/json` body.
//! - Parameters are passed **by name** (JSON object), never positionally.
//! - `getLedgerEntries` accepts at most **200 keys per request**; this client
//!   batches larger sets.
//! - Missing keys are silently omitted from `getLedgerEntries` responses.
//! - Ledger keys are base64-encoded XDR (`LedgerKey`); entry values are the
//!   base64-encoded `LedgerEntryData` (the RPC does not include the wrapping
//!   `LedgerEntry` header).

use std::time::Duration;
use stellar_xdr::{LedgerEntryData, LedgerKey, Limits, ReadXdr, WriteXdr};

use crate::error::{malformed, RpcError};
use crate::types::{GetLedgerEntriesResponse, LatestLedger, LedgerEntryInfo, NetworkInfo};

/// Maximum number of ledger keys accepted by a single `getLedgerEntries` request.
///
/// Verified against the Soroban RPC API reference ("The maximum number of ledger
/// keys accepted is 200"), protocol 28 era, September 2026. This is an RPC
/// transport limit, not a protocol constant, so it lives here rather than in the
/// network configuration.
pub const MAX_KEYS_PER_REQUEST: usize = 200;

/// Default request timeout for a single RPC call.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// A JSON-RPC 2.0 error object, as returned by the server.
#[derive(Debug, serde::Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// Generic JSON-RPC 2.0 response envelope.
#[derive(Debug, serde::Deserialize)]
struct JsonRpcResponse<T> {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    result: Option<T>,
    error: Option<JsonRpcError>,
}

/// Wire shape of a `getLedgerEntries` entry.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireEntry {
    key: String,
    xdr: String,
    last_modified_ledger_seq: u32,
    live_until_ledger_seq: Option<u32>,
}

/// Wire shape of the `getLedgerEntries` result.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireLedgerEntriesResult {
    latest_ledger: u32,
    entries: Vec<WireEntry>,
}

/// Wire shape of the `getLatestLedger` result.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireLatestLedger {
    id: String,
    protocol_version: u32,
    sequence: u32,
    close_time: String,
}

/// Wire shape of the `getNetwork` result.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireNetworkInfo {
    passphrase: String,
    protocol_version: u32,
}

/// A minimal, typed Soroban RPC client.
///
/// Read-only by construction: it never builds, signs, or submits transactions.
#[derive(Debug)]
pub struct RpcClient {
    http: reqwest::Client,
    url: String,
    next_id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create a client for the given RPC endpoint URL (e.g.
    /// `https://soroban-testnet.stellar.org`).
    pub fn new(url: &str) -> Result<Self, RpcError> {
        // Validate the URL up front so misconfiguration fails at construction,
        // not on the first request.
        let parsed = url
            .parse::<reqwest::Url>()
            .map_err(|e| malformed("invalid RPC URL", e))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(RpcError::Malformed(format!(
                "invalid RPC URL scheme '{}' (expected http or https)",
                parsed.scheme()
            )));
        }
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| malformed("failed to build HTTP client", e))?;
        Ok(Self {
            http,
            url: parsed.to_string(),
            next_id: std::sync::atomic::AtomicU64::new(1),
        })
    }

    /// The RPC endpoint this client talks to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Issue a JSON-RPC call and decode the typed `result`.
    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, RpcError> {
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let response = self
            .http
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|source| RpcError::Transport {
                url: self.url.clone(),
                source,
            })?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|source| RpcError::Transport {
                url: self.url.clone(),
                source,
            })?;

        let parsed: JsonRpcResponse<T> = serde_json::from_str(&body)
            .map_err(|e| malformed(&format!("non-JSON-RPC response (HTTP {status})"), e))?;

        if parsed.jsonrpc != "2.0" {
            return Err(RpcError::Malformed(format!(
                "JSON-RPC response for '{method}' is not version 2.0 (HTTP {status})"
            )));
        }
        if let Some(resp_id) = &parsed.id {
            if resp_id.as_u64() != Some(id) {
                return Err(RpcError::Malformed(format!(
                    "JSON-RPC response for '{method}' has mismatched id (expected {id})"
                )));
            }
        }
        if let Some(err) = parsed.error {
            return Err(RpcError::Rpc {
                code: err.code,
                message: err.message,
            });
        }
        parsed.result.ok_or_else(|| {
            RpcError::Malformed(format!(
                "JSON-RPC response for '{method}' has neither result nor error (HTTP {status})"
            ))
        })
    }

    /// Fetch the latest ledger's sequence, protocol version, and close time.
    pub async fn get_latest_ledger(&self) -> Result<LatestLedger, RpcError> {
        let wire: WireLatestLedger = self.call("getLatestLedger", serde_json::json!({})).await?;
        let close_time = wire
            .close_time
            .parse::<u64>()
            .map_err(|e| malformed("getLatestLedger closeTime is not an integer", e))?;
        Ok(LatestLedger {
            id: wire.id,
            protocol_version: wire.protocol_version,
            sequence: wire.sequence,
            close_time,
        })
    }

    /// Fetch network-level information: passphrase and protocol version.
    pub async fn get_network(&self) -> Result<NetworkInfo, RpcError> {
        let wire: WireNetworkInfo = self.call("getNetwork", serde_json::json!({})).await?;
        Ok(NetworkInfo {
            passphrase: wire.passphrase,
            protocol_version: wire.protocol_version,
        })
    }

    /// Fetch ledger entries for the given keys, batching transparently into
    /// requests of at most [`MAX_KEYS_PER_REQUEST`] keys.
    ///
    /// Duplicate keys are removed first (the live testnet RPC rejects requests
    /// containing the same key twice with a captive-core error), preserving
    /// first-seen order.
    ///
    /// Keys that do not correspond to live entries are omitted from the result
    /// (this is how the sentinel detects archived entries).
    pub async fn get_ledger_entries(
        &self,
        keys: &[LedgerKey],
    ) -> Result<GetLedgerEntriesResponse, RpcError> {
        // Deduplicate while preserving first-seen order. LedgerKey implements
        // Hash/Eq, so a HashSet is enough.
        let mut seen = std::collections::HashSet::new();
        let deduped: Vec<LedgerKey> = keys
            .iter()
            .filter(|k| seen.insert((*k).clone()))
            .cloned()
            .collect();

        let mut all_entries: Vec<LedgerEntryInfo> = Vec::new();
        let mut latest_ledger: Option<u32> = None;

        for chunk in deduped.chunks(MAX_KEYS_PER_REQUEST) {
            let wire_keys = chunk
                .iter()
                .map(|k| k.to_xdr_base64(Limits::none()))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| malformed("failed to base64-encode LedgerKey", e))?;

            let wire: WireLedgerEntriesResult = self
                .call("getLedgerEntries", serde_json::json!({ "keys": wire_keys }))
                .await?;

            latest_ledger = Some(wire.latest_ledger);
            for w in wire.entries {
                let key = LedgerKey::from_xdr_base64(&w.key, Limits::none())
                    .map_err(|e| malformed("failed to decode response LedgerKey", e))?;
                let entry = LedgerEntryData::from_xdr_base64(&w.xdr, Limits::none())
                    .map_err(|e| malformed("failed to decode response LedgerEntryData", e))?;
                all_entries.push(LedgerEntryInfo {
                    key,
                    entry,
                    last_modified_ledger_seq: w.last_modified_ledger_seq,
                    live_until_ledger_seq: w.live_until_ledger_seq,
                });
            }
        }

        Ok(GetLedgerEntriesResponse {
            latest_ledger: latest_ledger.ok_or_else(|| {
                RpcError::Malformed("getLedgerEntries returned no batches".to_string())
            })?,
            entries: all_entries,
        })
    }

    /// Fetch a single ledger entry, distinguishing "absent" from transport/parse
    /// failures.
    ///
    /// Returns `Ok(None)` when the key is not present in the live state — which,
    /// for a key that should exist, is how an archived entry manifests.
    pub async fn get_entry(&self, key: &LedgerKey) -> Result<Option<LedgerEntryData>, RpcError> {
        let resp = self.get_ledger_entries(std::slice::from_ref(key)).await?;
        Ok(resp
            .entries
            .into_iter()
            .find(|info| &info.key == key)
            .map(|info| info.entry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{Hash, LedgerEntryData, TtlEntry};

    #[test]
    fn max_keys_per_request_is_verified_value() {
        // 200 is the verified RPC transport limit; if it ever changes upstream
        // this test is the canary.
        assert_eq!(MAX_KEYS_PER_REQUEST, 200);
    }

    #[test]
    fn xdr_base64_round_trip_for_ttl_entry() {
        // Exercises the exact encode/decode path the client uses for entries.
        let ttl = TtlEntry {
            key_hash: Hash([7u8; 32]),
            live_until_ledger_seq: 4_500_000,
        };
        let data = LedgerEntryData::Ttl(ttl);
        let b64 = data.to_xdr_base64(Limits::none()).expect("encode");
        let back = LedgerEntryData::from_xdr_base64(&b64, Limits::none()).expect("decode");
        assert_eq!(data, back);
    }
}
