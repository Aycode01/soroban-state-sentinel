//! Output formatting for `scan`: JSON (stable schema, see SCHEMA.md), Markdown,
//! and a terminal table.

pub mod json;
pub mod markdown;
pub mod table;

use crate::args::OutputFormat;
use crate::commands::scan::ScanReport;
use crate::commands::CliError;

/// Emit a scan report in the requested format to stdout.
pub fn emit_scan(report: &ScanReport, format: OutputFormat) -> Result<(), CliError> {
    match format {
        OutputFormat::Json => {
            let doc = json::ScanJson::from_report(report);
            println!("{}", serde_json::to_string_pretty(&doc)?);
        }
        OutputFormat::Markdown => markdown::emit(report),
        OutputFormat::Table => table::emit(report),
    }
    Ok(())
}
