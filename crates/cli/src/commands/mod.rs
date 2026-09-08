//! Command implementations.

pub mod extend;
pub mod restore;
pub mod scan;

use sentinel_rent_model::{
    compute_transaction_resource_fee, FeeConfiguration, TransactionResources,
};
use sentinel_rpc_client::{FeeRates, RpcClient};
use sentinel_xdr_builder::{build_unsigned_envelope, decode_account_id, KeyEntry};
use stellar_xdr::{LedgerEntryData, LedgerKey, LedgerKeyAccount, Limits, WriteXdr};

/// Outcome of a command run; drives the process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Success; exit 0.
    Ok,
    /// `--fail-on-critical` triggered; exit 1.
    FailOnCritical,
}

/// CLI-level error: wraps every error type the commands produce.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0}")]
    Msg(String),

    #[error("RPC error: {0}")]
    Rpc(#[from] sentinel_rpc_client::RpcError),

    #[error("scan error: {0}")]
    Scan(#[from] sentinel_ttl_scanner::ScannerError),

    #[error("XDR build error: {0}")]
    Build(#[from] sentinel_xdr_builder::BuildError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XDR error: {0}")]
    Xdr(#[from] stellar_xdr::Error),
}

impl CliError {
    /// Convenience constructor for plain-message errors.
    pub fn msg(msg: impl Into<String>) -> Self {
        CliError::Msg(msg.into())
    }
}

/// Base fee per operation in stroops (Stellar base fee).
const BASE_FEE_PER_OP: u64 = 100;

/// Fetch the account's next sequence number from the ledger.
///
/// Shared by `restore` and `extend`: both may emit a complete unsigned
/// `TransactionV1Envelope`, which needs the account's next sequence number.
pub(crate) async fn fetch_next_sequence(
    rpc: &RpcClient,
    source: &str,
    rpc_url: &str,
) -> Result<i64, CliError> {
    let account_id = decode_account_id(source)?;
    let key = LedgerKey::Account(LedgerKeyAccount { account_id });
    let entry = rpc.get_entry(&key).await?;
    let Some(entry) = entry else {
        return Err(CliError::msg(format!(
            "source account {source} not found on {rpc_url} — fund it first (e.g. via friendbot)"
        )));
    };
    match entry {
        LedgerEntryData::Account(account) => Ok(account.seq_num.0.saturating_add(1)),
        other => Err(CliError::msg(format!(
            "expected account entry for {source}, got {other:?}"
        ))),
    }
}

/// Estimate the total transaction fee: base fee + resource fee + rent fee.
///
/// Shared by `restore` and `extend`; the caller supplies the rent fee computed
/// from its own action's rent changes.
///
/// # Units
///
/// Result in stroops (`u64`).
pub(crate) fn estimate_total_fee(
    fee_override: Option<u64>,
    ttl_entry_size: u32,
    ops: &[stellar_xdr::Operation],
    key_entries: &[KeyEntry],
    rent_fee: i64,
    fee_rates: &FeeRates,
) -> Result<u32, CliError> {
    if let Some(explicit) = fee_override {
        let base = BASE_FEE_PER_OP.saturating_mul(ops.len() as u64);
        if explicit < base {
            eprintln!(
                "warning: --fee {explicit} is below the base fee {base} for {} operation(s); the transaction may be rejected",
                ops.len()
            );
        }
        return u32::try_from(explicit)
            .map_err(|_| CliError::msg("--fee exceeds u32::MAX stroops"));
    }

    let mut read_entries: u64 = 0;
    let mut write_entries: u64 = 0;
    let mut read_bytes: u64 = 0;
    let mut write_bytes: u64 = 0;
    for ke in key_entries {
        // TTL entries are read+written alongside each data/code entry.
        read_entries += 2;
        write_entries += 2;
        read_bytes =
            read_bytes.saturating_add(u64::from(ke.size_bytes) + u64::from(ttl_entry_size));
        write_bytes =
            write_bytes.saturating_add(u64::from(ke.size_bytes) + u64::from(ttl_entry_size));
    }
    let fee_config = FeeConfiguration {
        fee_per_instruction_increment: fee_rates.fee_per_instruction_increment,
        fee_per_disk_read_entry: fee_rates.fee_per_disk_read_entry,
        fee_per_write_entry: fee_rates.fee_per_write_entry,
        fee_per_disk_read_1kb: fee_rates.fee_per_disk_read_1kb,
        fee_per_write_1kb: fee_rates.fee_per_write_1kb,
        fee_per_historical_1kb: fee_rates.fee_per_historical_1kb,
        fee_per_contract_event_1kb: fee_rates.fee_per_contract_event_1kb,
        fee_per_transaction_size_1kb: fee_rates.fee_per_transaction_size_1kb,
    };

    // Two-pass transaction-size estimate: build the envelope with fee=0 to
    // measure its XDR length (the fee field is fixed-width, so the size is
    // correct), then compute the fee with that size.
    let probe = build_unsigned_envelope(
        stellar_xdr::MuxedAccount::Ed25519(stellar_xdr::Uint256([0u8; 32])),
        0,
        0,
        ops.to_vec(),
    )?;
    let tx_size = probe
        .to_xdr(Limits::none())
        .map(|b| u32::try_from(b.len()).unwrap_or(u32::MAX))
        .unwrap_or(0);

    let resources = TransactionResources {
        instructions: 0, // neither action executes wasm
        disk_read_entries: u32::try_from(read_entries).unwrap_or(u32::MAX),
        write_entries: u32::try_from(write_entries).unwrap_or(u32::MAX),
        disk_read_bytes: u32::try_from(read_bytes).unwrap_or(u32::MAX),
        write_bytes: u32::try_from(write_bytes).unwrap_or(u32::MAX),
        contract_events_size_bytes: 0,
        transaction_size_bytes: tx_size,
    };
    let (non_refundable, _refundable) = compute_transaction_resource_fee(&resources, &fee_config);

    let base = BASE_FEE_PER_OP.saturating_mul(ops.len() as u64);
    let total = base
        .saturating_add(non_refundable.max(0) as u64)
        .saturating_add(rent_fee.max(0) as u64);
    u32::try_from(total).map_err(|_| CliError::msg("estimated fee exceeds u32::MAX stroops"))
}
