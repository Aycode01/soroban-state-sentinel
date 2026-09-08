//! The stable JSON output schema for `scan --json`.
//!
//! Versioned in `SCHEMA.md` as **schema v1**. Any breaking change to this shape
//! requires a version bump there — repo 3 (`action-state-watch`) parses this
//! exact document.

use serde::Serialize;

use crate::commands::scan::key_xdr;
use crate::commands::scan::{EntryProjection, ScanReport};
use crate::context::FeeSource;

/// Current schema version. Bump (breaking) or extend (non-breaking) per SCHEMA.md.
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Top-level scan document (schema v1).
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ScanJson {
    pub schema_version: String,
    pub generated_at_unix: u64,
    pub command: CommandInfo,
    pub network: NetworkInfo,
    pub health_config: HealthConfigInfo,
    pub summary: Summary,
    pub entries: Vec<EntryJson>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct CommandInfo {
    pub subcommand: String,
    pub contract_id: String,
    pub rpc_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkInfo {
    pub passphrase: String,
    pub protocol_version: u32,
    pub latest_ledger: u32,
    pub ledger_close_seconds: u64,
    /// `default` when the 5s target was assumed, `explicit` when supplied.
    pub ledger_close_seconds_source: &'static str,
    pub fee_per_rent_1kb: i64,
    pub fee_per_rent_1kb_source: &'static str,
    pub average_soroban_state_size_bytes: Option<i64>,
    pub max_entry_ttl: u32,
    pub min_persistent_ttl: u32,
    pub min_temporary_ttl: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct HealthConfigInfo {
    pub healthy_min_days: u32,
    pub critical_max_days: u32,
    pub healthy_min_ledgers: u32,
    pub critical_max_ledgers: u32,
    pub extend_horizon_ledgers: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Summary {
    pub entries_scanned: usize,
    pub healthy: usize,
    pub expiring_soon: usize,
    pub critical: usize,
    pub archived: usize,
    pub has_critical: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct EntryJson {
    pub id: String,
    pub label: String,
    pub kind: &'static str,
    pub durability: Option<&'static str>,
    pub band: &'static str,
    pub current_ledger_seq: u32,
    pub live_until_ledger_seq: Option<u32>,
    pub ledgers_remaining: Option<u32>,
    pub days_remaining: Option<u64>,
    pub estimated_archive_unix: Option<u64>,
    pub size_bytes: Option<u32>,
    pub key_xdr: String,
    pub ttl_key_xdr: String,
    pub extend_to_healthy_cost_stroops: Option<i64>,
    pub restore_cost_stroops: Option<i64>,
}

fn durability_name(d: Option<stellar_xdr::ContractDataDurability>) -> Option<&'static str> {
    match d {
        Some(stellar_xdr::ContractDataDurability::Persistent) => Some("persistent"),
        Some(stellar_xdr::ContractDataDurability::Temporary) => Some("temporary"),
        None => None,
    }
}

impl ScanJson {
    /// Build a schema-v1 document from a scan report.
    pub fn from_report(report: &ScanReport) -> Self {
        let result = &report.result;
        let generated_at_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let ledger_close_seconds_source = if report.result.ledger_close_seconds
            == sentinel_ttl_scanner::DEFAULT_LEDGER_CLOSE_SECONDS
        {
            "default"
        } else {
            "explicit"
        };

        let health = &result.health_config;
        let entries = result
            .entries
            .iter()
            .zip(report.projections.iter())
            .map(|(e, p)| entry_json(e, p, result.latest_ledger))
            .collect();

        ScanJson {
            schema_version: SCHEMA_VERSION.to_string(),
            generated_at_unix,
            command: CommandInfo {
                subcommand: "scan".to_string(),
                contract_id: report.contract_id.clone(),
                rpc_url: report.rpc_url.clone(),
            },
            network: NetworkInfo {
                passphrase: result.network.passphrase.clone(),
                protocol_version: result.network.protocol_version,
                latest_ledger: result.latest_ledger,
                ledger_close_seconds: result.ledger_close_seconds,
                ledger_close_seconds_source,
                fee_per_rent_1kb: report.rent_settings.fee_per_rent_1kb,
                fee_per_rent_1kb_source: match report.rent_settings.fee_per_rent_1kb_source {
                    FeeSource::Explicit => "explicit",
                    FeeSource::AverageStateSize => "average_state_size",
                    FeeSource::StateSizeHigh => "state_size_high",
                },
                average_soroban_state_size_bytes: report.rent_settings.average_state_size_bytes,
                max_entry_ttl: report.network_config.state_archival.max_entry_ttl,
                min_persistent_ttl: report.network_config.state_archival.min_persistent_ttl,
                min_temporary_ttl: report.network_config.state_archival.min_temporary_ttl,
            },
            health_config: HealthConfigInfo {
                healthy_min_days: report.healthy_min_days,
                critical_max_days: report.critical_max_days,
                healthy_min_ledgers: health.healthy_min_ledgers,
                critical_max_ledgers: health.critical_max_ledgers,
                extend_horizon_ledgers: health.healthy_min_ledgers,
            },
            summary: Summary {
                entries_scanned: result.summary.entries_scanned,
                healthy: result.summary.healthy,
                expiring_soon: result.summary.expiring_soon,
                critical: result.summary.critical,
                archived: result.summary.archived,
                has_critical: result.summary.has_critical,
            },
            entries,
        }
    }
}

fn entry_json(
    e: &sentinel_ttl_scanner::ScannedEntry,
    p: &EntryProjection,
    current_ledger_seq: u32,
) -> EntryJson {
    EntryJson {
        id: e.id.clone(),
        label: e.label.clone(),
        kind: e.kind.as_str(),
        durability: durability_name(e.durability),
        band: e.band.as_str(),
        current_ledger_seq,
        live_until_ledger_seq: e.live_until_ledger_seq,
        ledgers_remaining: e.ledgers_remaining,
        days_remaining: e.days_remaining,
        estimated_archive_unix: e.estimated_archive_unix,
        size_bytes: e.size_bytes,
        key_xdr: key_xdr(&e.key),
        ttl_key_xdr: key_xdr(&e.ttl_key),
        extend_to_healthy_cost_stroops: p.extend_to_healthy_cost_stroops,
        restore_cost_stroops: p.restore_cost_stroops,
    }
}
