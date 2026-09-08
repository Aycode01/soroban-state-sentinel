//! Deriving the `LedgerKey::Ttl` key for a data/code entry.
//!
//! Stellar Core derives the TTL key for a ledger key as
//! `LedgerKey(TTL, sha256(opaqueXDR(ledgerKey)))` (`getTTLKey` in
//! `src/transactions/TransactionUtils.cpp`). The sentinel needs the same
//! derivation to fetch TTL entries over RPC.

use sha2::{Digest, Sha256};
use stellar_xdr::{Hash, LedgerKey, LedgerKeyTtl, Limits, WriteXdr};

use crate::error::BuildError;

/// Compute the `LedgerKey::Ttl` that governs the given ledger key.
///
/// `key` must be a key of an evictable entry type (contract data or contract
/// code); the result is undefined for other types (the caller validates).
pub fn ttl_key_for(key: &LedgerKey) -> Result<LedgerKey, BuildError> {
    let xdr = key.to_xdr(Limits::none())?;
    let digest = Sha256::digest(&xdr);
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&digest);
    Ok(LedgerKey::Ttl(LedgerKeyTtl {
        key_hash: Hash(bytes),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::{
        ContractDataDurability, ContractId, LedgerKeyContractData, ScAddress, ScVal,
    };

    #[test]
    fn ttl_key_is_deterministic() {
        let key = LedgerKey::ContractData(LedgerKeyContractData {
            contract: ScAddress::Contract(ContractId(Hash([1u8; 32]))),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
        });
        let a = ttl_key_for(&key).expect("derive");
        let b = ttl_key_for(&key).expect("derive again");
        assert_eq!(a, b);
        assert!(matches!(a, LedgerKey::Ttl(_)));
    }

    #[test]
    fn ttl_key_differs_across_keys() {
        let key_a = LedgerKey::ContractData(LedgerKeyContractData {
            contract: ScAddress::Contract(ContractId(Hash([1u8; 32]))),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
        });
        let key_b = LedgerKey::ContractData(LedgerKeyContractData {
            contract: ScAddress::Contract(ContractId(Hash([2u8; 32]))),
            key: ScVal::LedgerKeyContractInstance,
            durability: ContractDataDurability::Persistent,
        });
        assert_ne!(
            ttl_key_for(&key_a).expect("derive"),
            ttl_key_for(&key_b).expect("derive")
        );
    }
}
