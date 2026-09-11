# Mock RPC for Testing

Integration tests in `tests/` validate CLI subcommands against an in-process mock Soroban RPC server (`tests/mock_rpc.rs`).

## Mock Server Architecture

`MockRpc` binds a `tokio::net::TcpListener` to an ephemeral local port (`127.0.0.1:0`) and speaks HTTP/1.1 JSON-RPC. It requires no external network access or third-party mock crates.

It responds to three core Soroban RPC methods:
- **`getLatestLedger`**: Returns ledger sequence, protocol version, and close time.
- **`getNetwork`**: Returns network passphrase and protocol version.
- **`getLedgerEntries`**: Matches base64-encoded `LedgerKey` strings in incoming request params against registered fixture entries.

## Seeding a Test Fixture Scenario

To add a new integration test scenario:

1. **Load Base Fixtures**:
   ```rust
   let mock = MockRpc::from_fixtures("tests/fixtures")
       .with_config_fixture("tests/fixtures");
   ```
2. **Register Synthetic Entries**:
   Add custom ledger entries using base64 `LedgerKey` strings and raw JSON entry representations:
   ```rust
   let mock = mock.with_entry(key_b64, entry_json_str);
   ```
3. **Start the Mock Server**:
   ```rust
   let rpc_url = mock.start().await;
   ```
4. **Execute CLI Binary**:
   Pass `--rpc-url <rpc_url>` to the compiled CLI binary and assert stdout, stderr, exit code, or XDR output files.

## Historical Bugs Caught by the Mock Harness

The mock RPC test harness serves as a regression prevention system. Past bugs caught include:

1. **Binary Path Resolution**: Integration tests panicked if the CLI binary path (`target/debug/soroban-state-sentinel`) was not compiled prior to test execution. The harness now verifies binary existence before spawning subprocesses.
2. **Tokio Runtime Flavor**: Mismatches between CLI async runtime macros (`#[tokio::main]`) and test multithreaded runtimes caused hanging socket connections.
3. **Fixture Key Mismatch**: Discrepancies between expected base64 `LedgerKey::ContractData` XDR bytes and test key strings led to unhandled entry lookup misses.
4. **Missing Account Entry**: Requesting `--source-account` for an account not present in `getLedgerEntries` resulted in unhandled RPC null responses during sequence fetching.