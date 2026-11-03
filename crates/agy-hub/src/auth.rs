use crate::error::HubError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use zeroize::Zeroizing;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserInfo {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub groups: Vec<String>,
    pub role: String,
}

#[async_trait::async_trait]
pub trait Authenticator: Send + Sync {
    async fn authenticate(&self, username: &str, password: &str) -> Result<UserInfo, HubError>;
}

/// In-memory authenticator for testing and decoupled development
#[derive(Clone)]
pub struct MockAuthenticator {
    users: Arc<RwLock<HashMap<String, (String, UserInfo)>>>,
    allowed_group: String,
}

impl MockAuthenticator {
    pub fn new(allowed_group: &str) -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            allowed_group: allowed_group.to_string(),
        }
    }

    pub fn add_user(&self, username: &str, password: &str, uid: u32, gid: u32, groups: Vec<String>) {
        let user = UserInfo {
            username: username.to_string(),
            uid,
            gid,
            groups,
            role: "developer".to_string(),
        };
        let mut map = self.users.write().unwrap();
        map.insert(username.to_string(), (password.to_string(), user));
    }
}

#[async_trait::async_trait]
impl Authenticator for MockAuthenticator {
    async fn authenticate(&self, username: &str, password: &str) -> Result<UserInfo, HubError> {
        let password_zero = Zeroizing::new(password.to_string());
        let map = self.users.read().unwrap();
        if let Some((stored_pw, user_info)) = map.get(username) {
            if stored_pw == password_zero.as_str() {
                if !user_info.groups.iter().any(|g| g == &self.allowed_group) {
                    return Err(HubError::Forbidden(format!(
                        "User '{}' is not a member of authorized group '{}'",
                        username, self.allowed_group
                    )));
                }
                return Ok(user_info.clone());
            }
        }
        Err(HubError::AuthFailed("Invalid username or password".to_string()))
    }
}

/// Linux PAM Authenticator
pub struct PamAuthenticator {
    service_name: String,
    allowed_group: String,
}

impl PamAuthenticator {
    pub fn new(service_name: &str, allowed_group: &str) -> Self {
        Self {
            service_name: service_name.to_string(),
            allowed_group: allowed_group.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Authenticator for PamAuthenticator {
    async fn authenticate(&self, username: &str, password: &str) -> Result<UserInfo, HubError> {
        let user_string = username.to_string();
        let pw_zero = Zeroizing::new(password.to_string());
        let service = self.service_name.clone();
        let allowed = self.allowed_group.clone();

        tokio::task::spawn_blocking(move || {
            #[cfg(target_os = "linux")]
            {
                let pam_result = authenticate_pam_linux(&service, &user_string, pw_zero.as_str());
                if let Err(e) = pam_result {
                    return Err(HubError::AuthFailed(e));
                }

                let user_info = resolve_linux_user(&user_string, &allowed)?;
                Ok(user_info)
            }

            #[cfg(not(target_os = "linux"))]
            {
                let _ = (user_string, pw_zero, allowed);
                Err(HubError::AuthFailed(format!(
                    "PAM authentication requires Linux host with /etc/pam.d/{}",
                    service
                )))
            }
        })
        .await
        .map_err(|e| HubError::AuthFailed(format!("Auth task panicked: {}", e)))?
    }
}

#[cfg(target_os = "linux")]
fn authenticate_pam_linux(service: &str, user: &str, password: &str) -> Result<(), String> {
    let mut client = pam::Client::with_password(service)
        .map_err(|e| format!("Failed to initialize PAM service '{}': {:?}", service, e))?;
    client.conversation_mut().set_credentials(user, password);
    client
        .authenticate()
        .map_err(|e| format!("PAM authentication failed for user '{}': {:?}", user, e))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn resolve_linux_user(username: &str, allowed_group: &str) -> Result<UserInfo, HubError> {
    let user = uzers::get_user_by_name(username)
        .ok_or_else(|| HubError::AuthFailed(format!("User '{}' not found in system NSS database", username)))?;

    let uid = user.uid();
    let gid = user.primary_group_id();

    let groups_list = user.groups();
    let mut groups: Vec<String> = Vec::new();

    // Resolve primary group name
    if let Some(primary_group) = uzers::get_group_by_gid(gid) {
        groups.push(primary_group.name().to_string_lossy().into_owned());
    }

    // Resolve all supplementary groups
    if let Some(supp_groups) = groups_list {
        for g in supp_groups {
            let gname = g.name().to_string_lossy().into_owned();
            if !groups.contains(&gname) {
                groups.push(gname);
            }
        }
    }

    // Verify authorized group membership
    let is_authorized = groups.iter().any(|g| g == allowed_group);
    if !is_authorized {
        return Err(HubError::Forbidden(format!(
            "User '{}' is not a member of authorized group '{}'",
            username, allowed_group
        )));
    }

    // Determine role based on administrative group membership
    let is_admin = groups.iter().any(|g| g == "wheel" || g == "sudo" || g == "admin");
    let role = if is_admin {
        "admin".to_string()
    } else {
        "developer".to_string()
    };

    Ok(UserInfo {
        username: username.to_string(),
        uid,
        gid,
        groups,
        role,
    })
}

