#![allow(clippy::result_large_err)] // error enums wrap large source errors; acceptable for a CLI

//! # sentinel-rent-model
//!
//! Canonical port of the Soroban rent-fee computation, plus stroop projections
//! for the two remediation actions the sentinel generates.
//!
//! - [`fees`] — the exact port of `soroban-env-host`'s `fees.rs` (v28.0.0):
//!   `compute_rent_fee`, `compute_rent_write_fee_per_1kb`, and their helpers.
//!   The port carries a source citation; never "improve" the math here without
//!   re-verifying against upstream.
//! - [`projection`] — builds the `LedgerEntryRentChange` lists for
//!   `ExtendFootprintTtl` and `RestoreFootprint` operations exactly the way
//!   Stellar Core does (`ExtendFootprintTTLOpFrame.cpp` /
//!   `RestoreFootprintOpFrame.cpp`, protocol 28), then prices them.
//!
//! ## Units
//!
//! All fees are in **stroops** (`i64`), all sizes in **bytes** (`u32`), all TTL
//! values in **ledger sequence numbers** (`u32`). No floats anywhere — checked
//! integer arithmetic only.

pub mod fees;
pub mod projection;

pub use fees::{
    compute_fee_per_increment, compute_rent_fee, compute_rent_write_fee_per_1kb,
    compute_transaction_resource_fee, FeeConfiguration, LedgerEntryRentChange,
    RentFeeConfiguration, RentWriteFeeConfiguration, TransactionResources, DATA_SIZE_1KB_INCREMENT,
    INSTRUCTIONS_INCREMENT, MINIMUM_RENT_WRITE_FEE_PER_1KB, TTL_ENTRY_SIZE,
};
pub use projection::{extend_stroop_cost, restore_stroop_cost, EntryForRent, ProjectionConfig};
