# JSON schema reference (`scan --json`)

This page documents the stable output contract of `scan --json`. It mirrors
[`SCHEMA.md`](../SCHEMA.md) in the repository, which is the canonical source.
Downstream consumers — notably `action-state-watch`, which parses `scan --json`
and depends on the `--fail-on-critical` exit code — should treat this document
as the "what does it return" contract.

**Binary name.** Consumers shell out to `soroban-state-sentinel` (the explicit
`[[bin]]` name in `crates/cli/Cargo.toml`; the crate is `sentinel-cli`). The
invocation name is part of the contract and will not change without a major
version bump.

## Schema version

Current version: **`1.1.0`** (constant `SCHEMA_VERSION` in
`crates/cli/src/output/json.rs`).

Version history:

- **`1.1.0`** (additive) — documents the `extend` subcommand's contract. The
  `scan` JSON document shape is **unchanged** from `1.0.0`; existing consumers
  need no migration.
- **`1.0.0`** — initial contract: `scan --json` document + exit codes.

## Exit codes (all subcommands)

| Code | Meaning |
| --- | --- |
| `0` | Success. With `--fail-on-critical`: no entry is in the `critical` or `archived` band. |
| `1` | `--fail-on-critical` set **and** at least one entry is `critical` or `archived`. Only `scan` ever sets this. |
| `2` | Usage or operational error: bad arguments, invalid strkey/SCVal, RPC failure, XDR build/serialization failure, I/O failure. |

`extend` and `restore` never set exit `1`. `--fail-on-critical` is the
automation hook: a watcher runs
`soroban-state-sentinel scan <id> --fail-on-critical --json`, and exit code `1`
means "action required now". The triggering condition is exactly
`summary.has_critical == true`.

## Top-level document

All field names are `snake_case`.

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | string | Current schema version (`1.1.0`). |
| `generated_at_unix` | u64 | Unix time in seconds when the document was generated. |
| `command` | object | What was invoked. |
| `network` | object | Live network parameters the scan read or resolved. |
| `health_config` | object | The band thresholds that were applied. |
| `summary` | object | Band counts across all scanned entries. |
| `entries` | array | One entry object per scanned ledger entry. |

### `command`

| Field | Type | Meaning |
| --- | --- | --- |
| `subcommand` | string | Always `"scan"`. |
| `contract_id` | string | The contract id as given on the command line (`C…` strkey). |
| `rpc_url` | string | The RPC endpoint used. |

### `network`

| Field | Type | Meaning |
| --- | --- | --- |
| `passphrase` | string | Network passphrase (e.g. `Test SDF Network ; September 2015`). |
| `protocol_version` | u32 | Protocol version of the latest ledger. |
| `latest_ledger` | u32 | Sequence of the latest ledger at scan time. |
| `ledger_close_seconds` | u64 | Ledger close time used for day conversions. |
| `ledger_close_seconds_source` | `"default"` \| `"explicit"` | `default` = the 5 s target was assumed; `explicit` = supplied via `--ledger-close-seconds`. |
| `fee_per_rent_1kb` | i64 | Resolved rent fee per 1 KB in stroops. |
| `fee_per_rent_1kb_source` | `"explicit"` \| `"average_state_size"` \| `"state_size_high"` | Which resolution path produced `fee_per_rent_1kb` (see [Economics of rent](economics-of-rent.md)). |
| `average_soroban_state_size_bytes` | i64 \| null | The state size used, when `--average-state-size-bytes` was supplied. |
| `max_entry_ttl` | u32 | Live network `max_entry_ttl` (upper bound for extend targets). |
| `min_persistent_ttl` | u32 | Live network minimum TTL for persistent entries (drives restore pricing). |
| `min_temporary_ttl` | u32 | Live network minimum TTL for temporary entries. |

### `health_config`

| Field | Type | Meaning |
| --- | --- | --- |
| `healthy_min_days` | u32 | Healthy lower bound in days (default 30). |
| `critical_max_days` | u32 | Critical upper bound in days (default 7). |
| `healthy_min_ledgers` | u32 | The day bound converted to ledgers (default 518,400). |
| `critical_max_ledgers` | u32 | The day bound converted to ledgers (default 120,960). |
| `extend_horizon_ledgers` | u32 | Horizon used for the per-entry extend-cost projection (defaults to `healthy_min_ledgers`). |

### `summary`

| Field | Type | Meaning |
| --- | --- | --- |
| `entries_scanned` | usize | Total entries scanned. |
| `healthy` | usize | Count in the `healthy` band. |
| `expiring_soon` | usize | Count in the `expiring_soon` band. |
| `critical` | usize | Count in the `critical` band. |
| `archived` | usize | Count in the `archived` band. |
| `has_critical` | bool | True if any entry is `critical` or `archived`. Drives `--fail-on-critical`. |

### `entries[]`

| Field | Type | Meaning |
| --- | --- | --- |
| `id` | string | Stable row id: `"instance"`, `"code"`, or `"key.<index>"`. |
| `label` | string | Human-readable label (e.g. `contract instance`, `contract code (wasm)`). |
| `kind` | string | `"contract_instance"` \| `"contract_code"` \| `"contract_data"`. |
| `durability` | `"persistent"` \| `"temporary"` \| null | Known for contract-data entries, `null` otherwise. |
| `band` | string | `"healthy"` \| `"expiring_soon"` \| `"critical"` \| `"archived"`. Stable machine-readable names. |
| `current_ledger_seq` | u32 | Latest ledger at scan time. |
| `live_until_ledger_seq` | u32 \| null | Entry's live-until ledger. `null` ⇒ archived. |
| `ledgers_remaining` | u32 \| null | `live_until_ledger_seq − current_ledger_seq`, clamped at 0. `null` ⇒ archived. |
| `days_remaining` | u64 \| null | `floor(ledgers_remaining × ledger_close_seconds / 86400)`. |
| `estimated_archive_unix` | u64 \| null | Estimated Unix time (seconds) of archival, if computable. |
| `size_bytes` | u32 \| null | XDR size of the live entry in bytes. |
| `key_xdr` | string | Base64-XDR `LedgerKey` of the entry. |
| `ttl_key_xdr` | string | Base64-XDR `LedgerKey::Ttl` governing the entry. |
| `extend_to_healthy_cost_stroops` | i64 \| null | Stroop cost to extend this entry to `extend_horizon_ledgers` from the current ledger. `null` when archived (extend does nothing for archived entries). |
| `restore_cost_stroops` | i64 \| null | Non-null **only** for archived entries. |

## Semantics

- `band` values are the stable machine-readable names: `healthy`,
  `expiring_soon`, `critical`, `archived`.
- `ledgers_remaining = live_until_ledger_seq - current_ledger_seq` (clamped at
  0). `null` means the entry is not readable in the live state (archived), and
  `band` is `archived`.
- `days_remaining` is `floor(ledgers_remaining * ledger_close_seconds / 86400)`.
- `extend_to_healthy_cost_stroops` is the stroop cost to extend the entry to
  `extend_horizon_ledgers` from the current ledger; it is `null` for archived
  entries.
- `restore_cost_stroops` is non-null **only** for archived entries.
- `durability` is known for contract-data entries and `null` otherwise.

## Extension policy

- **Non-breaking** (additive) changes — e.g. new optional fields — bump the
  minor component (`1.x`) and are safe for existing consumers.
- **Breaking** changes — renames, removed fields, changed types, changed band
  names — bump the major component and must be coordinated with
  `action-state-watch` before merging. Breaking changes require a version bump
  in `SCHEMA.md`, never a silent reshape.

## Output formats other than JSON

`--markdown` and `--table` are human-oriented and **not** part of this
contract; they may change freely. `extend` and `restore` write a file, not
stdout JSON, and are not consumed by `action-state-watch`; their file formats
are therefore not versioned.

## Real response example

Verbatim `scan --json` output from `docs/live-verification.md` (2026-09-08,
protocol 28, live testnet) — not hand-typed:

```json
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