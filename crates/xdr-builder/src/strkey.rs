//! Decoding of `C…` contract ids and `G…` account public keys.
//!
//! Uses `stellar-strkey` (SEP-23). Only public identifiers are handled — this
//! crate deliberately has no `S…` secret-key handling anywhere.

use stellar_xdr::{AccountId, Hash, MuxedAccount, PublicKey, Uint256};

use crate::error::BuildError;

/// Decode a `C…` contract id into its 32-byte hash.
///
/// # Units
///
/// The returned `Hash` is the raw contract id used inside
/// `LedgerKey::ContractData { contract: ScAddress::Contract(hash), .. }`.
pub fn decode_contract_id(s: &str) -> Result<Hash, BuildError> {
    let contract: stellar_strkey::Contract = s
        .parse()
        .map_err(|e| BuildError::InvalidStrkey(format!("'{s}': {e}")))?;
    Ok(Hash(contract.0))
}

/// Decode a `G…` account public key into an XDR `AccountId`.
pub fn decode_account_id(s: &str) -> Result<AccountId, BuildError> {
    let pk: stellar_strkey::ed25519::PublicKey = s
        .parse()
        .map_err(|e| BuildError::InvalidStrkey(format!("'{s}': {e}")))?;
    Ok(AccountId(PublicKey::PublicKeyTypeEd25519(Uint256(pk.0))))
}

/// Decode a `G…` account public key into the `MuxedAccount` form used as a
/// transaction source account.
pub fn decode_muxed_account(s: &str) -> Result<MuxedAccount, BuildError> {
    let pk: stellar_strkey::ed25519::PublicKey = s
        .parse()
        .map_err(|e| BuildError::InvalidStrkey(format!("'{s}': {e}")))?;
    Ok(MuxedAccount::Ed25519(Uint256(pk.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known testnet contract id from the Soroban RPC docs example.
    const TESTNET_CONTRACT: &str = "CCPYZFKEAXHHS5VVW5J45TOU7S2EODJ7TZNJIA5LKDVL3PESCES6FNCI";
    // A valid testnet account strkey (checksum-verified; generated throwaway).
    const TESTNET_ACCOUNT: &str = "GDM2XGXMVSK7YVEQLDUC3FPIC6VTNWN57JRP4627DJNULHC5DD5HZPC5";

    #[test]
    fn decodes_known_contract_id() {
        let hash = decode_contract_id(TESTNET_CONTRACT).expect("valid contract id");
        assert_eq!(hash.0.len(), 32);
    }

    #[test]
    fn rejects_garbage_contract_id() {
        assert!(decode_contract_id("not-a-strkey").is_err());
        // An S… secret key must never be accepted where a contract id is expected.
        assert!(
            decode_contract_id("SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA").is_err()
        );
    }

    #[test]
    fn decodes_known_account_id() {
        let account = decode_account_id(TESTNET_ACCOUNT).expect("valid account id");
        assert!(matches!(
            account,
            AccountId(PublicKey::PublicKeyTypeEd25519(_))
        ));
        let muxed = decode_muxed_account(TESTNET_ACCOUNT).expect("valid muxed account");
        assert!(matches!(muxed, MuxedAccount::Ed25519(_)));
    }
}
