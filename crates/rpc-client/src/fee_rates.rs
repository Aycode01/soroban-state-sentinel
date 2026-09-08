//! Fetching the fee rates needed to estimate transaction resource fees.
//!
//! Mirrors Stellar Core's `SorobanNetworkConfig::rustBridgeFeeConfiguration`
//! (`src/ledger/NetworkConfig.cpp`): the per-increment instruction fee comes
//! from `CONTRACT_COMPUTE_V0`, the read/write entry+byte fees from
//! `CONTRACT_LEDGER_COST_V0`, the flat write-per-1KB fee from
//! `CONTRACT_LEDGER_COST_EXT_V0`, and the historical / events / bandwidth fees
//! from their respective settings.

use stellar_xdr::{
    ConfigSettingEntry, ConfigSettingId, LedgerEntryData, LedgerKey, LedgerKeyConfigSetting,
};

use crate::client::RpcClient;
use crate::error::RpcError;

/// Fee rates used to estimate a transaction's resource fee.
///
/// # Units
///
/// All fees in stroops; the `*_1kb` rates are stroops per 1KB, the
/// `fee_per_instruction_increment` is stroops per 10k instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeRates {
    /// `feeRatePerInstructionsIncrement` from `CONTRACT_COMPUTE_V0`.
    pub fee_per_instruction_increment: i64,
    /// `feeDiskReadLedgerEntry` from `CONTRACT_LEDGER_COST_V0`.
    pub fee_per_disk_read_entry: i64,
    /// `feeWriteLedgerEntry` from `CONTRACT_LEDGER_COST_V0`.
    pub fee_per_write_entry: i64,
    /// `feeDiskRead1KB` from `CONTRACT_LEDGER_COST_V0`.
    pub fee_per_disk_read_1kb: i64,
    /// `feeWrite1KB` from `CONTRACT_LEDGER_COST_EXT_V0` (protocol 23+ flat rate).
    pub fee_per_write_1kb: i64,
    /// `feeHistorical1KB` from `CONTRACT_HISTORICAL_DATA_V0`.
    pub fee_per_historical_1kb: i64,
    /// `feeContractEvents1KB` from `CONTRACT_EVENTS_V0`.
    pub fee_per_contract_event_1kb: i64,
    /// `feeTxSize1KB` from `CONTRACT_BANDWIDTH_V0`.
    pub fee_per_transaction_size_1kb: i64,
}

impl FeeRates {
    /// The ledger keys to fetch all required settings.
    pub fn config_setting_keys() -> Vec<LedgerKey> {
        [
            ConfigSettingId::ContractComputeV0,
            ConfigSettingId::ContractLedgerCostV0,
            ConfigSettingId::ContractLedgerCostExtV0,
            ConfigSettingId::ContractHistoricalDataV0,
            ConfigSettingId::ContractEventsV0,
            ConfigSettingId::ContractBandwidthV0,
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
    /// Fetch the fee rates from the ledger. Every required setting must be
    /// present, mirroring core's `releaseAssertOrThrow` behavior — the sentinel
    /// refuses to estimate fees with guessed rates.
    pub async fn fetch_fee_rates(&self) -> Result<FeeRates, RpcError> {
        let resp = self
            .get_ledger_entries(&FeeRates::config_setting_keys())
            .await?;

        let mut compute = None;
        let mut ledger_cost = None;
        let mut ledger_cost_ext = None;
        let mut historical = None;
        let mut events = None;
        let mut bandwidth = None;

        for info in &resp.entries {
            match &info.entry {
                LedgerEntryData::ConfigSetting(cfg) => match cfg {
                    ConfigSettingEntry::ContractComputeV0(c) => compute = Some(c.clone()),
                    ConfigSettingEntry::ContractLedgerCostV0(c) => ledger_cost = Some(c.clone()),
                    ConfigSettingEntry::ContractLedgerCostExtV0(c) => {
                        ledger_cost_ext = Some(c.clone())
                    }
                    ConfigSettingEntry::ContractHistoricalDataV0(c) => historical = Some(c.clone()),
                    ConfigSettingEntry::ContractEventsV0(c) => events = Some(c.clone()),
                    ConfigSettingEntry::ContractBandwidthV0(c) => bandwidth = Some(c.clone()),
                    other => {
                        return Err(RpcError::NetworkConfig(format!(
                            "unexpected config setting entry while fetching fee rates: {other:?}"
                        )))
                    }
                },
                other => {
                    return Err(RpcError::NetworkConfig(format!(
                        "expected ConfigSetting entry, got {:?}",
                        other
                    )))
                }
            }
        }

        let compute = compute.ok_or_else(|| {
            RpcError::NetworkConfig("CONFIG_SETTING_CONTRACT_COMPUTE_V0 not found".to_string())
        })?;
        let ledger_cost = ledger_cost.ok_or_else(|| {
            RpcError::NetworkConfig("CONFIG_SETTING_CONTRACT_LEDGER_COST_V0 not found".to_string())
        })?;
        let ledger_cost_ext = ledger_cost_ext.ok_or_else(|| {
            RpcError::NetworkConfig(
                "CONFIG_SETTING_CONTRACT_LEDGER_COST_EXT_V0 not found".to_string(),
            )
        })?;
        let historical = historical.ok_or_else(|| {
            RpcError::NetworkConfig(
                "CONFIG_SETTING_CONTRACT_HISTORICAL_DATA_V0 not found".to_string(),
            )
        })?;
        let events = events.ok_or_else(|| {
            RpcError::NetworkConfig("CONFIG_SETTING_CONTRACT_EVENTS_V0 not found".to_string())
        })?;
        let bandwidth = bandwidth.ok_or_else(|| {
            RpcError::NetworkConfig("CONFIG_SETTING_CONTRACT_BANDWIDTH_V0 not found".to_string())
        })?;

        Ok(FeeRates {
            fee_per_instruction_increment: compute.fee_rate_per_instructions_increment,
            fee_per_disk_read_entry: ledger_cost.fee_disk_read_ledger_entry,
            fee_per_write_entry: ledger_cost.fee_write_ledger_entry,
            fee_per_disk_read_1kb: ledger_cost.fee_disk_read1_kb,
            fee_per_write_1kb: ledger_cost_ext.fee_write1_kb,
            fee_per_historical_1kb: historical.fee_historical1_kb,
            fee_per_contract_event_1kb: events.fee_contract_events1_kb,
            fee_per_transaction_size_1kb: bandwidth.fee_tx_size1_kb,
        })
    }
}
