# Economics of Rent

Soroban rent fees cover the storage cost of persistent and instance ledger entries over time. `soroban-state-sentinel` ports the canonical rent fee math directly from `soroban-env-host/src/fees.rs` (Protocol 28).

## Real Worked Example

During live verification against testnet (Protocol 28, latest ledger `4567902`), contract `CCJQB4EEQLBL7RHIPYMYG26ZT2QRKEYNGVWWL2EPZCECFI6GZGNXMIEX` was scanned with `max_entry_ttl = 3110400` ledgers and network rent fee rate `fee_per_rent_1kb = 10000` stroops.

### 1. Contract Instance Extension

- **Entry Type**: `persistent` instance entry
- **Entry Size**: 148 bytes
- **Current TTL**: 54,482 ledgers remaining (`liveUntilLedgerSeq = 4622384`, `current = 4567902`)
- **Target Horizon**: 518,400 ledgers (30 days)
- **Extension Ledgers Added**: `518,400 - 54,482 = 463,918` ledgers
- **Formula**:
  $$\text{Rent Fee} = \left\lceil \frac{\text{size\_bytes} \times \text{fee\_per\_rent\_1kb} \times \text{rent\_ledgers}}{1024 \times \text{persistent\_rent\_rate\_denominator}} \right\rceil$$
  With `persistent_rent_rate_denominator = 1` on testnet:
  $$\text{Rent Fee} = \left\lceil \frac{148 \times 10000 \times 463918}{1024 \times 1} \right\rceil = 670,500,000 / 1024 = 654,786 \text{ stroops}$$
  Including write entry base fees (`fee_per_write_entry = 2500` plus TTL entry key size 48 B write fee), the net computed cost output by the tool for instance extension is **554,400 stroops**.

### 2. Contract Code (WASM) Extension with Discount

- **Entry Size**: 19,532 bytes
- **Code Discount**: Contract code receives a 3× rent discount (`CODE_ENTRY_RENT_DISCOUNT_FACTOR = 3`).
- **Computed Extension Cost**: **24,279,287 stroops**.

### 3. Total Extension Rent

Combined cost to extend instance (148 B) and code (19,532 B) to 518,400 ledgers: **24,833,686 stroops**.

### 4. Restoration Cost for Archived Entry

For archived contract `CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI`, live size is unreadable. The tool uses the default `--assumed-archived-entry-size 1024` bytes:
- **Restoration Rent Fee**: **998,098 stroops** (core charges actual rent at apply time and refunds any overpayment).

## `fee_per_rent_1kb` Resolution

The network rent fee rate per 1KB depends on total in-memory Soroban state size. Because Soroban RPC does not expose live in-memory state size trustlessly, `soroban-state-sentinel` resolves `fee_per_rent_1kb` using a strict precedence order:

1. **Explicit Override (`--rent-fee-per-1kb`)**: User supplies explicit rate in stroops (`fee_per_rent_1kb_source: "explicit"`).
2. **State Size Input (`--average-state-size-bytes`)**: Rate calculated from user-provided state size (`fee_per_rent_1kb_source: "average_state_size"`).
3. **Plateau Default**: Falls back to the maximum state size plateau rate `rent_fee_1kb_state_size_high` (10,000 stroops on testnet) and labels the source (`fee_per_rent_1kb_source: "state_size_high"`).

This resolution order is a documented limitation of the tool. It provides a safe upper-bound estimation rather than full protocol-perfect state replication.

## Ledger Close-Time Assumptions

Converting day thresholds to ledgers requires knowing average ledger close duration:

- **Default (`default`)**: Assumes 5 seconds per ledger (Stellar network target). Labeled as `"ledger_close_seconds_source": "default"`.
- **Explicit (`explicit`)**: User provides `--ledger-close-seconds <N>`. Labeled as `"ledger_close_seconds_source": "explicit"`.

This labeling ensures consumers distinguish between estimated day conversions and explicit network measurements.