use std::time::Duration;

use agy_proxy::failover::{FailoverDecision, FailoverEngine};
use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData};
use tempfile::tempdir;

#[tokio::test]
async fn test_strict_25s_debounce_boundary() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let now_sec = chrono::Utc::now().timestamp();
    let initial_data = VaultData {
        slots: vec![
            AccountSlot::new("s1", "s1@corp.com", "token1", "rt1", now_sec + 3600, SlotState::Active),
            AccountSlot::new("s2", "s2@corp.com", "token2", "rt2", now_sec + 3600, SlotState::Ready),
            AccountSlot::new("s3", "s3@corp.com", "token3", "rt3", now_sec + 3600, SlotState::Ready),
        ],
        active_slot_id: Some("s1".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    let failover = FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000));

    // Baseline: Simulate an initial failover event at T0
    let now_ms = chrono::Utc::now().timestamp_millis();
    let t0 = now_ms - 25_000; // T0 is 25s in the past initially
    failover.set_last_failover_epoch_ms(t0);

    // Initial genuine 429 triggers failover from s1 to s2
    let decision1 = failover.on_quota_error().await.unwrap();
    match decision1 {
        FailoverDecision::Rotated { ref new_slot, ref old_slot_id } => {
            assert_eq!(old_slot_id, "s1");
            assert_eq!(new_slot.id, "s2");
            assert_eq!(new_slot.state, SlotState::Active);
        }
        _ => panic!("Expected Rotated decision on first failover"),
    }

    // Capture the exact timestamp of this failover
    let failover_time = failover.last_failover_epoch_ms();
    assert!(failover_time > 0);

    // Verify s1 is now in cooldown and s2 is active
    let s1 = vault.get_slot("s1").await.unwrap();
    assert!(matches!(s1.state, SlotState::Cooldown { .. }));
    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.id, "s2");

    // TEST BOUNDARY 1: At 24.9s (24,900 ms) elapsed
    // Override last_failover to simulate exactly 24,900ms elapsed
    let current_simulated_now = chrono::Utc::now().timestamp_millis();
    failover.set_last_failover_epoch_ms(current_simulated_now - 24_900);

    let decision_suppressed = failover.on_quota_error().await.unwrap();
    match decision_suppressed {
        FailoverDecision::Debounced { active_slot, elapsed_millis } => {
            assert_eq!(active_slot.id, "s2", "Active slot must stay s2 during debounce");
            assert!(
                (24_800..=25_000).contains(&elapsed_millis),
                "Elapsed millis {} must be around 24900ms (< 25000ms)",
                elapsed_millis
            );
        }
        FailoverDecision::Rotated { .. } => {
            panic!("Debounce violation! Failover triggered prematurely at 24.9s");
        }
    }

    // Verify carousel was NOT rotated
    let current_active = vault.get_active_slot().await.unwrap();
    assert_eq!(current_active.id, "s2");
    let s3 = vault.get_slot("s3").await.unwrap();
    assert_eq!(s3.state, SlotState::Ready, "s3 must still be Ready");

    // TEST BOUNDARY 2: At 25.1s (25,100 ms) elapsed
    // Override last_failover to simulate exactly 25,100ms elapsed
    let current_simulated_now = chrono::Utc::now().timestamp_millis();
    failover.set_last_failover_epoch_ms(current_simulated_now - 25_100);

    let decision_triggered = failover.on_quota_error().await.unwrap();
    match decision_triggered {
        FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "s2");
            assert_eq!(new_slot.id, "s3");
            assert_eq!(new_slot.state, SlotState::Active);
        }
        FailoverDecision::Debounced { elapsed_millis, .. } => {
            panic!(
                "Debounce failed to expire at 25.1s! elapsed: {}ms",
                elapsed_millis
            );
        }
    }

    // Verify s2 is now in cooldown and s3 is active
    let s2 = vault.get_slot("s2").await.unwrap();
    assert!(matches!(s2.state, SlotState::Cooldown { .. }));
    let active_final = vault.get_active_slot().await.unwrap();
    assert_eq!(active_final.id, "s3");
}
