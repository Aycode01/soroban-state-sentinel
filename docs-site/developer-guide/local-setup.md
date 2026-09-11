# Local Setup

This guide details cloning, building, and running tests for the `soroban-state-sentinel` codebase.

## Prerequisites

- **Rust**: Pinned to stable channel (minimum supported Rust version `1.84`, as defined in `Cargo.toml` and `rust-toolchain.toml`).
- **Cargo Components**: `rustfmt`, `clippy`.

## Workspace Architecture

The workspace consists of 5 modular library crates and 1 root integration test crate:

```
crates/
├── cli/          # sentinel-cli: CLI binary interface & output formatting
├── rent-model/   # sentinel-rent-model: Canonical fee math ported from soroban-env-host
├── rpc-client/   # sentinel-rpc-client: Soroban JSON-RPC protocol 28 communication
├── ttl-scanner/  # sentinel-ttl-scanner: TTL health classification & horizon rules
└── xdr-builder/  # sentinel-xdr-builder: Unsigned XDR op and envelope generation
tests/            # integration_test.rs & mock_rpc.rs test harness
```

## Build Steps

Clone the repository and build all workspace crates:

```bash
git clone https://github.com/stellar-archival-labs/soroban-state-sentinel.git
cd soroban-state-sentinel
cargo build --workspace
```

To build the optimized release binary:

```bash
cargo build --release -p sentinel-cli
```

The release binary will be placed at `target/release/soroban-state-sentinel`.

## Running Tests

Run the full workspace unit and integration test suite:

```bash
cargo test --workspace
```

### Real Test Pass Count

The workspace test suite currently executes and passes **49 tests** in total:

- `sentinel-cli` (unit tests): 8 passed
- `sentinel-rent-model` (unit tests): 9 passed
- `sentinel-rpc-client` (unit tests): 2 passed
- `sentinel-ttl-scanner` (unit tests): 8 passed
- `sentinel-xdr-builder` (unit tests): 15 passed
- `soroban-state-sentinel-tests` (integration suite): 7 passed