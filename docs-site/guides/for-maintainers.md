# Guide for Maintainers

This guide walks through manually checking and maintaining the archival health of a deployed Soroban contract.

## 1. Install the CLI

Build the release binary using Cargo:

```bash
cargo build --release -p sentinel-cli
```

The compiled binary `soroban-state-sentinel` is located in `target/release/`.

## 2. Check Contract Health

Scan your target contract ID against the live network:

```bash
soroban-state-sentinel scan CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX
```

The output displays a table summarizing the health status of the contract instance and WASM code:
- **`healthy`**: More than 30 days of TTL remaining. No action required.
- **`expiring_soon`**: Between 7 and 30 days remaining. Schedule an extension.
- **`critical`**: Under 7 days remaining. Extend immediately.
- **`archived`**: Entry is evicted and unreadable. Must be restored.

## 3. Remediate Expiring Entries

If entries display `expiring_soon` or `critical`, generate an unsigned `ExtendFootprintTTLOp` transaction extending TTL by 30 days:

```bash
soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX \
  --extend-to-days 30 \
  --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 \
  --output /tmp/extend_tx.xdr
```

If entries display `archived`, generate an unsigned `RestoreFootprintOp` transaction instead:

```bash
soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI \
  --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 \
  --output /tmp/restore_tx.xdr
```

## 4. Sign and Submit

The generated XDR file contains a complete `TransactionV1Envelope` with empty signatures.

Pass `/tmp/extend_tx.xdr` or `/tmp/restore_tx.xdr` to your organization's keyholder, multisig coordinator, or hardware wallet (e.g. using `stellar-cli` or Stellar Laboratory) to sign and submit the transaction to the Stellar network.