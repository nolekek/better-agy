use agy_proxy::vault::VAULT_MAGIC;
use agy_proxy::{AccountSlot, SlotState, Vault};
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_f5_vault_generate_key_and_init() {
    let key = Vault::generate_key();
    assert_eq!(key.len(), 32);

    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault = Vault::new(&vault_path, key);
    let data = vault.load_or_init().await.expect("Vault init succeeds");

    assert_eq!(data.version, 1);
    assert!(vault_path.exists());
}

#[tokio::test]
async fn test_f5_vault_add_and_retrieve_slot() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    let slot = AccountSlot::new(
        "slot_1",
        "dev1@example.com",
        "ya29.access_token_1",
        "1//04_refresh_token_1",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );

    vault.upsert_slot(slot.clone()).await.unwrap();

    let active = vault.get_active_slot().await.expect("Active slot present");
    assert_eq!(active.id, "slot_1");
    assert_eq!(active.email, "dev1@example.com");
    assert_eq!(active.access_token, "ya29.access_token_1");
}

#[tokio::test]
async fn test_f5_vault_encryption_on_disk() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    let secret_token = "ya29.super_secret_corporate_token_xyz987";
    let slot = AccountSlot::new(
        "slot_secret",
        "corp@example.com",
        secret_token,
        "1//refresh",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );

    vault.upsert_slot(slot).await.unwrap();

    // Read raw file on disk
    let raw_bytes = fs::read(&vault_path).unwrap();

    // Verify magic header
    assert!(raw_bytes.starts_with(VAULT_MAGIC));

    // Verify secret token is NOT in plaintext on disk
    let raw_str = String::from_utf8_lossy(&raw_bytes);
    assert!(
        !raw_str.contains(secret_token),
        "Ciphertext must not leak plaintext tokens"
    );
}

#[tokio::test]
async fn test_f5_vault_reload_persisted_state() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    // Process 1 writes to vault
    {
        let vault1 = Vault::new(&vault_path, key);
        vault1.load_or_init().await.unwrap();

        let slot1 = AccountSlot::new(
            "slot_1",
            "dev1@example.com",
            "token_1",
            "ref_1",
            chrono::Utc::now().timestamp() + 3600,
            SlotState::Active,
        );
        let slot2 = AccountSlot::new(
            "slot_2",
            "dev2@example.com",
            "token_2",
            "ref_2",
            chrono::Utc::now().timestamp() + 3600,
            SlotState::Ready,
        );

        vault1.upsert_slot(slot1).await.unwrap();
        vault1.upsert_slot(slot2).await.unwrap();
    }

    // Process 2 re-opens the vault with same key
    {
        let vault2 = Vault::new(&vault_path, key);
        let data = vault2.load_or_init().await.unwrap();

        assert_eq!(data.slots.len(), 2);
        assert_eq!(data.slots[0].id, "slot_1");
        assert_eq!(data.slots[1].id, "slot_2");
        assert_eq!(data.active_slot_id, Some("slot_1".to_string()));
    }
}

#[tokio::test]
async fn test_f5_vault_slot_update() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    let mut slot = AccountSlot::new(
        "slot_1",
        "dev1@example.com",
        "token_old",
        "ref_1",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    vault.upsert_slot(slot.clone()).await.unwrap();

    // Update access token
    slot.access_token = "token_refreshed_new".to_string();
    vault.upsert_slot(slot).await.unwrap();

    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.access_token, "token_refreshed_new");
}
