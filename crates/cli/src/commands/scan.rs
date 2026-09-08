//! `scan` subcommand: fetch entries, classify health, project remediation costs,
//! and emit json/markdown/table output.

use sentinel_rent_model::{extend_stroop_cost, restore_stroop_cost, EntryForRent};
use sentinel_rpc_client::RpcClient;
use sentinel_ttl_scanner::{ScanOptions, ScanResult, ScannedEntry};
use sentinel_xdr_builder::decode_contract_id;
use stellar_xdr::{ContractDataDurability, Limits, ReadXdr, ScVal, WriteXdr};

use crate::args::{DurabilityArg, ScanArgs};
use crate::commands::{CliError, Outcome};
use crate::context::{build_rent_settings, with_current_ledger, RentSettings};

/// Projected remediation costs for one scanned entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryProjection {
    /// Stroops to extend this entry to the healthy horizon (null when archived).
    pub extend_to_healthy_cost_stroops: Option<i64>,
    /// Stroops to restore this entry (only meaningful when archived).
    pub restore_cost_stroops: Option<i64>,
}

/// A scan result enriched with the network config and per-entry projections.
#[derive(Debug, Clone)]
pub struct ScanReport {
    /// The raw scan result.
    pub result: ScanResult,
    /// Per-entry cost projections, aligned with `result.entries`.
    pub projections: Vec<EntryProjection>,
    /// The rent settings that were applied.
    pub rent_settings: RentSettings,
    /// The network config that was applied.
    pub network_config: sentinel_rpc_client::NetworkConfig,
    /// The contract id as given on the command line (C… strkey).
    pub contract_id: String,
    /// The RPC URL used.
    pub rpc_url: String,
    /// Healthy/critical thresholds in days (echoed into the JSON output).
    pub healthy_min_days: u32,
    pub critical_max_days: u32,
}

fn durability_arg_to_xdr(d: DurabilityArg) -> ContractDataDurability {
    match d {
        DurabilityArg::Persistent => ContractDataDurability::Persistent,
        DurabilityArg::Temporary => ContractDataDurability::Temporary,
    }
}

/// Parse a `--keys` value: base64-XDR-encoded `SCVal`.
pub fn parse_storage_key(s: &str) -> Result<ScVal, CliError> {
    ScVal::from_xdr_base64(s, Limits::none()).map_err(|e| {
        CliError::msg(format!(
            "--keys value '{s}' is not a valid base64 SCVal XDR: {e}"
        ))
    })
}

/// Build the `EntryForRent` view of a scanned entry for the rent model.
fn entry_for_rent(e: &ScannedEntry, assumed_size: u32) -> EntryForRent {
    EntryForRent {
        is_persistent: e.durability != Some(ContractDataDurability::Temporary),
        is_code_entry: e.is_code_entry,
        size_bytes: e.size_bytes.unwrap_or(assumed_size),
        live_until_ledger_seq: e.live_until_ledger_seq,
    }
}

/// Run the scan command.
pub async fn run(cmd: &ScanArgs) -> Result<Outcome, CliError> {
    let rpc = RpcClient::new(&cmd.common.rpc_url)?;
    let contract_id = decode_contract_id(&cmd.contract_id)?;

    let mut keys = Vec::new();
    for raw in &cmd.keys {
        keys.push(parse_storage_key(raw)?);
    }

    let options = ScanOptions {
        contract_id,
        explicit_keys: keys,
        explicit_durability: durability_arg_to_xdr(cmd.durability),
        healthy_min_days: cmd.healthy_days,
        critical_max_days: cmd.critical_days,
        ledger_close_seconds: cmd.common.ledger_close_seconds,
    };

    let result = options.scan(&rpc).await?;

    let network_config = rpc.fetch_network_config().await?;
    let mut rent_settings = build_rent_settings(&network_config, &cmd.common)?;
    rent_settings = with_current_ledger(rent_settings, result.latest_ledger);

    let horizon = cmd
        .extend_horizon_ledgers
        .unwrap_or(result.health_config.healthy_min_ledgers);
    let projections = project_entries(&result, &rent_settings, horizon);

    let report = ScanReport {
        result,
        projections,
        rent_settings,
        network_config,
        contract_id: cmd.contract_id.clone(),
        rpc_url: cmd.common.rpc_url.clone(),
        healthy_min_days: cmd.healthy_days,
        critical_max_days: cmd.critical_days,
    };

    crate::output::emit_scan(&report, cmd.output_format())?;

    let triggered = cmd.fail_on_critical && report.result.summary.has_critical;
    Ok(if triggered {
        Outcome::FailOnCritical
    } else {
        Outcome::Ok
    })
}

/// Compute per-entry extend/restore cost projections.
fn project_entries(
    result: &ScanResult,
    rent_settings: &RentSettings,
    horizon: u32,
) -> Vec<EntryProjection> {
    result
        .entries
        .iter()
        .map(|e| {
            let rent = entry_for_rent(e, 1024);
            let extend = match e.band {
                sentinel_ttl_scanner::HealthBand::Archived => None,
                _ => {
                    let cost = extend_stroop_cost(
                        &rent_settings.config,
                        std::slice::from_ref(&rent),
                        horizon,
                    );
                    Some(cost)
                }
            };
            let restore = if e.band == sentinel_ttl_scanner::HealthBand::Archived {
                Some(restore_stroop_cost(&rent_settings.config, &[rent]))
            } else {
                None
            };
            EntryProjection {
                extend_to_healthy_cost_stroops: extend,
                restore_cost_stroops: restore,
            }
        })
        .collect()
}

/// Base64 XDR of a ledger key (used in JSON output).
pub fn key_xdr(key: &stellar_xdr::LedgerKey) -> String {
    key.to_xdr_base64(Limits::none()).unwrap_or_default()
}
