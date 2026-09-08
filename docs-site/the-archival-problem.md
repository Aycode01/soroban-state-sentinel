# The archival problem

Soroban ledger entries that are not permanently referenced have a limited
lifespan. Each entry carries a TTL entry with a `liveUntilLedgerSeq`; when the
latest ledger passes that sequence, the entry stops being part of the live
state. What happens next depends on the entry's durability.

## Storage durability types

| Durability | At TTL zero |
| --- | --- |
| **Temporary** | Deleted permanently. The data is gone and cannot be recovered. |
| **Persistent** (contract instance, contract code, persistent data) | Archived / evicted. The entry leaves the live state; to read it again you must first restore it. |
| **Instance** | The contract instance is a persistent contract-data entry with the special `ScVal::LedgerKeyContractInstance` key. It archives like any other persistent entry. |

The important distinction: temporary entries are *deleted*, persistent and
instance entries are *archived*. Archived entries still exist — restoring them
brings them back into the live state with a fresh TTL.

## The lifecycle as a state machine

```
Healthy ──► ExpiringSoon ──► Critical ──► Archived ──► Healthy
     ▲                                             (restore)
     └─────────────────────────────────────────────┘
```

The tool classifies each entry into exactly one band from its ledgers remaining
until archival (`liveUntilLedgerSeq - currentLedgerSeq`):

- **`healthy`** — more than the healthy threshold of ledgers remaining. No
  action needed.
- **`expiring_soon`** — between the critical and healthy thresholds. Schedule an
  extension.
- **`critical`** — at or below the critical threshold, including 0 (the entry is
  live only through the current ledger). Extend now.
- **`archived`** — the entry is not readable in the live state at all. Restore
  it before you can read or extend it.

### The exact thresholds this tool uses

The thresholds are CLI inputs, not hardcoded constants. The defaults, straight
from `crates/cli/src/args.rs`, are:

- `--healthy-days 30` — entries with **more than** 30 days remaining are
  `healthy`.
- `--critical-days 7` — entries with **at most** 7 days remaining are
  `critical`.

Days are converted to ledgers at the ledger close time (5 seconds by default,
labeled `default` vs `explicit` — see [Economics of rent](economics-of-rent.md)):

- `healthy_min_ledgers = 518400` (30 × 86,400 / 5)
- `critical_max_ledgers = 120960` (7 × 86,400 / 5)

Classification (`crates/ttl-scanner/src/health.rs`):

| Ledgers remaining | Band |
| --- | --- |
| unreadable (no live entry) | `archived` |
| `> 518400` | `healthy` |
| `> 120960` | `expiring_soon` |
| `<= 120960` (including 0) | `critical` |

The boundary rules matter: exactly 30 days is `expiring_soon`, not `healthy`,
and exactly 7 days is `critical`. These ledger values appear in every `scan
--json` document under `health_config`.

## The two operations

There are exactly two remediation operations in current protocol-28 Stellar:

- **`ExtendFootprintTTLOp`** — extends the TTL of the footprint's live entries.
  This is the proactive fix for `expiring_soon` / `critical` entries.
- **`RestoreFootprintOp`** — restores archived entries back into the live
  state. This is the fix for `archived` entries.

Older material that calls these `BumpFootprintExpirationOp` /
`BumpFootprintInstanceOp` is describing a deprecated pre-protocol-20 name. The
sentinel only ever emits the current names, and it derives the field layout
from the `stellar-xdr` crate for the live protocol — never from memory.