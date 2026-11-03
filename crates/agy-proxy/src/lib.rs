pub mod error;
pub mod failover;
pub mod proxy;
pub mod refresh;
pub mod vault;

pub use error::{FailoverError, ProxyError, RefreshError, VaultError};
pub use failover::{FailoverDecision, FailoverEngine, QuotaDetector};
pub use proxy::{ProxyConfig, ProxyServer, ProxyService, DEFAULT_PROXY_ADDR};
pub use refresh::{TokenRefresher, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET, DEFAULT_TOKEN_URL};
pub use vault::{AccountSlot, EncryptedVaultPayload, SlotState, Vault, VaultData};
