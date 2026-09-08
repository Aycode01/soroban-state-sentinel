# Introduction

`soroban-state-sentinel` is a read-only command-line tool that reports the
TTL / state-archival health of deployed Soroban contract entries and generates
**unsigned** remediation XDR when an entry needs attention.

You point it at a contract id. It connects to a Soroban RPC endpoint, reads the
contract's ledger entries, and classifies each one into a health band:
`healthy`, `expiring_soon`, `critical`, or `archived`. For every entry it
reports how many ledgers (and whole days) remain before archival, plus the exact
stroop cost to extend or restore it.

## The failure mode this tool exists to catch

This is not a hypothetical. During this project's own verification pass
(`docs/live-verification.md`, captured 2026-09-08), a live testnet contract
(`CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX`) was found in
`critical` state with **54,482 ledgers (~3 days)** remaining before archival —
both its instance and its code were at or below the 7-day critical threshold.
That contract is a real, sourced example of exactly the "expiring now" scenario
the `extend` command exists for.

## How it works

1. **Point the tool at a contract.** `soroban-state-sentinel scan <contract-id>`
   — plus optional explicit storage keys for the data entries you care about.
2. **It reads TTL state from RPC.** The scan engine fetches each entry's
   `liveUntilLedgerSeq` from the RPC response and compares it against the
   latest ledger.
3. **It classifies health.** Entries are placed in one of the four bands using
   the healthy/critical thresholds (30 days / 7 days by default — see
   [The archival problem](the-archival-problem.md)).
4. **It optionally builds unsigned XDR to fix it.** `extend` produces unsigned
   `ExtendFootprintTTLOp` operations for entries approaching archival;
   `restore` produces unsigned `RestoreFootprintOp` operations for entries
   already archived.
5. **A human or keeper signs and submits.** The tool stops at unsigned XDR. The
   signing and submission happen outside the tool, in whatever system holds
   the key.

## Trust boundary

This tool never holds or uses a private key. There is deliberately no signing
capability anywhere in the repository, and it never submits a transaction. Its
outputs are unsigned by design: base64 XDR that a human, a multisig, or a
separately-secured keeper process signs and submits.