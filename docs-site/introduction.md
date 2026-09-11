# Introduction

`soroban-state-sentinel` is a read-only command-line interface (CLI) that inspects deployed Soroban smart contract ledger entries on Stellar networks, reports their Time-To-Live (TTL) and state-archival health, and builds unsigned remediation transactions to extend or restore them.

## The Archival Risk in Practice

State archival on Soroban automatically evicts inactive persistent ledger entries when their TTL counter reaches zero. During live testnet verification documented in `docs/live-verification.md`, the publicly deployed contract `CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX` was discovered in a `Critical` health state with only 54,482 ledgers remaining (approximately 3 days at 5 seconds per ledger). Without active monitoring and remediation, such contracts become unreadable on-chain, blocking execution until restored.

## How It Works

1. **Scan**: You point `soroban-state-sentinel` at a target contract ID (and optional storage keys).
2. **Read**: The tool queries the network RPC for `getLatestLedger` and `getLedgerEntries` to extract each entry's `liveUntilLedgerSeq`.
3. **Classify**: It evaluates remaining ledgers against configurable health thresholds (`Healthy`, `ExpiringSoon`, `Critical`, `Archived`).
4. **Remediate**: If entries need extension or restoration, the tool generates raw base64 operations or an unsigned `TransactionV1Envelope`.
5. **Submit**: A human operator, multisig coordinator, or external keeper process signs and submits the transaction to the network.

This tool never holds or uses a private key.