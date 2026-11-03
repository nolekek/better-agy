use agy_hub::{
    create_router, AppState, Authenticator, HubConfig, LinuxSystemdManager, MockAuthenticator,
    MockSystemdManager, PamAuthenticator, ReverseProxyEngine, SessionManager, SystemdManager,
};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let args: Vec<String> = env::args().collect();
    let mut config = HubConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bind" if i + 1 < args.len() => {
                config.bind_addr = args[i + 1].clone();
                i += 2;
            }
            "--config" if i + 1 < args.len() => {
                let path = &args[i + 1];
                if let Ok(content) = std::fs::read_to_string(path) {
                    if let Ok(loaded) = serde_json::from_str::<HubConfig>(&content) {
                        config = loaded;
                    }
                }
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let config = Arc::new(config);

    // Production Linux environment uses genuine PamAuthenticator and LinuxSystemdManager.
    // Non-Linux development runs require explicit AGY_DEV_MODE=1 environment variable.
    let (auth, systemd): (Arc<dyn Authenticator>, Arc<dyn SystemdManager>) = if cfg!(target_os = "linux") {
        (
            Arc::new(PamAuthenticator::new("antigravity-hub", &config.allowed_group)),
            Arc::new(LinuxSystemdManager::new(&config.socket_dir_template)),
        )
    } else {
        let dev_mode = env::var("AGY_DEV_MODE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if !dev_mode {
            eprintln!(
                "ERROR: agy-hub production requires Linux (PAM authentication and systemd user services).\n\
                 To run in local non-Linux development/testing mode, explicitly set AGY_DEV_MODE=1.\n\
                 Note: No hardcoded credentials are baked into the binary. Configure users via AGY_DEV_USERS env (user:password:uid:gid,...)."
            );
            std::process::exit(1);
        }

        info!("⚠️ AGY_DEV_MODE=1 active: Running with development MockAuthenticator and MockSystemdManager (no hardcoded credentials).");
        let mock = MockAuthenticator::new(&config.allowed_group);
        if let Ok(dev_users) = env::var("AGY_DEV_USERS") {
            for entry in dev_users.split(',') {
                let parts: Vec<&str> = entry.split(':').collect();
                if parts.len() >= 2 {
                    let u = parts[0];
                    let p = parts[1];
                    let uid: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(1001);
                    let gid: u32 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(1001);
                    mock.add_user(u, p, uid, gid, vec![config.allowed_group.clone()]);
                    info!("Loaded dev user: {} (UID: {})", u, uid);
                }
            }
        }

        (
            Arc::new(mock),
            Arc::new(MockSystemdManager::new(&config.socket_dir_template)),
        )
    };

    let session = Arc::new(SessionManager::new(
        &config.jwt_secret,
        config.session_ttl_secs,
        config.secure_cookies,
    ));

    let proxy = Arc::new(ReverseProxyEngine::new(config.clone(), systemd.clone()));


    let state = AppState {
        config: config.clone(),
        auth,
        session,
        systemd,
        proxy,
        active_presets: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    info!("🚀 agy-hub portal listening on http://{}", config.bind_addr);

    axum::serve(listener, app).await?;
    Ok(())
}
