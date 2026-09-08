# Output contract — `soroban-state-sentinel`

This document is the stable contract between the sentinel CLI and its
consumers. **`action-state-watch` parses `scan --json` and depends on the
`--fail-on-critical` exit code.** Breaking changes to either require a version
bump below, coordinated with consumers — never a silent reshape.

## Exit codes (all subcommands)

| Code | Meaning |
| --- | --- |
| `0` | Success. With `--fail-on-critical`: no entry is in the `critical` or `archived` band. |
| `1` | `--fail-on-critical` set **and** at least one entry is `critical` or `archived`. Only `scan` ever sets this. |
| `2` | Usage or operational error: bad arguments, invalid strkey/SCVal, RPC failure, XDR build/serialization failure, I/O failure. |

`extend` and `restore` never set exit `1`; their outcomes are `0` (success) or
`2` (error).

`--fail-on-critical` is the automation hook: a watcher runs
`soroban-state-sentinel scan <id> --fail-on-critical --json`, and exit code `1`
means "action required now". The band that triggers it is exactly
`summary.has_critical == true` (any entry with `band == "critical"` or
`band == "archived"`).

## JSON schema — `scan --json`

Current version: **`1.1.0`** (constant `SCHEMA_VERSION` in
`crates/cli/src/output/json.rs`).

Version history:

- **`1.1.0`** (additive) — documents the `extend` subcommand's contract. The
  `scan` JSON document shape is **unchanged** from `1.0.0`; existing consumers
  need no migration.
- **`1.0.0`** — initial contract: `scan --json` document + exit codes.

Top-level document (all field names `snake_case`):

```jsonc
{
  "schema_version": "1.0.0",
  "generated_at_unix": 1788858382,          // u64, seconds
  "command": {
    "subcommand": "scan",
    "contract_id": "C…",                    // as given on the command line
    "rpc_url": "https://soroban-testnet.stellar.org"
  },
  "network": {
    "passphrase": "Test SDF Network ; September 2015",
    "protocol_version": 28,
    "latest_ledger": 4566959,
    "ledger_close_seconds": 5,
    "ledger_close_seconds_source": "default",  // "default" | "explicit"
    "fee_per_rent_1kb": 3000,
    "fee_per_rent_1kb_source": "state_size_high", // "explicit" | "average_state_size" | "state_size_high"
    "average_soroban_state_size_bytes": null,     // i64 | null
    "max_entry_ttl": 3112000,
    "min_persistent_ttl": 4096,
    "min_temporary_ttl": 0
  },
  "health_config": {
    "healthy_min_days": 30,
    "critical_max_days": 7,
    "healthy_min_ledgers": 518400,
    "critical_max_ledgers": 120960,
    "extend_horizon_ledgers": 518400
  },
  "summary": {
    "entries_scanned": 3,
    "healthy": 1,
    "expiring_soon": 0,
    "critical": 1,
    "archived": 1,
    "has_critical": true
  },
  "entries": [
    {
      "id": "instance",                       // "instance" | "code" | "key.<index>"
      "label": "contract instance",
      "kind": "contract_instance",            // "contract_instance" | "contract_code" | "contract_data"
      "durability": "persistent",             // "persistent" | "temporary" | null
      "band": "critical",                     // "healthy" | "expiring_soon" | "critical" | "archived"
      "current_ledger_seq": 4566959,
      "live_until_ledger_seq": 4566920,       // u32 | null (null ⇒ archived)
      "ledgers_remaining": 0,                 // u32 | null
      "days_remaining": 0,                    // u64 | null (floor)
      "estimated_archive_unix": 1788858382,   // u64 | null
      "size_bytes": 152,                      // u32 | null
      "key_xdr": "AAAAAQAAAAA…",              // base64 LedgerKey
      "ttl_key_xdr": "AAAAAwAAAA…",           // base64 LedgerKey::Ttl
      "extend_to_healthy_cost_stroops": 10048, // i64 | null (null when archived)
      "restore_cost_stroops": null            // i64 | null (non-null only when archived)
    }
  ]
}
```

### Semantics

- `band` values are the stable machine-readable names:
  `healthy`, `expiring_soon`, `critical`, `archived`.
- `ledgers_remaining = live_until_ledger_seq - current_ledger_seq` (clamped at
  0). `null` means the entry is not readable in the live state (archived) and
  `band` is `archived`.
- `days_remaining` is `floor(ledgers_remaining * ledger_close_seconds /
  86400)`.
- `extend_to_healthy_cost_stroops` is the stroop cost to extend this entry to
  `extend_horizon_ledgers` from the current ledger; it is `null` for archived
  entries (extend does nothing for them).
- `restore_cost_stroops` is non-null **only** for archived entries.
- `durability` is known for contract-data entries and `null` otherwise.

### Extension policy

- **Non-breaking** (additive) changes — e.g. new optional fields — bump the
  minor component (`1.x`) and are safe for existing consumers.
- **Breaking** changes — renames, removed fields, changed types, changed band
  names — bump the major component and must be coordinated with
  `action-state-watch` before merging.

## Output formats other than JSON

`--markdown` and `--table` are human-oriented and **not** part of this
contract; they may change freely.

## `extend` / `restore` output

`extend` and `restore` write a file, not stdout JSON, and are not consumed by
`action-state-watch`; their file formats are therefore not versioned here. Both
follow identical conventions:

- Without `--source-account`: base64-XDR operations (`ExtendFootprintTtl` for
  `extend`, `RestoreFootprint` for `restore`), one per line.
- With `--source-account`: a single base64-XDR `TransactionV1Envelope` with
  empty `signatures`.

`extend --extend-to <LEDGERS>` is a **duration in ledgers from the current
ledger** (core applies `liveUntilLedgerSeq = currentLedger + extendTo`), not an
absolute target ledger sequence; the tool validates it against the live
network's `max_entry_ttl - 1` before writing anything and rejects `0` as a
no-op. `--extend-to-days <N>` resolves days to ledgers with the same
close-time logic `scan` uses and labels the close time `default` vs `explicit`
in its console output.