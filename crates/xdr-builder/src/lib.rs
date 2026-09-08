#![allow(clippy::result_large_err)] // error enums wrap large source errors; acceptable for a CLI

//! # sentinel-xdr-builder
//!
//! Builders for the unsigned remediation operations the sentinel produces:
//!
//! - [`ops::build_extend_ttl_ops`] — `ExtendFootprintTtl` operations, batched to
//!   stay within the network's footprint limits.
//! - [`ops::build_restore_ops`] — `RestoreFootprint` operations, batched the same
//!   way.
//! - [`envelope::build_unsigned_envelope`] — wraps the operations in an unsigned
//!   `TransactionV1Envelope` that a separately-held key can sign and submit.
//! - [`strkey`] — decodes `C…` contract ids and `G…` account ids.
//! - [`ttl::ttl_key_for`] — derives the `LedgerKey::Ttl` key for a data/code key
//!   (SHA-256 of the key's XDR, exactly as Stellar Core's `getTTLKey` does).
//!
//! This crate never signs anything and never touches a private key.

pub mod envelope;
pub mod error;
pub mod ops;
pub mod strkey;
pub mod ttl;

pub use envelope::{build_unsigned_envelope, unsigned_envelope_xdr_base64};
pub use error::BuildError;
pub use ops::{build_extend_ttl_ops, build_restore_ops, validate_extend_to, BatchLimits, KeyEntry};
pub use strkey::{decode_account_id, decode_contract_id, decode_muxed_account};
pub use ttl::ttl_key_for;
