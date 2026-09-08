# Local setup

This page gets a contributor from clone to a passing test suite. It mirrors the
repo's actual `Cargo.toml` and `rust-toolchain.toml`.

## Toolchain

The toolchain is pinned in [`rust-toolchain.toml`](../../rust-toolchain.toml):

- `channel = "stable"` with `profile = "minimal"` and the `rustfmt` and
  `clippy` components. `rustup` installs it on first use.
- The workspace declares `rust-version = "1.84"` and `edition = "2021"`
  (workspace `Cargo.toml`). The rent model and XDR code target protocol 28
  (`stellar-xdr` 28.x), which requires Rust ≥ 1.84.

## Clone and build

```bash
git clone https://github.com/stellar-archival-labs/soroban-state-sentinel
cd soroban-state-sentinel
cargo build --release            # release binary
cargo build -p sentinel-cli      # debug binary, needed before running tests
```

The workspace (`resolver = "2"`) has five member crates, each with one job:

| Crate | Responsibility |
| --- | --- |
| `crates/rpc-client` (`sentinel-rpc-client`) | Typed JSON-RPC client; read-only by construction. |
| `crates/ttl-scanner` (`sentinel-ttl-scanner`) | Entry health classification into the four bands. |
| `crates/rent-model` (`sentinel-rent-model`) | Ported rent-fee computation + stroop projections. |
| `crates/xdr-builder` (`sentinel-xdr-builder`) | Unsigned `ExtendFootprintTtl` / `RestoreFootprint` builders. |
| `crates/cli` (`sentinel-cli`) | The `soroban-state-sentinel` binary. |

The root package owns the integration tests (`tests/`) and has no library or
binary of its own — the tests spawn the compiled CLI binary and talk to a local
mock RPC server.

## Run the tests

```bash
cargo build -p sentinel-cli   # integration tests spawn this binary
cargo test --workspace
```

Current pass count: **49 tests** (42 unit tests across the five crates plus 7
integration tests in `tests/` that spawn the CLI against the mock RPC), 0
failures, captured from a local `cargo test --workspace` run on 2026-09-08.

<!-- VERIFY: update this count whenever the test suite changes. The authoritative
number is the latest CI run (.github/workflows/ci.yml runs
`cargo test --workspace`) or a fresh local run; this page should never show a
stale count. -->

## Quality gates (enforced in CI)

CI (`.github/workflows/ci.yml`) runs on every push to `main` and every pull
request:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build -p sentinel-cli   # before tests
cargo test --workspace
```

## A note on network access

The integration tests do **not** hit a live network. They run against the mock
RPC server (`tests/mock_rpc.rs`) seeded with fixtures captured from the live
testnet on 2026-09-08 — see [Mock RPC for testing](mock-rpc-for-testing.md).
The only thing a live network is needed for is the `docs/live-verification.md`
pass, which is a hand-run exercise, not part of CI.