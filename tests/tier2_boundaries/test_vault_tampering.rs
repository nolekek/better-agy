use agy_proxy::vault::{MIN_CIPHERTEXT_LEN, VAULT_MAGIC};
use agy_proxy::{AccountSlot, SlotState, Vault, VaultError};
use std::fs;
use tempfile::tempdir;

#[tokio::test]
async fn test_f5_vault_ciphertext_bit_flip_fails_tag_validation() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    let slot = AccountSlot::new(
        "slot_1",
        "dev1@example.com",
        "ya29.secret_token",
        "1//ref",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    vault.upsert_slot(slot).await.unwrap();

    // Read on-disk ciphertext
    let mut ciphertext = fs::read(&vault_path).unwrap();
    assert!(ciphertext.len() > MIN_CIPHERTEXT_LEN);

    // Flip 1 bit in the ciphertext payload (after magic + nonce)
    let flip_index = VAULT_MAGIC.len() + 12 + 5;
    ciphertext[flip_index] ^= 0x01;

    // Overwrite file with tampered ciphertext
    fs::write(&vault_path, &ciphertext).unwrap();

    // Reopen vault with same key - must fail tag validation!
    let tampered_vault = Vault::new(&vault_path, key);
    let res = tampered_vault.load_or_init().await;

    assert!(
        res.is_err(),
        "Vault must reject ciphertext with flipped bit via AEAD authentication tag mismatch"
    );
}

#[tokio::test]
async fn test_f5_vault_truncated_file_fails() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    // Write truncated file (fewer bytes than minimum header + nonce + tag)
    fs::write(&vault_path, b"AGY_VAULT_V1_SHORT").unwrap();

    let vault = Vault::new(&vault_path, key);
    let res = vault.load_or_init().await;

    assert!(res.is_err(), "Truncated vault file must be rejected");
}

#[tokio::test]
async fn test_f5_vault_corrupted_magic_header_fails() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    // Read ciphertext and corrupt the magic header
    let mut data = fs::read(&vault_path).unwrap();
    data[0] = b'X';
    data[1] = b'X';
    data[2] = b'X';
    fs::write(&vault_path, &data).unwrap();

    let reopened = Vault::new(&vault_path, key);
    let res = reopened.load_or_init().await;

    assert!(res.is_err(), "Invalid magic header must be rejected");
    match res.unwrap_err() {
        VaultError::InvalidMagic => {}
        other => panic!("Expected InvalidMagic error, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_f5_vault_wrong_key_fails_decryption() {
    let key1 = Vault::generate_key();
    let key2 = Vault::generate_key();
    assert_ne!(key1, key2);

    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");

    let vault1 = Vault::new(&vault_path, key1);
    vault1.load_or_init().await.unwrap();

    let slot = AccountSlot::new(
        "slot_1",
        "dev1@example.com",
        "ya29.secret",
        "1//ref",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    vault1.upsert_slot(slot).await.unwrap();

    // Open with wrong key key2
    let vault2 = Vault::new(&vault_path, key2);
    let res = vault2.load_or_init().await;

    assert!(res.is_err(), "Decryption with wrong key must fail");
}

#[tokio::test]
async fn test_f5_vault_nonexistent_path_inits_empty_safely() {
    let key = Vault::generate_key();
    let tmp = tempdir().unwrap();
    let non_existent = tmp.path().join("subdir").join("nested").join("vault.enc");

    assert!(!non_existent.exists());

    let vault = Vault::new(&non_existent, key);
    let data = vault.load_or_init().await.expect("Init empty vault");

    assert_eq!(data.slots.len(), 0);
    assert_eq!(data.version, 1);
    assert!(non_existent.exists());
}
