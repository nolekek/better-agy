use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData, VAULT_MAGIC};
use agy_proxy::error::VaultError;
use tempfile::tempdir;

#[tokio::test]
async fn test_aes_256_gcm_roundtrip() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();

    let vault = Vault::new(&vault_path, key);
    let original_data = VaultData {
        version: 1,
        updated_at: 1730000000,
        active_slot_id: Some("slot_1".to_string()),
        auto_failover: true,
        debounce_seconds: 25,
        slots: vec![
            AccountSlot::new(
                "slot_1",
                "dev1@company.com",
                "ya29.access_token_1",
                "1//refresh_token_1",
                1730003600,
                SlotState::Active,
            ),
            AccountSlot::new(
                "slot_2",
                "dev2@company.com",
                "ya29.access_token_2",
                "1//refresh_token_2",
                1730003600,
                SlotState::Ready,
            ),
        ],
    };

    vault.save_data(original_data.clone()).await.unwrap();

    // Verify file exists on disk and is encrypted (does not contain plaintext)
    let raw_bytes = std::fs::read(&vault_path).unwrap();
    assert!(raw_bytes.starts_with(VAULT_MAGIC));
    let raw_str = String::from_utf8_lossy(&raw_bytes);
    assert!(!raw_str.contains("ya29.access_token_1"));
    assert!(!raw_str.contains("dev1@company.com"));

    // Read back and verify exact match
    let loaded_vault = Vault::new(&vault_path, key);
    let loaded_data = loaded_vault.load_or_init().await.unwrap();
    assert_eq!(loaded_data, original_data);
}

#[tokio::test]
async fn test_tampering_rejection() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();

    let vault = Vault::new(&vault_path, key);
    let mut data = VaultData::default();
    data.slots.push(AccountSlot::new(
        "slot_1",
        "dev@company.com",
        "secret_token",
        "refresh_token",
        1800000000,
        SlotState::Active,
    ));
    vault.save_data(data).await.unwrap();

    let valid_bytes = std::fs::read(&vault_path).unwrap();

    // 1. Truncated payload (< 40 bytes)
    let truncated = &valid_bytes[..20];
    let res = vault.decrypt_bytes(truncated);
    assert!(matches!(res, Err(VaultError::CorruptData(_))));

    // 2. Corrupt Magic bytes
    let mut bad_magic = valid_bytes.clone();
    bad_magic[0] = b'X';
    let res = vault.decrypt_bytes(&bad_magic);
    assert!(matches!(res, Err(VaultError::InvalidMagic)));

    // 3. Corrupted Nonce (bytes 12..24)
    let mut bad_nonce = valid_bytes.clone();
    bad_nonce[15] ^= 0xFF;
    let res = vault.decrypt_bytes(&bad_nonce);
    assert!(matches!(res, Err(VaultError::DecryptionFailed)));

    // 4. Corrupted Ciphertext payload
    let mut bad_ciphertext = valid_bytes.clone();
    let mid = (bad_ciphertext.len() - 16) / 2 + 12;
    bad_ciphertext[mid] ^= 0xFF;
    let res = vault.decrypt_bytes(&bad_ciphertext);
    assert!(matches!(res, Err(VaultError::DecryptionFailed)));

    // 5. Corrupted GCM authentication tag (last 16 bytes)
    let mut bad_tag = valid_bytes.clone();
    let tag_idx = bad_tag.len() - 1;
    bad_tag[tag_idx] ^= 0xFF;
    let res = vault.decrypt_bytes(&bad_tag);
    assert!(matches!(res, Err(VaultError::DecryptionFailed)));

    // 6. Wrong decryption key
    let wrong_key = Vault::generate_key();
    let wrong_vault = Vault::new(&vault_path, wrong_key);
    let res = wrong_vault.decrypt_bytes(&valid_bytes);
    assert!(matches!(res, Err(VaultError::DecryptionFailed)));
}

#[tokio::test]
async fn test_vault_crud_and_slot_management() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    // Add slot
    let slot1 = AccountSlot::new(
        "s1",
        "user1@test.com",
        "token1",
        "refresh1",
        1700000000,
        SlotState::Ready,
    );
    vault.upsert_slot(slot1.clone()).await.unwrap();

    let slots = vault.list_slots().await;
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].id, "s1");

    // Update tokens
    vault
        .update_slot_tokens("s1", "new_token_1".to_string(), 1700003600)
        .await
        .unwrap();

    let updated = vault.get_slot("s1").await.unwrap();
    assert_eq!(updated.access_token, "new_token_1");
    assert_eq!(updated.expires_at, 1700003600);

    // Update state to Cooldown
    vault
        .update_slot_state("s1", SlotState::Cooldown { until: 1700004000 })
        .await
        .unwrap();
    let in_cooldown = vault.get_slot("s1").await.unwrap();
    assert_eq!(
        in_cooldown.state,
        SlotState::Cooldown { until: 1700004000 }
    );
}

#[test]
fn test_key_generation_and_loading() {
    let dir = tempdir().unwrap();
    let key_file = dir.path().join("vault.key");

    // Key file does not exist initially; should generate and persist
    let key1 = Vault::load_key_from_env_or_file(Some(&key_file)).unwrap();
    assert!(key_file.exists());
    assert_eq!(std::fs::read(&key_file).unwrap().len(), 32);

    // Loading from existing file should return the exact same key
    let key2 = Vault::load_key_from_env_or_file(Some(&key_file)).unwrap();
    assert_eq!(key1, key2);
}
