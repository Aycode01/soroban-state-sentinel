//! `restore` subcommand: build unsigned `RestoreFootprint` XDR for archived
//! entries and write it to a file.
//!
//! The output is **unsigned by design**. With `--source-account`, the output is
//! a complete `TransactionV1Envelope` (account sequence number fetched from the
//! ledger, fee estimated) with empty signatures — a separately-held key signs
//! and submits it. Without `--source-account`, the output is the raw operations,
//! one base64-XDR per line.
//!
//! ## Fee estimate caveat
//!
//! For archived entries the live entry is unreadable over RPC, so its size (and
//! therefore the exact rent fee) is unknown. The CLI uses
//! `--assumed-archived-entry-size` (default 1024 bytes) for the estimate and
//! labels it as such; Stellar Core charges the actual rent at apply time and
//! refunds the unused refundable fee, so a conservative estimate is safe. Use
//! `--fee` to override the total transaction fee entirely.

use sentinel_rent_model::{restore_stroop_cost, EntryForRent};
use sentinel_rpc_client::RpcClient;
use sentinel_xdr_builder::{
    build_restore_ops, build_unsigned_envelope, decode_contract_id, decode_muxed_account,
    unsigned_envelope_xdr_base64, BatchLimits, KeyEntry,
};
use stellar_xdr::{
    ContractDataDurability, ContractId, LedgerEntryData, LedgerKey, LedgerKeyContractCode,
    LedgerKeyContractData, Limits, ScAddress, ScVal, WriteXdr,
};

use crate::args::{DurabilityArg, RestoreArgs};
use crate::commands::scan::parse_storage_key;
use crate::commands::{estimate_total_fee, fetch_next_sequence, CliError, Outcome};
use crate::context::{build_rent_settings, with_current_ledger};

/// Run the restore command.
pub async fn run(cmd: &RestoreArgs) -> Result<Outcome, CliError> {
    let rpc = RpcClient::new(&cmd.common.rpc_url)?;
    let contract_id = decode_contract_id(&cmd.contract_id)?;

    let network_config = rpc.fetch_network_config().await?;
    let latest = rpc.get_latest_ledger().await?;
    let mut rent_settings = build_rent_settings(&network_config, &cmd.common)?;
    rent_settings = with_current_ledger(rent_settings, latest.sequence);

    // --- determine the keys to restore ---
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
        // No explicit keys: restore the contract instance + its code (when the
        // instance is still readable and points at wasm).
        let instance_key = sentinel_ttl_scanner::instance_key(contract_id.clone());
        keys.push(instance_key.clone());
        let instance_entry = rpc.get_entry(&instance_key).await?;
        if let Some(entry) = instance_entry {
            if let LedgerEntryData::ContractData(cd) = &entry {
                if let ScVal::ContractInstance(instance) = &cd.val {
                    if let stellar_xdr::ContractExecutable::Wasm(hash) = &instance.executable {
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
            "nothing to restore: no --keys given and no discoverable instance/code",
        ));
    }

    // --- sizes: live entries are exact; archived entries use the assumption ---
    let mut key_entries: Vec<KeyEntry> = Vec::new();
    for key in &keys {
        let entry = rpc.get_entry(key).await?;
        let size_bytes = match &entry {
            Some(e) => u32::try_from(e.to_xdr(Limits::none()).map(|b| b.len()).unwrap_or(0))
                .unwrap_or(u32::MAX),
            None => cmd.assumed_archived_entry_size,
        };
        key_entries.push(KeyEntry {
            key: key.clone(),
            size_bytes,
        });
    }

    // --- build the operations, batched to the network's footprint limits ---
    let limits = BatchLimits::from_network_config(
        &network_config.ledger_cost,
        &network_config.ledger_cost_ext,
        cmd.common.ttl_entry_size,
    )?;
    let ops = build_restore_ops(&key_entries, &limits)?;

    // --- fee estimate ---
    let rent_entries: Vec<EntryForRent> = key_entries
        .iter()
        .map(|ke| EntryForRent {
            is_persistent: true, // only persistent entries can be restored
            is_code_entry: matches!(ke.key, LedgerKey::ContractCode(_)),
            size_bytes: ke.size_bytes,
            live_until_ledger_seq: None, // archived
        })
        .collect();
    let rent_fee = restore_stroop_cost(&rent_settings.config, &rent_entries);

    let fee_rates = rpc.fetch_fee_rates().await?;
    let estimated_fee = estimate_total_fee(
        cmd.fee,
        cmd.common.ttl_entry_size,
        &ops,
        &key_entries,
        rent_fee,
        &fee_rates,
    )?;

    // --- output ---
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
                "wrote unsigned RestoreFootprint envelope ({} op(s), est. fee {} stroops) to {}",
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
                "wrote {} unsigned RestoreFootprint operation(s) (base64 XDR, one per line) to {}",
                ops.len(),
                cmd.output
            );
            println!(
                "no --source-account given: re-run with --source-account to get a complete unsigned envelope"
            );
        }
    }

    println!(
        "fee note: rent fee est. {} stroops (archived entries use assumed size {} B; core charges actual rent at apply and refunds unused refundable fee)",
        rent_fee, cmd.assumed_archived_entry_size
    );

    Ok(Outcome::Ok)
}
