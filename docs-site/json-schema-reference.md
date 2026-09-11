# JSON Schema Reference

`soroban-state-sentinel scan --json` outputs a machine-readable JSON document designed for automated consuming pipelines such as `action-state-watch`.

## Current Version & Stability Policy

- **Current Version**: `1.1.0` (defined by constant `SCHEMA_VERSION` in `crates/cli/src/output/json.rs`).
- **Additive Policy**: Minor version bumps (`1.x.0`) strictly contain non-breaking additive fields. Major version bumps (`2.0.0`) mark breaking changes (field removals, renames, type changes) and require explicit downstream coordination.

## Field-by-Field Reference

### Top-Level Document

| Field | Type | Description |
| --- | --- | --- |
| `schema_version` | `string` | Contract version string (`"1.1.0"`). |
| `generated_at_unix` | `u64` | UTC Unix timestamp of scan execution in seconds. |
| `command` | `object` | Execution parameters object. |
| `network` | `object` | Network state and configuration parameters. |
| `health_config` | `object` | Effective health band boundaries used for classification. |
| `summary` | `object` | Aggregate entry counts and status flags. |
| `entries` | `array` | Array of per-entry health and cost objects. |

### `command` Object

| Field | Type | Description |
| --- | --- | --- |
| `subcommand` | `string` | Subcommand executed (`"scan"`). |
| `contract_id` | `string` | Contract ID strkey (`C...`) requested. |
| `rpc_url` | `string` | Soroban RPC endpoint queried. |

### `network` Object

| Field | Type | Description |
| --- | --- | --- |
| `passphrase` | `string` | Network passphrase string. |
| `protocol_version` | `u32` | Protocol version running on network. |
| `latest_ledger` | `u32` | Latest sequence number read from RPC. |
| `ledger_close_seconds` | `u64` | Average ledger close duration in seconds. |
| `ledger_close_seconds_source` | `string` | Source of close time (`"default"` or `"explicit"`). |
| `fee_per_rent_1kb` | `i64` | Rent fee rate per 1KB in stroops. |
| `fee_per_rent_1kb_source` | `string` | Calculation source (`"explicit"`, `"average_state_size"`, or `"state_size_high"`). |
| `average_soroban_state_size_bytes` | `i64 \| null` | Input state size in bytes, or `null` if default plateau used. |
| `max_entry_ttl` | `u32` | Maximum TTL cap enforced by protocol. |
| `min_persistent_ttl` | `u32` | Minimum persistent entry TTL after restoration. |
| `min_temporary_ttl` | `u32` | Minimum temporary entry TTL. |

### `health_config` Object

| Field | Type | Description |
| --- | --- | --- |
| `healthy_min_days` | `u32` | Healthy band threshold in days. |
| `critical_max_days` | `u32` | Critical band threshold in days. |
| `healthy_min_ledgers` | `u32` | Healthy band threshold converted to ledgers. |
| `critical_max_ledgers` | `u32` | Critical band threshold converted to ledgers. |
| `extend_horizon_ledgers` | `u32` | Target horizon in ledgers used for fee projections. |

### `summary` Object

| Field | Type | Description |
| --- | --- | --- |
| `entries_scanned` | `u32` | Total number of entries evaluated. |
| `healthy` | `u32` | Count of entries in `healthy` band. |
| `expiring_soon` | `u32` | Count of entries in `expiring_soon` band. |
| `critical` | `u32` | Count of entries in `critical` band. |
| `archived` | `u32` | Count of entries in `archived` band. |
| `has_critical` | `boolean` | `true` if `critical > 0` or `archived > 0` (triggers exit code 1 with `--fail-on-critical`). |

### `entries[]` Element Object

| Field | Type | Description |
| --- | --- | --- |
| `id` | `string` | Identifier (`"instance"`, `"code"`, or `"key.<index>"`). |
| `label` | `string` | Human-readable entry label. |
| `kind` | `string` | Entry category (`"contract_instance"`, `"contract_code"`, `"contract_data"`). |
| `durability` | `string \| null` | Storage durability (`"persistent"`, `"temporary"`, or `null`). |
| `band` | `string` | Machine-readable health band (`"healthy"`, `"expiring_soon"`, `"critical"`, `"archived"`). |
| `current_ledger_seq` | `u32` | Current network ledger sequence. |
| `live_until_ledger_seq` | `u32 \| null` | Expiration ledger sequence (`null` if archived). |
| `ledgers_remaining` | `u32 \| null` | Ledgers remaining until archival (`null` if archived). |
| `days_remaining` | `u64 \| null` | Floor of remaining days (`null` if archived). |
| `estimated_archive_unix` | `u64 \| null` | Estimated UTC Unix archival timestamp (`null` if archived). |
| `size_bytes` | `u32 \| null` | Entry size in bytes (`null` if archived). |
| `key_xdr` | `string` | Base64-encoded `LedgerKey`. |
| `ttl_key_xdr` | `string` | Base64-encoded `LedgerKey::Ttl`. |
| `extend_to_healthy_cost_stroops` | `i64 \| null` | Estimated stroop cost to extend to healthy horizon (`null` if archived). |
| `restore_cost_stroops` | `i64 \| null` | Estimated stroop cost to restore (`null` if live). |

## Real Verbatim JSON Output Example

Taken from live verification against contract `CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX`:

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