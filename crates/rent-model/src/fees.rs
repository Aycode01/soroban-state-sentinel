//! Canonical rent-fee computation.
//!
//! This module is a line-by-line port of `soroban-env-host/src/fees.rs` from the
//! `stellar/rs-soroban-env` repository at tag **v28.0.0**
//! (commit `d0f1330`, "Bump version to 28.0.0"), which is the fee protocol
//! active on Stellar networks running protocol 28.
//!
//! Source: <https://github.com/stellar/rs-soroban-env/blob/v28.0.0/soroban-env-host/src/fees.rs>
//!
//! Comments from upstream are retained where they explain the math. The
//! configuration plumbing (`RentFeeConfiguration` → network config settings) is
//! documented in the crate docs and in `projection.rs`.
//!
//! ## Divergences from upstream
//!
//! - `num_integer::div_ceil` is replaced by local [`div_ceil`] helpers (checked,
//!   saturating) to avoid the dependency.
//! - Upstream assumes "sane configuration" and may panic on overflow; this port
//!   uses checked/saturating arithmetic and never panics.
//! - Fee values are returned as `i64` exactly like upstream; negative fees
//!   (refunds) are only meaningful in the ledger context, never for the
//!   projections this crate exposes (those clamp to `>= 0`).

/// Estimate for any `TtlEntry` ledger entry (upstream `TTL_ENTRY_SIZE`).
///
/// This is a constant in soroban-env-host (not a network-config setting), but it
/// is a protocol parameter that can change; callers may override it via
/// `ProjectionConfig::ttl_entry_size`.
pub const TTL_ENTRY_SIZE: u32 = 48;

/// Fee increments (upstream constants).
pub const INSTRUCTIONS_INCREMENT: i64 = 10_000;
/// Data-size increment for fee computation: 1KB.
pub const DATA_SIZE_1KB_INCREMENT: i64 = 1024;

/// Minimum effective rent write fee per 1KB (upstream).
pub const MINIMUM_RENT_WRITE_FEE_PER_1KB: i64 = 1000;

/// Contract-code rent discount factor (upstream; a constant for now, may become
/// a network setting in future protocols).
pub const CODE_ENTRY_RENT_DISCOUNT_FACTOR: i64 = 3;

/// Change in a single ledger entry with parameters relevant for rent fee
/// computations (upstream `LedgerEntryRentChange`).
///
/// # Units
///
/// Sizes in bytes, live-until values in ledger sequence numbers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntryRentChange {
    /// Whether this is a persistent or temporary entry.
    pub is_persistent: bool,
    /// Whether this is a contract code entry (applies the rent discount).
    pub is_code_entry: bool,
    /// Size of the entry before modification; `0` for newly-created entries.
    pub old_size_bytes: u32,
    /// Size of the entry after modification; `0` for removed entries.
    pub new_size_bytes: u32,
    /// Live-until ledger before modification; `0` for newly-created entries.
    pub old_live_until_ledger: u32,
    /// Live-until ledger after modification; `0` for removed entries.
    pub new_live_until_ledger: u32,
}

/// Rent fee-related network configuration (upstream `RentFeeConfiguration`).
///
/// Normally loaded from the ledger (see the crate docs); `fee_per_rent_1kb` is
/// computed via [`compute_rent_write_fee_per_1kb`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentFeeConfiguration {
    /// Fee per 1KB written to the ledger (protocol 23+: the flat
    /// `feeWrite1KB` from `ContractLedgerCostExtV0`).
    pub fee_per_write_1kb: i64,
    /// Fee per 1KB of rented ledger space, computed via
    /// [`compute_rent_write_fee_per_1kb`].
    pub fee_per_rent_1kb: i64,
    /// Fee per entry written to the ledger (`feeWriteLedgerEntry` from
    /// `ContractLedgerCostV0`).
    pub fee_per_write_entry: i64,
    /// Denominator for the total rent fee for persistent storage
    /// (`persistentRentRateDenominator` from `StateArchivalSettings`).
    pub persistent_rent_rate_denominator: i64,
    /// Denominator for the total rent fee for temporary storage
    /// (`tempRentRateDenominator` from `StateArchivalSettings`).
    pub temporary_rent_rate_denominator: i64,
}

/// Configuration for [`compute_rent_write_fee_per_1kb`]
/// (upstream `RentWriteFeeConfiguration`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RentWriteFeeConfiguration {
    /// Write fee grows linearly until the Soroban state reaches this size.
    pub state_target_size_bytes: i64,
    /// Fee per 1KB write when the state size is 0.
    pub rent_fee_1kb_state_size_low: i64,
    /// Fee per 1KB write when the Soroban state has reached
    /// `state_target_size_bytes`.
    pub rent_fee_1kb_state_size_high: i64,
    /// Write-fee multiplier for additional data past `state_target_size_bytes`.
    pub state_size_rent_fee_growth_factor: u32,
}

/// `ceil(a / b)` for `i64` with saturating behavior; `b == 0` yields `i64::MAX`
/// (never divide by zero).
fn div_ceil_i64(a: i64, b: i64) -> i64 {
    if b <= 0 {
        return i64::MAX;
    }
    if a <= 0 {
        return 0;
    }
    let q = a / b;
    let r = a % b;
    q.saturating_add(i64::from(r > 0))
}

/// `ceil(a / b)` for `i128` with saturating behavior.
fn div_ceil_i128(a: i128, b: i128) -> i128 {
    if b <= 0 {
        return i128::MAX;
    }
    if a <= 0 {
        return 0;
    }
    let q = a / b;
    let r = a % b;
    q.saturating_add(i128::from(r > 0))
}

// Helper for clamping values to the range of positive i64, with invalid cases
// mapped to i64::MAX (upstream `ClampFee`).
fn clamp_fee_i64(v: i64) -> i64 {
    if v < 0 {
        i64::MAX
    } else {
        v
    }
}

fn clamp_fee_i128(v: i128) -> i64 {
    if v < 0 {
        i64::MAX
    } else {
        i64::try_from(v).unwrap_or(i64::MAX)
    }
}

/// Computes the effective rent fee per 1KB of ledger space
/// (upstream `compute_rent_write_fee_per_1kb`).
///
/// Depends only on the current Soroban in-memory state size.
///
/// # Units
///
/// `soroban_state_size_bytes` in bytes; result in stroops per 1KB.
pub fn compute_rent_write_fee_per_1kb(
    soroban_state_size_bytes: i64,
    fee_config: &RentWriteFeeConfiguration,
) -> i64 {
    let fee_rate_multiplier = clamp_fee_i64(
        fee_config
            .rent_fee_1kb_state_size_high
            .saturating_sub(fee_config.rent_fee_1kb_state_size_low),
    );
    let mut rent_write_fee_per_1kb: i64;
    if soroban_state_size_bytes < fee_config.state_target_size_bytes {
        // Convert multipliers to i128 to handle large bucket list sizes.
        rent_write_fee_per_1kb = clamp_fee_i128(div_ceil_i128(
            (fee_rate_multiplier as i128).saturating_mul(soroban_state_size_bytes as i128),
            (fee_config.state_target_size_bytes as i128).max(1),
        ));
        rent_write_fee_per_1kb =
            rent_write_fee_per_1kb.saturating_add(fee_config.rent_fee_1kb_state_size_low);
    } else {
        rent_write_fee_per_1kb = fee_config.rent_fee_1kb_state_size_high;
        let bucket_list_size_after_reaching_target =
            soroban_state_size_bytes.saturating_sub(fee_config.state_target_size_bytes);
        let post_target_fee = clamp_fee_i128(div_ceil_i128(
            (fee_rate_multiplier as i128)
                .saturating_mul(bucket_list_size_after_reaching_target as i128)
                .saturating_mul(fee_config.state_size_rent_fee_growth_factor as i128),
            (fee_config.state_target_size_bytes as i128).max(1),
        ));
        rent_write_fee_per_1kb = rent_write_fee_per_1kb.saturating_add(post_target_fee);
    }

    rent_write_fee_per_1kb.max(MINIMUM_RENT_WRITE_FEE_PER_1KB)
}

/// Computes the total rent-related fee for the provided ledger entry changes
/// (upstream `compute_rent_fee`).
///
/// Rent-related fees consist of the fees for TTL extensions and fees for
/// increasing the entry size (with or without TTL extensions).
///
/// # Units
///
/// Result in stroops (`i64`). `current_ledger_seq` in ledger sequence numbers.
pub fn compute_rent_fee(
    changed_entries: &[LedgerEntryRentChange],
    fee_config: &RentFeeConfiguration,
    current_ledger_seq: u32,
) -> i64 {
    let mut fee: i64 = 0;
    let mut extended_entries: i64 = 0;
    let mut extended_entry_key_size_bytes: u32 = 0;
    for e in changed_entries {
        fee = fee.saturating_add(rent_fee_per_entry_change(e, fee_config, current_ledger_seq));
        if e.old_live_until_ledger < e.new_live_until_ledger {
            extended_entries = extended_entries.saturating_add(1);
            extended_entry_key_size_bytes =
                extended_entry_key_size_bytes.saturating_add(TTL_ENTRY_SIZE);
        }
    }
    // The TTL extensions need to be written to the ledger. As they have a
    // constant size, charge for writing them independently of the actual entry
    // size.
    fee = fee.saturating_add(
        fee_config
            .fee_per_write_entry
            .saturating_mul(extended_entries),
    );
    fee = fee.saturating_add(compute_fee_per_increment(
        extended_entry_key_size_bytes,
        fee_config.fee_per_write_1kb,
        DATA_SIZE_1KB_INCREMENT,
    ));

    fee
}

// Size of half-open range (lo, hi], or None if lo > hi (upstream).
fn exclusive_ledger_diff(lo: u32, hi: u32) -> Option<u32> {
    hi.checked_sub(lo)
}

// Size of closed range [lo, hi], or None if lo > hi (upstream).
fn inclusive_ledger_diff(lo: u32, hi: u32) -> Option<u32> {
    exclusive_ledger_diff(lo, hi).map(|diff| diff.saturating_add(1))
}

impl LedgerEntryRentChange {
    /// Whether the entry is newly created (upstream `entry_is_new`).
    fn entry_is_new(&self) -> bool {
        self.old_size_bytes == 0 && self.old_live_until_ledger == 0
    }

    /// Number of ledgers the extension adds (upstream `extension_ledgers`).
    fn extension_ledgers(&self, current_ledger: u32) -> Option<u32> {
        let ledger_before_extension = if self.entry_is_new() {
            current_ledger.saturating_sub(1)
        } else {
            self.old_live_until_ledger
        };
        exclusive_ledger_diff(ledger_before_extension, self.new_live_until_ledger)
    }

    /// Ledgers already paid for at the old size (upstream `prepaid_ledgers`).
    fn prepaid_ledgers(&self, current_ledger: u32) -> Option<u32> {
        if self.entry_is_new() {
            None
        } else {
            inclusive_ledger_diff(current_ledger, self.old_live_until_ledger)
        }
    }

    /// Size increase in bytes (upstream `size_increase`).
    fn size_increase(&self) -> Option<u32> {
        self.new_size_bytes.checked_sub(self.old_size_bytes)
    }
}

fn rent_fee_per_entry_change(
    entry_change: &LedgerEntryRentChange,
    fee_config: &RentFeeConfiguration,
    current_ledger: u32,
) -> i64 {
    let mut fee: i64 = 0;
    // If there was a difference in expiration, pay for the new ledger range at
    // the new size.
    if let Some(rent_ledgers) = entry_change.extension_ledgers(current_ledger) {
        fee = fee.saturating_add(rent_fee_for_size_and_ledgers(
            entry_change.is_persistent,
            entry_change.new_size_bytes,
            rent_ledgers,
            fee_config,
        ));
    }

    // If there were some ledgers already paid for at an old size, and the size
    // of the entry increased, those pre-paid ledgers need to pay top-up fees to
    // account for the change in size.
    if let (Some(rent_ledgers), Some(entry_size)) = (
        entry_change.prepaid_ledgers(current_ledger),
        entry_change.size_increase(),
    ) {
        fee = fee.saturating_add(rent_fee_for_size_and_ledgers(
            entry_change.is_persistent,
            entry_size,
            rent_ledgers,
            fee_config,
        ));
    }
    if entry_change.is_code_entry {
        fee = div_ceil_i64(fee, CODE_ENTRY_RENT_DISCOUNT_FACTOR);
    }
    fee
}

fn rent_fee_for_size_and_ledgers(
    is_persistent: bool,
    entry_size: u32,
    rent_ledgers: u32,
    fee_config: &RentFeeConfiguration,
) -> i64 {
    let num = (entry_size as i64)
        .saturating_mul(fee_config.fee_per_rent_1kb)
        .saturating_mul(rent_ledgers as i64);
    let storage_coef = if is_persistent {
        fee_config.persistent_rent_rate_denominator
    } else {
        fee_config.temporary_rent_rate_denominator
    };
    let denom = DATA_SIZE_1KB_INCREMENT.saturating_mul(storage_coef);
    div_ceil_i64(num, denom)
}

/// Fee per increment of a resource (upstream `compute_fee_per_increment`).
///
/// # Units
///
/// `resource_value` in resource units (bytes, instructions), `fee_rate` in
/// stroops per `increment`, `increment` in resource units. Result in stroops.
pub fn compute_fee_per_increment(resource_value: u32, fee_rate: i64, increment: i64) -> i64 {
    let resource_val: i64 = i64::from(resource_value);
    div_ceil_i64(resource_val.saturating_mul(fee_rate), increment)
}

/// Resource consumption of a Soroban transaction (upstream
/// `TransactionResources`).
///
/// These are the resource upper bounds declared by the transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionResources {
    /// Number of CPU instructions.
    pub instructions: u32,
    /// Number of ledger entries the transaction reads.
    pub disk_read_entries: u32,
    /// Number of ledger entries the transaction writes (also counted as read
    /// for the respective fees).
    pub write_entries: u32,
    /// Number of bytes read from the ledger.
    pub disk_read_bytes: u32,
    /// Number of bytes written to the ledger.
    pub write_bytes: u32,
    /// Size of the contract events XDR.
    pub contract_events_size_bytes: u32,
    /// Size of the transaction XDR.
    pub transaction_size_bytes: u32,
}

/// Fee-related network configuration for [`compute_transaction_resource_fee`]
/// (upstream `FeeConfiguration`), loaded from the ledger config settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeConfiguration {
    /// Fee per `INSTRUCTIONS_INCREMENT` (10k) instructions.
    pub fee_per_instruction_increment: i64,
    /// Fee per entry read from the ledger.
    pub fee_per_disk_read_entry: i64,
    /// Fee per entry written to the ledger.
    pub fee_per_write_entry: i64,
    /// Fee per 1KB read from the ledger.
    pub fee_per_disk_read_1kb: i64,
    /// Fee per 1KB of state written to the ledger (protocol 23+: the flat rate
    /// from `ContractLedgerCostExtV0`).
    pub fee_per_write_1kb: i64,
    /// Fee per 1KB written to history.
    pub fee_per_historical_1kb: i64,
    /// Fee per 1KB of contract events written.
    pub fee_per_contract_event_1kb: i64,
    /// Fee per 1KB of transaction size.
    pub fee_per_transaction_size_1kb: i64,
}

/// Computes the resource fee for a transaction based on the resource consumption
/// and the fee-related network configuration (upstream
/// `compute_transaction_resource_fee`).
///
/// Returns a pair of `(non_refundable_fee, refundable_fee)` in stroops.
pub fn compute_transaction_resource_fee(
    tx_resources: &TransactionResources,
    fee_config: &FeeConfiguration,
) -> (i64, i64) {
    let compute_fee = compute_fee_per_increment(
        tx_resources.instructions,
        fee_config.fee_per_instruction_increment,
        INSTRUCTIONS_INCREMENT,
    );
    let ledger_read_entry_fee: i64 = fee_config
        .fee_per_disk_read_entry
        .saturating_mul(i64::from(tx_resources.disk_read_entries));
    let ledger_write_entry_fee = fee_config
        .fee_per_write_entry
        .saturating_mul(i64::from(tx_resources.write_entries));
    let ledger_read_bytes_fee = compute_fee_per_increment(
        tx_resources.disk_read_bytes,
        fee_config.fee_per_disk_read_1kb,
        DATA_SIZE_1KB_INCREMENT,
    );
    let ledger_write_bytes_fee = compute_fee_per_increment(
        tx_resources.write_bytes,
        fee_config.fee_per_write_1kb,
        DATA_SIZE_1KB_INCREMENT,
    );

    let historical_fee = compute_fee_per_increment(
        tx_resources.transaction_size_bytes.saturating_add(300), // TX_BASE_RESULT_SIZE
        fee_config.fee_per_historical_1kb,
        DATA_SIZE_1KB_INCREMENT,
    );

    let events_fee = compute_fee_per_increment(
        tx_resources.contract_events_size_bytes,
        fee_config.fee_per_contract_event_1kb,
        DATA_SIZE_1KB_INCREMENT,
    );

    let bandwidth_fee = compute_fee_per_increment(
        tx_resources.transaction_size_bytes,
        fee_config.fee_per_transaction_size_1kb,
        DATA_SIZE_1KB_INCREMENT,
    );

    let refundable_fee = events_fee;
    let non_refundable_fee = compute_fee
        .saturating_add(ledger_read_entry_fee)
        .saturating_add(ledger_write_entry_fee)
        .saturating_add(ledger_read_bytes_fee)
        .saturating_add(ledger_write_bytes_fee)
        .saturating_add(historical_fee)
        .saturating_add(bandwidth_fee);

    (non_refundable_fee, refundable_fee)
}
