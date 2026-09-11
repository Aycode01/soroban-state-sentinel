# Contributing Guidelines

This document summarizes the core principles for contributing to `soroban-state-sentinel`. The canonical source of truth is the repo's [CONTRIBUTING.md](../CONTRIBUTING.md) file.

## Non-Negotiable Rules Summary

1. **No Signing Capability**: This tool is strictly read-only and unsigned-by-design. Never introduce private key handling or transaction signing code. See [SECURITY.md](../SECURITY.md).
2. **No Invented Protocol Parameters**: Protocol constants (TTL bounds, fee rates, ledger limits) must come from live network RPC responses or explicit CLI inputs. Never hardcode training memory defaults.
3. **Ported Fee Math**: Rent fee calculations in `crates/rent-model` are a line-by-line port of `soroban-env-host/src/fees.rs` (Protocol 28). Any changes must cite and mirror upstream host code.
4. **Explicit Unit Documentation**: Document units (ledgers, seconds, days, stroops, bytes) on every public function and data structure.
5. **No `unwrap()` / `expect()` Outside Tests**: Production code must use checked and saturating integer arithmetic to avoid panics.
6. **JSON Contract Stability**: Output changes to `scan --json` must comply with [SCHEMA.md](../SCHEMA.md) versioning rules to prevent breaking downstream consumers like `action-state-watch`.

## Quality Gates & CI

All PRs must pass the following checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
```

For full details on development setup, PR rules, and security policies, refer to the primary [CONTRIBUTING.md](../CONTRIBUTING.md).