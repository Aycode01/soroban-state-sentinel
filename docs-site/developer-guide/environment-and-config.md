# Environment and configuration

## Environment variables

There are **none**. The tool reads no environment variables anywhere in the
codebase (`std::env` is unused outside tests). Every knob is a CLI flag with an
explicit default or an explicit requirement. This is deliberate: a monitoring
tool's behavior should be visible in its invocation, not hidden in the
environment.

## Configuration flags

### Global flags (all subcommands)

These are `global = true` in the `clap` definitions (`crates/cli/src/args.rs`),
so they can appear before or after the subcommand.

| Flag | Example | Omitted behavior |
| --- | --- | --- |
| `--rpc-url <URL>` | `--rpc-url https://soroban-testnet.stellar.org` | Defaults to the public testnet (`DEFAULT_RPC_URL` in `args.rs`). |
| `--ledger-close-seconds <N>` | `--ledger-close-seconds 7` | Defaults to `5` (the Stellar target). The output labels the value `default` vs `explicit`, so the assumption is never silent. |
| `--average-state-size-bytes <N>` | `--average-state-size-bytes 3000000000` | When absent, `fee_per_rent_1kb` falls back to the state-size-high plateau; the output labels the path `state_size_high`. |
| `--rent-fee-per-1kb <N>` | `--rent-fee-per-1kb 10000` | When absent, resolution proceeds to `--average-state-size-bytes`, then the plateau. Overrides both when present (`explicit`). |
| `--ttl-entry-size <N>` | `--ttl-entry-size 48` | Defaults to `48` (the protocol constant from `soroban-env-host`, overridable). |

`fee_per_rent_1kb` resolution precedence: `--rent-fee-per-1kb` →
`--average-state-size-bytes` → state-size-high plateau. See
[Economics of rent](../economics-of-rent.md) for why this limitation exists.

### `scan`

| Flag | Example | Omitted behavior |
| --- | --- | --- |
| `contract_id` (positional) | `CCJQB4…MXIEX` | Required; clap exits 2 without it. |
| `--keys <SCVAL_BASE64>` | `--keys AAAAAQAAAAA…` (repeatable) | No explicit keys: scan covers the contract instance + code (discovered from the instance's wasm hash). |
| `--durability <persistent\|temporary>` | `--durability temporary` | Defaults to `persistent`. Only affects `--keys` entries. |
| `--healthy-days <N>` | `--healthy-days 60` | Defaults to `30`. |
| `--critical-days <N>` | `--critical-days 3` | Defaults to `7`. |
| `--extend-horizon-ledgers <N>` | `--extend-horizon-ledgers 1000000` | Defaults to the healthy threshold in ledgers (518,400 at defaults). |
| `--fail-on-critical` | flag | Off. When set, exit 1 if any entry is `critical` or `archived`. |
| `--json` / `--markdown` / `--table` | flag | Default output is the terminal table. The three are mutually exclusive. |

### `extend`

| Flag | Example | Omitted behavior |
| --- | --- | --- |
| `contract_id` (positional) | `CCJQB4…MXIEX` | Required. |
| `--extend-to <LEDGERS>` | `--extend-to 518400` | Required unless `--extend-to-days` is given. Conflicts with `--extend-to-days`. |
| `--extend-to-days <DAYS>` | `--extend-to-days 30` | Conflicts with `--extend-to`. Resolved to ledgers via the ledger close time; output labels it `default` vs `explicit`. |
| `--output <FILE>` | `--output /tmp/extend.xdr` | Required; clap exits 2 without it. |
| `--keys <SCVAL_BASE64>` | repeatable | When omitted, the instance (+ code, if discoverable) is extended. |
| `--source-account <G…>` | `--source-account GDM2X…ZPC5` | Omitted: raw operations, one base64 per line. Given: a complete unsigned `TransactionV1Envelope`. |
| `--fee <stroops>` | `--fee 1000000` | Omitted: fee estimated from base + resource + rent components. |
| `--sequence <N>` | `--sequence 19614875122663425` | Omitted: next sequence fetched from the ledger. |
| `--assumed-archived-entry-size <N>` | `--assumed-archived-entry-size 2048` | Defaults to `1024`. |

### `restore`

| Flag | Example | Omitted behavior |
| --- | --- | --- |
| `contract_id` (positional) | `CCPYZF…FNCI` | Required. |
| `--output <FILE>` | `--output /tmp/restore.xdr` | Required. |
| `--keys <SCVAL_BASE64>` | repeatable | When omitted, the instance (+ code, if discoverable) is restored. |
| `--source-account <G…>` | `--source-account GDM2X…ZPC5` | Omitted: raw operations, one base64 per line. Given: a complete unsigned `TransactionV1Envelope`. |
| `--fee <stroops>` | `--fee 1000000` | Omitted: fee estimated. |
| `--sequence <N>` | `--sequence 19614875122663425` | Omitted: next sequence fetched from the ledger. |
| `--assumed-archived-entry-size <N>` | `--assumed-archived-entry-size 2048` | Defaults to `1024`. |

## What the tool reads from the network instead

Values that are not CLI inputs come from the live network on every run: the
network passphrase and protocol version (`getNetwork`), the latest ledger
(`getLatestLedger`), and the fee/TTL config settings (`getLedgerEntries` on the
`CONFIG_SETTING` keys — `ContractLedgerCostV0`, `ContractLedgerCostExtV0`,
`StateArchival`, `ContractComputeV0`, `ContractHistoricalDataV0`,
`ContractEventsV0`, `ContractBandwidthV0`). On the protocol-28 testnet these
included `max_entry_ttl` 3,110,400, `min_persistent_ttl` 120,960,
`min_temporary_ttl` 720, `fee_write_ledger_entry` 2,500, and
`fee_write_1kb` 875. Nothing here is hardcoded; every assumption the tool does
make is labeled in its output.