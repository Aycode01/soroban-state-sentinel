//! End-to-end integration tests.
//!
//! These spawn the compiled `soroban-state-sentinel` binary and drive it against
//! a local mock Soroban RPC server (`mock_rpc.rs`) seeded with fixtures captured
//! from the live testnet (see `tests/fixtures/README.md`) plus synthetic
//! live-contract entries built with the real `stellar-xdr` types.
//!
//! The binary must be built first (`cargo build`), which CI does before running
//! tests.

mod mock_rpc;

use std::path::PathBuf;
use std::process::{Command, Output};

use stellar_xdr::{
    ContractDataDurability, ContractExecutable, ContractId, Hash, LedgerEntryData, LedgerKey,
    LedgerKeyContractCode, LedgerKeyContractData, Limits, ReadXdr, ScAddress, ScContractInstance,
    ScSymbol, ScVal, WriteXdr,
};

/// The docs `Counter` contract id used in the archived-instance scenario.
const COUNTER_CONTRACT: &str = "CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI";
/// A throwaway testnet account public key.
const SOURCE_ACCOUNT: &str = "GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5";
/// Directory containing the fixture JSON files.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// Path to the compiled CLI binary.
fn binary() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../target/debug/soroban-state-sentinel");
    assert!(
        path.exists(),
        "CLI binary not found at {path:?} — run `cargo build` first"
    );
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("spawn soroban-state-sentinel")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn json_of(output: &Output) -> serde_json::Value {
    serde_json::from_str(&stdout_of(output)).expect("scan --json output parses as JSON")
}

/// Build the base64 `LedgerKey::ContractData` for a synthetic contract storage key.
fn data_key(contract_bytes: [u8; 32], key: ScVal, durability: ContractDataDurability) -> String {
    let key = LedgerKey::ContractData(LedgerKeyContractData {
        contract: ScAddress::Contract(ContractId(Hash(contract_bytes))),
        key,
        durability,
    });
    key.to_xdr_base64(Limits::none()).expect("encode key")
}

/// Build the base64 `LedgerKey::ContractCode` for a synthetic wasm hash.
fn code_key(hash: [u8; 32]) -> String {
    let key = LedgerKey::ContractCode(LedgerKeyContractCode { hash: Hash(hash) });
    key.to_xdr_base64(Limits::none()).expect("encode key")
}

/// Build a `getLedgerEntries` entry object for a synthetic contract-data entry.
fn data_entry_json(
    contract_bytes: [u8; 32],
    key: ScVal,
    durability: ContractDataDurability,
    val: ScVal,
    live_until: u32,
) -> (String, String) {
    let ledger_key = data_key(contract_bytes, key.clone(), durability);
    let data = LedgerEntryData::ContractData(stellar_xdr::ContractDataEntry {
        ext: stellar_xdr::ExtensionPoint::V0,
        contract: ScAddress::Contract(ContractId(Hash(contract_bytes))),
        key,
        durability,
        val,
    });
    let xdr = data.to_xdr_base64(Limits::none()).expect("encode entry");
    let entry = serde_json::json!({
        "key": ledger_key,
        "xdr": xdr,
        "lastModifiedLedgerSeq": 4_500_000,
        "liveUntilLedgerSeq": live_until,
    });
    (ledger_key, entry.to_string())
}

/// A synthetic contract-instance entry whose code is wasm with the given hash.
fn instance_entry_json(contract_bytes: [u8; 32], wasm_hash: [u8; 32], live_until: u32) -> (String, String) {
    let instance = ScVal::ContractInstance(ScContractInstance {
        executable: ContractExecutable::Wasm(Hash(wasm_hash)),
        storage: None,
    });
    data_entry_json(
        contract_bytes,
        ScVal::LedgerKeyContractInstance,
        ContractDataDurability::Persistent,
        instance,
        live_until,
    )
}

#[tokio::test]
async fn scan_reports_archived_docs_counter_contract() {
    let url = mock_rpc::MockRpc::from_fixtures(FIXTURES)
        .with_config_fixture(FIXTURES)
        .start()
        .await;

    let out = run(&[
        "scan",
        COUNTER_CONTRACT,
        "--rpc-url",
        &url,
        "--json",
    ]);
    assert!(
        out.status.success(),
        "scan failed: {}",
        stderr_of(&out)
    );
    let doc = json_of(&out);

    assert_eq!(doc["schema_version"], "1.0.0");
    assert_eq!(doc["network"]["protocol_version"], 28);
    assert_eq!(doc["summary"]["entries_scanned"], 1);
    assert_eq!(doc["summary"]["archived"], 1);
    assert!(doc["summary"]["has_critical"].as_bool().unwrap());
    assert_eq!(doc["entries"][0]["id"], "instance");
    assert_eq!(doc["entries"][0]["band"], "archived");
    assert!(doc["entries"][0]["restore_cost_stroops"].as_i64().unwrap() > 0);
    assert!(doc["entries"][0]["extend_to_healthy_cost_stroops"].is_null());
}

#[tokio::test]
async fn scan_classifies_live_entries_into_bands() {
    let contract = [0xABu8; 32];
    let wasm_hash = [0xCDu8; 32];

    let mut mock = mock_rpc::MockRpc::from_fixtures(FIXTURES).with_config_fixture(FIXTURES);

    // Latest ledger in the fixtures is 4_566_959.
    let latest = 4_566_959u32;

    // Instance: ~34.7 days left (600_000 ledgers) -> Healthy (> 518_400).
    let (instance_key, instance_entry) = instance_entry_json(contract, wasm_hash, latest + 600_000);
    mock = mock.with_entry(instance_key, instance_entry);

    // Code: ~11.6 days left (200_000 ledgers) -> ExpiringSoon.
    let code_key = code_key(wasm_hash);
    let code_data = LedgerEntryData::ContractCode(stellar_xdr::ContractCodeEntry {
        ext: stellar_xdr::ContractCodeEntryExt::V0,
        hash: Hash(wasm_hash),
        code: stellar_xdr::BytesM::try_from(vec![0u8; 32]).expect("bytes"),
    });
    let code_entry_json = serde_json::json!({
        "key": code_key,
        "xdr": code_data.to_xdr_base64(Limits::none()).expect("encode"),
        "lastModifiedLedgerSeq": 4_500_000,
        "liveUntilLedgerSeq": latest + 200_000,
    });
    mock = mock.with_entry(code_key, code_entry_json.to_string());

    // Explicit key "counter": ~0.29 days left (5_000 ledgers) -> Critical.
    let counter_val = ScVal::I32(42);
    let (counter_key, counter_entry) = data_entry_json(
        contract,
        ScVal::Symbol(ScSymbol::try_from("counter").expect("symbol")),
        ContractDataDurability::Persistent,
        counter_val,
        latest + 5_000,
    );
    mock = mock.with_entry(counter_key, counter_entry);

    let url = mock.start().await;

    // Base64 SCVal for the "counter" key, as the CLI's --keys expects.
    let counter_scval = ScVal::Symbol(ScSymbol::try_from("counter").expect("symbol"));
    let counter_scval_b64 = counter_scval.to_xdr_base64(Limits::none()).expect("encode");
    // A second explicit key that has no entry -> archived.
    let missing_scval = ScVal::Symbol(ScSymbol::try_from("gone").expect("symbol"));
    let missing_scval_b64 = missing_scval.to_xdr_base64(Limits::none()).expect("encode");
    let _ = counter_key;

    let out = run(&[
        "scan",
        COUNTER_CONTRACT, // the CLI just needs a valid C… strkey; the mock keys are synthetic
        "--rpc-url",
        &url,
        "--keys",
        &counter_scval_b64,
        "--keys",
        &missing_scval_b64,
        "--json",
    ]);
    assert!(
        out.status.success(),
        "scan failed: {}",
        stderr_of(&out)
    );
    let doc = json_of(&out);

    let bands: Vec<&str> = doc["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["band"].as_str().unwrap())
        .collect();
    assert_eq!(bands, vec!["healthy", "expiring_soon", "critical", "archived"]);

    let entries = doc["entries"].as_array().unwrap();
    // Healthy instance already beyond the horizon: extend cost 0.
    assert_eq!(entries[0]["extend_to_healthy_cost_stroops"], 0);
    // ExpiringSoon code needs an extension.
    assert!(entries[1]["extend_to_healthy_cost_stroops"].as_i64().unwrap() > 0);
    assert!(entries[1]["restore_cost_stroops"].is_null());
    // Critical key needs an extension too.
    assert!(entries[2]["extend_to_healthy_cost_stroops"].as_i64().unwrap() > 0);
    // Archived key can only be restored.
    assert!(entries[3]["extend_to_healthy_cost_stroops"].is_null());
    assert!(entries[3]["restore_cost_stroops"].as_i64().unwrap() > 0);

    assert_eq!(doc["summary"]["healthy"], 1);
    assert_eq!(doc["summary"]["expiring_soon"], 1);
    assert_eq!(doc["summary"]["critical"], 1);
    assert_eq!(doc["summary"]["archived"], 1);
    assert!(doc["summary"]["has_critical"].as_bool().unwrap());
}

#[tokio::test]
async fn fail_on_critical_sets_exit_code() {
    let url = mock_rpc::MockRpc::from_fixtures(FIXTURES)
        .with_config_fixture(FIXTURES)
        .start()
        .await;

    // The docs Counter contract is archived -> fail-on-critical must exit 1.
    let out = run(&[
        "scan",
        COUNTER_CONTRACT,
        "--rpc-url",
        &url,
        "--fail-on-critical",
    ]);
    assert_eq!(out.status.code(), Some(1));

    // A scan of a live, healthy contract exits 0 even with --fail-on-critical.
    let (k, e) = instance_entry_json([0x11u8; 32], [0x22u8; 32], 4_566_959 + 900_000);
    let url2 = mock_rpc::MockRpc::from_fixtures(FIXTURES)
        .with_config_fixture(FIXTURES)
        .with_entry(k, e)
        .start()
        .await;
    let out2 = run(&[
        "scan",
        COUNTER_CONTRACT,
        "--rpc-url",
        &url2,
        "--fail-on-critical",
    ]);
    assert_eq!(out2.status.code(), Some(0));
}

#[tokio::test]
async fn restore_writes_unsigned_envelope_with_source_account() {
    let url = mock_rpc::MockRpc::from_fixtures(FIXTURES)
        .with_config_fixture(FIXTURES)
        .start()
        .await;

    let out_path = std::env::temp_dir().join("sentinel_unsigned_env.xdr");
    let out = run(&[
        "restore",
        COUNTER_CONTRACT,
        "--rpc-url",
        &url,
        "--source-account",
        SOURCE_ACCOUNT,
        "--output",
        out_path.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "restore failed: {}", stderr_of(&out));

    let xdr = std::fs::read_to_string(&out_path).expect("read output");
    let xdr = xdr.trim();
    let envelope = stellar_xdr::TransactionEnvelope::from_xdr_base64(xdr, Limits::none())
        .expect("output parses as a transaction envelope");
    match &envelope {
        stellar_xdr::TransactionEnvelope::Tx(env) => {
            assert_eq!(env.signatures.len(), 0, "must be unsigned");
            assert_eq!(env.tx.operations.len(), 1);
            assert!(
                matches!(
                    env.tx.operations.first().map(|o| &o.body),
                    Some(stellar_xdr::OperationBody::RestoreFootprint(_))
                ),
                "expected a RestoreFootprint operation"
            );
        }
        other => panic!("unexpected envelope: {other:?}"),
    }
}

#[tokio::test]
async fn restore_writes_raw_operations_without_source_account() {
    let url = mock_rpc::MockRpc::from_fixtures(FIXTURES)
        .with_config_fixture(FIXTURES)
        .start()
        .await;

    let out_path = std::env::temp_dir().join("sentinel_ops.xdr");
    let out = run(&[
        "restore",
        COUNTER_CONTRACT,
        "--rpc-url",
        &url,
        "--output",
        out_path.to_str().unwrap(),
    ]);
    assert!(out.status.success(), "restore failed: {}", stderr_of(&out));

    let content = std::fs::read_to_string(&out_path).expect("read output");
    let ops: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(ops.len(), 1);
    let op = stellar_xdr::Operation::from_xdr_base64(ops[0], Limits::none())
        .expect("operation XDR parses");
    assert!(matches!(
        op.body,
        stellar_xdr::OperationBody::RestoreFootprint(_)
    ));
}

