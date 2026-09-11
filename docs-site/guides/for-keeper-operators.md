# Guide for Keeper Operators

This guide explains how to integrate `soroban-state-sentinel` into automated monitoring pipelines, CI actions, and keeper bots (such as `action-state-watch`).

## 1. Automated Health Scanning

Run `scan` with `--json` and `--fail-on-critical`:

```bash
soroban-state-sentinel scan CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX \
  --json \
  --fail-on-critical \
  > /tmp/scan_results.json
```

### Exit Code Handling

- **Exit Code `0`**: All entries are in `healthy` or `expiring_soon` bands (`summary.has_critical == false`). No immediate action required.
- **Exit Code `1`**: One or more entries are in `critical` or `archived` bands (`summary.has_critical == true`). Immediate action required.
- **Exit Code `2`**: RPC error or invalid parameters. Pipeline should alert on operational failure.

## 2. Parsing JSON Output

In automated scripts, parse `/tmp/scan_results.json` to inspect individual entry status:

- Inspect `summary.has_critical` to detect urgent conditions.
- Iterate over `entries[]`:
  - If `band == "critical"`, inspect `extend_to_healthy_cost_stroops`.
  - If `band == "archived"`, inspect `restore_cost_stroops`.

## 3. Automated XDR Generation vs Alerting

When `exit code 1` occurs, your keeper pipeline can choose between two remediation strategies:

### Option A: Auto-generate Unsigned Remediation XDR

Automated pipelines can invoke `extend` or `restore` with `--source-account` to generate an unsigned `TransactionV1Envelope` artifact:

```bash
soroban-state-sentinel extend CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX \
  --extend-to-days 30 \
  --source-account GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5 \
  --output /tmp/unsigned_extend_envelope.xdr
```

The resulting file can be uploaded as a pipeline artifact or pushed to a queue for an external signer.

### Option B: Human Alerting

Send formatted alerts (Slack, Discord, GitHub Issues) containing contract ID, remaining ledgers, health band, and calculated stroop costs.

## 4. Trust Boundary Reminder

`soroban-state-sentinel` is strictly read-only and unsigned-by-design. It never stores private keys and cannot sign transactions. Automated keeper systems must pass generated XDR artifacts to a separately secured signing process or KMS to execute transactions on-chain.