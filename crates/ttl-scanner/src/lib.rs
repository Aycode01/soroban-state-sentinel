#![allow(clippy::result_large_err)] // error enums wrap large source errors; acceptable for a CLI

//! # sentinel-ttl-scanner
//!
//! Computes ledgers-remaining for Soroban ledger entries and classifies them into
//! health bands:
//!
//! - [`health`] — the band definitions and the ledgers-remaining math.
//! - [`scan`] — the end-to-end scan engine: takes a contract id (+ optional
//!   explicit storage keys), fetches the associated ledger entries and their TTL
//!   entries over RPC, and produces a [`scan::ScanResult`].
//!
//! ## Units
//!
//! Ledger math in this crate is done in **ledger sequence numbers** (absolute)
//! and **ledgers remaining** (relative). Days are derived from ledgers using the
//! network's average ledger close time (seconds per ledger), which is always
//! supplied explicitly (fetched or configured) — never hardcoded.

pub mod health;
pub mod scan;

pub use health::{classify, HealthBand, HealthConfig};
pub use scan::{
    instance_key, EntryKind, ScanOptions, ScanResult, ScanSummary, ScannedEntry, ScannerError,
    DEFAULT_LEDGER_CLOSE_SECONDS,
};
