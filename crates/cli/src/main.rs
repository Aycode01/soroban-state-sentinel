#![allow(clippy::result_large_err)] // error enums wrap large source errors; acceptable for a CLI

//! `soroban-state-sentinel` — monitor Soroban contracts for TTL/state-archival
//! risk and generate unsigned remediation XDR.
//!
//! Exit codes:
//! - `0` — success (and, with `--fail-on-critical`, no Critical/Archived entries).
//! - `1` — `--fail-on-critical` triggered.
//! - `2` — usage or operational error.

mod args;
mod commands;
mod context;
mod output;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = args::Cli::parse();
    let outcome = match &cli.command {
        args::Commands::Scan(cmd) => commands::scan::run(cmd).await,
        args::Commands::Extend(cmd) => commands::extend::run(cmd).await,
        args::Commands::Restore(cmd) => commands::restore::run(cmd).await,
    };

    match outcome {
        Ok(commands::Outcome::Ok) => std::process::exit(0),
        Ok(commands::Outcome::FailOnCritical) => {
            eprintln!("fail-on-critical: at least one entry is Critical or Archived");
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    }
}
