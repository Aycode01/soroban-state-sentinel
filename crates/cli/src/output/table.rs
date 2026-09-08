//! Terminal table output for `scan` (default format).

use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, Table};

use crate::commands::scan::ScanReport;

/// Emit a terminal table to stdout.
pub fn emit(report: &ScanReport) {
    let result = &report.result;

    println!(
        "contract {} — {} (protocol {})",
        report.contract_id, result.network.passphrase, result.network.protocol_version
    );
    println!(
        "latest ledger {} · ledger close {}s{} · fee_per_rent_1kb {} ({})",
        result.latest_ledger,
        result.ledger_close_seconds,
        if result.ledger_close_seconds == sentinel_ttl_scanner::DEFAULT_LEDGER_CLOSE_SECONDS {
            " (default assumption)"
        } else {
            ""
        },
        report.rent_settings.fee_per_rent_1kb,
        report.rent_settings.fee_per_rent_1kb_source.as_str()
    );
    println!();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL).set_header(vec![
        "id",
        "kind",
        "durability",
        "band",
        "ledgers left",
        "days left",
        "size (B)",
        "extend cost",
        "restore cost",
    ]);

    for (e, p) in result.entries.iter().zip(report.projections.iter()) {
        let band_color = match e.band {
            sentinel_ttl_scanner::HealthBand::Healthy => Color::Green,
            sentinel_ttl_scanner::HealthBand::ExpiringSoon => Color::Yellow,
            sentinel_ttl_scanner::HealthBand::Critical => Color::Red,
            sentinel_ttl_scanner::HealthBand::Archived => Color::Magenta,
        };
        table.add_row(vec![
            Cell::new(&e.id),
            Cell::new(e.kind.as_str()),
            Cell::new(
                e.durability
                    .map(|d| match d {
                        stellar_xdr::ContractDataDurability::Persistent => "persistent",
                        stellar_xdr::ContractDataDurability::Temporary => "temporary",
                    })
                    .unwrap_or("—"),
            ),
            Cell::new(e.band.as_str()).fg(band_color),
            Cell::new(
                e.ledgers_remaining
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".to_string()),
            ),
            Cell::new(
                e.days_remaining
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".to_string()),
            ),
            Cell::new(
                e.size_bytes
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".to_string()),
            ),
            Cell::new(
                p.extend_to_healthy_cost_stroops
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".to_string()),
            ),
            Cell::new(
                p.restore_cost_stroops
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "—".to_string()),
            ),
        ]);
    }

    println!("{table}");

    let s = &result.summary;
    println!();
    println!(
        "summary: {} healthy · {} expiring_soon · {} critical · {} archived · has_critical={}",
        s.healthy, s.expiring_soon, s.critical, s.archived, s.has_critical
    );
}
