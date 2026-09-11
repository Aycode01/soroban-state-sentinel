# Environment & Configuration

`soroban-state-sentinel` is configured via command-line flags defined globally or per subcommand.

## Global Configuration Flags

Global flags apply to `scan`, `extend`, and `restore`.

| Flag Name | Value Type | Default Value | Example Value | Behavior When Omitted |
| --- | --- | --- | --- | --- |
| `--rpc-url` | `String` | `https://soroban-testnet.stellar.org` | `https://mainnet.stellar.org` | Queries public testnet RPC endpoint. |
| `--ledger-close-seconds` | `u64` | `5` | `6` | Assumes target 5s close time; output labels source as `"default"`. When supplied, output labels source as `"explicit"`. |
| `--average-state-size-bytes` | `i64` | `None` | `50000000` | Uses maximum state size plateau rate (`rent_fee_1kb_state_size_high`) for rent calculations. |
| `--rent-fee-per-1kb` | `i64` | `None` | `10000` | Overrides `--average-state-size-bytes` and plateau default with explicit rate. |
| `--ttl-entry-size` | `u32` | `48` | `48` | Uses protocol default 48 bytes for `LedgerKey::Ttl` size calculations. |

## Subcommand Flags

### `scan` Specific Flags

| Flag Name | Value Type | Default Value | Behavior When Omitted |
| --- | --- | --- | --- |
| `--healthy-days` | `u32` | `30` | Healthy band threshold is set to 30 days. |
| `--critical-days` | `u32` | `7` | Critical band threshold is set to 7 days. |
| `--extend-horizon-ledgers` | `u32` | `healthy_min_ledgers` | Sets cost projection horizon to `--healthy-days` equivalent. |
| `--fail-on-critical` | `bool` | `false` | Exits code 0 regardless of health bands. |
| `--json` | `bool` | `false` | Outputs human-readable terminal table. |

### `extend` Specific Flags

| Flag Name | Value Type | Default Value | Behavior When Omitted |
| --- | --- | --- | --- |
| `--extend-to` | `u32` | Required unless `--extend-to-days` | Must specify extension target. |
| `--extend-to-days` | `u32` | Required unless `--extend-to` | Converts days to ledgers using `--ledger-close-seconds`. |
| `--output` | `String` | Required | Writes XDR output to target file path. |
| `--source-account` | `String` | `None` | Writes raw base64 operations (one per line). When given, writes complete `TransactionV1Envelope`. |
| `--assumed-archived-entry-size` | `u32` | `1024` | Uses 1024 bytes for unreadable archived entry fee estimates. |

### `restore` Specific Flags

| Flag Name | Value Type | Default Value | Behavior When Omitted |
| --- | --- | --- | --- |
| `--output` | `String` | Required | Writes XDR output to target file path. |
| `--source-account` | `String` | `None` | Writes raw base64 operations. When given, writes complete `TransactionV1Envelope`. |
| `--assumed-archived-entry-size` | `u32` | `1024` | Uses 1024 bytes for unreadable archived entry fee estimates. |