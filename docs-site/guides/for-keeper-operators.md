# For keeper operators

You are running `soroban-state-sentinel` inside an automated system — a cron
job, a CI pipeline, or something like the `action-state-watch` consumer this
tool's contract was designed for. The tool's job in your pipeline is one line:
tell you, reliably, when a contract's entries need attention. This guide covers
scripting that check, parsing the output, and deciding what to automate.

## The automation hook: exit code 1

The contract (see [`SCHEMA.md`](../../SCHEMA.md) and the
[JSON schema reference](../json-schema-reference.md)) is built around one
signal:

```bash
soroban-state-sentinel scan <contract-id> --fail-on-critical --json
echo "exit=$?"
```

- `0` — no entry is `critical` or `archived`. Nothing to do.
- `1` — at least one entry is `critical` or `archived`. Action required now.
- `2` — the invocation failed (bad arguments, RPC failure, bad XDR). Treat this
  as an infrastructure problem, not a contract-health problem.

Only `scan` ever sets exit 1, and only when `--fail-on-critical` is given. The
triggering condition is exactly `summary.has_critical == true`.

## Parsing the JSON

The `--json` document is versioned (`schema_version`, currently `1.1.0`).
Check the version before assuming field shapes; additive minor bumps never
reshape existing fields, but a major bump means coordinated migration.

The fields that matter to a keeper:

- `summary.has_critical` — boolean, mirrors the exit-code trigger.
- `summary.critical`, `summary.archived` — how many entries in each band.
- `entries[].band` — per-entry classification: `healthy`, `expiring_soon`,
  `critical`, `archived`.
- `entries[].ledgers_remaining`, `entries[].days_remaining` — how much time is
  left (the `days_remaining` value is a floor).
- `entries[].extend_to_healthy_cost_stroops` — cost to extend this entry to the
  healthy horizon (`null` when archived).
- `entries[].restore_cost_stroops` — cost to restore this entry (non-null only
  when archived).
- `network.max_entry_ttl`, `network.min_persistent_ttl` — the live bounds that
  govern what extend/restore can and will cost.
- `network.fee_per_rent_1kb_source`, `network.ledger_close_seconds_source` —
  provenance labels. If a cost projection matters, know whether it used an
  assumption (`state_size_high`, `default`) or an explicit input.

Example consumer logic:

```text
if summary.has_critical:
    for entry in entries:
        if entry.band in ("critical", "archived"):
            emit_alert(entry.id, entry.band, entry.days_remaining)
```

## Deciding: auto-extend vs. alert a human

Two remediation actions exist, and they have different risk profiles:

- **`extend`** (`ExtendFootprintTTLOp`) is low-risk and idempotent in spirit: it
  pushes TTL out for entries that are still live. A keeper can reasonably
  auto-generate (and even auto-sign and submit, in a system that holds keys)
  an extend for a `critical` entry — the downside of an over-eager extension is
  mostly a small extra fee.
- **`restore`** (`RestoreFootprintOp`) is the consequence of having missed the
  window. The entry was already unarchived-unreadable; restoring costs the full
  minimum-TTL rent and is the signal that the extend cadence was wrong. Alert a
  human.

A concrete policy this tool supports:

1. Run the check every N ledgers (e.g. every 6 hours at 5 s/ledger).
2. On exit 1 with only `critical` entries: generate the extend XDR for the
   affected contract and route it to your signing service.
3. On exit 1 with any `archived` entry: page a human. Also consider extending
   *more aggressively* for entries that were near the boundary last run.

## Generating XDR inside the pipeline

`extend` and `restore` write a file, not stdout JSON:

```bash
soroban-state-sentinel extend <contract-id> \
  --extend-to 518400 --output extend.xdr
```

Without `--source-account`, `extend.xdr` is the raw operations, one base64-XDR
per line. With `--source-account <G…>`, it is a complete unsigned
`TransactionV1Envelope` — sequence number fetched from the ledger, fee
estimated, `signatures: []` — ready for your signing service to fill in. The
fee estimates are labeled (`fee note: … assumed size … B`): archived entries'
sizes are assumed (1,024 B default) because RPC cannot read them; core charges
the actual rent at apply and refunds unused refundable fee.

Validate targets in code: extend targets must be in `1..=max_entry_ttl - 1`
(3,110,399 on the protocol-28 testnet this tool was verified against), or the
command exits 2 and writes nothing.

## The trust boundary

`soroban-state-sentinel` will **never** sign or submit anything for you. There
is no private-key handling anywhere in the repository, and this is a deliberate,
load-bearing design decision (see [`CONTRIBUTING.md`](../../CONTRIBUTING.md) —
no signing capability, ever). Your automation must supply signing and
submission separately: generate the unsigned XDR here, hand it to a
separately-secured key-holding service, and submit the signed transaction with
your own tooling. Do not build key handling into anything that consumes this
tool's output.