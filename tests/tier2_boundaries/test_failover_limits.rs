use agy_proxy::{AccountSlot, FailoverDecision, FailoverEngine, FailoverError, SlotState, Vault};
use tempfile::tempdir;

#[tokio::test]
async fn test_f8_single_account_pool_returns_no_available_slot() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let only_slot = AccountSlot::new(
        "slot_solo",
        "solo@example.com",
        "ya29.solo",
        "1//ref",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    vault.upsert_slot(only_slot).await.unwrap();

    let engine = FailoverEngine::new(vault.clone());

    // Single account gets 429 quota error -> cannot rotate, returns NoAvailableSlot
    let res = engine.on_quota_error().await;

    assert!(res.is_err());
    match res.unwrap_err() {
        FailoverError::AllSlotsExhausted => {}
        other => panic!("Expected AllSlotsExhausted, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_f8_all_slots_in_cooldown_returns_error() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now = chrono::Utc::now().timestamp();
    let slot1 = AccountSlot::new(
        "slot_1",
        "dev1@example.com",
        "token_1",
        "ref_1",
        now + 3600,
        SlotState::Active,
    );
    let slot2 = AccountSlot::new(
        "slot_2",
        "dev2@example.com",
        "token_2",
        "ref_2",
        now + 3600,
        SlotState::Cooldown { until: now + 300 }, // in cooldown
    );
    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    let engine = FailoverEngine::new(vault.clone());

    // slot1 fails with 429 -> slot2 is in cooldown -> NoAvailableSlot
    let res = engine.on_quota_error().await;
    assert!(matches!(res.unwrap_err(), FailoverError::AllSlotsExhausted));
}

#[tokio::test]
async fn test_f8_skip_disabled_and_revoked_slots() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now = chrono::Utc::now().timestamp();
    let slot1 = AccountSlot::new("slot_1", "dev1@example.com", "t1", "r1", now + 3600, SlotState::Active);
    let slot2_revoked = AccountSlot::new("slot_2", "dev2@example.com", "t2", "r2", now + 3600, SlotState::Revoked);
    let slot3_disabled = AccountSlot::new(
        "slot_3",
        "dev3@example.com",
        "t3",
        "r3",
        now + 3600,
        SlotState::Disabled { reason: "OAuth error".to_string() },
    );
    let slot4_ready = AccountSlot::new("slot_4", "dev4@example.com", "t4", "r4", now + 3600, SlotState::Ready);

    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2_revoked).await.unwrap();
    vault.upsert_slot(slot3_disabled).await.unwrap();
    vault.upsert_slot(slot4_ready).await.unwrap();

    let engine = FailoverEngine::new(vault.clone());

    // Failover from slot_1 should skip slot_2 (Revoked) and slot_3 (Disabled) and land on slot_4!
    let decision = engine.on_quota_error().await.unwrap();
    match decision {
        FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "slot_1");
            assert_eq!(new_slot.id, "slot_4");
            assert_eq!(new_slot.access_token, "t4");
        }
        _ => panic!("Expected Rotated decision"),
    }
}

#[tokio::test]
async fn test_f8_cooldown_expiration_makes_slot_ready_again() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now = chrono::Utc::now().timestamp();
    let slot1 = AccountSlot::new("slot_1", "dev1@example.com", "t1", "r1", now + 3600, SlotState::Active);
    // Cooldown ended 5 seconds ago
    let slot2_expired_cooldown = AccountSlot::new(
        "slot_2",
        "dev2@example.com",
        "t2",
        "r2",
        now + 3600,
        SlotState::Cooldown { until: now - 5 },
    );

    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2_expired_cooldown).await.unwrap();

    let engine = FailoverEngine::new(vault.clone());

    // slot2 cooldown expired, so it must be selected
    let decision = engine.on_quota_error().await.unwrap();
    match decision {
        FailoverDecision::Rotated { new_slot, .. } => {
            assert_eq!(new_slot.id, "slot_2");
        }
        _ => panic!("Expected slot_2 to be selected after cooldown expiration"),
    }
}

#[tokio::test]
async fn test_f8_carousel_wrap_around_selection() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now = chrono::Utc::now().timestamp();
    // 3 slots. Slot 3 is Active, Slot 1 is Ready, Slot 2 is Cooldown
    let slot1 = AccountSlot::new("slot_1", "dev1@example.com", "t1", "r1", now + 3600, SlotState::Ready);
    let slot2 = AccountSlot::new("slot_2", "dev2@example.com", "t2", "r2", now + 3600, SlotState::Cooldown { until: now + 300 });
    let slot3 = AccountSlot::new("slot_3", "dev3@example.com", "t3", "r3", now + 3600, SlotState::Active);

    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();
    vault.upsert_slot(slot3).await.unwrap();
    vault.set_active_slot_id("slot_3").await.unwrap();

    let engine = FailoverEngine::new(vault.clone());

    // slot 3 fails with 429 -> carousel wraps around: skips slot 2 (cooldown) -> selects slot 1!
    let decision = engine.on_quota_error().await.unwrap();
    match decision {
        FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "slot_3");
            assert_eq!(new_slot.id, "slot_1");
        }
        _ => panic!("Expected carousel wrap-around to slot_1"),
    }
}
