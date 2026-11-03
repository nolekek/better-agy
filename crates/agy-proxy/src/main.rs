use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use agy_proxy::failover::FailoverEngine;
use agy_proxy::proxy::{ProxyConfig, ProxyServer};
use agy_proxy::refresh::TokenRefresher;
use agy_proxy::vault::Vault;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("Starting agy-proxy daemon...");

    let port: u16 = std::env::var("AGY_PROXY_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(51128);

    let vault_path = std::env::var("AGY_VAULT_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let default_sys = PathBuf::from("/etc/antigravity/vault.enc");
            if default_sys.parent().map(|p| p.exists()).unwrap_or(false) {
                default_sys
            } else {
                PathBuf::from("vault.enc")
            }
        });

    let key_path = std::env::var("AGY_VAULT_KEY_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let default_sys = PathBuf::from("/etc/antigravity/vault.key");
            if default_sys.parent().map(|p| p.exists()).unwrap_or(false) {
                default_sys
            } else {
                PathBuf::from("vault.key")
            }
        });

    let key = match Vault::load_key_from_env_or_file(Some(&key_path)) {
        Ok(k) => k,
        Err(e) => {
            error!("Failed to load or generate vault key: {}", e);
            return Err(e.into());
        }
    };

    let vault = Vault::new(&vault_path, key);
    if let Err(e) = vault.load_or_init().await {
        error!("Failed to initialize vault: {}", e);
        return Err(e.into());
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()));
    let refresher = Arc::new(TokenRefresher::new(vault.clone()));

    let (shutdown_tx, shutdown_rx_worker) = broadcast::channel(1);
    let shutdown_rx_server = shutdown_tx.subscribe();

    // Start background proactive token refresh worker (polling every 30s)
    let worker_handle = refresher.start_worker(Duration::from_secs(30), shutdown_rx_worker);

    // Bind proxy listener
    let bind_addr: SocketAddr = format!("127.0.0.1:{}", port).parse()?;
    let listener = TcpListener::bind(bind_addr).await?;
    info!("agy-proxy loopback listening on http://{}", bind_addr);

    let proxy_config = ProxyConfig {
        bind_addr,
        ..Default::default()
    };

    let server = ProxyServer::new(proxy_config, failover);
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run(listener, shutdown_rx_server).await {
            error!("Proxy server error: {}", e);
        }
    });

    // Wait for termination signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal (Ctrl+C)");
        }
    }

    let _ = shutdown_tx.send(());
    let _ = tokio::join!(worker_handle, server_handle);

    info!("agy-proxy daemon shut down gracefully.");
    Ok(())
}
