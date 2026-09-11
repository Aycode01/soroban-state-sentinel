# `restore`

The `restore` subcommand builds unsigned XDR containing `RestoreFootprintOp` operations for archived contract instance, code, or storage entries.

## Command Syntax

```bash
soroban-state-sentinel restore <CONTRACT_ID> --output <FILE> [FLAGS]
```

## Options & Arguments

### Positional Arguments

- **`<CONTRACT_ID>`** (`String`): Contract ID strkey (`C...`) whose archived entries to restore.

### Command Flags

- **`--keys <SCVAL_BASE64>`** (`Vec<String>`): Storage keys to restore, each a base64-encoded `SCVal`. When omitted, the contract instance (and WASM code) is targeted.
- **`--durability <DURABILITY>`** (`Enum`, default: `persistent`): Storage durability for `--keys` entries (`persistent` or `temporary`).
- **`--output <FILE>`** (`String`): File path to write the generated base64 XDR.
- **`--source-account <ACCOUNT>`** (`Option<String>`): Public key (`G...`) of the submitting account. When provided, outputs a full unsigned `TransactionV1Envelope`. When omitted, outputs raw base64 operations (one per line).
- **`--fee <STROOPS>`** (`Option<u64>`): Custom transaction inclusion fee override in stroops.
- **`--sequence <SEQ>`** (`Option<i64>`): Custom account sequence number override.
- **`--assumed-archived-entry-size <BYTES>`** (`u32`, default: `1024`): Assumed entry size for unreadable archived entries used in fee estimation.

## Exit Codes

| Exit Code | Meaning |
| --- | --- |
| `0` | Success. XDR file written. |
| `2` | Error (invalid flags, RPC error, XDR build error). |

Note: `restore` never exits with code `1`.

## Real Verbatim Output Examples

### 1. Generating Raw Operations (`--output`)

Restoring the archived Stellar docs Counter contract (`CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI`):

```console
$ soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --output /tmp/counter_restore.xdr
wrote 1 unsigned RestoreFootprint operation(s) (base64 XDR, one per line) to /tmp/counter_restore.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
fee note: rent fee est. 998098 stroops (archived entries use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ cat /tmp/counter_restore.xdr
AAAAAAAAABoAAAAA
```

### 2. Generating Unsigned Envelope (`--source-account`)

```console
$ soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 --output /tmp/counter_restore_env.xdr
wrote unsigned RestoreFootprint envelope (1 op(s), est. fee 1009266 stroops) to /tmp/counter_restore_env.xdr
source account: GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 · sequence: 19614875122663425 · signatures: EMPTY (sign with your own key)
fee note: rent fee est. 998098 stroops (archived entries use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
```