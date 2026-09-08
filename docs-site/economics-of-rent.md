# Economics of rent

Rent is what Soroban charges to keep an entry in the live state. The sentinel
computes exact stroop costs for the two remediation actions using a line-by-line
port of `soroban-env-host`'s fee math (`crates/rent-model/src/fees.rs`, ported
from `stellar/rs-soroban-env` at tag `v28.0.0`), fed with the live network's
config-setting values. This page walks one real example end to end so you can
reproduce every number by hand.

## The live inputs

The worked example uses the AXIS testnet contract from `docs/live-verification.md`
(captured 2026-09-08, protocol 28, latest ledger 4,567,902, 5 s/ledger). The
values below are the network config the tool read from the live ledger — the
test fixtures (`tests/fixtures/getLedgerEntries_config.json`) are byte-identical
to the live values, per the verification checklist:

| Input | Value | Source |
| --- | --- | --- |
| `fee_per_rent_1kb` | 10,000 stroops / 1 KB | resolved at the state-size-high plateau (`state_size_high`) |
| `persistent_rent_rate_denominator` | 1,215 | `StateArchival` config setting |
| `fee_write_ledger_entry` | 2,500 | `ContractLedgerCostV0` |
| `fee_write_1kb` | 875 | `ContractLedgerCostExtV0` |
| TTL entry size | 48 bytes | protocol constant (overridable via `--ttl-entry-size`) |
| code rent discount factor | 3 | protocol constant (from `soroban-env-host`) |
| `max_entry_ttl` | 3,110,400 | `StateArchival` config setting |
| `min_persistent_ttl` | 120,960 | `StateArchival` config setting |

## The fee formula

For one entry extended by `extension_ledgers`, the size-based rent fee is:

```
size_fee = ceil( size_bytes × fee_per_rent_1kb × extension_ledgers
                 / (1024 × rent_rate_denominator) )
```

`rent_rate_denominator` is 1,215 for persistent entries and 2,430 for
temporary entries. On top of that, every extended entry pays a TTL-write fee:

```
ttl_write_fee = fee_write_ledger_entry + ceil( 48 × fee_write_1kb / 1024 )
              = 2500 + ceil(48 × 875 / 1024)
              = 2500 + 42
              = 2542 stroops
```

Contract-code entries get the size component divided by 3 (ceil); the
TTL-write component is not discounted.

## Example 1: extend the AXIS instance to the healthy horizon

The instance entry is 148 bytes with `liveUntilLedgerSeq` 4,622,384. The scan
projects the cost to extend it to the healthy horizon (518,400 ledgers from the
current ledger — the `healthy_min_ledgers` threshold):

1. New live-until: `4,567,902 + 518,400 = 5,086,302`.
2. Extension ledgers: `5,086,302 − 4,622,384 = 463,918`.
3. Size fee: `ceil(148 × 10,000 × 463,918 / (1024 × 1215))`
   = `ceil(686,598,640,000 / 1,244,160)` = `551,858`.
4. TTL-write fee: `2,542`.
5. Total: `551,858 + 2,542 = 554,400` stroops.

That is exactly the `extend_to_healthy_cost_stroops` value in the live scan
JSON for the instance entry.

## Example 2: extend the AXIS code entry

The code entry (19,532-byte wasm) has the same live-until, so the extension
ledger count is the same:

1. Raw size fee: `ceil(19,532 × 10,000 × 463,918 / 1,244,160)`
   = `ceil(90,612,463,760,000 / 1,244,160)` = `72,830,234`.
2. Code discount: `ceil(72,830,234 / 3)` = `24,276,745`.
3. TTL-write fee: `2,542` (not discounted).
4. Total: `24,276,745 + 2,542 = 24,279,287` stroops.

Matches the live scan JSON. The code entry costs ~44× the instance despite only
~132× the size — the discount and the shared extension ledgers are why.

### One stroop difference between per-entry and batched numbers

The live `extend --extend-to 518400` run reported a rent fee estimate of
**24,833,686** stroops for instance + code in one call. The scan's per-entry
projections sum to 24,833,687. The one-stroop gap is real and explainable: the
TTL-write byte fee is ceiled once across the batch — `ceil(2 × 48 × 875 /
1024) = 83` — whereas per-entry projections ceil each entry separately
(`2 × ceil(48 × 875 / 1024) = 84`). The size components sum exactly.

## Example 3: restore the archived Counter instance

Restoring models each archived entry as newly created with
`new_live = current + min_persistent_ttl − 1` (the "minimum TTL including the
current ledger" rule from `RestoreFootprintOpFrame.cpp`). The archived
instance's live size is unreadable over RPC, so the scan uses the assumed size
(1,024 bytes default):

1. New live-until: `4,567,902 + 120,960 − 1 = 4,688,861`.
2. Extension ledgers (new entry): `4,688,861 − (4,567,902 − 1) = 120,960`.
3. Size fee: `ceil(1,024 × 10,000 × 120,960 / 1,244,160)`
   = `ceil(1,238,630,400,000 / 1,244,160)` = `995,556`.
4. TTL-write fee: `2,542`.
5. Total: `995,556 + 2,542 = 998,098` stroops.

Matches the live `restore_cost_stroops` for the archived Counter instance.

## How `fee_per_rent_1kb` is resolved — and its documented limitation

Core derives the effective rent fee per 1 KB from its internal state-size
sampling window. RPC does not expose that window, and there is no trustless way
to read it. The tool therefore **resolves** `fee_per_rent_1kb` with explicit
precedence (`crates/cli/src/context.rs`) and always labels which path it took in
the output (`fee_per_rent_1kb_source`):

1. `--rent-fee-per-1kb <stroops>` — explicit override. Label: `explicit`.
2. `--average-state-size-bytes <bytes>` — fed through the canonical
   `compute_rent_write_fee_per_1kb` formula. Label: `average_state_size`.
3. Nothing supplied — the state-size-high plateau
   (`rent_fee_1kb_soroban_state_size_high`, 10,000 on this testnet). Label:
   `state_size_high`.

This is a real, documented limitation of the tool, not a claim of
protocol-perfect fee replication. On the live pass the plateau path was taken
(`fee_per_rent_1kb: 10000`, `fee_per_rent_1kb_source: "state_size_high"`).

## The ledger-close-time assumption

RPC does not expose the network's actual average ledger close time. The tool
uses a default of **5 seconds** (the Stellar target), and it always labels the
value it used:

- `ledger_close_seconds_source: "default"` when 5 s was assumed.
- `ledger_close_seconds_source: "explicit"` when `--ledger-close-seconds` was
  supplied.

The label exists so a reader always knows whether the tool is estimating (day
conversions, `days_remaining`, `--extend-to-days`) or using a value they
supplied. Days convert to ledgers with `ceil(days × 86,400 / close_seconds)`:
30 days at 5 s → 518,400 ledgers; 7 days at 5 s → 120,960.