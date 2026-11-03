use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::VaultError;

pub const VAULT_MAGIC: &[u8; 12] = b"AGY_VAULT_V1";
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
pub const MIN_CIPHERTEXT_LEN: usize = VAULT_MAGIC.len() + NONCE_LEN + TAG_LEN;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotState {
    Ready,
    Active,
    Cooldown { until: i64 },
    Revoked,
    Disabled { reason: String },
}

impl SlotState {
    pub fn is_available(&self, now: i64) -> bool {
        match self {
            SlotState::Ready | SlotState::Active => true,
            SlotState::Cooldown { until } => now >= *until,
            SlotState::Revoked | SlotState::Disabled { .. } => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountSlot {
    pub id: String,
    pub email: String,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,      // Unix timestamp (seconds)
    pub state: SlotState,
}

impl AccountSlot {
    pub fn new(
        id: impl Into<String>,
        email: impl Into<String>,
        access_token: impl Into<String>,
        refresh_token: impl Into<String>,
        expires_at: i64,
        state: SlotState,
    ) -> Self {
        Self {
            id: id.into(),
            email: email.into(),
            access_token: access_token.into(),
            refresh_token: refresh_token.into(),
            expires_at,
            state,
        }
    }

    pub fn is_token_valid(&self, now: i64, margin_secs: i64) -> bool {
        self.expires_at > now + margin_secs
    }

    pub fn is_ready_for_use(&self, now: i64) -> bool {
        self.state.is_available(now) && self.is_token_valid(now, 60)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultData {
    pub version: u32,
    pub updated_at: i64,
    pub active_slot_id: Option<String>,
    pub auto_failover: bool,
    pub debounce_seconds: u64,
    pub slots: Vec<AccountSlot>,
}

impl Default for VaultData {
    fn default() -> Self {
        Self {
            version: 1,
            updated_at: chrono::Utc::now().timestamp(),
            active_slot_id: None,
            auto_failover: true,
            debounce_seconds: 25,
            slots: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedVaultPayload {
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

#[derive(Clone)]
pub struct Vault {
    vault_path: PathBuf,
    key: [u8; 32],
    cached: Arc<RwLock<VaultData>>,
}

impl Vault {
    pub fn generate_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        key
    }

    pub fn load_key_from_env_or_file(key_path: Option<&Path>) -> Result<[u8; 32], VaultError> {
        if let Ok(env_key) = std::env::var("AGY_VAULT_KEY") {
            let trimmed = env_key.trim();
            if trimmed.len() == 64 {
                if let Ok(bytes) = hex_decode(trimmed) {
                    if bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        return Ok(arr);
                    }
                }
            }
            if let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, trimmed) {
                if bytes.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    return Ok(arr);
                }
            }
            if trimmed.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(trimmed.as_bytes());
                return Ok(arr);
            }
            return Err(VaultError::InvalidKey(
                "AGY_VAULT_KEY must be 32 raw bytes, 64 hex chars, or 44 base64 chars".to_string(),
            ));
        }

        if let Some(path) = key_path {
            if path.exists() {
                let mut file = File::open(path)?;
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                if buf.len() == 32 {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&buf);
                    return Ok(arr);
                }
                let str_content = String::from_utf8_lossy(&buf).trim().to_string();
                if str_content.len() == 64 {
                    if let Ok(bytes) = hex_decode(&str_content) {
                        if bytes.len() == 32 {
                            let mut arr = [0u8; 32];
                            arr.copy_from_slice(&bytes);
                            return Ok(arr);
                        }
                    }
                }
                if let Ok(bytes) = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &str_content) {
                    if bytes.len() == 32 {
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&bytes);
                        return Ok(arr);
                    }
                }
                return Err(VaultError::InvalidKey(format!(
                    "Key file {} has invalid size or format (expected 32 bytes)",
                    path.display()
                )));
            } else {
                let key = Self::generate_key();
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)?;
                file.write_all(&key)?;
                file.sync_all()?;
                return Ok(key);
            }
        }

        Err(VaultError::InvalidKey(
            "No vault key provided via AGY_VAULT_KEY or key_path".to_string(),
        ))
    }

    pub fn new(vault_path: impl Into<PathBuf>, key: [u8; 32]) -> Self {
        let vault_path = vault_path.into();
        Self {
            vault_path,
            key,
            cached: Arc::new(RwLock::new(VaultData::default())),
        }
    }

    pub async fn load_or_init(&self) -> Result<VaultData, VaultError> {
        let mut cache = self.cached.write().await;
        if self.vault_path.exists() {
            let data = self.read_from_disk()?;
            *cache = data.clone();
            Ok(data)
        } else {
            let default_data = VaultData::default();
            self.write_to_disk(&default_data)?;
            *cache = default_data.clone();
            Ok(default_data)
        }
    }

    pub fn encrypt_bytes(&self, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;

        let mut output = Vec::with_capacity(VAULT_MAGIC.len() + NONCE_LEN + ciphertext.len());
        output.extend_from_slice(VAULT_MAGIC);
        output.extend_from_slice(&nonce_bytes);
        output.extend_from_slice(&ciphertext);
        Ok(output)
    }

    pub fn decrypt_bytes(&self, raw: &[u8]) -> Result<Vec<u8>, VaultError> {
        if raw.len() < MIN_CIPHERTEXT_LEN {
            return Err(VaultError::CorruptData(format!(
                "Payload length {} is less than minimum {}",
                raw.len(),
                MIN_CIPHERTEXT_LEN
            )));
        }

        if &raw[..VAULT_MAGIC.len()] != VAULT_MAGIC {
            return Err(VaultError::InvalidMagic);
        }

        let nonce_start = VAULT_MAGIC.len();
        let nonce_end = nonce_start + NONCE_LEN;
        let nonce = Nonce::from_slice(&raw[nonce_start..nonce_end]);

        let ciphertext = &raw[nonce_end..];
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|e| VaultError::EncryptionError(e.to_string()))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| VaultError::DecryptionFailed)?;

        Ok(plaintext)
    }

    pub fn read_from_disk(&self) -> Result<VaultData, VaultError> {
        let mut file = File::open(&self.vault_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let plaintext = self.decrypt_bytes(&buffer)?;
        let data: VaultData = serde_json::from_slice(&plaintext)?;
        Ok(data)
    }

    pub fn write_to_disk(&self, data: &VaultData) -> Result<(), VaultError> {
        let json_bytes = serde_json::to_vec_pretty(data)?;
        let encrypted = self.encrypt_bytes(&json_bytes)?;

        let parent = self.vault_path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;

        let file_stem = self
            .vault_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("vault");

        // Unique temporary file in the same parent directory for atomic same-volume rename
        let rand_val: u64 = rand::random();
        let tmp_path = parent.join(format!(
            ".{}_{}_{:016x}.tmp",
            file_stem,
            std::process::id(),
            rand_val
        ));

        {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);

            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }

            let mut file = options.open(&tmp_path)?;
            file.write_all(&encrypted)?;
            file.sync_all()?;
        }

        // Atomic rename with micro-retry backoff for Windows transient handle releases
        let mut rename_result = fs::rename(&tmp_path, &self.vault_path);
        if rename_result.is_err() {
            for _ in 0..5 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                rename_result = fs::rename(&tmp_path, &self.vault_path);
                if rename_result.is_ok() {
                    break;
                }
            }
        }

        if let Err(e) = rename_result {
            let _ = fs::remove_file(&tmp_path);
            return Err(e.into());
        }

        Ok(())
    }

    pub async fn get_data(&self) -> VaultData {
        self.cached.read().await.clone()
    }

    pub async fn save_data(&self, data: VaultData) -> Result<(), VaultError> {
        let mut cache = self.cached.write().await;
        self.write_to_disk(&data)?;
        *cache = data;
        Ok(())
    }

    pub async fn list_slots(&self) -> Vec<AccountSlot> {
        self.cached.read().await.slots.clone()
    }

    pub async fn get_slot(&self, id: &str) -> Option<AccountSlot> {
        self.cached.read().await.slots.iter().find(|s| s.id == id).cloned()
    }

    pub async fn get_active_slot(&self) -> Option<AccountSlot> {
        let cache = self.cached.read().await;
        if let Some(ref active_id) = cache.active_slot_id {
            cache.slots.iter().find(|s| &s.id == active_id).cloned()
        } else {
            cache.slots.first().cloned()
        }
    }

    pub async fn set_active_slot_id(&self, slot_id: &str) -> Result<(), VaultError> {
        let mut cache = self.cached.write().await;
        if !cache.slots.iter().any(|s| s.id == slot_id) {
            return Err(VaultError::SlotNotFound(slot_id.to_string()));
        }
        let mut data = cache.clone();
        data.active_slot_id = Some(slot_id.to_string());
        data.updated_at = chrono::Utc::now().timestamp();
        self.write_to_disk(&data)?;
        *cache = data;
        Ok(())
    }

    pub async fn upsert_slot(&self, slot: AccountSlot) -> Result<(), VaultError> {
        let mut cache = self.cached.write().await;
        let mut data = cache.clone();
        if let Some(idx) = data.slots.iter().position(|s| s.id == slot.id) {
            data.slots[idx] = slot;
        } else {
            data.slots.push(slot);
        }
        if data.active_slot_id.is_none() && !data.slots.is_empty() {
            data.active_slot_id = Some(data.slots[0].id.clone());
        }
        data.updated_at = chrono::Utc::now().timestamp();
        self.write_to_disk(&data)?;
        *cache = data;
        Ok(())
    }

    pub async fn update_slot_tokens(
        &self,
        slot_id: &str,
        access_token: String,
        expires_at: i64,
    ) -> Result<(), VaultError> {
        let mut cache = self.cached.write().await;
        let mut data = cache.clone();
        if let Some(slot) = data.slots.iter_mut().find(|s| s.id == slot_id) {
            slot.access_token = access_token;
            slot.expires_at = expires_at;
            data.updated_at = chrono::Utc::now().timestamp();
            self.write_to_disk(&data)?;
            *cache = data;
            Ok(())
        } else {
            Err(VaultError::SlotNotFound(slot_id.to_string()))
        }
    }

    pub async fn update_slot_state(&self, slot_id: &str, state: SlotState) -> Result<(), VaultError> {
        let mut cache = self.cached.write().await;
        let mut data = cache.clone();
        if let Some(slot) = data.slots.iter_mut().find(|s| s.id == slot_id) {
            slot.state = state;
            data.updated_at = chrono::Utc::now().timestamp();
            self.write_to_disk(&data)?;
            *cache = data;
            Ok(())
        } else {
            Err(VaultError::SlotNotFound(slot_id.to_string()))
        }
    }
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if !s.len().is_multiple_of(2) {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}
