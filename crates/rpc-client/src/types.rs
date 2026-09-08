//! Typed response shapes for the Soroban RPC methods used by the sentinel.
//!
//! Field naming follows the RPC API reference (`camelCase` on the wire, snake_case
//! here). Ledger keys and entries are decoded from their base64-XDR string forms
//! into `stellar_xdr` types so the rest of the pipeline works on typed values.

use stellar_xdr::{LedgerEntryData, LedgerKey};

/// Response of `getLatestLedger`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatestLedger {
    /// Hash of the latest ledger, hex-encoded.
    pub id: String,
    /// Protocol version of the latest ledger.
    pub protocol_version: u32,
    /// Sequence number of the latest ledger (units: ledger sequence numbers).
    pub sequence: u32,
    /// Unix timestamp (seconds) at which the latest ledger closed.
    pub close_time: u64,
}

/// Response of `getNetwork`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInfo {
    /// Network passphrase, e.g. `Test SDF Network ; September 2015`.
    pub passphrase: String,
    /// Protocol version the network is running.
    pub protocol_version: u32,
}

/// One entry returned by `getLedgerEntries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntryInfo {
    /// The requested key, decoded from base64 XDR.
    pub key: LedgerKey,
    /// The live `LedgerEntryData` value, decoded from base64 XDR.
    ///
    /// Soroban RPC serializes only the entry *data* (not the wrapping
    /// `LedgerEntry` with `lastModifiedLedgerSeq`); verified against the live
    /// testnet RPC and soroban-rpc v28.0.1's `get_ledger_entries.go`
    /// (`xdr.MarshalBase64(keyEntry.Entry.Data)`).
    pub entry: LedgerEntryData,
    /// Ledger sequence number of the last modification of this entry.
    pub last_modified_ledger_seq: u32,
    /// Ledger sequence up to which the entry is live, if the RPC surfaced it.
    ///
    /// Soroban RPC refuses to serve `LedgerKey::Ttl` entries directly ("ledger
    /// ttl entries cannot be queried directly"), and instead populates this
    /// field on the entries it returns — the sentinel treats it as the
    /// authoritative live-until value. `0`/absent means the RPC surfaced no
    /// live-until info (the entry may be archived or not TTL-governed).
    pub live_until_ledger_seq: Option<u32>,
}

/// Response of `getLedgerEntries`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetLedgerEntriesResponse {
    /// Sequence number of the latest ledger at request handling time.
    pub latest_ledger: u32,
    /// Entries that were found. Missing keys are simply omitted — `getLedgerEntries`
    /// never reports them as errors.
    pub entries: Vec<LedgerEntryInfo>,
}
