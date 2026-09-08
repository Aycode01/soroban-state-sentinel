# `scan`

```text
soroban-state-sentinel scan <contract-id> [flags]
```

Scans a contract's ledger entries (instance + code + explicit storage keys) and
classifies each into a health band. This is the read-and-report command; it
never writes XDR and never signs anything.

## Flags

All flag names, types, and defaults below are from the `clap` definitions in
`crates/cli/src/args.rs`.

| Flag | Type | Default | Notes |
| --- | --- | --- | --- |
| `contract_id` | positional | — | Contract id (`C…` strkey). Required. |
| `--keys <SCVAL_BASE64>` | repeatable | — | Explicit storage keys to scan, each a base64-XDR-encoded `SCVal`. |
| `--durability <persistent\|temporary>` | enum | `persistent` | Durability assumed for `--keys` entries. |
| `--healthy-days <N>` | u32 | `30` | Healthy lower bound in days. Entries with more ledgers left are Healthy. |
| `--critical-days <N>` | u32 | `7` | Critical upper bound in days. Entries with fewer ledgers left are Critical. |
| `--extend-horizon-ledgers <N>` | u32 | — | Horizon (ledgers from now) for the per-entry extend-cost projection. Defaults to the healthy threshold in ledgers (518,400 at defaults). |
| `--fail-on-critical` | flag | off | Exit with code 1 if any entry is Critical or Archived. |
| `--json` | flag | off | Machine-readable JSON output (the stable schema — see [JSON schema reference](../json-schema-reference.md)). |
| `--markdown` | flag | off | Markdown table output. |
| `--table` | flag | off | Terminal table output (the default). |

`--json`, `--markdown`, and `--table` are mutually exclusive; the default is the
terminal table.

### Global flags (shared by all subcommands)

| Flag | Type | Default | Notes |
| --- | --- | --- | --- |
| `--rpc-url <URL>` | string | `https://soroban-testnet.stellar.org` | Soroban RPC endpoint. |
| `--ledger-close-seconds <N>` | u64 | `5` | Average ledger close time in seconds; used to convert ledgers to days. RPC does not expose the actual average, so an explicit value is labeled `explicit` in the output. |
| `--average-state-size-bytes <N>` | i64 | — | Explicit average live Soroban state size in bytes, used to compute `fee_per_rent_1kb`. |
| `--rent-fee-per-1kb <N>` | i64 | — | Explicit fee per 1 KB of rented space (stroops). Overrides both `--average-state-size-bytes` and the default plateau. |
| `--ttl-entry-size <N>` | u32 | `48` | Size of a TTL entry in bytes (protocol constant, overridable). |

## What triggers each flag

- **A maintainer checking one contract by hand** runs
  `scan <contract-id>` and reads the terminal table. They pass `--keys` only for
  specific persistent data entries they care about, because the scan cannot
  enumerate a contract's full key set (RPC has no "list all keys" method).
- **A keeper in a scheduled pipeline** runs
  `scan <contract-id> --fail-on-critical --json` and treats exit code 1 as
  "action required now". See [For keeper operators](../guides/for-keeper-operators.md).
- **`--healthy-days` / `--critical-days`** let an operator tighten or loosen the
  bands. They convert to ledgers at the ledger close time and appear in the JSON
  output as `health_config.healthy_min_ledgers` / `critical_max_ledgers`.
- **`--extend-horizon-ledgers`** changes only the cost projection, not the band
  classification.
- **`--average-state-size-bytes` / `--rent-fee-per-1kb`** change how
  `fee_per_rent_1kb` is resolved — see [Economics of rent](../economics-of-rent.md).

## Exit codes

From `SCHEMA.md` (the stable contract; `action-state-watch` depends on the
`--fail-on-critical` code):

| Code | Meaning |
| --- | --- |
| `0` | Success. With `--fail-on-critical`: no entry is in the `critical` or `archived` band. |
| `1` | `--fail-on-critical` set **and** at least one entry is `critical` or `archived`. Only `scan` ever sets this. |
| `2` | Usage or operational error: bad arguments, invalid strkey/SCVal, RPC failure, XDR build/serialization failure, I/O failure. |

The band that triggers exit 1 is exactly `summary.has_critical == true` (any
entry with `band == "critical"` or `band == "archived"`).

## Real example (verbatim from the live verification pass)

`docs/live-verification.md` (2026-09-08, protocol 28, latest ledger 4,567,902)
against the live testnet RPC:

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

Both entries are `critical` with 54,482 ledgers (~3 days) remaining — the exact
scenario this tool exists to catch. The output contract is documented field by
field in [JSON schema reference](../json-schema-reference.md).