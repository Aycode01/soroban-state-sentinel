//! Errors produced while building XDR.

use stellar_xdr::Error as XdrError;

/// Errors produced by the XDR builder.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// An invalid strkey (`C…`, `G…`) was supplied.
    #[error("invalid strkey: {0}")]
    InvalidStrkey(String),

    /// A key of an unsupported ledger-entry type was passed to an operation
    /// builder. Only contract-data and contract-code entries can have their TTL
    /// extended or be restored.
    #[error("unsupported ledger key for TTL operation: {0:?}")]
    UnsupportedKey(stellar_xdr::LedgerKey),

    /// The list of operations exceeds the transaction limit
    /// (`VecM<Operation, 100>`).
    #[error("too many operations for one transaction: {0} > 100")]
    TooManyOperations(usize),

    /// The batch limits are unusable (e.g. a zero cap).
    #[error("invalid batch limits: {0}")]
    InvalidBatchLimits(String),

    /// An XDR encode/decode failure.
    #[error("XDR error: {0}")]
    Xdr(#[from] XdrError),

    /// The `extend_to` target is outside the allowed range.
    #[error("extend_to {extend_to} must be in 1..={max} (max_entry_ttl - 1)")]
    ExtendToOutOfRange {
        /// The requested extension, in ledgers.
        extend_to: u32,
        /// The network maximum (`max_entry_ttl - 1`).
        max: u32,
    },
}
