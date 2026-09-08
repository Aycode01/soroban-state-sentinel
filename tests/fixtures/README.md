# Test fixtures

These fixtures were captured from the **live testnet RPC** and are used by the
integration tests through the local mock server (`tests/mock_rpc.rs`). No
fabricated numbers: every value below came from an actual `getLedgerEntries` /
`getLatestLedger` / `getNetwork` call against
`https://soroban-testnet.stellar.org` on **2026-09-08** (protocol 28, network
`Test SDF Network ; September 2015`).

| File | Source | Notes |
| --- | --- | --- |
| `getNetwork.json` | `getNetwork` | Network passphrase + protocol version. |
| `getLatestLedger.json` | `getLatestLedger` | Trimmed to the fields the client consumes (the raw `headerXdr`/`metadataXdr` are dropped here; the values are from the real response). |
| `getLedgerEntries_config.json` | `getLedgerEntries` on the 7 `CONFIG_SETTING` keys | Real values for `ContractLedgerCostV0`, `ContractLedgerCostExtV0`, `StateArchival`, `ContractComputeV0`, `ContractHistoricalDataV0`, `ContractEventsV0`, `ContractBandwidthV0`. |
| `getLedgerEntries_counter_instance.json` | `getLedgerEntries` on the docs `Counter` contract's instance key (`CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI`) | Returns `entries: []` — the instance is archived, which is exactly the "needs restoration" case the sentinel detects. |

The live-contract entries used in the band-classification tests are **synthetic
by design** (built in `tests/integration_test.rs` with the real `stellar-xdr`
types and served by the mock): the testnet has no contract in each band at a
predictable TTL, so the mock synthesizes entries at `Healthy` /
`ExpiringSoon` / `Critical` TTLs. The *shapes* are real XDR; only the TTL values
are chosen by the test.