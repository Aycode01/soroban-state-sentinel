//! Construction of `ExtendFootprintTtl` / `RestoreFootprint` operations, batched
//! to stay within the network's per-transaction footprint limits.
//!
//! Semantics follow Stellar Core's op frames (protocol 28):
//!
//! - `ExtendFootprintTtl` places the target keys in the **read-only** footprint;
//!   `RestoreFootprint` in the **read-write** footprint.
//! - `extendTo` must be ≤ `max_entry_ttl - 1`.
//! - Only contract-data / contract-code keys are valid for either operation.
//!
//! ## `extendTo` semantics (verified, protocol 28)
//!
//! `ExtendFootprintTTLOp.extendTo` is a **duration in ledgers measured from the
//! current ledger**, not an absolute target ledger sequence. Stellar Core's
//! `src/transactions/ExtendFootprintTTLOpFrame.cpp` (master, protocol 28 era)
//! applies it as:
//!
//! ```cpp
//! // Extend for `extendTo` more ledgers since the current ledger.
//! uint32_t newLiveUntilLedgerSeq = getLedgerSeq() + extendTo;
//! ```
//!
//! and rejects `extendTo > maxEntryTTL - 1` as malformed in
//! `doCheckValidForSoroban`. Callers must validate via [`validate_extend_to`]
//! against the live network's `max_entry_ttl`.

use stellar_xdr::{
    ExtendFootprintTtlOp, ExtensionPoint, LedgerKey, Operation, OperationBody, RestoreFootprintOp,
};

use crate::error::BuildError;

/// A ledger key plus its live entry size in bytes, used for footprint byte
/// accounting when batching.
///
/// # Units
///
/// `size_bytes` is the XDR-encoded size of the live `LedgerEntry` (the data or
/// code entry itself, excluding its TTL entry).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEntry {
    /// The contract-data or contract-code ledger key.
    pub key: LedgerKey,
    /// XDR size of the live entry in bytes.
    pub size_bytes: u32,
}

/// Per-transaction footprint limits used for batching.
///
/// Values come from the live network configuration
/// (`ConfigSettingContractLedgerCostV0` / `…ExtV0`), never hardcoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchLimits {
    /// `tx_max_footprint_entries` — max ledger entries in one transaction
    /// footprint.
    pub max_footprint_entries: u32,
    /// `tx_max_disk_read_bytes` — max bytes read by one transaction.
    pub max_read_bytes: u32,
    /// `tx_max_write_bytes` — max bytes written by one transaction.
    pub max_write_bytes: u32,
    /// Size of a TTL entry, used in the byte estimate (default 48 from
    /// soroban-env-host, overridable).
    pub ttl_entry_size: u32,
    /// Max operations per transaction (the XDR `VecM<Operation, 100>` limit).
    pub max_ops_per_tx: usize,
}

impl BatchLimits {
    /// Build limits from the network configuration.
    pub fn from_network_config(
        ledger_cost: &stellar_xdr::ConfigSettingContractLedgerCostV0,
        ledger_cost_ext: &stellar_xdr::ConfigSettingContractLedgerCostExtV0,
        ttl_entry_size: u32,
    ) -> Result<Self, BuildError> {
        let limits = Self {
            max_footprint_entries: ledger_cost_ext.tx_max_footprint_entries,
            max_read_bytes: ledger_cost.tx_max_disk_read_bytes,
            max_write_bytes: ledger_cost.tx_max_write_bytes,
            ttl_entry_size,
            max_ops_per_tx: 100,
        };
        limits.validate()?;
        Ok(limits)
    }

    /// Reject unusable limits (zero caps would make batching impossible).
    pub fn validate(&self) -> Result<(), BuildError> {
        if self.max_footprint_entries == 0 {
            return Err(BuildError::InvalidBatchLimits(
                "max_footprint_entries is 0".to_string(),
            ));
        }
        if self.max_read_bytes == 0 || self.max_write_bytes == 0 {
            return Err(BuildError::InvalidBatchLimits(
                "max_read_bytes/max_write_bytes must be non-zero".to_string(),
            ));
        }
        if self.max_ops_per_tx == 0 {
            return Err(BuildError::InvalidBatchLimits(
                "max_ops_per_tx is 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Estimated read bytes for one key in an `ExtendFootprintTtl` footprint: the
/// entry itself plus its TTL entry.
fn extend_read_bytes(entry: &KeyEntry, ttl_entry_size: u32) -> u32 {
    entry.size_bytes.saturating_add(ttl_entry_size)
}

/// Estimated write bytes for one key in an `ExtendFootprintTtl` footprint: only
/// the TTL entry is rewritten.
fn extend_write_bytes(ttl_entry_size: u32) -> u32 {
    ttl_entry_size
}

/// Estimated read bytes for one key in a `RestoreFootprint` footprint.
fn restore_read_bytes(entry: &KeyEntry, ttl_entry_size: u32) -> u32 {
    entry.size_bytes.saturating_add(ttl_entry_size)
}

/// Estimated write bytes for one key in a `RestoreFootprint` footprint: the
/// entry and its TTL entry are both (re)written.
fn restore_write_bytes(entry: &KeyEntry, ttl_entry_size: u32) -> u32 {
    entry.size_bytes.saturating_add(ttl_entry_size)
}

/// A single `ExtendFootprintTtl` operation.
pub fn extend_ttl_operation(extend_to: u32) -> Operation {
    Operation {
        source_account: None,
        body: OperationBody::ExtendFootprintTtl(ExtendFootprintTtlOp {
            ext: ExtensionPoint::V0,
            extend_to,
        }),
    }
}

/// A single `RestoreFootprint` operation.
pub fn restore_operation() -> Operation {
    Operation {
        source_account: None,
        body: OperationBody::RestoreFootprint(RestoreFootprintOp {
            ext: ExtensionPoint::V0,
        }),
    }
}

/// Validate that every key is an evictable contract entry.
fn validate_keys(entries: &[KeyEntry]) -> Result<(), BuildError> {
    for e in entries {
        match e.key {
            LedgerKey::ContractData(_) | LedgerKey::ContractCode(_) => {}
            ref other => return Err(BuildError::UnsupportedKey(other.clone())),
        }
    }
    Ok(())
}

/// Split `entries` into batches that each fit within `limits`.
///
/// `is_restore` selects the byte-accounting convention (read-only vs read-write
/// footprint).
fn batch_entries(
    entries: &[KeyEntry],
    limits: &BatchLimits,
    is_restore: bool,
) -> Vec<Vec<KeyEntry>> {
    let mut batches: Vec<Vec<KeyEntry>> = Vec::new();
    let mut current: Vec<KeyEntry> = Vec::new();
    let mut read_bytes: u64 = 0;
    let mut write_bytes: u64 = 0;

    for entry in entries {
        let (r, w) = if is_restore {
            (
                u64::from(restore_read_bytes(entry, limits.ttl_entry_size)),
                u64::from(restore_write_bytes(entry, limits.ttl_entry_size)),
            )
        } else {
            (
                u64::from(extend_read_bytes(entry, limits.ttl_entry_size)),
                u64::from(extend_write_bytes(limits.ttl_entry_size)),
            )
        };

        let would_overflow = current.len() as u64 >= u64::from(limits.max_footprint_entries)
            || read_bytes.saturating_add(r) > u64::from(limits.max_read_bytes)
            || write_bytes.saturating_add(w) > u64::from(limits.max_write_bytes)
            || current.len() >= limits.max_ops_per_tx;

        if would_overflow && !current.is_empty() {
            batches.push(std::mem::take(&mut current));
            read_bytes = 0;
            write_bytes = 0;
        }
        read_bytes = read_bytes.saturating_add(r);
        write_bytes = write_bytes.saturating_add(w);
        current.push(entry.clone());
    }
    if !current.is_empty() {
        batches.push(current);
    }
    batches
}

/// Build `ExtendFootprintTtl` operations for the given entries, split across
/// operations so each stays within the network's footprint limits.
///
/// `extend_to` is a duration in ledgers measured from the current ledger (the
/// operation's `extendTo` field; core computes
/// `liveUntilLedgerSeq = current + extendTo`). The caller is responsible for
/// validating it against the network's `max_entry_ttl - 1` (or call
/// [`validate_extend_to`]).
pub fn build_extend_ttl_ops(
    entries: &[KeyEntry],
    extend_to: u32,
    limits: &BatchLimits,
) -> Result<Vec<Operation>, BuildError> {
    limits.validate()?;
    validate_keys(entries)?;
    Ok(batch_entries(entries, limits, false)
        .into_iter()
        .map(|_| extend_ttl_operation(extend_to))
        .collect())
}

/// Build `RestoreFootprint` operations for the given entries, split across
/// operations so each stays within the network's footprint limits.
pub fn build_restore_ops(
    entries: &[KeyEntry],
    limits: &BatchLimits,
) -> Result<Vec<Operation>, BuildError> {
    limits.validate()?;
    validate_keys(entries)?;
    Ok(batch_entries(entries, limits, true)
        .into_iter()
        .map(|_| restore_operation())
        .collect())
}

/// Validate an `extend_to` target against the network's `max_entry_ttl`.
///
/// Core rejects `extendTo > max_entry_ttl - 1` as malformed
/// (`ExtendFootprintTTLOpFrame::doCheckValidForSoroban`).
pub fn validate_extend_to(extend_to: u32, max_entry_ttl: u32) -> Result<(), BuildError> {
    let max = max_entry_ttl.saturating_sub(1);
    if extend_to == 0 || extend_to > max {
        return Err(BuildError::ExtendToOutOfRange { extend_to, max });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{
        AccountId, ContractDataDurability, ContractId, Hash, LedgerKeyAccount,
        LedgerKeyContractData, Limits, PublicKey, ReadXdr, ScAddress, ScVal, Uint256, WriteXdr,
    };

    fn data_key(i: u8) -> KeyEntry {
        KeyEntry {
            key: LedgerKey::ContractData(LedgerKeyContractData {
                contract: ScAddress::Contract(ContractId(Hash([i; 32]))),
                key: ScVal::LedgerKeyContractInstance,
                durability: ContractDataDurability::Persistent,
            }),
            size_bytes: 100,
        }
    }

    fn limits(max_entries: u32, max_read: u32, max_write: u32) -> BatchLimits {
        BatchLimits {
            max_footprint_entries: max_entries,
            max_read_bytes: max_read,
            max_write_bytes: max_write,
            ttl_entry_size: 48,
            max_ops_per_tx: 100,
        }
    }

    #[test]
    fn extend_op_has_correct_shape() {
        let op = extend_ttl_operation(1000);
        match op.body {
            OperationBody::ExtendFootprintTtl(inner) => {
                assert_eq!(inner.extend_to, 1000);
                assert!(matches!(inner.ext, ExtensionPoint::V0));
            }
            other => panic!("unexpected body: {other:?}"),
        }
    }

    #[test]
    fn extend_op_xdr_round_trips() {
        // Build, serialize to base64 (the CLI's output format), parse back, and
        // assert the operation survives unchanged.
        let entries: Vec<KeyEntry> = (0..2).map(data_key).collect();
        let ops = build_extend_ttl_ops(&entries, 518_400, &limits(100, u32::MAX, u32::MAX))
            .expect("build");
        assert_eq!(ops.len(), 1, "both entries fit in one operation");

        let b64 = ops[0].to_xdr_base64(Limits::none()).expect("encode");
        let back = Operation::from_xdr_base64(&b64, Limits::none()).expect("decode");
        assert_eq!(ops[0], back, "round-trip must preserve the operation");
        match back.body {
            OperationBody::ExtendFootprintTtl(inner) => {
                assert_eq!(inner.extend_to, 518_400);
                assert!(matches!(inner.ext, ExtensionPoint::V0));
            }
            other => panic!("unexpected body after round-trip: {other:?}"),
        }
    }

    #[test]
    fn restore_op_has_correct_shape() {
        let op = restore_operation();
        assert!(matches!(op.body, OperationBody::RestoreFootprint(_)));
    }

    #[test]
    fn batches_by_entry_count() {
        let entries: Vec<KeyEntry> = (0..5).map(data_key).collect();
        let ops =
            build_extend_ttl_ops(&entries, 1000, &limits(2, u32::MAX, u32::MAX)).expect("build");
        // 5 entries, 2 per op -> 3 ops.
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn batches_by_read_bytes() {
        let entries: Vec<KeyEntry> = (0..4).map(data_key).collect();
        // read per entry = 100 + 48 = 148; two entries = 296 > 200 -> 1 per op.
        let ops = build_extend_ttl_ops(&entries, 1000, &limits(100, 200, u32::MAX)).expect("build");
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn empty_input_yields_no_ops() {
        let ops = build_extend_ttl_ops(&[], 1000, &limits(100, u32::MAX, u32::MAX)).expect("build");
        assert!(ops.is_empty());
        let ops = build_restore_ops(&[], &limits(100, u32::MAX, u32::MAX)).expect("build");
        assert!(ops.is_empty());
    }

    #[test]
    fn rejects_non_contract_keys() {
        let account_key = LedgerKey::Account(LedgerKeyAccount {
            account_id: AccountId(PublicKey::PublicKeyTypeEd25519(Uint256([0u8; 32]))),
        });
        let entries = vec![KeyEntry {
            key: account_key,
            size_bytes: 10,
        }];
        assert!(build_extend_ttl_ops(&entries, 1000, &limits(100, u32::MAX, u32::MAX)).is_err());
    }

    #[test]
    fn validate_extend_to_uses_network_max() {
        assert!(validate_extend_to(0, 3_112_000).is_err());
        assert!(validate_extend_to(3_112_000, 3_112_000).is_err()); // == max_entry_ttl
        assert!(validate_extend_to(3_111_999, 3_112_000).is_ok()); // == max_entry_ttl - 1
        assert!(validate_extend_to(100, 3_112_000).is_ok());
    }
}
