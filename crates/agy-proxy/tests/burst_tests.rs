use std::sync::Arc;
use std::time::Duration;

use agy_proxy::failover::{FailoverDecision, FailoverEngine};
use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData};
use tempfile::tempdir;

#[tokio::test]
async fn test_burst_10_rapid_429s_trigger_exactly_one_failover() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let now_sec = chrono::Utc::now().timestamp();
    let initial_data = VaultData {
        slots: vec![
            AccountSlot::new("slot_A", "a@test.com", "token_a", "rt_a", now_sec + 3600, SlotState::Active),
            AccountSlot::new("slot_B", "b@test.com", "token_b", "rt_b", now_sec + 3600, SlotState::Ready),
            AccountSlot::new("slot_C", "c@test.com", "token_c", "rt_c", now_sec + 3600, SlotState::Ready),
            AccountSlot::new("slot_D", "d@test.com", "token_d", "rt_d", now_sec + 3600, SlotState::Ready),
        ],
        active_slot_id: Some("slot_A".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    let failover = Arc::new(FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000)));

    // Send 10 rapid 429 error notifications concurrently in quick succession
    let mut handles = Vec::new();
    for i in 0..10 {
        let engine = failover.clone();
        handles.push(tokio::spawn(async move {
            // Slight sub-millisecond stagger to simulate high-concurrency burst
            tokio::time::sleep(Duration::from_millis(i * 2)).await;
            engine.on_quota_error().await
        }));
    }

    let mut rotated_count = 0;
    let mut debounced_count = 0;

    for handle in handles {
        let res = handle.await.unwrap().unwrap();
        match res {
            FailoverDecision::Rotated { new_slot, old_slot_id } => {
                rotated_count += 1;
                assert_eq!(old_slot_id, "slot_A");
                assert_eq!(new_slot.id, "slot_B");
            }
            FailoverDecision::Debounced { active_slot, .. } => {
                debounced_count += 1;
                assert_eq!(active_slot.id, "slot_B");
            }
        }
    }

    // VERIFICATION: Exactly 1 failover occurred, 9 were debounced
    assert_eq!(
        rotated_count, 1,
        "Burst of 10 rapid 429s must trigger exactly 1 failover"
    );
    assert_eq!(
        debounced_count, 9,
        "Subsequent 9 errors in burst must be suppressed by debounce"
    );

    // Verify slots state: slot_A in Cooldown, slot_B Active, slot_C and slot_D untouched
    let slot_a = vault.get_slot("slot_A").await.unwrap();
    assert!(matches!(slot_a.state, SlotState::Cooldown { .. }));

    let slot_b = vault.get_slot("slot_B").await.unwrap();
    assert_eq!(slot_b.state, SlotState::Active);

    let slot_c = vault.get_slot("slot_C").await.unwrap();
    assert_eq!(slot_c.state, SlotState::Ready, "slot_C must not be thrashed");

    let slot_d = vault.get_slot("slot_D").await.unwrap();
    assert_eq!(slot_d.state, SlotState::Ready, "slot_D must not be thrashed");
}
