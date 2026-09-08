//! Building unsigned `TransactionV1Envelope` XDR.
//!
//! The sentinel produces **unsigned** envelopes only: the fee, sequence number
//! and operations are filled in, but `signatures` is empty. A separately-held
//! key (e.g. the demo repo's testnet throwaway key) signs the envelope hash and
//! submits it. The sentinel itself never holds or handles a private key.

use stellar_xdr::{
    Limits, Memo, MuxedAccount, Operation, Preconditions, SequenceNumber, Transaction,
    TransactionEnvelope, TransactionExt, TransactionV1Envelope, VecM, WriteXdr,
};

use crate::error::BuildError;

/// Wrap operations in an unsigned transaction envelope.
///
/// # Parameters
///
/// - `source_account` — the public key of the account that will sign/submit.
/// - `seq_num` — the account's current sequence number **plus one** (i.e. the
///   sequence number the submitted transaction must carry). The caller fetches
///   the account entry and increments.
/// - `fee` — the transaction fee in stroops (base fee + estimated resource fee).
/// - `operations` — at most 100 operations.
pub fn build_unsigned_envelope(
    source_account: MuxedAccount,
    seq_num: i64,
    fee: u32,
    operations: Vec<Operation>,
) -> Result<TransactionEnvelope, BuildError> {
    if operations.len() > 100 {
        return Err(BuildError::TooManyOperations(operations.len()));
    }
    let tx = Transaction {
        source_account,
        fee,
        seq_num: SequenceNumber(seq_num),
        cond: Preconditions::None,
        memo: Memo::None,
        operations: VecM::try_from(operations).map_err(|_| {
            BuildError::TooManyOperations(usize::MAX) // unreachable; guarded above
        })?,
        ext: TransactionExt::V0,
    };
    Ok(TransactionEnvelope::Tx(TransactionV1Envelope {
        tx,
        signatures: VecM::default(),
    }))
}

/// Serialize an envelope to base64 XDR (the wire format for submission).
pub fn unsigned_envelope_xdr_base64(envelope: &TransactionEnvelope) -> Result<String, BuildError> {
    Ok(envelope.to_xdr_base64(Limits::none())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::ReadXdr;

    #[test]
    fn envelope_round_trips() {
        let source = MuxedAccount::Ed25519(stellar_xdr::Uint256([9u8; 32]));
        let op = crate::ops::extend_ttl_operation(1000);
        let envelope = build_unsigned_envelope(source, 1234, 100, vec![op]).expect("build");
        let b64 = unsigned_envelope_xdr_base64(&envelope).expect("encode");
        let back = TransactionEnvelope::from_xdr_base64(&b64, Limits::none()).expect("decode");
        assert_eq!(envelope, back);

        match back {
            TransactionEnvelope::Tx(env) => {
                assert_eq!(env.signatures.len(), 0, "must be unsigned");
                assert_eq!(env.tx.fee, 100);
                assert_eq!(env.tx.seq_num.0, 1234);
                assert_eq!(env.tx.operations.len(), 1);
            }
            other => panic!("unexpected envelope: {other:?}"),
        }
    }

    #[test]
    fn rejects_too_many_operations() {
        let source = MuxedAccount::Ed25519(stellar_xdr::Uint256([0u8; 32]));
        let ops = vec![crate::ops::restore_operation(); 101];
        assert!(build_unsigned_envelope(source, 1, 100, ops).is_err());
    }
}
