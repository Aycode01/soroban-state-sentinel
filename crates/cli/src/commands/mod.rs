//! Command implementations.

pub mod restore;
pub mod scan;

/// Outcome of a command run; drives the process exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Success; exit 0.
    Ok,
    /// `--fail-on-critical` triggered; exit 1.
    FailOnCritical,
}

/// CLI-level error: wraps every error type the commands produce.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0}")]
    Msg(String),

    #[error("RPC error: {0}")]
    Rpc(#[from] sentinel_rpc_client::RpcError),

    #[error("scan error: {0}")]
    Scan(#[from] sentinel_ttl_scanner::ScannerError),

    #[error("XDR build error: {0}")]
    Build(#[from] sentinel_xdr_builder::BuildError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("XDR error: {0}")]
    Xdr(#[from] stellar_xdr::Error),
}

impl CliError {
    /// Convenience constructor for plain-message errors.
    pub fn msg(msg: impl Into<String>) -> Self {
        CliError::Msg(msg.into())
    }
}
