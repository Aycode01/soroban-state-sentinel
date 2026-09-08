//! Dev helper: prints the base64-XDR `LedgerKey::ConfigSetting` keys the client
//! fetches. Used to capture RPC fixtures against a live network:
//!
//! ```text
//! curl -X POST https://soroban-testnet.stellar.org \
//!   -H 'Content-Type: application/json' \
//!   -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getLedgerEntries\",\"params\":{\"keys\":[<keys from this program>]}}"
//! ```

use stellar_xdr::{Limits, WriteXdr};

fn main() {
    for key in sentinel_rpc_client::NetworkConfig::config_setting_keys() {
        println!("{}", key.to_xdr_base64(Limits::none()).expect("encode"));
    }
    for key in sentinel_rpc_client::FeeRates::config_setting_keys() {
        println!("{}", key.to_xdr_base64(Limits::none()).expect("encode"));
    }
}