use agy_proxy::{AccountSlot, FailoverDecision, FailoverEngine, SlotState, Vault};
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_f9_debounce_suppresses_at_24_999_seconds() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

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

    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    let engine = FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000));

    // Simulate initial failover at T = now - 24,999 ms
    let now_ms = chrono::Utc::now().timestamp_millis();
    engine.set_last_failover_epoch_ms(now_ms - 24_999);

    let decision = engine.on_quota_error().await.unwrap();

    match decision {
        FailoverDecision::Debounced {
            active_slot,
            elapsed_millis,
        } => {
            assert_eq!(active_slot.id, "slot_1");
            assert!(
                elapsed_millis < 25_000,
                "Elapsed millis {} should be < 25000ms",
                elapsed_millis
            );
        }
        FailoverDecision::Rotated { .. } => {
            panic!("Failover must be SUPPRESSED at 24,999 ms!");
        }
    }
}

#[tokio::test]
async fn test_f9_debounce_triggers_at_25_001_seconds() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

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

    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    let engine = FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000));

    // Simulate initial failover at T = now - 25_001 ms
    let now_ms = chrono::Utc::now().timestamp_millis();
    engine.set_last_failover_epoch_ms(now_ms - 25_001);

    let decision = engine.on_quota_error().await.unwrap();

    match decision {
        FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "slot_1");
            assert_eq!(new_slot.id, "slot_2");
        }
        FailoverDecision::Debounced { .. } => {
            panic!("Failover must be TRIGGERED at 25,001 ms!");
        }
    }
}

#[tokio::test]
async fn test_f9_burst_of_10_rapid_429s_triggers_exactly_one_failover() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    // Create 5 accounts
    for i in 1..=5 {
        let slot = AccountSlot::new(
            format!("slot_{i}"),
            format!("dev{i}@example.com"),
            format!("token_{i}"),
            format!("ref_{i}"),
            chrono::Utc::now().timestamp() + 3600,
            if i == 1 { SlotState::Active } else { SlotState::Ready },
        );
        vault.upsert_slot(slot).await.unwrap();
    }

    let engine = FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000));

    let mut rotated_count = 0;
    let mut debounced_count = 0;

    // Simulate burst of 10 quota errors in rapid succession (< 100ms)
    for _ in 0..10 {
        match engine.on_quota_error().await.unwrap() {
            FailoverDecision::Rotated { .. } => rotated_count += 1,
            FailoverDecision::Debounced { .. } => debounced_count += 1,
        }
    }

    assert_eq!(
        rotated_count, 1,
        "Burst of 10 requests must trigger exactly ONE rotation"
    );
    assert_eq!(
        debounced_count, 9,
        "Remaining 9 requests in burst must be debounced"
    );

    // Active slot should be slot_2
    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.id, "slot_2");
}

#[tokio::test]
async fn test_f9_custom_debounce_duration() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

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
    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    // Custom 10-second debounce window
    let engine = FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(10_000));

    let now_ms = chrono::Utc::now().timestamp_millis();
    engine.set_last_failover_epoch_ms(now_ms - 9_900);
    assert!(matches!(
        engine.on_quota_error().await.unwrap(),
        FailoverDecision::Debounced { .. }
    ));

    engine.set_last_failover_epoch_ms(now_ms - 10_100);
    assert!(matches!(
        engine.on_quota_error().await.unwrap(),
        FailoverDecision::Rotated { .. }
    ));
}

#[tokio::test]
async fn test_f9_trailing_error_during_debounce_preserves_active_slot() {
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

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
    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    let engine = FailoverEngine::new(vault.clone());

    // 1st error triggers rotation from slot_1 to slot_2
    let d1 = engine.on_quota_error().await.unwrap();
    assert!(matches!(d1, FailoverDecision::Rotated { .. }));

    // 2nd error arriving 5 seconds later is trailing from before the switch
    let d2 = engine.on_quota_error().await.unwrap();
    match d2 {
        FailoverDecision::Debounced { active_slot, .. } => {
            assert_eq!(active_slot.id, "slot_2", "Debounced slot remains slot_2");
        }
        FailoverDecision::Rotated { .. } => panic!("Trailing error must be debounced"),
    }
}
