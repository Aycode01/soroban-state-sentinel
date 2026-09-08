# `extend`

```text
soroban-state-sentinel extend <contract-id> --extend-to <LEDGERS> --output <FILE> [flags]
soroban-state-sentinel extend <contract-id> --extend-to-days <DAYS> --output <FILE> [flags]
```

Builds unsigned `ExtendFootprintTTLOp` XDR for entries approaching archival —
the proactive remedy for the `expiring_soon` / `critical` bands, before they
archive. Output is **unsigned by design**: a human, multisig, or keeper signs
and submits it separately.

## Flags

All flag names, types, and defaults below are from the `clap` definitions in
`crates/cli/src/args.rs`.

| Flag | Type | Default | Notes |
| --- | --- | --- | --- |
| `contract_id` | positional | — | Contract id (`C…` strkey). Required. |
| `--extend-to <LEDGERS>` | u32 | — | Ledgers to extend the entries' TTL by, measured from the current ledger (the operation's `extendTo`; core sets `liveUntilLedgerSeq = current + extendTo`). Required unless `--extend-to-days` is given; conflicts with it. |
| `--extend-to-days <DAYS>` | u32 | — | Extend by this many days, resolved to ledgers via `--ledger-close-seconds`. Conflicts with `--extend-to`. |
| `--keys <SCVAL_BASE64>` | repeatable | — | Storage keys to extend. When omitted, the contract instance (+ its code, if discoverable) is extended. |
| `--durability <persistent\|temporary>` | enum | `persistent` | Durability assumed for `--keys` entries. |
| `--output <FILE>` | string | — | Output path for the unsigned XDR (writes base64). Required. |
| `--source-account <G…>` | string | — | Public key of the account that will sign and submit. When given, the output is a complete unsigned `TransactionV1Envelope` (account sequence fetched from the ledger). When omitted, the output is the raw operations, one base64 per line. |
| `--fee <stroops>` | u64 | — | Transaction fee override in stroops (otherwise estimated). |
| `--sequence <N>` | i64 | — | Account sequence number override (otherwise fetched from the ledger). |
| `--assumed-archived-entry-size <N>` | u32 | `1024` | Assumed size in bytes per archived entry whose live entry cannot be fetched (footprint accounting and fee estimate only). |

Plus the shared global flags: `--rpc-url` (default
`https://soroban-testnet.stellar.org`), `--ledger-close-seconds` (default 5),
`--average-state-size-bytes`, `--rent-fee-per-1kb`, `--ttl-entry-size` (default
48).

## `--extend-to` vs `--extend-to-days`

- `--extend-to <LEDGERS>` is a **duration in ledgers from the current ledger**,
  not an absolute target ledger sequence. Stellar Core applies
  `liveUntilLedgerSeq = currentLedger + extendTo` (verified against
  `ExtendFootprintTTLOpFrame.cpp`).
- `--extend-to-days <N>` is a convenience form. N days resolve to ledgers with
  the same close-time logic `scan` uses: `ceil(N × 86,400 / close_seconds)`.
  30 days at 5 s/ledger → 518,400 ledgers. The console output labels the close
  time `default` (the 5 s target) vs `explicit` — never a silent assumption.

### Validation against `max_entry_ttl`

Before writing anything, the tool validates the resolved target against the
live network's `max_entry_ttl`:

- `0` is rejected as a no-op extension.
- Values above `max_entry_ttl − 1` are rejected as malformed (core rejects them
  in `doCheckValidForSoroban`).

On the live testnet pass, `max_entry_ttl` was **3,110,400**, so the valid range
was `1..=3110399`:

```console
$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 999999999 --output /tmp/axis_invalid.xdr; echo "exit=$?"
error: XDR build error: extend_to 999999999 must be in 1..=3110399 (max_entry_ttl - 1)
exit=2
```

The boundary comes from the live network config every run, never from a
hardcoded constant. A violation exits with code 2 and writes no file.

## Exit codes

From `SCHEMA.md`:

| Code | Meaning |
| --- | --- |
| `0` | Success. |
| `1` | Never set by `extend`. |
| `2` | Usage or operational error: bad arguments, invalid strkey/SCVal, `--extend-to` out of range, RPC failure, XDR build/serialization failure, I/O failure. |

## Output

- **Without `--source-account`**: raw `ExtendFootprintTtl` operations, one
  base64-XDR per line.
- **With `--source-account`**: a single base64-XDR `TransactionV1Envelope` with
  empty `signatures`. The account's next sequence number is fetched from the
  ledger unless `--sequence` is given; the fee is estimated unless `--fee` is
  given.

## Real examples (verbatim from the live verification pass)

`docs/live-verification.md` (2026-09-08, protocol 28, latest ledger 4,567,902):

```console
$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 518400 --output /tmp/axis_extend.xdr
wrote 1 unsigned ExtendFootprintTtl operation(s) (base64 XDR, one per line) to /tmp/axis_extend.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
extend target: 518400 ledgers from the current ledger (ledger 4567902)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ cat /tmp/axis_extend.xdr
AAAAAAAAABkAAAAAAAfpAA==

$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to-days 30 --output /tmp/axis_extend_days.xdr
wrote 1 unsigned ExtendFootprintTtl operation(s) (base64 XDR, one per line) to /tmp/axis_extend_days.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
extend target: 518400 ledgers from ledger 4567902 (30 day(s) at 5s/ledger, close-time source: default)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)

$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 518400 --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 --output /tmp/axis_extend_env.xdr
wrote unsigned ExtendFootprintTtl envelope (1 op(s), est. fee 24877143 stroops) to /tmp/axis_extend_env.xdr
source account: GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 · sequence: 19614875122663425 · signatures: EMPTY (sign with your own key)
extend target: 518400 ledgers from the current ledger (ledger 4567902)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
```

The output was independently decoded with `stellar-xdr`'s `from_xdr_base64` in
the verification pass: `ExtendFootprintTtlOp { ext: V0, extend_to: 518400 }`
for both the ledgers and days forms, and the envelope decodes as a
`TransactionV1Envelope` with **zero signatures**.