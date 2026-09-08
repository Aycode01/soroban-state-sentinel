# Mock RPC for testing

The integration tests (`tests/integration_test.rs`) never touch a live network.
They spawn the compiled CLI binary and drive it against a local mock Soroban RPC
server (`tests/mock_rpc.rs`) — a bare `tokio::net::TcpListener` speaking
HTTP/1.1, with no extra dependencies — seeded with fixtures captured from the
live testnet on 2026-09-08.

## How the harness works

`MockRpc` has three building blocks, all chained before `start()`:

```rust
let url = mock_rpc::MockRpc::from_fixtures(FIXTURES)   // getLatestLedger.json + getNetwork.json
    .with_config_fixture(FIXTURES)                     // getLedgerEntries_config.json (the 7 CONFIG_SETTING keys)
    .with_entry(key_b64, entry_json)                   // synthetic entries, keyed by base64 LedgerKey
    .start()                                           // binds 127.0.0.1:0, returns the base URL
    .await;
```

It serves three JSON-RPC methods:

- `getLatestLedger` — from `getLatestLedger.json` (fixture latest ledger:
  4,566,959; `closeTime` 1788858382).
- `getNetwork` — from `getNetwork.json` (passphrase
  `Test SDF Network ; September 2015`, protocol 28).
- `getLedgerEntries` — looks each requested key up in the entry map and returns
  the ones present. Missing keys are simply absent from `entries` (which is how
  the CLI sees an archived entry).

The config fixture (`getLedgerEntries_config.json`) carries the real
protocol-28 settings: `max_entry_ttl` 3,110,400, `min_persistent_ttl` 120,960,
`min_temporary_ttl` 720, `fee_write_ledger_entry` 2,500,
`fee_write_1kb` 875, `persistent_rent_rate_denominator` 1,215 — byte-identical
to the live values the tool read during the verification pass
(`docs/live-verification.md`).

## Seeding a new scenario

Two kinds of entries exist in the harness:

1. **Fixture entries** — real testnet responses, read from `tests/fixtures/`.
   The `Counter` instance fixture (`getLedgerEntries_counter_instance.json`)
   returns `entries: []`, which is the archived-instance case.
2. **Synthetic entries** — built in the test with real `stellar-xdr` types and
   registered via `with_entry`. The testnet has no contract sitting in each
   band at a predictable TTL, so tests synthesize entries at Healthy /
   ExpiringSoon / Critical TTLs. The shapes are real XDR; only the TTL values
   are chosen by the test.

To seed a new scenario, build the entry with `stellar-xdr`, encode it to base64,
and register it under the base64 `LedgerKey` the CLI will request:

```rust
let (key, entry) = instance_entry_json(contract_bytes(COUNTER_CONTRACT), wasm_hash, latest + 600_000);
mock = mock.with_entry(key, entry);
```

`liveUntilLedgerSeq` is the lever: `latest + 600_000` is Healthy
(> 518,400), `latest + 200_000` is ExpiringSoon, `latest + 5_000` is Critical,
and a missing entry is Archived.

## Historical bugs this harness caught

Each of these was a real failure mode; keep them in mind when adding scenarios.

1. **Binary path resolution.** The integration tests spawn
   `target/debug/soroban-state-sentinel` and assert it exists — the binary must
   be built first (`cargo build -p sentinel-cli`). CI builds before testing.
   A test that assumes the binary exists without building it fails at spawn
   time with a confusing "not found" error.

2. **Tokio runtime flavor.** The mock server runs on the test runtime's worker
   threads, while the test blocks on `Command::output()` waiting for the CLI to
   finish talking to it. Tests therefore must use
   `#[tokio::test(flavor = "multi_thread")]` — a single-threaded runtime
   deadlocks, because the mock can never accept a connection while the test
   thread is blocked.

3. **Fixture key mismatch.** The CLI decodes the `C…` strkey it is given and
   scans under exactly those bytes. Mock entries must be keyed under the same
   decoded bytes or the scan reports every entry as archived. Tests use the
   `contract_bytes()` helper to derive the raw 32 bytes from the strkey before
   building keys — never hardcode a key that does not match the contract id the
   CLI will actually decode.

4. **Missing account entry.** `restore` (and `extend`) with `--source-account`
   fetch the account's ledger entry to compute the next sequence number. A test
   that requests an envelope must seed the account entry too
   (`with_entry(account_key(SOURCE_ACCOUNT), account_entry_json(...))`) or the
   command fails with "source account … not found".

A fifth related fix lives in the scanner itself: the contract code entry is
fetched in its own RPC round (round 2), because the wasm hash is only known
after the instance entry returns — looking it up in the round-1 response always
reports it as archived. New scan scenarios must seed both the instance and the
code entry for live-contract cases.