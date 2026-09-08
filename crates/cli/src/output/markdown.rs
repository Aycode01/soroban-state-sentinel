//! Markdown output for `scan --markdown`.

use crate::commands::scan::ScanReport;

/// Emit a Markdown report to stdout.
pub fn emit(report: &ScanReport) {
    let result = &report.result;
    println!("# TTL health scan: `{}`", report.contract_id);
    println!();
    println!(
        "- Network: `{}` (protocol {})",
        result.network.passphrase, result.network.protocol_version
    );
    println!("- Latest ledger: {}", result.latest_ledger);
    println!(
        "- Ledger close time: {}s ({})",
        result.ledger_close_seconds,
        if result.ledger_close_seconds == sentinel_ttl_scanner::DEFAULT_LEDGER_CLOSE_SECONDS {
            "default assumption"
        } else {
            "explicit"
        }
    );
    println!(
        "- fee_per_rent_1kb: {} stroops/1KB ({})",
        report.rent_settings.fee_per_rent_1kb,
        report.rent_settings.fee_per_rent_1kb_source.as_str()
    );
    println!();
    println!("## Summary");
    println!();
    let s = &result.summary;
    println!(
        "| Band | Count |\n| --- | --- |\n| Healthy | {} |\n| ExpiringSoon | {} |\n| Critical | {} |\n| Archived | {} |\n| **Total** | **{}** |",
        s.healthy, s.expiring_soon, s.critical, s.archived, s.entries_scanned
    );
    println!();
    println!("## Entries");
    println!();
    println!(
        "| id | kind | durability | band | ledgers left | days left | size (B) | extend cost (stroops) | restore cost (stroops) |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    for (e, p) in result.entries.iter().zip(report.projections.iter()) {
        println!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            e.id,
            e.kind.as_str(),
            e.durability
                .map(|d| match d {
                    stellar_xdr::ContractDataDurability::Persistent => "persistent",
                    stellar_xdr::ContractDataDurability::Temporary => "temporary",
                })
                .unwrap_or("—"),
            e.band.as_str(),
            e.ledgers_remaining
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
            e.days_remaining
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
            e.size_bytes
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
            p.extend_to_healthy_cost_stroops
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
            p.restore_cost_stroops
                .map(|v| v.to_string())
                .unwrap_or_else(|| "—".to_string()),
        );
    }
    println!();
}
