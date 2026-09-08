//! Assembles the rent-model configuration from live network config and CLI
//! overrides.
//!
//! The one value RPC cannot supply is the **average live Soroban state size**
//! (core derives it from the state-size sampling window; it is not exposed as a
//! ledger entry). The sentinel therefore resolves `fee_per_rent_1kb` with this
//! precedence, and always labels which path was taken so the assumption is
//! never silent:
//!
//! 1. `--rent-fee-per-1kb` (explicit override),
//! 2. `--average-state-size-bytes` (computed via the canonical
//!    `compute_rent_write_fee_per_1kb`),
//! 3. the state-size-high plateau (`rent_fee_1kb_soroban_state_size_high`),
//!    which is the fee at/above the target state size — the common steady-state
//!    case.

use sentinel_rent_model::{
    compute_rent_write_fee_per_1kb, ProjectionConfig, RentFeeConfiguration,
    RentWriteFeeConfiguration,
};
use sentinel_rpc_client::{NetworkConfig, RpcError};

use crate::args::CommonArgs;

/// How `fee_per_rent_1kb` was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeSource {
    /// `--rent-fee-per-1kb` was supplied.
    Explicit,
    /// `--average-state-size-bytes` was supplied and fed through the canonical
    /// formula.
    AverageStateSize,
    /// No input; the state-size-high plateau rate was assumed.
    StateSizeHigh,
}

impl FeeSource {
    /// Stable machine-readable name for JSON output.
    pub fn as_str(&self) -> &'static str {
        match self {
            FeeSource::Explicit => "explicit",
            FeeSource::AverageStateSize => "average_state_size",
            FeeSource::StateSizeHigh => "state_size_high",
        }
    }
}

/// Resolved rent-model inputs plus provenance labels.
#[derive(Debug, Clone)]
pub struct RentSettings {
    /// The projection config (rent fee rates + network TTL bounds).
    pub config: ProjectionConfig,
    /// The resolved `fee_per_rent_1kb` (stroops per 1KB).
    pub fee_per_rent_1kb: i64,
    /// How `fee_per_rent_1kb` was resolved.
    pub fee_per_rent_1kb_source: FeeSource,
    /// The average state size used, if any.
    pub average_state_size_bytes: Option<i64>,
}

/// Build `RentSettings` from the live network config and CLI overrides.
pub fn build_rent_settings(
    network: &NetworkConfig,
    args: &CommonArgs,
) -> Result<RentSettings, RpcError> {
    let (fee_per_rent_1kb, source, avg) = if let Some(explicit) = args.rent_fee_per_1kb {
        (explicit, FeeSource::Explicit, None)
    } else if let Some(state_size) = args.average_state_size_bytes {
        let computed = compute_rent_write_fee_per_1kb(
            state_size,
            &RentWriteFeeConfiguration {
                state_target_size_bytes: network.ledger_cost.soroban_state_target_size_bytes,
                rent_fee_1kb_state_size_low: network
                    .ledger_cost
                    .rent_fee1_kb_soroban_state_size_low,
                rent_fee_1kb_state_size_high: network
                    .ledger_cost
                    .rent_fee1_kb_soroban_state_size_high,
                state_size_rent_fee_growth_factor: network
                    .ledger_cost
                    .soroban_state_rent_fee_growth_factor,
            },
        );
        (computed, FeeSource::AverageStateSize, Some(state_size))
    } else {
        (
            network.ledger_cost.rent_fee1_kb_soroban_state_size_high,
            FeeSource::StateSizeHigh,
            None,
        )
    };

    let rent_fee_config = RentFeeConfiguration {
        fee_per_write_1kb: network.ledger_cost_ext.fee_write1_kb,
        fee_per_rent_1kb,
        fee_per_write_entry: network.ledger_cost.fee_write_ledger_entry,
        persistent_rent_rate_denominator: network.state_archival.persistent_rent_rate_denominator,
        temporary_rent_rate_denominator: network.state_archival.temp_rent_rate_denominator,
    };

    Ok(RentSettings {
        config: ProjectionConfig {
            current_ledger_seq: 0, // filled in by the caller once the latest ledger is known
            rent_fee_config,
            max_entry_ttl: network.state_archival.max_entry_ttl,
            min_persistent_ttl: network.state_archival.min_persistent_ttl,
            ttl_entry_size: args.ttl_entry_size,
        },
        fee_per_rent_1kb,
        fee_per_rent_1kb_source: source,
        average_state_size_bytes: avg,
    })
}

/// Set the current ledger sequence on a rent config.
pub fn with_current_ledger(mut settings: RentSettings, current_ledger_seq: u32) -> RentSettings {
    settings.config.current_ledger_seq = current_ledger_seq;
    settings
}
