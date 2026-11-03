use agy_proxy::vault::{AccountSlot, SlotState, Vault};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn resolve_vault_key(key_path: Option<&Path>) -> Result<[u8; 32], Box<dyn std::error::Error>> {
    if let Ok(key) = Vault::load_key_from_env_or_file(key_path) {
        return Ok(key);
    }
    // If not found and a key_path is provided, generate and save it
    if let Some(path) = key_path {
        let key = Vault::generate_key();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = File::create(path)?;
        file.write_all(&key)?;
        println!("[+] Generated and saved new vault key to: {}", path.display());
        return Ok(key);
    }
    // Default fallback dev key
    let mut key = [0u8; 32];
    key[..18].copy_from_slice(b"agy-vault-dev-key!");
    Ok(key)
}

pub async fn run_vault_init(
    vault_path: &Path,
    key_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = resolve_vault_key(key_path)?;
    let vault = Vault::new(vault_path, key);
    vault.load_or_init().await?;
    println!("[+] Initialized encrypted vault at: {}", vault_path.display());
    Ok(())
}

pub async fn run_vault_add(
    vault_path: &Path,
    key_path: Option<&Path>,
    email: &str,
    refresh_token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = resolve_vault_key(key_path)?;
    let vault = Vault::new(vault_path, key);
    vault.load_or_init().await?;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let slot_id = format!("slot_{:x}", nanos);
    let now = chrono::Utc::now().timestamp();
    let slot = AccountSlot::new(
        slot_id.clone(),
        email,
        "initial_dummy_access_token",
        refresh_token,
        now + 3600,
        SlotState::Ready,
    );

    vault.upsert_slot(slot).await?;
    println!("[+] Added account slot '{}' ({}) to vault.", slot_id, email);
    Ok(())
}

pub async fn run_vault_list(
    vault_path: &Path,
    key_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let key = resolve_vault_key(key_path)?;
    let vault = Vault::new(vault_path, key);
    vault.load_or_init().await?;

    let slots = vault.list_slots().await;
    let active_slot = vault.get_active_slot().await;

    println!("========================================================");
    println!(" Antigravity Account Vault");
    println!(" Location: {}", vault_path.display());
    println!(" Total Slots: {}", slots.len());
    println!("========================================================");
    for slot in slots {
        let is_active = active_slot.as_ref().map(|s| s.id == slot.id).unwrap_or(false);
        let active_mark = if is_active { " [ACTIVE]" } else { "" };
        println!(
            "  * Slot: {:<12} | Email: {:<25} | State: {:?}{}",
            slot.id, slot.email, slot.state, active_mark
        );
    }
    println!("========================================================");
    Ok(())
}
