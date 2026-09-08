//! Live network configuration.
//!
//! The Soroban fee/rent parameters are not hardcoded anywhere in the sentinel.
//! Instead they are read from the live ledger via `getLedgerEntries` on the
//! `CONFIG_SETTING` ledger keys, mirroring exactly how Stellar Core loads
//! `SorobanNetworkConfig` (see `src/ledger/NetworkConfig.cpp` in stellar-core).
//!
//! Three config settings are fetched:
//!
//! - [`stellar_xdr::ConfigSettingId::ContractLedgerCostV0`] — resource limits and
//!   fee rates, including the rent-rate ramp inputs
//!   (`soroban_state_target_size_bytes`, `rent_fee1_kb_soroban_state_size_low/high`,
//!   `soroban_state_rent_fee_growth_factor`).
//! - [`stellar_xdr::ConfigSettingId::ContractLedgerCostExtV0`] — the
//!   per-transaction footprint entry limit and the flat per-1KB write fee used by
//!   the rent model from protocol 23 on.
//! - [`stellar_xdr::ConfigSettingId::StateArchival`] — `max_entry_ttl`,
//!   `min_temporary_ttl`, `min_persistent_ttl`, and the persistent/temporary rent
//!   rate denominators.

use stellar_xdr::{
    ConfigSettingEntry, ConfigSettingId, LedgerEntryData, LedgerKey, LedgerKeyConfigSetting,
};

use crate::client::RpcClient;
use crate::error::RpcError;

/// The subset of live network configuration the sentinel consumes.
///
/// All values are protocol parameters as read from the ledger; units are the XDR
/// units (ledger sequence numbers for TTLs, stroops per 1KB / per entry for fees,
/// bytes for sizes). See each field's XDR definition for authoritative docs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkConfig {
    /// `ContractLedgerCostV0` — per-ledger/per-transaction resource limits and
    /// fee rates.
    pub ledger_cost: stellar_xdr::ConfigSettingContractLedgerCostV0,
    /// `ContractLedgerCostExtV0` — transaction footprint entry limit and flat
    /// write fee per 1KB.
    pub ledger_cost_ext: stellar_xdr::ConfigSettingContractLedgerCostExtV0,
    /// `StateArchivalSettings` — TTL bounds and rent-rate denominators.
    pub state_archival: stellar_xdr::StateArchivalSettings,
}

impl NetworkConfig {
    /// Build the `LedgerKey::ConfigSetting` keys to fetch this configuration.
    pub fn config_setting_keys() -> Vec<LedgerKey> {
        [
            ConfigSettingId::ContractLedgerCostV0,
            ConfigSettingId::ContractLedgerCostExtV0,
            ConfigSettingId::StateArchival,
        ]
        .into_iter()
        .map(|id| {
            LedgerKey::ConfigSetting(LedgerKeyConfigSetting {
                config_setting_id: id,
            })
        })
        .collect()
    }
}

impl RpcClient {
    /// Fetch the live network configuration from the ledger.
    ///
    /// All three settings must be present; a missing one is an error (the sentinel
    /// refuses to proceed with guessed protocol parameters).
    pub async fn fetch_network_config(&self) -> Result<NetworkConfig, RpcError> {
        let keys = NetworkConfig::config_setting_keys();
        let resp = self.get_ledger_entries(&keys).await?;

        let mut ledger_cost = None;
        let mut ledger_cost_ext = None;
        let mut state_archival = None;

        for info in &resp.entries {
            match &info.entry {
                LedgerEntryData::ConfigSetting(cfg) => match cfg {
                    ConfigSettingEntry::ContractLedgerCostV0(c) => ledger_cost = Some(c.clone()),
                    ConfigSettingEntry::ContractLedgerCostExtV0(c) => {
                        ledger_cost_ext = Some(c.clone())
                    }
                    ConfigSettingEntry::StateArchival(c) => state_archival = Some(c.clone()),
                    other => {
                        return Err(RpcError::NetworkConfig(format!(
                            "unexpected config setting entry: {other:?}"
                        )))
                    }
                },
                other => {
                    return Err(RpcError::NetworkConfig(format!(
                        "expected ConfigSetting entry for key {:?}, got {:?}",
                        info.key, other
                    )))
                }
            }
        }

        let ledger_cost = ledger_cost.ok_or_else(|| {
            RpcError::NetworkConfig("CONFIG_SETTING_CONTRACT_LEDGER_COST_V0 not found".to_string())
        })?;
        let ledger_cost_ext = ledger_cost_ext.ok_or_else(|| {
            RpcError::NetworkConfig(
                "CONFIG_SETTING_CONTRACT_LEDGER_COST_EXT_V0 not found".to_string(),
            )
        })?;
        let state_archival = state_archival.ok_or_else(|| {
            RpcError::NetworkConfig("CONFIG_SETTING_STATE_ARCHIVAL not found".to_string())
        })?;

        Ok(NetworkConfig {
            ledger_cost,
            ledger_cost_ext,
            state_archival,
        })
    }
}
