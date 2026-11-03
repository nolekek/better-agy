use crate::error::HubError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceState {
    pub name: String,
    pub active_state: String,
    pub sub_state: String,
    pub socket_path: String,
    pub socket_ready: bool,
    pub uptime_seconds: u64,
}

#[async_trait::async_trait]
pub trait SystemdManager: Send + Sync {
    async fn start_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError>;
    async fn stop_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError>;
    async fn restart_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError>;
    async fn get_unit_state(&self, uid: u32, unit_name: &str) -> Result<ServiceState, HubError>;
}

struct UnitRuntime {
    active_state: String,
    sub_state: String,
    started_at: Option<Instant>,
    socket_ready: bool,
}

/// Thread-safe in-memory systemd manager for testing and non-Linux hosts
#[derive(Clone)]
pub struct MockSystemdManager {
    units: Arc<RwLock<HashMap<(u32, String), UnitRuntime>>>,
    socket_template: String,
}

impl MockSystemdManager {
    pub fn new(socket_template: &str) -> Self {
        Self {
            units: Arc::new(RwLock::new(HashMap::new())),
            socket_template: socket_template.to_string(),
        }
    }

    pub fn set_socket_ready(&self, uid: u32, unit_name: &str, ready: bool) {
        let mut map = self.units.write().unwrap();
        let entry = map.entry((uid, unit_name.to_string())).or_insert(UnitRuntime {
            active_state: "active".to_string(),
            sub_state: "running".to_string(),
            started_at: Some(Instant::now()),
            socket_ready: ready,
        });
        entry.socket_ready = ready;
    }
}

#[async_trait::async_trait]
impl SystemdManager for MockSystemdManager {
    async fn start_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError> {
        let mut map = self.units.write().unwrap();
        map.insert(
            (uid, unit_name.to_string()),
            UnitRuntime {
                active_state: "active".to_string(),
                sub_state: "running".to_string(),
                started_at: Some(Instant::now()),
                socket_ready: true,
            },
        );
        Ok(format!("/org/freedesktop/systemd1/job/{}", uid))
    }

    async fn stop_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError> {
        let mut map = self.units.write().unwrap();
        map.insert(
            (uid, unit_name.to_string()),
            UnitRuntime {
                active_state: "inactive".to_string(),
                sub_state: "dead".to_string(),
                started_at: None,
                socket_ready: false,
            },
        );
        Ok(format!("/org/freedesktop/systemd1/job/{}", uid))
    }

    async fn restart_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError> {
        self.stop_unit(uid, unit_name).await?;
        self.start_unit(uid, unit_name).await
    }

    async fn get_unit_state(&self, uid: u32, unit_name: &str) -> Result<ServiceState, HubError> {
        let socket_path = self.socket_template.replace("{uid}", &uid.to_string());
        let map = self.units.read().unwrap();
        if let Some(runtime) = map.get(&(uid, unit_name.to_string())) {
            let uptime = runtime
                .started_at
                .map(|t| t.elapsed().as_secs())
                .unwrap_or(0);

            Ok(ServiceState {
                name: unit_name.to_string(),
                active_state: runtime.active_state.clone(),
                sub_state: runtime.sub_state.clone(),
                socket_path,
                socket_ready: runtime.socket_ready,
                uptime_seconds: uptime,
            })
        } else {
            Ok(ServiceState {
                name: unit_name.to_string(),
                active_state: "inactive".to_string(),
                sub_state: "dead".to_string(),
                socket_path,
                socket_ready: false,
                uptime_seconds: 0,
            })
        }
    }
}

#[cfg(target_os = "linux")]
#[zbus::proxy(
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1",
    interface = "org.freedesktop.systemd1.Manager"
)]
trait SystemdManagerApi {
    fn start_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn stop_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn restart_unit(&self, name: &str, mode: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
    fn get_unit(&self, name: &str) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;
}

#[cfg(target_os = "linux")]
use SystemdManagerApiProxy as SystemdManagerProxy;

#[cfg(target_os = "linux")]
#[zbus::proxy(
    default_service = "org.freedesktop.systemd1",
    interface = "org.freedesktop.systemd1.Unit"
)]
trait SystemdUnit {
    #[zbus(property)]
    fn active_state(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn sub_state(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn active_enter_timestamp(&self) -> zbus::Result<u64>;
}

#[cfg(target_os = "linux")]
async fn get_user_systemd_connection(uid: u32) -> Result<zbus::Connection, HubError> {
    let private_path = format!("/run/user/{}/systemd/private", uid);
    let bus_path = format!("/run/user/{}/bus", uid);

    // 1. Try private socket via P2P connection first
    if std::path::Path::new(&private_path).exists() {
        let addr = format!("unix:path={}", private_path);
        if let Ok(builder) = zbus::connection::Builder::address(addr.as_str()) {
            if let Ok(conn) = builder.p2p().build().await {
                return Ok(conn);
            }
        }
    }

    // 2. Try user session bus
    if std::path::Path::new(&bus_path).exists() {
        let addr = format!("unix:path={}", bus_path);
        if let Ok(builder) = zbus::connection::Builder::address(addr.as_str()) {
            if let Ok(conn) = builder.build().await {
                return Ok(conn);
            }
        }
    }

    // 3. Fallback direct connection
    let addr = format!("unix:path={}", private_path);
    let conn = zbus::connection::Builder::address(addr.as_str())
        .map_err(|e| HubError::DbusError(format!("Invalid D-Bus address '{}': {}", addr, e)))?
        .p2p()
        .build()
        .await
        .map_err(|e| {
            HubError::DbusError(format!(
                "Failed to connect to user systemd D-Bus for UID {} at '{}': {}",
                uid, private_path, e
            ))
        })?;

    Ok(conn)
}

/// Linux systemd D-Bus manager via private user socket
pub struct LinuxSystemdManager {
    socket_template: String,
}

impl LinuxSystemdManager {
    pub fn new(socket_template: &str) -> Self {
        Self {
            socket_template: socket_template.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl SystemdManager for LinuxSystemdManager {
    async fn start_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError> {
        #[cfg(target_os = "linux")]
        {
            let conn = get_user_systemd_connection(uid).await?;
            let proxy = SystemdManagerProxy::new(&conn)
                .await
                .map_err(|e| HubError::DbusError(format!("Failed to init systemd manager proxy: {}", e)))?;
            let job = proxy
                .start_unit(unit_name, "replace")
                .await
                .map_err(|e| HubError::DbusError(format!("StartUnit failed for {}: {}", unit_name, e)))?;
            Ok(job.as_str().to_string())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (uid, unit_name);
            Err(HubError::DbusError(
                "LinuxSystemdManager requires Linux systemd user manager over D-Bus".to_string(),
            ))
        }
    }

    async fn stop_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError> {
        #[cfg(target_os = "linux")]
        {
            let conn = get_user_systemd_connection(uid).await?;
            let proxy = SystemdManagerProxy::new(&conn)
                .await
                .map_err(|e| HubError::DbusError(format!("Failed to init systemd manager proxy: {}", e)))?;
            let job = proxy
                .stop_unit(unit_name, "replace")
                .await
                .map_err(|e| HubError::DbusError(format!("StopUnit failed for {}: {}", unit_name, e)))?;
            Ok(job.as_str().to_string())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (uid, unit_name);
            Err(HubError::DbusError(
                "LinuxSystemdManager requires Linux systemd user manager over D-Bus".to_string(),
            ))
        }
    }

    async fn restart_unit(&self, uid: u32, unit_name: &str) -> Result<String, HubError> {
        #[cfg(target_os = "linux")]
        {
            let conn = get_user_systemd_connection(uid).await?;
            let proxy = SystemdManagerProxy::new(&conn)
                .await
                .map_err(|e| HubError::DbusError(format!("Failed to init systemd manager proxy: {}", e)))?;
            let job = proxy
                .restart_unit(unit_name, "replace")
                .await
                .map_err(|e| HubError::DbusError(format!("RestartUnit failed for {}: {}", unit_name, e)))?;
            Ok(job.as_str().to_string())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (uid, unit_name);
            Err(HubError::DbusError(
                "LinuxSystemdManager requires Linux systemd user manager over D-Bus".to_string(),
            ))
        }
    }

    async fn get_unit_state(&self, uid: u32, unit_name: &str) -> Result<ServiceState, HubError> {
        let socket_path = self.socket_template.replace("{uid}", &uid.to_string());
        let socket_exists = std::path::Path::new(&socket_path).exists();

        #[cfg(target_os = "linux")]
        {
            let conn = match get_user_systemd_connection(uid).await {
                Ok(c) => c,
                Err(_) => {
                    return Ok(ServiceState {
                        name: unit_name.to_string(),
                        active_state: "inactive".to_string(),
                        sub_state: "dead".to_string(),
                        socket_path,
                        socket_ready: socket_exists,
                        uptime_seconds: 0,
                    });
                }
            };

            let manager = SystemdManagerProxy::new(&conn)
                .await
                .map_err(|e| HubError::DbusError(format!("Failed to init systemd manager proxy: {}", e)))?;

            let unit_path = match manager.get_unit(unit_name).await {
                Ok(p) => p,
                Err(_) => {
                    return Ok(ServiceState {
                        name: unit_name.to_string(),
                        active_state: "inactive".to_string(),
                        sub_state: "dead".to_string(),
                        socket_path,
                        socket_ready: socket_exists,
                        uptime_seconds: 0,
                    });
                }
            };

            let unit_proxy = SystemdUnitProxy::builder(&conn)
                .path(unit_path)
                .map_err(|e| HubError::DbusError(format!("Invalid unit object path: {}", e)))?
                .build()
                .await
                .map_err(|e| HubError::DbusError(format!("Failed to build unit proxy: {}", e)))?;

            let active_state = unit_proxy.active_state().await.unwrap_or_else(|_| "unknown".to_string());
            let sub_state = unit_proxy.sub_state().await.unwrap_or_else(|_| "unknown".to_string());
            let enter_timestamp_us = unit_proxy.active_enter_timestamp().await.unwrap_or(0);

            let uptime_seconds = if enter_timestamp_us > 0 {
                let now_us = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_micros() as u64)
                    .unwrap_or(0);
                if now_us > enter_timestamp_us {
                    (now_us - enter_timestamp_us) / 1_000_000
                } else {
                    0
                }
            } else {
                0
            };

            Ok(ServiceState {
                name: unit_name.to_string(),
                active_state,
                sub_state,
                socket_path,
                socket_ready: socket_exists,
                uptime_seconds,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = unit_name;
            Ok(ServiceState {
                name: unit_name.to_string(),
                active_state: "inactive".to_string(),
                sub_state: "dead".to_string(),
                socket_path,
                socket_ready: socket_exists,
                uptime_seconds: 0,
            })
        }
    }
}

