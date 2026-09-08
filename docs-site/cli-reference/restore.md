# `restore`

```text
soroban-state-sentinel restore <contract-id> --output <FILE> [flags]
```

Builds unsigned `RestoreFootprintOp` XDR for archived entries — the remedy for
the `archived` band. Restoring brings an archived entry back into the live state
with its TTL reset to the network's minimum for newly created entries
(`min_persistent_ttl`). Output is **unsigned by design**.

## Flags

All flag names, types, and defaults below are from the `clap` definitions in
`crates/cli/src/args.rs`.

| Flag | Type | Default | Notes |
| --- | --- | --- | --- |
| `contract_id` | positional | — | Contract id (`C…` strkey). Required. |
| `--output <FILE>` | string | — | Output path for the unsigned XDR (writes base64). Required. |
| `--keys <SCVAL_BASE64>` | repeatable | — | Storage keys to restore. When omitted, the contract instance (+ its code, if discoverable) is restored. |
| `--durability <persistent\|temporary>` | enum | `persistent` | Durability assumed for `--keys` entries. |
| `--source-account <G…>` | string | — | Public key of the account that will sign and submit. When given, the output is a complete unsigned `TransactionV1Envelope` (account sequence fetched from the ledger). When omitted, the output is the raw operations, one base64 per line. |
| `--fee <stroops>` | u64 | — | Transaction fee override in stroops (otherwise estimated). |
| `--sequence <N>` | i64 | — | Account sequence number override (otherwise fetched from the ledger). |
| `--assumed-archived-entry-size <N>` | u32 | `1024` | Assumed size in bytes per archived entry when its live entry cannot be fetched. |

Plus the shared global flags: `--rpc-url` (default
`https://soroban-testnet.stellar.org`), `--ledger-close-seconds` (default 5),
`--average-state-size-bytes`, `--rent-fee-per-1kb`, `--ttl-entry-size` (default
48).

## Exit codes

From `SCHEMA.md`:

| Code | Meaning |
| --- | --- |
| `0` | Success. |
| `1` | Never set by `restore`. |
| `2` | Usage or operational error: bad arguments, invalid strkey/SCVal, RPC failure, XDR build/serialization failure, I/O failure. |

## Output

- **Without `--source-account`**: raw `RestoreFootprint` operations, one
  base64-XDR per line.
- **With `--source-account`**: a single base64-XDR `TransactionV1Envelope` with
  empty `signatures`. The account's next sequence number is fetched from the
  ledger unless `--sequence` is given; the fee is estimated unless `--fee` is
  given.

## Fee estimate caveat

For archived entries the live entry is unreadable over RPC, so its size — and
therefore the exact rent fee — is unknown. The estimate uses
`--assumed-archived-entry-size` (default 1,024 bytes) and labels it as such.
Stellar Core charges the actual rent at apply time and refunds the unused
refundable fee, so a conservative estimate is safe.

## Real examples (verbatim from the live verification pass)

`docs/live-verification.md` (2026-09-08, protocol 28). The docs `Counter`
contract instance is archived on testnet — a real restore case:

```console
$ soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --output /tmp/counter_restore.xdr
wrote 1 unsigned RestoreFootprint operation(s) (base64 XDR, one per line) to /tmp/counter_restore.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
fee note: rent fee est. 998098 stroops (archived entries use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ cat /tmp/counter_restore.xdr
AAAAAAAAABoAAAAA

$ soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 --output /tmp/counter_restore_env.xdr
wrote unsigned RestoreFootprint envelope (1 op(s), est. fee 1009266 stroops) to /tmp/counter_restore_env.xdr
source account: GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 · sequence: 19614875122663425 · signatures: EMPTY (sign with your own key)
fee note: rent fee est. 998098 stroops (archived entries use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
```

The output was independently decoded with `stellar-xdr`'s `from_xdr_base64` in
the verification pass: `RestoreFootprintOp { ext: V0 }`, and the envelope
decodes as a `TransactionV1Envelope` with **zero signatures**. The 998,098
stroop restore cost is reproduced by hand in
[Economics of rent](../economics-of-rent.md).