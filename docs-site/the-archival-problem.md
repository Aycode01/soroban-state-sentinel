# The Archival Problem

Soroban state archival balances ledger growth and state access costs. Every entry written to the ledger carries a Time-To-Live (TTL) sequence counter.

## Storage Durability Types

Soroban defines three storage durability categories:

1. **Temporary Storage**: Holds transient data. When TTL reaches zero, temporary entries are permanently deleted from the ledger and cannot be restored.
2. **Instance Storage**: Holds contract executable metadata and state bound to the contract instance. When TTL reaches zero, the entry is archived (evicted from live state) and becomes unreadable until restored.
3. **Persistent Storage**: Holds user-defined contract data. When TTL reaches zero, persistent entries are archived and become unreadable until restored.

Both `Instance` and `Persistent` storage require issuing a `RestoreFootprintOp` to re-enter live state after archival.

## Health Band Lifecycle

`soroban-state-sentinel` tracks entry lifecycle across four distinct health bands:

```
[Healthy] ---> [ExpiringSoon] ---> [Critical] ---> [Archived]
    ^                                                   |
    |-------------- (RestoreFootprintOp) ---------------|
```

The CLI uses the following default thresholds (configurable via `--healthy-days` and `--critical-days` assuming 5-second ledger close times):

| Health Band | Default Ledger Threshold | Default Days | Description |
| --- | --- | --- | --- |
| `Healthy` | `> 518,400` ledgers | `> 30` days | Entry TTL is secure. No action needed. |
| `ExpiringSoon` | `120,961` to `518,400` ledgers | `7` to `30` days | Entry TTL is decaying. Schedule extension. |
| `Critical` | `0` to `120,960` ledgers | `0` to `7` days | Entry is at immediate risk. Issue `ExtendFootprintTTLOp`. |
| `Archived` | `None` (unreadable in live state) | `N/A` | Entry has expired. Issue `RestoreFootprintOp`. |

## Canonical Operation Names

Remediation transactions MUST use the protocol-standard operation names:

- **`ExtendFootprintTTLOp`**: Extends the live TTL of persistent or instance storage entries.
- **`RestoreFootprintOp`**: Restores archived persistent or instance storage entries back into live state.

Legacy documentation or earlier protocol proposals referencing `BumpFootprintExpirationOp` or `BumpFootprintInstanceOp` reflect deprecated pre-protocol-20 terminology.