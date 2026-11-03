use agy_mock::MockServer;
use agy_proxy::{AccountSlot, SlotState, TokenRefresher, Vault, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET};
use tempfile::tempdir;

#[tokio::test]
async fn test_f6_refresh_slot_token_against_mock() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);
    vault.load_or_init().await.unwrap();

    let slot = AccountSlot::new(
        "slot_dev",
        "dev@example.com",
        "ya29.old_token",
        "1//refresh_token_dev",
        chrono::Utc::now().timestamp() + 100, // expiring soon
        SlotState::Active,
    );
    vault.upsert_slot(slot.clone()).await.unwrap();

    let refresher = TokenRefresher::new(vault.clone()).with_endpoint(
        mock.oauth_url(),
        DEFAULT_CLIENT_ID,
        DEFAULT_CLIENT_SECRET,
    );

    let updated = refresher
        .refresh_slot_token(&slot)
        .await
        .expect("Refresh slot token succeeds");

    assert!(updated.access_token.starts_with("ya29.synthetic_refreshed_access_token_mock_"));
    assert!(updated.expires_at > chrono::Utc::now().timestamp() + 3000);

    // Verify vault persisted the refreshed token
    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.access_token, updated.access_token);
    assert_eq!(controller.oauth_refresh_count().await, 1);

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f6_proactive_refresh_threshold_triggered() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    // Expiring in 200s (less than default 300s threshold)
    let slot = AccountSlot::new(
        "slot_expiring",
        "expiring@example.com",
        "ya29.old",
        "1//ref",
        chrono::Utc::now().timestamp() + 200,
        SlotState::Active,
    );
    vault.upsert_slot(slot).await.unwrap();

    let refresher = TokenRefresher::new(vault.clone()).with_endpoint(
        mock.oauth_url(),
        DEFAULT_CLIENT_ID,
        DEFAULT_CLIENT_SECRET,
    );

    let results = refresher.refresh_due_slots().await;
    assert_eq!(results.len(), 1, "Should have triggered refresh proactively for expiring slot");
    assert!(results[0].is_ok());
    assert_eq!(controller.oauth_refresh_count().await, 1);

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f6_proactive_refresh_threshold_skipped_when_fresh() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    // Expiring in 3500s (greater than 300s threshold)
    let slot = AccountSlot::new(
        "slot_fresh",
        "fresh@example.com",
        "ya29.fresh_token",
        "1//ref",
        chrono::Utc::now().timestamp() + 3500,
        SlotState::Active,
    );
    vault.upsert_slot(slot).await.unwrap();

    let refresher = TokenRefresher::new(vault.clone()).with_endpoint(
        mock.oauth_url(),
        DEFAULT_CLIENT_ID,
        DEFAULT_CLIENT_SECRET,
    );

    let results = refresher.refresh_due_slots().await;
    assert!(results.is_empty(), "Should not trigger refresh for fresh token");
    assert_eq!(controller.oauth_refresh_count().await, 0);

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f6_refresh_invalid_grant_disables_slot() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();
    controller.set_oauth_invalid_grant().await;

    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let slot = AccountSlot::new(
        "slot_revoked",
        "revoked@example.com",
        "ya29.bad",
        "1//revoked_ref",
        chrono::Utc::now().timestamp() + 100,
        SlotState::Active,
    );
    vault.upsert_slot(slot.clone()).await.unwrap();

    let refresher = TokenRefresher::new(vault.clone()).with_endpoint(
        mock.oauth_url(),
        DEFAULT_CLIENT_ID,
        DEFAULT_CLIENT_SECRET,
    );

    let res = refresher.refresh_slot_token(&slot).await;
    assert!(res.is_err(), "Should return error on invalid_grant");

    // Slot state should be disabled/revoked in vault
    let reloaded_slots = vault.list_slots().await;
    let reloaded_slot = reloaded_slots.iter().find(|s| s.id == "slot_revoked").unwrap();
    match &reloaded_slot.state {
        SlotState::Disabled { reason } => {
            assert!(reason.contains("400"));
        }
        other => panic!("Expected Disabled slot state, got: {:?}", other),
    }

    mock.shutdown().await;
}
