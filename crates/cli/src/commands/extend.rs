//! `extend` subcommand: build unsigned `ExtendFootprintTtl` XDR for entries
//! approaching archival and write it to a file.
//!
//! This is the proactive remedy for the `expiring_soon` / `critical` bands that
//! `scan --fail-on-critical` surfaces: instead of waiting for an entry to be
//! archived (the `restore` case), extend its TTL *before* that happens.
//!
//! The output is **unsigned by design** and mirrors `restore` exactly: with
//! `--source-account` the output is a complete `TransactionV1Envelope` (account
//! sequence number fetched from the ledger, fee estimated) with empty
//! signatures; without it, the raw operations, one base64-XDR per line.
//!
//! ## `extendTo` semantics
//!
//! `--extend-to <LEDGERS>` is a **duration in ledgers measured from the current
//! ledger**, matching the protocol: Stellar Core applies
//! `liveUntilLedgerSeq = currentLedger + extendTo` (see
//! `ExtendFootprintTTLOpFrame.cpp`). `--extend-to-days <N>` is a convenience
//! form that resolves N days to ledgers with the same ledger-close-time logic
//! `ttl-scanner` uses for day conversion, and labels the close time as
//! `default` (the 5s target) or `explicit` — never a silent assumption.

use sentinel_rent_model::{extend_stroop_cost, EntryForRent};
use sentinel_rpc_client::{LedgerEntryInfo, RpcClient};
use sentinel_ttl_scanner::health::{days_to_ledgers, DaysConversion};
use sentinel_ttl_scanner::DEFAULT_LEDGER_CLOSE_SECONDS;
use sentinel_xdr_builder::{
    build_extend_ttl_ops, build_unsigned_envelope, decode_contract_id, decode_muxed_account,
    unsigned_envelope_xdr_base64, validate_extend_to, BatchLimits, KeyEntry,
};
use stellar_xdr::{
    ContractDataDurability, ContractExecutable, ContractId, LedgerEntryData, LedgerKey,
    LedgerKeyContractCode, LedgerKeyContractData, Limits, ScAddress, ScVal, WriteXdr,
};

use crate::args::{DurabilityArg, ExtendArgs};
use crate::commands::scan::parse_storage_key;
use crate::commands::{estimate_total_fee, fetch_next_sequence, CliError, Outcome};
use crate::context::{build_rent_settings, with_current_ledger};

/// How the extension horizon was specified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtendTargetSource {
    /// `--extend-to <ledgers>` was given directly.
    Ledgers,
    /// `--extend-to-days <days>` was resolved via the ledger close time.
    Days,
}

/// A resolved extension horizon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedExtendTo {
    /// Duration in ledgers from the current ledger (the op's `extendTo` field).
    ///
    /// # Units
    ///
    /// Ledgers, measured from the current ledger (core computes
    /// `liveUntilLedgerSeq = current + extendTo`).
    pub extend_to: u32,
    /// How the horizon was specified.
    pub source: ExtendTargetSource,
    /// Ledger close time used, when the days path was taken.
    ///
    /// # Units
    ///
    /// Seconds per ledger.
    pub ledger_close_seconds: Option<u64>,
}

/// Label a ledger close time as `default` (the 5s Stellar target) or
/// `explicit`, matching how `scan` labels it in its JSON output.
///
/// # Units
///
/// `ledger_close_seconds` in seconds per ledger.
pub fn close_time_source(ledger_close_seconds: u64) -> &'static str {
    if ledger_close_seconds == DEFAULT_LEDGER_CLOSE_SECONDS {
        "default"
    } else {
        "explicit"
    }
}

/// Resolve the extension horizon from the CLI flags.
///
/// Exactly one of `--extend-to` / `--extend-to-days` is present (clap enforces
/// it); the days path reuses `ttl-scanner`'s `days_to_ledgers` so there is a
/// single close-time assumption in the codebase.
///
/// # Units
///
/// `extend_to` in ledgers, `days` in days, `ledger_close_seconds` in seconds
/// per ledger. Returns the duration in ledgers from the current ledger.
pub fn resolve_extend_to(
    extend_to: Option<u32>,
    extend_to_days: Option<u32>,
    ledger_close_seconds: u64,
) -> Result<ResolvedExtendTo, CliError> {
    match (extend_to, extend_to_days) {
        (Some(ledgers), None) => Ok(ResolvedExtendTo {
            extend_to: ledgers,
            source: ExtendTargetSource::Ledgers,
            ledger_close_seconds: None,
        }),
        (None, Some(days)) => {
            if days == 0 {
                return Err(CliError::msg("--extend-to-days must be > 0"));
            }
            if ledger_close_seconds == 0 {
                return Err(CliError::msg("--ledger-close-seconds must be > 0"));
            }
            let ledgers = match days_to_ledgers(days, ledger_close_seconds) {
                Some(DaysConversion::Ledgers(l)) => l,
                Some(DaysConversion::Overflow) => {
                    return Err(CliError::msg(format!(
                        "--extend-to-days {days} exceeds u32::MAX ledgers at {ledger_close_seconds}s/ledger"
                    )))
                }
                None => return Err(CliError::msg("--ledger-close-seconds must be > 0")),
            };
            Ok(ResolvedExtendTo {
                extend_to: ledgers,
                source: ExtendTargetSource::Days,
                ledger_close_seconds: Some(ledger_close_seconds),
            })
        }
        // clap enforces conflicts/requirements, so these are unreachable; keep
        // them explicit so the match is total.
        (Some(_), Some(_)) | (None, None) => Err(CliError::msg(
            "exactly one of --extend-to or --extend-to-days is required",
        )),
    }
}

/// Run the extend command.
pub async fn run(cmd: &ExtendArgs) -> Result<Outcome, CliError> {
    let rpc = RpcClient::new(&cmd.common.rpc_url)?;
    let contract_id = decode_contract_id(&cmd.contract_id)?;

    let network_config = rpc.fetch_network_config().await?;
    let latest = rpc.get_latest_ledger().await?;
    let mut rent_settings = build_rent_settings(&network_config, &cmd.common)?;
    rent_settings = with_current_ledger(rent_settings, latest.sequence);

    // --- resolve and validate the extension horizon ---
    let resolved = resolve_extend_to(
        cmd.extend_to,
        cmd.extend_to_days,
        cmd.common.ledger_close_seconds,
    )?;
    // Core rejects extendTo > max_entry_ttl - 1 as malformed; 0 is rejected as
    // a no-op extension. Both surface before any XDR is written.
    validate_extend_to(
        resolved.extend_to,
        network_config.state_archival.max_entry_ttl,
    )?;

    // --- determine the keys to extend ---
    let mut keys: Vec<LedgerKey> = Vec::new();
    for raw in &cmd.keys {
        let scval = parse_storage_key(raw)?;
        keys.push(LedgerKey::ContractData(LedgerKeyContractData {
            contract: ScAddress::Contract(ContractId(contract_id.clone())),
            key: scval,
            durability: match cmd.durability {
                DurabilityArg::Persistent => ContractDataDurability::Persistent,
                DurabilityArg::Temporary => ContractDataDurability::Temporary,
            },
        }));
    }

    let mut discovered_code_key: Option<LedgerKey> = None;
    if keys.is_empty() {
        // No explicit keys: extend the contract instance + its code (when the
        // instance is still readable and points at wasm).
        let instance_key = sentinel_ttl_scanner::instance_key(contract_id.clone());
        keys.push(instance_key.clone());
        let instance_entry = rpc.get_entry(&instance_key).await?;
        if let Some(entry) = instance_entry {
            if let LedgerEntryData::ContractData(cd) = &entry {
                if let ScVal::ContractInstance(instance) = &cd.val {
                    if let ContractExecutable::Wasm(hash) = &instance.executable {
                        let code_key =
                            LedgerKey::ContractCode(LedgerKeyContractCode { hash: hash.clone() });
                        discovered_code_key = Some(code_key);
                    }
                }
            }
        }
        if let Some(code_key) = discovered_code_key {
            keys.push(code_key);
        }
    }

    if keys.is_empty() {
        return Err(CliError::msg(
            "nothing to extend: no --keys given and no discoverable instance/code",
        ));
    }

    // --- sizes + live-until from one batched fetch ---
    // A single getLedgerEntries call gives both the XDR size (for footprint
    // accounting) and liveUntilLedgerSeq (for the rent projection). Keys with
    // no live entry are archived and use the assumed size; core skips archived
    // entries at apply time and refunds the unused refundable fee.
    let resp = rpc.get_ledger_entries(&keys).await?;
    let by_key: std::collections::HashMap<LedgerKey, LedgerEntryInfo> = resp
        .entries
        .iter()
        .map(|info| (info.key.clone(), info.clone()))
        .collect();

    let mut key_entries: Vec<KeyEntry> = Vec::new();
    let mut rent_entries: Vec<EntryForRent> = Vec::new();
    for key in &keys {
        let info = by_key.get(key);
        let (size_bytes, live_until, durability) = match info {
            Some(i) => (
                u32::try_from(i.entry.to_xdr(Limits::none()).map(|b| b.len()).unwrap_or(0))
                    .unwrap_or(u32::MAX),
                i.live_until_ledger_seq.filter(|seq| *seq > 0),
                match &i.entry {
                    LedgerEntryData::ContractData(cd) => cd.durability,
                    _ => ContractDataDurability::Persistent,
                },
            ),
            None => (
                cmd.assumed_archived_entry_size,
                None,
                ContractDataDurability::Persistent,
            ),
        };
        key_entries.push(KeyEntry {
            key: key.clone(),
            size_bytes,
        });
        rent_entries.push(EntryForRent {
            is_persistent: durability != ContractDataDurability::Temporary,
            is_code_entry: matches!(key, LedgerKey::ContractCode(_)),
            size_bytes,
            live_until_ledger_seq: live_until,
        });
    }

    // --- build the operations, batched to the network's footprint limits ---
    let limits = BatchLimits::from_network_config(
        &network_config.ledger_cost,
        &network_config.ledger_cost_ext,
        cmd.common.ttl_entry_size,
    )?;
    let ops = build_extend_ttl_ops(&key_entries, resolved.extend_to, &limits)?;

    // --- fee estimate ---
    let rent_fee = extend_stroop_cost(&rent_settings.config, &rent_entries, resolved.extend_to);
    let fee_rates = rpc.fetch_fee_rates().await?;
    let estimated_fee = estimate_total_fee(
        cmd.fee,
        cmd.common.ttl_entry_size,
        &ops,
        &key_entries,
        rent_fee,
        &fee_rates,
    )?;

    // --- output (mirror restore) ---
    match &cmd.source_account {
        Some(source) => {
            let muxed = decode_muxed_account(source)?;
            let seq_num = match cmd.sequence {
                Some(s) => s,
                None => fetch_next_sequence(&rpc, source, &cmd.common.rpc_url).await?,
            };
            let envelope = build_unsigned_envelope(muxed, seq_num, estimated_fee, ops)?;
            let b64 = unsigned_envelope_xdr_base64(&envelope)?;
            std::fs::write(&cmd.output, format!("{b64}\n"))?;
            let op_count = match &envelope {
                stellar_xdr::TransactionEnvelope::Tx(env) => env.tx.operations.len(),
                other => {
                    return Err(CliError::msg(format!(
                        "unexpected envelope kind: {other:?}"
                    )))
                }
            };
            println!(
                "wrote unsigned ExtendFootprintTtl envelope ({} op(s), est. fee {} stroops) to {}",
                op_count, estimated_fee, cmd.output
            );
            println!(
                "source account: {} · sequence: {} · signatures: EMPTY (sign with your own key)",
                source, seq_num
            );
        }
        None => {
            let mut out = String::new();
            for op in &ops {
                let b64 = op.to_xdr_base64(Limits::none())?;
                out.push_str(&b64);
                out.push('\n');
            }
            std::fs::write(&cmd.output, out)?;
            println!(
                "wrote {} unsigned ExtendFootprintTtl operation(s) (base64 XDR, one per line) to {}",
                ops.len(),
                cmd.output
            );
            println!(
                "no --source-account given: re-run with --source-account to get a complete unsigned envelope"
            );
        }
    }

    // --- resolution provenance (never a silent assumption) ---
    match resolved.source {
        ExtendTargetSource::Ledgers => {
            println!(
                "extend target: {} ledgers from the current ledger (ledger {})",
                resolved.extend_to, latest.sequence
            );
        }
        ExtendTargetSource::Days => {
            let close = resolved
                .ledger_close_seconds
                .unwrap_or(cmd.common.ledger_close_seconds);
            println!(
                "extend target: {} ledgers from ledger {} ({} day(s) at {}s/ledger, close-time source: {})",
                resolved.extend_to,
                latest.sequence,
                cmd.extend_to_days.unwrap_or(0),
                close,
                close_time_source(close)
            );
        }
    }

    println!(
        "fee note: rent fee est. {} stroops (entries with no live entry use assumed size {} B; core charges actual rent at apply and refunds unused refundable fee)",
        rent_fee, cmd.assumed_archived_entry_size
    );

    Ok(Outcome::Ok)
}
