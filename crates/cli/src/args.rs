//! CLI argument definitions.
//!
//! Exit codes (documented in README.md):
//!
//! - `0` — success (and, with `--fail-on-critical`, no Critical/Archived entries).
//! - `1` — `--fail-on-critical` triggered (some entry is Critical or Archived).
//! - `2` — usage or operational error (bad arguments, RPC failure, bad XDR, …).

use clap::{Parser, Subcommand};

/// Default Soroban RPC endpoint (public testnet).
pub const DEFAULT_RPC_URL: &str = "https://soroban-testnet.stellar.org";

#[derive(Debug, Parser)]
#[command(
    name = "soroban-state-sentinel",
    version,
    about = "Monitor Soroban contracts for TTL/state-archival risk and generate unsigned remediation XDR.\n\nRead-only by design: this tool never signs transactions or touches a private key.",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Scan a contract's ledger entries and classify TTL health.
    Scan(ScanArgs),
    /// Build unsigned RestoreFootprint XDR for archived entries.
    Restore(RestoreArgs),
}

/// Shared RPC/network flags.
#[derive(Debug, clap::Args)]
pub struct CommonArgs {
    /// Soroban RPC endpoint URL.
    #[arg(long, default_value = DEFAULT_RPC_URL, global = true)]
    pub rpc_url: String,

    /// Average ledger close time in seconds, used to convert ledgers to days.
    /// RPC does not expose the actual average, so this is an explicit input
    /// (the Stellar target is 5s).
    #[arg(long, default_value_t = 5, global = true)]
    pub ledger_close_seconds: u64,

    /// Explicit average live Soroban state size in bytes, used to compute
    /// fee_per_rent_1kb. When absent, the state-size-high plateau rate is used
    /// and the output labels the assumption.
    #[arg(long, global = true)]
    pub average_state_size_bytes: Option<i64>,

    /// Explicit fee per 1KB of rented space (stroops). Overrides both
    /// --average-state-size-bytes and the default plateau assumption.
    #[arg(long, global = true)]
    pub rent_fee_per_1kb: Option<i64>,

    /// Size of a TTL entry in bytes (protocol constant; default 48 from
    /// soroban-env-host, overridable).
    #[arg(long, default_value_t = 48, global = true)]
    pub ttl_entry_size: u32,
}

#[derive(Debug, clap::Args)]
pub struct ScanArgs {
    /// Contract id to scan (C… strkey).
    pub contract_id: String,

    /// Explicit storage keys to scan, each a base64-XDR-encoded SCVal. Repeatable.
    #[arg(long = "keys", value_name = "SCVAL_BASE64")]
    pub keys: Vec<String>,

    /// Durability to assume for --keys entries.
    #[arg(long, value_enum, default_value_t = DurabilityArg::Persistent)]
    pub durability: DurabilityArg,

    /// Healthy lower bound, in days (entries with more ledgers left are Healthy).
    #[arg(long, default_value_t = 30)]
    pub healthy_days: u32,

    /// Critical upper bound, in days (entries with fewer ledgers left are Critical).
    #[arg(long, default_value_t = 7)]
    pub critical_days: u32,

    /// Horizon (ledgers from now) used for the per-entry extend-cost projection.
    /// Defaults to the healthy threshold in ledgers.
    #[arg(long)]
    pub extend_horizon_ledgers: Option<u32>,

    /// Exit with code 1 if any entry is Critical or Archived.
    #[arg(long)]
    pub fail_on_critical: bool,

    /// Output as JSON (machine-readable; schema documented in SCHEMA.md).
    #[arg(long, conflicts_with_all = ["markdown", "table"])]
    pub json: bool,
    /// Output as Markdown.
    #[arg(long, conflicts_with_all = ["json", "table"])]
    pub markdown: bool,
    /// Output as a terminal table (default).
    #[arg(long, conflicts_with_all = ["json", "markdown"])]
    pub table: bool,

    #[command(flatten)]
    pub common: CommonArgs,
}

#[derive(Debug, clap::Args)]
pub struct RestoreArgs {
    /// Contract id whose archived entries to restore (C… strkey).
    pub contract_id: String,

    /// Storage keys to restore, each a base64-XDR-encoded SCVal. Repeatable.
    /// When omitted, the contract instance (+ its code, if discoverable) is restored.
    #[arg(long = "keys", value_name = "SCVAL_BASE64")]
    pub keys: Vec<String>,

    /// Durability to assume for --keys entries.
    #[arg(long, value_enum, default_value_t = DurabilityArg::Persistent)]
    pub durability: DurabilityArg,

    /// Output path for the unsigned XDR (writes base64).
    #[arg(long, value_name = "FILE")]
    pub output: String,

    /// Public key (G…) of the account that will sign and submit. When given,
    /// the output is a complete unsigned TransactionV1Envelope (account sequence
    /// number fetched from the ledger). When omitted, the output is the raw
    /// operations, one base64 per line.
    #[arg(long)]
    pub source_account: Option<String>,

    /// Transaction fee override in stroops (otherwise estimated).
    #[arg(long)]
    pub fee: Option<u64>,

    /// Account sequence number override (otherwise fetched from the ledger).
    #[arg(long)]
    pub sequence: Option<i64>,

    /// Assumed size in bytes per archived entry when its live entry cannot be
    /// fetched (restore fee is then an estimate; core charges the actual rent at
    /// apply time and refunds the unused refundable fee).
    #[arg(long, default_value_t = 1024)]
    pub assumed_archived_entry_size: u32,

    #[command(flatten)]
    pub common: CommonArgs,
}

/// Output format selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Markdown,
    Table,
}

/// Durability argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum DurabilityArg {
    Persistent,
    Temporary,
}

impl ScanArgs {
    /// Resolve the effective output format from the mutually-exclusive flags.
    pub fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if self.markdown {
            OutputFormat::Markdown
        } else {
            OutputFormat::Table
        }
    }
}
