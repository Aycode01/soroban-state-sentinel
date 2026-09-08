# For maintainers

You deployed a Soroban contract and you want to know, in plain terms, whether it
is about to disappear from the network — and what to do if it is. This guide is
the "what do I actually do" walkthrough. For exact flags, exit codes, and output
fields, see the [CLI reference](../cli-reference/scan.md) and
[JSON schema reference](../json-schema-reference.md).

## 1. Install

The repo pins a stable Rust toolchain via `rust-toolchain.toml` (workspace
`rust-version` 1.84, edition 2021). Build the binary:

```bash
cargo build --release
```

The binary is `target/release/soroban-state-sentinel`. Nothing is installed
system-wide; either copy the binary onto your `PATH` or call it by its full
path.

## 2. Scan your contract

Point the tool at your contract id:

```bash
soroban-state-sentinel scan <contract-id>
```

By default this checks the contract **instance** and its **code** (wasm). It
reports, per entry: how many ledgers (and whole days) remain before archival,
which health band the entry falls in, and what it would cost to extend or
restore it.

It uses the public testnet RPC by default. If your contract lives elsewhere,
pass `--rpc-url <endpoint>`. If you have persistent data keys you care about,
pass each one with `--keys <base64-SCVal>` — the tool cannot discover a
contract's full key set, so the keys you care about must be named explicitly.

## 3. Read the health band

Four bands, and only two of them need your attention:

| Band | Meaning | What to do |
| --- | --- | --- |
| `healthy` | More than 30 days left | Nothing. |
| `expiring_soon` | Between 7 and 30 days left | Schedule an extension. |
| `critical` | 7 days or fewer left | Extend now. |
| `archived` | No longer readable in the live state | Restore before you can read it. |

The 30-day and 7-day thresholds are defaults; you can change them with
`--healthy-days` and `--critical-days` if your risk tolerance differs.

## 4. If you see `expiring_soon` or `critical`: extend

Generate the unsigned XDR that pushes the entries' TTL out:

```bash
soroban-state-sentinel extend <contract-id> --extend-to 518400 --output extend.xdr
```

`518400` is 30 days at 5 seconds per ledger — enough to bring a `critical`
entry back into the `healthy` band. (The tool labels the 5-second close-time
assumption as `default` in its output; pass `--ledger-close-seconds` if you
have a measured value for your network.) The command writes one base64-XDR
`ExtendFootprintTtl` operation per line to `extend.xdr`.

## 5. Get it signed and submit it

This is the part the tool deliberately does **not** do. `soroban-state-sentinel`
never holds a private key and never signs or submits anything.

Take `extend.xdr` to whoever holds your key — you, your multisig, your
institution's signing service. For a complete unsigned transaction envelope
(sequence number pre-filled), add `--source-account <G…>` when generating and
hand the resulting file to your signing flow. Once signed, submit it to the
network through your normal tooling.

## 6. If you see `archived`: restore

If an entry is already archived, extending does nothing for it. Generate a
restore instead:

```bash
soroban-state-sentinel restore <contract-id> --output restore.xdr
```

Sign and submit the same way. After the restore is applied, run `scan` again:
the entry should come back as `healthy` (restored persistent entries get their
TTL reset to the network's minimum, `min_persistent_ttl` — 120,960 ledgers on
the protocol-28 testnet this tool was verified against).

## A real example

During this project's verification pass, a live testnet contract was found in
`critical` state with ~3 days left (54,482 ledgers). Extending it by 518,400
ledgers cost an estimated 554,400 stroops for the instance and 24,279,287
stroops for its 19,532-byte code — see
[Economics of rent](../economics-of-rent.md) for the hand-reproducible
arithmetic. That is the failure mode this tool exists to catch, and the
remediation is the five steps above.