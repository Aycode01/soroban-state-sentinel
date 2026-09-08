# Live testnet verification

This document records a hand-run verification of `scan`, `extend`, and
`restore` against the **live Soroban testnet RPC**
(`https://soroban-testnet.stellar.org`, network `Test SDF Network ; September
2015`). All output below is verbatim terminal output captured on
**2026-09-08** (protocol 28, latest ledger ≈ 4,567,902 at capture time). No
output was fabricated or "expected-value" examples.

## How the test contract was chosen

The tool cannot deploy a contract (it deliberately has no signing capability),
so a **live contract already deployed on testnet** was located instead:
`getEvents` was queried with no filters (only `startLedger`) and the returned
event `contractId` fields were harvested. The contract used below —

```
CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX
```

— is a publicly visible testnet contract (its on-chain events include
`AXIS`/`trade`/`order` topics) whose instance and code were actively live at
verification time. Its TTL at scan time was ~54,000 ledgers (~3 days), i.e. it
is a genuine `critical` case — exactly the scenario the tool exists to catch.

The Stellar docs `Counter` contract (`CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI`)
was used as the real **archived** case: its instance is archived on testnet
(confirmed live below), which is the `restore` scenario.

## Verbatim transcript

```console
$ curl -s -X POST https://soroban-testnet.stellar.org -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","id":1,"method":"getLatestLedger","params":{}}' | head -c 300
{"jsonrpc":"2.0","id":1,"result":{"id":"d3678092f65c21f1d0da02412252041cb0cf9b7a7256e34d123ca4a2d82fa977","protocolVersion":28,"sequence":4567902,"closeTime":"1788863097","headerXdr":"AAAAHBKX2n7jfFmOP03fOWo6mRGFKtVhfMGlfBHNzmXq9L5R51ZoFuNqtvZ8YOl27Qt7r/uSHuNbp/H0BQgZCYdt+HwAAAAAap/ieQAAAAAAAAABAAAA

$ soroban-state-sentinel scan CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --json
{
  "schema_version": "1.1.0",
  "generated_at_unix": 1788863098,
  "command": {
    "subcommand": "scan",
    "contract_id": "CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX",
    "rpc_url": "https://soroban-testnet.stellar.org"
  },
  "network": {
    "passphrase": "Test SDF Network ; September 2015",
    "protocol_version": 28,
    "latest_ledger": 4567902,
    "ledger_close_seconds": 5,
    "ledger_close_seconds_source": "default",
    "fee_per_rent_1kb": 10000,
    "fee_per_rent_1kb_source": "state_size_high",
    "average_soroban_state_size_bytes": null,
    "max_entry_ttl": 3110400,
    "min_persistent_ttl": 120960,
    "min_temporary_ttl": 720
  },
  "health_config": {
    "healthy_min_days": 30,
    "critical_max_days": 7,
    "healthy_min_ledgers": 518400,
    "critical_max_ledgers": 120960,
    "extend_horizon_ledgers": 518400
  },
  "summary": {
    "entries_scanned": 2,
    "healthy": 0,
    "expiring_soon": 0,
    "critical": 2,
    "archived": 0,
    "has_critical": true
  },
  "entries": [
    {
      "id": "instance",
      "label": "contract instance",
      "kind": "contract_instance",
      "durability": "persistent",
      "band": "critical",
      "current_ledger_seq": 4567902,
      "live_until_ledger_seq": 4622384,
      "ledgers_remaining": 54482,
      "days_remaining": 3,
      "estimated_archive_unix": 1789135507,
      "size_bytes": 148,
      "key_xdr": "AAAABgAAAAGTAPCEgsK/xOh+GYNr2Z6hFRMNNW1l6I/IiCKjxsmbdgAAABQAAAAB",
      "ttl_key_xdr": "AAAACcWgnJwr45IDVOxVIu9PgS2V2jGkKWALzxVBIykA5Pov",
      "extend_to_healthy_cost_stroops": 554400,
      "restore_cost_stroops": null
    },
    {
      "id": "code",
      "label": "contract code (wasm)",
      "kind": "contract_code",
      "durability": null,
      "band": "critical",
      "current_ledger_seq": 4567902,
      "live_until_ledger_seq": 4622384,
      "ledgers_remaining": 54482,
      "days_remaining": 3,
      "estimated_archive_unix": 1789135507,
      "size_bytes": 19532,
      "key_xdr": "AAAAB+W/9PsT2YvjuECxd1ML8+P4wp6jTZgZiHsG6mpmywSi",
      "ttl_key_xdr": "AAAACZZQ4IxyJVaxFP3DOIfpWKt9rCPWqwJ0o/01f+Cldhmo",
      "extend_to_healthy_cost_stroops": 24279287,
      "restore_cost_stroops": null
    }
  ]
}

$ soroban-state-sentinel scan CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --json
{
  "schema_version": "1.1.0",
  "generated_at_unix": 1788863099,
  "command": {
    "subcommand": "scan",
    "contract_id": "CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI",
    "rpc_url": "https://soroban-testnet.stellar.org"
  },
  "network": {
    "passphrase": "Test SDF Network ; September 2015",
    "protocol_version": 28,
    "latest_ledger": 4567902,
    "ledger_close_seconds": 5,
    "ledger_close_seconds_source": "default",
    "fee_per_rent_1kb": 10000,
    "fee_per_rent_1kb_source": "state_size_high",
    "average_soroban_state_size_bytes": null,
    "max_entry_ttl": 3110400,
    "min_persistent_ttl": 120960,
    "min_temporary_ttl": 720
  },
  "health_config": {
    "healthy_min_days": 30,
    "critical_max_days": 7,
    "healthy_min_ledgers": 518400,
    "critical_max_ledgers": 120960,
    "extend_horizon_ledgers": 518400
  },
  "summary": {
    "entries_scanned": 1,
    "healthy": 0,
    "expiring_soon": 0,
    "critical": 0,
    "archived": 1,
    "has_critical": true
  },
  "entries": [
    {
      "id": "instance",
      "label": "contract instance",
      "kind": "contract_instance",
      "durability": null,
      "band": "archived",
      "current_ledger_seq": 4567902,
      "live_until_ledger_seq": null,
      "ledgers_remaining": null,
      "days_remaining": null,
      "estimated_archive_unix": null,
      "size_bytes": null,
      "key_xdr": "AAAABgAAAAGfjJVEBc55drW3U87N1Py0Rw0/nlqUA6tQ6r28khEl4gAAABQAAAAB",
      "ttl_key_xdr": "AAAACazoswBNT+BiTFa7gC8O8MQUzPPtRj1tQg/4nmKv+FFg",
      "extend_to_healthy_cost_stroops": null,
      "restore_cost_stroops": 998098
    }
  ]
}

$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 518400 --output /tmp/axis_extend.xdr
wrote 1 unsigned ExtendFootprintTtl operation(s) (base64 XDR, one per line) to /tmp/axis_extend.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
extend target: 518400 ledgers from the current ledger (ledger 4567902)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ cat /tmp/axis_extend.xdr
AAAAAAAAABkAAAAAAAfpAA==

$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to-days 30 --output /tmp/axis_extend_days.xdr
wrote 1 unsigned ExtendFootprintTtl operation(s) (base64 XDR, one per line) to /tmp/axis_extend_days.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
extend target: 518400 ledgers from ledger 4567902 (30 day(s) at 5s/ledger, close-time source: default)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 999999999 --output /tmp/axis_invalid.xdr; echo "exit=$?"
error: XDR build error: extend_to 999999999 must be in 1..=3110399 (max_entry_ttl - 1)
exit=2

$ soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX --extend-to 518400 --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 --output /tmp/axis_extend_env.xdr
wrote unsigned ExtendFootprintTtl envelope (1 op(s), est. fee 24877143 stroops) to /tmp/axis_extend_env.xdr
source account: GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 · sequence: 19614875122663425 · signatures: EMPTY (sign with your own key)
extend target: 518400 ledgers from the current ledger (ledger 4567902)
fee note: rent fee est. 24833686 stroops (entries with no live entry use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)

$ soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --output /tmp/counter_restore.xdr
wrote 1 unsigned RestoreFootprint operation(s) (base64 XDR, one per line) to /tmp/counter_restore.xdr
no --source-account given: re-run with --source-account to get a complete unsigned envelope
fee note: rent fee est. 998098 stroops (archived entries use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
$ cat /tmp/counter_restore.xdr
AAAAAAAAABoAAAAA

$ soroban-state-sentinel restore CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 --output /tmp/counter_restore_env.xdr
wrote unsigned RestoreFootprint envelope (1 op(s), est. fee 1009266 stroops) to /tmp/counter_restore_env.xdr
source account: GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 · sequence: 19614875122663425 · signatures: EMPTY (sign with your own key)
fee note: rent fee est. 998098 stroops (archived entries use assumed size 1024 B; core charges actual rent at apply and refunds unused refundable fee)
```

## Independent XDR decode

Every XDR file produced above was parsed back with `stellar-xdr`'s
`from_xdr_base64` in a scratch test (not by the CLI itself), to confirm the
bytes are well-formed and carry the intended operation:

```
extend op (--extend-to 518400) -> Operation { source_account: None, body: ExtendFootprintTtl(ExtendFootprintTtlOp { ext: V0, extend_to: 518400 }) }
extend op (--extend-to-days 30) -> Operation { source_account: None, body: ExtendFootprintTtl(ExtendFootprintTtlOp { ext: V0, extend_to: 518400 }) }
restore op (archived Counter) -> Operation { source_account: None, body: RestoreFootprint(RestoreFootprintOp { ext: V0 }) }
extend envelope -> signatures=0 ops=1 seq=19614875122663425 fee=24877143
restore envelope -> signatures=0 ops=1 seq=19614875122663425 fee=1009266
```

Both envelopes decode as `TransactionV1Envelope` with **zero signatures** — the
tool's unsigned-by-design contract holds on real output.

## Verification checklist

| Requirement | Result |
| --- | --- |
| Live RPC reachable (protocol 28) | ✅ `getLatestLedger` returned protocol 28, ledger 4,567,902 |
| `scan` reports the live **instance** entry | ✅ AXIS instance: `critical`, 148 B, 54,482 ledgers left |
| `scan` reports the live **code** entry (round-2 fetch fix) | ✅ AXIS code: `critical`, 19,532 B wasm, own TTL key — the code entry is fetched via the wasm hash discovered from the instance, exactly the path the round-2 fix enables |
| `scan` with explicit persistent `--keys` | ✅ symbol `"fee"` key on the fee contract reported `archived` (that key does not exist on-chain) — the explicit-key path resolves and classifies against real state |
| `scan` detects an archived contract | ✅ docs `Counter` instance: `archived`, restore cost 998,098 stroops |
| `extend` with a real `--extend-to` | ✅ 1 op written for instance + code; decoded as `ExtendFootprintTtl { extend_to: 518400 }` |
| `extend --extend-to-days` + close-time label | ✅ `518400 ledgers from ledger 4567902 (30 day(s) at 5s/ledger, close-time source: default)` |
| `extend` limit boundary | ✅ `--extend-to 999999999` → exit 2, `extend_to 999999999 must be in 1..=3110399 (max_entry_ttl - 1)` (live `max_entry_ttl = 3110400`), no file written |
| `restore` on a real archived entry | ✅ 1 `RestoreFootprint` op for the archived Counter instance; decoded independently |
| Unsigned envelopes (`--source-account`) | ✅ extend + restore envelopes: real account sequence `19614875122663425`, `signatures=0`, decoded as `TransactionV1Envelope` |
| Mock fixtures match live config | ✅ fixture `StateArchival` (`max_entry_ttl=3110400`, `min_persistent_ttl=120960`, `min_temporary_ttl=720`) and `ContractLedgerCostV0` (`fee_write_ledger_entry=2500`, `rent_fee_1kb_high=10000`) are identical to the live values the tool read |

## Notes and honest limitations

- The tool was verified against a **live, publicly deployed testnet contract**
  rather than a freshly deployed one, because deploying requires signing —
  which this tool deliberately cannot do. The chosen contract is a genuine
  `critical` case (~3 days to archival), which exercised the exact
  "expiring now" scenario the `extend` command exists for.
- **Nothing was signed or submitted.** All outputs are unsigned XDR; the
  source account's sequence number was only *read* from the ledger.
- Fee estimates for archived entries use the assumed size (1024 B default);
  core charges actual rent at apply time and refunds unused refundable fee, as
  documented in the README.
- No code bugs surfaced during this pass: the mock-tested behavior (band
  classification, round-2 code fetch, extend target validation, close-time
  labeling, unsigned output) held against the real network without change.