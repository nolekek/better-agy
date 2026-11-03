//! Central Authentication, Systemd Lifecycle & Dynamic Reverse Proxy Hub for Antigravity Server

pub mod auth;
pub mod config;
pub mod dbus;
pub mod error;
pub mod proxy;
pub mod router;
pub mod session;
pub mod web;

pub use auth::{Authenticator, MockAuthenticator, PamAuthenticator, UserInfo};
pub use config::HubConfig;
pub use dbus::{LinuxSystemdManager, MockSystemdManager, ServiceState, SystemdManager};
pub use error::HubError;
pub use proxy::ReverseProxyEngine;
pub use router::{create_router, AppState};
pub use session::{Claims, SessionManager};
