//! Stroop projections for the two remediation actions the sentinel generates.
//!
//! The rent-change construction mirrors Stellar Core's op frames exactly
//! (protocol 28):
//!
//! - **Extend** (`ExtendFootprintTTLOpFrame.cpp`): for every live entry with
//!   `live_until < current_ledger + extend_to`, the change is
//!   `(old_size == new_size, old_live, new_live = current + extend_to)`.
//!   `extend_to` must already be validated against `max_entry_ttl - 1`.
//! - **Restore** (`RestoreFootprintOpFrame.cpp`): every restored entry is
//!   modeled as newly created with
//!   `new_live_until = current_ledger + min_persistent_ttl - 1`
//!   ("Extend the TTL on the restored entry to minimum TTL, including the
//!   current ledger"). Only persistent entries can be restored.
//!
//! The prices themselves come from [`crate::fees::compute_rent_fee`].

use crate::fees::{compute_rent_fee, LedgerEntryRentChange, RentFeeConfiguration};

/// An entry as seen by the rent model.
///
/// # Units
///
/// `size_bytes` in bytes (XDR size of the live entry; for contract-code entries
/// the sentinel approximates the "size for rent" with the XDR size — see
/// [`ProjectionConfig`] notes), `live_until_ledger_seq` in ledger sequence
/// numbers (`None` when archived/absent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryForRent {
    /// Whether the entry is persistent (temporary entries cannot be restored).
    pub is_persistent: bool,
    /// Whether this is a contract-code entry (rent discount applies).
    pub is_code_entry: bool,
    /// XDR size of the live entry in bytes.
    pub size_bytes: u32,
    /// Current live-until ledger, if the entry is live.
    pub live_until_ledger_seq: Option<u32>,
}

/// Inputs for a projection run.
///
/// # Units
///
/// `current_ledger_seq` in ledger sequence numbers; `extend_to` / `min_ttl`
/// values in ledgers; fees in stroops; sizes in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionConfig {
    /// Current ledger sequence (from `getLatestLedger`).
    pub current_ledger_seq: u32,
    /// Rent fee configuration (from the network config + live state size).
    pub rent_fee_config: RentFeeConfiguration,
    /// Network `max_entry_ttl` (validates extend targets).
    pub max_entry_ttl: u32,
    /// Network `min_persistent_ttl` (drives restore pricing).
    pub min_persistent_ttl: u32,
    /// Size of a TTL entry in bytes (protocol constant, overridable).
    pub ttl_entry_size: u32,
}

impl ProjectionConfig {
    /// Build the `LedgerEntryRentChange` list for extending the given entries to
    /// `extend_to` ledgers from the current ledger.
    ///
    /// Mirrors `ExtendFootprintTTLOpFrame::apply`: entries that are already
    /// beyond the target, or that are not live, are skipped.
    ///
    /// # Units
    ///
    /// `extend_to` in ledgers from the current ledger.
    pub fn extend_rent_changes(
        &self,
        entries: &[EntryForRent],
        extend_to: u32,
    ) -> Vec<LedgerEntryRentChange> {
        let new_live_until = self.current_ledger_seq.saturating_add(extend_to).min(
            self.current_ledger_seq
                .saturating_add(self.max_entry_ttl.saturating_sub(1)),
        );
        let mut changes = Vec::new();
        for e in entries {
            let Some(old_live) = e.live_until_ledger_seq else {
                // Archived entries must be restored first; extending does
                // nothing for them.
                continue;
            };
            if old_live >= new_live_until {
                continue;
            }
            changes.push(LedgerEntryRentChange {
                is_persistent: e.is_persistent,
                is_code_entry: e.is_code_entry,
                old_size_bytes: e.size_bytes,
                new_size_bytes: e.size_bytes,
                old_live_until_ledger: old_live,
                new_live_until_ledger: new_live_until,
            });
        }
        changes
    }

    /// Build the `LedgerEntryRentChange` list for restoring the given entries.
    ///
    /// Mirrors `RestoreFootprintOpFrame::apply`: each entry is modeled as newly
    /// created with `new_live_until = current + min_persistent_ttl - 1`.
    pub fn restore_rent_changes(&self, entries: &[EntryForRent]) -> Vec<LedgerEntryRentChange> {
        let new_live_until = self
            .current_ledger_seq
            .saturating_add(self.min_persistent_ttl)
            .saturating_sub(1);
        entries
            .iter()
            .map(|e| LedgerEntryRentChange {
                is_persistent: e.is_persistent,
                is_code_entry: e.is_code_entry,
                old_size_bytes: 0,
                new_size_bytes: e.size_bytes,
                old_live_until_ledger: 0,
                new_live_until_ledger: new_live_until,
            })
            .collect()
    }
}

/// Stroop cost to extend the given entries to `extend_to` ledgers from now.
///
/// # Units
///
/// Result in stroops (`i64`, never negative for pure extensions).
pub fn extend_stroop_cost(
    config: &ProjectionConfig,
    entries: &[EntryForRent],
    extend_to: u32,
) -> i64 {
    let changes = config.extend_rent_changes(entries, extend_to);
    compute_rent_fee(&changes, &config.rent_fee_config, config.current_ledger_seq)
}

/// Stroop cost to restore the given (archived) entries.
///
/// # Units
///
/// Result in stroops (`i64`, never negative).
pub fn restore_stroop_cost(config: &ProjectionConfig, entries: &[EntryForRent]) -> i64 {
    let changes = config.restore_rent_changes(entries);
    compute_rent_fee(&changes, &config.rent_fee_config, config.current_ledger_seq)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::{compute_rent_fee, RentFeeConfiguration};

    /// A concrete, realistic protocol-28 configuration (testnet-era values, with
    /// fee_per_rent_1kb computed at the state-size-high plateau).
    fn test_config() -> ProjectionConfig {
        let rent_fee_config = RentFeeConfiguration {
            fee_per_write_1kb: 1_000,
            fee_per_rent_1kb: 3_000,
            fee_per_write_entry: 10_000,
            persistent_rent_rate_denominator: 155_520_000,
            temporary_rent_rate_denominator: 1_693_440,
        };
        ProjectionConfig {
            current_ledger_seq: 4_500_000,
            rent_fee_config,
            max_entry_ttl: 3_112_000,
            min_persistent_ttl: 4_096,
            ttl_entry_size: 48,
        }
    }

    fn entry(size: u32, live_until: Option<u32>, persistent: bool, code: bool) -> EntryForRent {
        EntryForRent {
            is_persistent: persistent,
            is_code_entry: code,
            size_bytes: size,
            live_until_ledger_seq: live_until,
        }
    }

    #[test]
    fn known_fee_values_for_single_entry_extension() {
        // Hand-computed against the canonical formula for one 100-byte
        // persistent entry extended by 518400 ledgers (30 days at 5s):
        //   size_fee = ceil(100 * 3000 * 518400 / (1024 * 155520000))
        //            = ceil(155_520_000_000 / 159_252_480_000)
        //            = 1
        //   ttl writes: fee_per_write_entry + ceil(48 * 1000 / 1024)
        //            = 10000 + 47 = 10047
        //   total = 10048
        let cfg = test_config();
        let entries = vec![entry(100, Some(4_500_010), true, false)];
        let cost = extend_stroop_cost(&cfg, &entries, 518_400);
        assert_eq!(cost, 10_048);
    }

    #[test]
    fn extending_beyond_target_is_noop() {
        let cfg = test_config();
        // Entry already lives 10 ledgers past the requested extension target.
        let entries = vec![entry(100, Some(4_501_000), true, false)];
        let cost = extend_stroop_cost(&cfg, &entries, 100);
        assert_eq!(cost, 0);
    }

    #[test]
    fn archived_entries_are_skipped_by_extend() {
        let cfg = test_config();
        let entries = vec![entry(100, None, true, false)];
        let cost = extend_stroop_cost(&cfg, &entries, 518_400);
        assert_eq!(
            cost, 0,
            "extending an archived entry costs nothing (it must be restored)"
        );
    }

    #[test]
    fn restore_prices_as_new_entry_at_min_persistent_ttl() {
        // Restore of a 100-byte persistent entry:
        //   new_live = current + 4096 - 1
        //   extension_ledgers (new entry) = new_live - (current - 1) = 4096
        //   size_fee = ceil(100 * 3000 * 4096 / (1024 * 155520000))
        //            = ceil(1_228_800_000 / 159_252_480_000) = 1
        //   ttl writes = 10000 + 47 = 10047
        //   total = 10048
        let cfg = test_config();
        let entries = vec![entry(100, None, true, false)];
        let cost = restore_stroop_cost(&cfg, &entries);
        assert_eq!(cost, 10_048);
    }

    #[test]
    fn code_entries_get_rent_discount() {
        let cfg = test_config();
        let persistent = vec![entry(100, None, true, false)];
        let code = vec![entry(100, None, true, true)];
        let base = restore_stroop_cost(&cfg, &persistent);
        let discounted = restore_stroop_cost(&cfg, &code);
        // The size component is divided by 3 (ceil); the TTL-write component is
        // not discounted.
        assert!(discounted <= base);
        assert!(discounted > 0);
    }

    #[test]
    fn multiple_entries_sum() {
        let cfg = test_config();
        let one = vec![entry(100, None, true, false)];
        let two = vec![entry(100, None, true, false), entry(100, None, true, false)];
        let cost_one = restore_stroop_cost(&cfg, &one);
        let cost_two = restore_stroop_cost(&cfg, &two);
        assert_eq!(cost_two, cost_one * 2);
    }

    #[test]
    fn temporary_entries_use_temp_denominator() {
        let cfg = test_config();
        let persistent = vec![entry(100, Some(4_500_010), true, false)];
        let temporary = vec![entry(100, Some(4_500_010), false, false)];
        let p = extend_stroop_cost(&cfg, &persistent, 518_400);
        let t = extend_stroop_cost(&cfg, &temporary, 518_400);
        // Both pay the same TTL-write component (10047). The size component for
        // the temporary entry uses the much smaller temp denominator (1693440
        // vs 155520000), so it is 90 stroops vs 1:
        //   temp:    ceil(100 * 3000 * 518400 / (1024 * 1693440))   = 90
        //   persist: ceil(100 * 3000 * 518400 / (1024 * 155520000)) = 1
        assert_eq!(t - p, 89);
        assert!(t > p);
    }

    #[test]
    fn extend_to_is_clamped_to_network_max() {
        let cfg = test_config();
        let entries = vec![entry(100, Some(4_500_010), true, false)];
        let cost_at_max = extend_stroop_cost(&cfg, &entries, cfg.max_entry_ttl);
        let cost_past_max = extend_stroop_cost(&cfg, &entries, cfg.max_entry_ttl + 10_000);
        assert_eq!(cost_at_max, cost_past_max);
    }

    #[test]
    fn canonical_compute_rent_fee_matches_manual_calculation() {
        // Direct check against the ported function with a known configuration.
        let cfg = test_config();
        let changes = vec![LedgerEntryRentChange {
            is_persistent: true,
            is_code_entry: false,
            old_size_bytes: 100,
            new_size_bytes: 100,
            old_live_until_ledger: 4_500_010,
            new_live_until_ledger: 4_500_010 + 518_400,
        }];
        let fee = compute_rent_fee(&changes, &cfg.rent_fee_config, cfg.current_ledger_seq);
        assert_eq!(fee, 10_048);
    }
}
