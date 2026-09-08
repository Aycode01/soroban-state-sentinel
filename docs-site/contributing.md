# Contributing

The full contribution guide lives in
[`CONTRIBUTING.md`](../CONTRIBUTING.md) — read it before opening a PR. This
page summarizes its non-negotiable rules so a contributor knows what to expect;
the canonical file is the source of truth, and this summary intentionally does
not duplicate it verbatim.

## Non-negotiable rules

These are load-bearing disciplines for a monitoring / unsigned-transaction
tool, not style preferences:

1. **No signing capability, ever.** The tool never holds a private key, never
   signs, and never submits a transaction. Do not add key handling, signing
   code, or transaction submission under any circumstance — mixing that into a
   monitoring tool is a security design mistake (see
   [`SECURITY.md`](../SECURITY.md)).
2. **Never invent protocol parameters.** Rent-rate denominators, TTL bounds,
   resource limits, ledger close time, contract size limits — all of it comes
   from the live network (RPC config settings, `getLatestLedger`) or is an
   explicit, labeled CLI input. No hardcoded constants from training data or
   memory. Live operation names are `ExtendFootprintTTLOp` /
   `RestoreFootprintOp` with `liveUntilLedgerSeq`; `BumpFootprint*` /
   `expirationLedgerSeq` references are stale pre-protocol-20 material.
3. **Fee math is ported, not invented.** `crates/rent-model` is a line-by-line
   port of `soroban-env-host`'s `fees.rs` with a source citation at the top of
   the file. Changed math must be re-verified against upstream and the citation
   updated.
4. **Units are documented everywhere.** Every function touching ledger sequence
   numbers, TTL values, or fees documents its units (ledgers vs days vs seconds
   vs stroops). This is the most common source of silent bugs in TTL tooling.
5. **No `unwrap()`/`expect()` outside `#[cfg(test)]`.** Integer math only — no
   floats for fees, ledgers, or bytes; checked/saturating arithmetic.
6. **Don't break the JSON contract.** `scan --json` emits a versioned schema
   documented in [`SCHEMA.md`](../SCHEMA.md) that `action-state-watch` parses.
   Breaking changes require a version bump there, coordinated with consumers —
   never a silent reshape.

## Quality gates

All enforced in CI on every push and pull request:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

Build the CLI first (`cargo build -p sentinel-cli`) — the integration tests
spawn the compiled binary.

## Git workflow

- Conventional commit format: `type(scope): description`
  (`feat(rpc-client): …`, `fix(ttl-scanner): …`, `test(cli): …`, `docs: …`,
  `ci: …`, `chore: …`).
- One commit per logical unit. Never `git add .` — stage specific files.
- Push after each commit (no batching).
- PRs should be small and reviewable; a PR that touches the rent math must
  include the re-verified upstream diff in its description.

## Reporting issues

Bugs and feature requests go to GitHub issues. Security issues do **not** —
see [`SECURITY.md`](../SECURITY.md).

## This site

This documentation site lives in `docs-site/` as plain GitBook-compatible
Markdown. The README stays a quick-start; this site is the full reference.
Facts here are sourced from the repo — `SCHEMA.md`, the `clap` definitions,
`docs/live-verification.md`, and the test fixtures. If you change any of those,
update the matching page here rather than leaving it stale; numbers that cannot
be sourced are marked with a `<!-- VERIFY: ... -->` comment.