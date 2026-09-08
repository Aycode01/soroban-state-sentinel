# Security Policy

## Trust boundary

`soroban-state-sentinel` is a **read-and-report tool with an optional
unsigned-transaction-builder mode**. It is designed to be safe to run in
monitoring environments where a private key must never be present:

- It never holds, parses, or derives a private key.
- It never signs a transaction.
- It never submits a transaction.
- Its only outputs are reports and **unsigned** XDR (`signatures: []`).

This boundary is deliberate and permanent. A tool that can both observe state
and spend from a key is a single point of compromise; separating them means a
compromised sentinel can mislead (via reports) or mis-build (via unsigned XDR
that still requires a separate signer to approve), but cannot move funds.

## What this means in practice

- The `S…` strkey (secret key) format is **rejected** anywhere a public
  identifier is expected (`decode_contract_id`, `decode_account_id`), by
  construction — the codebase has no secret-key handling at all.
- Do not add signing, secret-key handling, or transaction submission to this
  repository. Do not "convenience-wrap" the output builder with a key.
  Proposals that do will be rejected.
- The signing step belongs in a separate process under its own trust boundary
  (a human wallet, a multisig, or a separately-secured keeper).

## Threat model

| Capability | Attacker controlling an RPC endpoint | Attacker controlling the sentinel process |
| --- | --- | --- |
| Misreport TTL health | Yes — responses are taken at face value (they are validated as typed XDR, but not cross-checked against another source). | Yes |
| Build malicious unsigned XDR | Yes (keys/values are what the RPC says) | Yes |
| Sign or submit a transaction | **No** — nothing to sign with, nothing to submit | **No** |
| Exfiltrate a private key | **No** — no key exists in the process | **No** |

The mitigation for the first two rows is operational: run the sentinel against
an RPC endpoint you trust, and have the separate signing process
human/multisig-review the unsigned XDR before approval.

## Reporting a vulnerability

If you believe you have found a security vulnerability in this repository —
including anything that could violate the trust boundary above (e.g. accidental
secret-key handling, signing capability, or a way to make the tool emit
malformed XDR that a signer would approve without noticing) — please report it
privately:

- Do **not** open a public issue.
- Email the maintainers (see the `authors` field in
  [`Cargo.toml`](Cargo.toml)) with a description of the issue and, if possible,
  a minimal reproduction.
- You should receive an acknowledgment within 72 hours. Please give us a
  reasonable window to fix and release before public disclosure.

## Supported versions

Security fixes land on the current `main` and are released as patch version
bumps. Older protocol-era branches are supported on a best-effort basis.