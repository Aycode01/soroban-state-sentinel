# `scan`

The `scan` subcommand queries Soroban RPC for a contract's instance, code, and optional storage entries, evaluating each entry's TTL health against defined thresholds.

## Command Syntax

```bash
soroban-state-sentinel scan <CONTRACT_ID> [FLAGS]
```

## Options & Arguments

### Positional Arguments

- **`<CONTRACT_ID>`** (`String`): Contract ID strkey (`C...`) to scan.

### Command Flags

- **`--keys <SCVAL_BASE64>`** (`Vec<String>`): Explicit storage keys to scan, each a base64-XDR-encoded `SCVal`. Flag can be repeated.
- **`--durability <DURABILITY>`** (`Enum`, default: `persistent`): Storage durability for `--keys` entries. Allowed values: `persistent`, `temporary`.
- **`--healthy-days <DAYS>`** (`u32`, default: `30`): Lower bound in days for `Healthy` band classification.
- **`--critical-days <DAYS>`** (`u32`, default: `7`): Upper bound in days for `Critical` band classification.
- **`--extend-horizon-ledgers <LEDGERS>`** (`Option<u32>`): Horizon in ledgers used for per-entry extension cost projections. Defaults to `healthy_min_ledgers`.
- **`--fail-on-critical`** (`bool`, default: `false`): Exit with code `1` if any entry is classified as `critical` or `archived`. Used in automated keeper pipelines.
- **`--json`** (`bool`): Output formatted JSON document matching Schema Version `1.1.0`. Conflicts with `--markdown` and `--table`.
- **`--markdown`** (`bool`): Output Github-flavored markdown table. Conflicts with `--json` and `--table`.
- **`--table`** (`bool`): Output formatted terminal table (default format). Conflicts with `--json` and `--markdown`.

### Global Network Flags

- **`--rpc-url <URL>`** (`String`, default: `https://soroban-testnet.stellar.org`): Soroban RPC endpoint URL.
- **`--ledger-close-seconds <SECONDS>`** (`u64`, default: `5`): Average ledger close duration in seconds.
- **`--average-state-size-bytes <BYTES>`** (`Option<i64>`): Explicit average live state size used to derive `fee_per_rent_1kb`.
- **`--rent-fee-per-1kb <STROOPS>`** (`Option<i64>`): Explicit rent write fee per 1KB override in stroops.
- **`--ttl-entry-size <BYTES>`** (`u32`, default: `48`): Protocol size of a TTL ledger entry in bytes.

## Execution Trigger Context

- **Manual Maintainer Checks**: Run without flags or with `--table`/`--markdown` to visually inspect contract state.
- **Automated Keeper Runs**: Run with `--json --fail-on-critical` inside cron jobs or CI actions to trigger alerts on exit code `1`.

## Exit Codes

| Exit Code | Meaning |
| --- | --- |
| `0` | Success. If `--fail-on-critical` is set, no entry is in `critical` or `archived` band. |
| `1` | Action required: `--fail-on-critical` is set **and** at least one entry is `critical` or `archived`. |
| `2` | Usage or operational error (bad arguments, invalid strkey/SCVal, RPC network failure). |

## Real Verbatim Output Example

Verbatim execution transcript from `docs/live-verification.md`:

```console
$ soroban-state-sentinel scan CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --json
{
  "schema_version": "1.1.0",
  "generated_at_unix": 1788863098,
  "command": {
    "subcommand": "scan",
    "contract_id": "CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX",
    "rpc_url": "https://soroban-testnet.stellar.org"
  },
  "network": {
    "passphrase": "Test SDF Network ; September 2015",
    "protocol_version": 28,
    "latest_ledger": 4567902,
    "ledger_close_seconds": 5,
    "ledger_close_seconds_source": "default",
    "fee_per_rent_1kb": 10000,
    "fee_per_rent_1kb_source": "state_size_high",
    "average_soroban_state_size_bytes": null,
    "max_entry_ttl": 3110400,
    "min_persistent_ttl": 120960,
    "min_temporary_ttl": 720
  },
  "health_config": {
    "healthy_min_days": 30,
    "critical_max_days": 7,
    "healthy_min_ledgers": 518400,
    "critical_max_ledgers": 120960,
    "extend_horizon_ledgers": 518400
  },
  "summary": {
    "entries_scanned": 2,
    "healthy": 0,
    "expiring_soon": 0,
    "critical": 2,
    "archived": 0,
    "has_critical": true
  },
  "entries": [
    {
      "id": "instance",
      "label": "contract instance",
      "kind": "contract_instance",
      "durability": "persistent",
      "band": "critical",
      "current_ledger_seq": 4567902,
      "live_until_ledger_seq": 4622384,
      "ledgers_remaining": 54482,
      "days_remaining": 3,
      "estimated_archive_unix": 1789135507,
      "size_bytes": 148,
      "key_xdr": "AAAABgAAAAGTAPCEgsK/xOh+GYNr2Z6hFRMNNW1l6I/IiCKjxsmbdgAAABQAAAAB",
      "ttl_key_xdr": "AAAACcWgnJwr45IDVOxVIu9PgS2V2jGkKWALzxVBIykA5Pov",
      "extend_to_healthy_cost_stroops": 554400,
      "restore_cost_stroops": null
    },
    {
      "id": "code",
      "label": "contract code (wasm)",
      "kind": "contract_code",
      "durability": null,
      "band": "critical",
      "current_ledger_seq": 4567902,
      "live_until_ledger_seq": 4622384,
      "ledgers_remaining": 54482,
      "days_remaining": 3,
      "estimated_archive_unix": 1789135507,
      "size_bytes": 19532,
      "key_xdr": "AAAAB+W/9PsT2YvjuECxd1ML8+P4wp6jTZgZiHsG6mpmywSi",
      "ttl_key_xdr": "AAAACZZQ4IxyJVaxFP3DOIfpWKt9rCPWqwJ0o/01f+Cldhmo",
      "extend_to_healthy_cost_stroops": 24279287,
      "restore_cost_stroops": null
    }
  ]
}
```