# Contributing

Thanks for working on `soroban-state-sentinel`. This is a monitoring /
unsigned-transaction-builder tool; several disciplines here are load-bearing,
not stylistic. Please read this whole file before opening a PR.

## Development setup

```bash
# The toolchain is pinned via rust-toolchain.toml; rustup installs it on first use.
cargo build -p sentinel-cli   # integration tests spawn this binary
cargo test --workspace
```

Quality gates (all enforced in CI):

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

## Non-negotiable rules

1. **No signing capability, ever.** This tool never holds a private key and
   never signs or submits a transaction. Do not add key handling, signing
   code, or transaction submission under any circumstance — that is a different
   trust boundary and mixing it into a monitoring tool is a security design
   mistake. See [SECURITY.md](SECURITY.md).
2. **Never invent protocol parameters.** Rent-rate denominators, TTL bounds,
   resource limits, ledger close time, contract size limits — all of it comes
   from the live network (RPC config settings, `getLatestLedger`) or is an
   explicit, labeled CLI input. No hardcoded constants from training data or
   memory. When a reference says `BumpFootprintInstanceOp` /
   `BumpFootprintExpirationOp` or `expirationLedgerSeq`, it is stale
   pre-protocol-20 material; the live names are `ExtendFootprintTTLOp` /
   `RestoreFootprintOp` and `liveUntilLedgerSeq`. Verify against the
   `stellar-xdr` crate for the live protocol before writing XDR code.
3. **Fee math is ported, not invented.** `crates/rent-model` is a line-by-line
   port of `soroban-env-host`'s `fees.rs` with a source citation at the top of
   the file. If you change the math, re-verify against upstream and update the
   citation; the port exists so it can be diffed when the network upgrades.
4. **Units are documented everywhere.** Every function touching ledger
   sequence numbers, TTL values, or fees documents its units (ledgers vs. days
   vs. seconds vs. stroops). This is the most common source of silent bugs in
   TTL tooling.
5. **No `unwrap()`/`expect()` outside `#[cfg(test)]`.** Integer math only —
   no floats for fees, ledgers, or bytes. Checked/saturating arithmetic.
6. **Don't break the JSON contract.** `scan --json` emits a versioned schema
   documented in [SCHEMA.md](SCHEMA.md) that `action-state-watch` parses.
   Breaking changes require a version bump there, not a silent reshape.

## Git workflow

- Conventional commit format: `type(scope): description`
  (`feat(rpc-client): …`, `fix(ttl-scanner): …`, `test(cli): …`, `docs: …`,
  `ci: …`, `chore: …`).
- One commit per logical unit. Never `git add .` — stage specific files.
- Push after each commit (no batching).
- PRs should be small and reviewable; a PR that touches the rent math must
  include the re-verified upstream diff in its description.

## What to verify against the live network

- RPC transport limits (e.g. `getLedgerEntries` key cap, footprint entry
  limits) — re-check against the Soroban RPC API reference when protocol
  versions change.
- `ExtendFootprintTTLOp` / `RestoreFootprintOp` field layouts — verify against
  the `stellar-xdr` crate for the protocol version in use.
- Restored persistent entries get their TTL reset to the network's minimum for
  newly created entries — confirm the exact minimum (`min_persistent_ttl`)
  against live network config; do not hardcode it.

## Testing

- **Unit tests** live next to the code (`#[cfg(test)]` modules) — boundary
  tests for each health band, known-fee-value tests for the rent model, XDR
  round-trip tests for the builders.
- **Integration tests** (`tests/`) spawn the compiled CLI against a local mock
  Soroban RPC server (`tests/mock_rpc.rs`) seeded with real testnet fixtures
  (see `tests/fixtures/README.md`) plus synthetic live entries built with real
  `stellar-xdr` types. Build the binary first: `cargo build -p sentinel-cli`.

## Reporting issues

Bugs and feature requests go to GitHub issues. Security issues do **not** — see
[SECURITY.md](SECURITY.md).