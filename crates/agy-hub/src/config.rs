use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubConfig {
    pub bind_addr: String,
    pub jwt_secret: String,
    pub session_ttl_secs: u64,
    pub presets_file: PathBuf,
    pub library_dir: PathBuf,
    pub socket_dir_template: String,
    pub proxy_url: String,
    pub secure_cookies: bool,
    pub allowed_group: String,
    pub socket_poll_timeout_secs: u64,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:8080".to_string(),
            jwt_secret: "antigravity-default-insecure-dev-secret-change-me".to_string(),
            session_ttl_secs: 86400,
            presets_file: PathBuf::from("data/skills-library/presets.yaml"),
            library_dir: PathBuf::from("data/skills-library"),
            socket_dir_template: "/run/user/{uid}/antigravity.sock".to_string(),
            proxy_url: "http://127.0.0.1:51128".to_string(),
            secure_cookies: false, // In development, can work with http; in prod, set to true
            allowed_group: "agy-users".to_string(),
            socket_poll_timeout_secs: 10,
        }
    }
}

impl HubConfig {
    pub fn socket_path_for_uid(&self, uid: u32) -> PathBuf {
        let path_str = self.socket_dir_template.replace("{uid}", &uid.to_string());
        PathBuf::from(path_str)
    }
}
