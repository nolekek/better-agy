use thiserror::Error;

#[derive(Error, Debug)]
pub enum VaultError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid magic bytes in vault header")]
    InvalidMagic,

    #[error("Corrupt vault data: {0}")]
    CorruptData(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Decryption failed (integrity check failed or wrong key)")]
    DecryptionFailed,

    #[error("Invalid key format: {0}")]
    InvalidKey(String),

    #[error("Slot not found: {0}")]
    SlotNotFound(String),
}

#[derive(Error, Debug)]
pub enum RefreshError {
    #[error("HTTP client error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("OAuth token refresh rejected: {status} - {body}")]
    RefreshRejected { status: u16, body: String },

    #[error("Invalid OAuth response: {0}")]
    InvalidResponse(String),

    #[error("Vault error during refresh: {0}")]
    Vault(#[from] VaultError),

    #[error("Slot has no refresh token: {0}")]
    NoRefreshToken(String),
}

#[derive(Error, Debug)]
pub enum FailoverError {
    #[error("Vault error: {0}")]
    Vault(#[from] VaultError),

    #[error("All account slots are exhausted or cooling down")]
    AllSlotsExhausted,

    #[error("No slots configured in vault")]
    NoSlotsConfigured,
}

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hyper error: {0}")]
    Hyper(#[from] hyper::Error),

    #[error("Failover error: {0}")]
    Failover(#[from] FailoverError),

    #[error("Vault error: {0}")]
    Vault(#[from] VaultError),

    #[error("Upstream error: {0}")]
    Upstream(String),
}
