# `extend`

The `extend` subcommand builds unsigned XDR containing `ExtendFootprintTTLOp` operations for live contract instance, code, or storage entries.

## Command Syntax

```bash
soroban-state-sentinel extend <CONTRACT_ID> (--extend-to <LEDGERS> | --extend-to-days <DAYS>) --output <FILE> [FLAGS]
```

## Options & Arguments

### Positional Arguments

- **`<CONTRACT_ID>`** (`String`): Contract ID strkey (`C...`) whose entries to extend.

### Command Flags

- **`--keys <SCVAL_BASE64>`** (`Vec<String>`): Storage keys to extend, each a base64-encoded `SCVal`. When omitted, the contract instance (and WASM code) is targeted.
- **`--durability <DURABILITY>`** (`Enum`, default: `persistent`): Storage durability for `--keys` entries (`persistent` or `temporary`).
- **`--extend-to <LEDGERS>`** (`u32`): Extension duration in ledgers from the current ledger (`liveUntil = current + extendTo`). Required unless `--extend-to-days` is given. Conflicts with `--extend-to-days`.
- **`--extend-to-days <DAYS>`** (`u32`): Extension duration in days, converted to ledgers using `--ledger-close-seconds`. Conflicts with `--extend-to`.
- **`--output <FILE>`** (`String`): File path to write the generated base64 XDR.
- **`--source-account <ACCOUNT>`** (`Option<String>`): Public key (`G...`) of the submitting account. When provided, outputs a full unsigned `TransactionV1Envelope`. When omitted, outputs raw base64 operations (one per line).
- **`--fee <STROOPS>`** (`Option<u64>`): Custom transaction inclusion fee override in stroops.
- **`--sequence <SEQ>`** (`Option<i64>`): Custom account sequence number override.
- **`--assumed-archived-entry-size <BYTES>`** (`u32`, default: `1024`): Assumed entry size for unreadable archived entries used in fee estimation.

## Extension Horizon Validation

The CLI validates `--extend-to` against the live network parameter `max_entry_ttl`. The value must satisfy `1 <= extend_to <= max_entry_ttl - 1`.

During live testnet verification (`max_entry_ttl = 3110400`), passing an out-of-bounds target (`--extend-to 999999999`) resulted in exit code `2`:

```console
$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 999999999 --output /tmp/axis_invalid.xdr; echo "exit=$?"
error: XDR build error: extend_to 999999999 must be in 1..=3110399 (max_entry_ttl - 1)
exit=2
```

## Exit Codes

| Exit Code | Meaning |
| --- | --- |
| `0` | Success. XDR file written. |
| `2` | Error (invalid flags, `extend-to` boundary violation, RPC error, XDR build error). |

Note: `extend` never exits with code `1`.

## Real Verbatim Output Examples

### 1. Generating Raw Operations (`--extend-to`)

```console
$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 518400 --output /tmp/axis_extend.xdr
wrote 1 unsigned ExtendFootprintTtl operation(s) (base64 XDR, one per line) to /tmp/axis_extend.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
extend target: 518400 ledgers from the current ledger (ledger 4567902)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ cat /tmp/axis_extend.xdr
AAAAAAAAABkAAAAAAAfpAA==
```

### 2. Generating Unsigned Envelope (`--source-account`)

```console
$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 518400 --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 --output /tmp/axis_extend_env.xdr
wrote unsigned ExtendFootprintTtl envelope (1 op(s), est. fee 24877143 stroops) to /tmp/axis_extend_env.xdr
source account: GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 · sequence: 19614875122663425 · signatures: EMPTY (sign with your own key)
extend target: 518400 ledgers from the current ledger (ledger 4567902)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
```