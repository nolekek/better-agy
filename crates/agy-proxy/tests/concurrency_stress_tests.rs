use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

use agy_proxy::failover::{FailoverDecision, FailoverEngine};
use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData};
use tempfile::tempdir;

/// Test 1: Barrier-synchronized 50 concurrent tasks on multi-threaded runtime
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_high_concurrency_burst_initial_50() {
    for iter in 0..20 {
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

        let num_tasks = 50;
        let barrier = Arc::new(Barrier::new(num_tasks));
        let mut handles = Vec::with_capacity(num_tasks);

        for _ in 0..num_tasks {
            let engine = failover.clone();
            let b = barrier.clone();
            handles.push(tokio::spawn(async move {
                b.wait().await;
                engine.on_quota_error().await
            }));
        }

        let mut rotated_count = 0;
        let mut debounced_count = 0;
        let mut error_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for handle in handles {
            match handle.await.unwrap() {
                Ok(FailoverDecision::Rotated { .. }) => rotated_count += 1,
                Ok(FailoverDecision::Debounced { .. }) => debounced_count += 1,
                Err(e) => {
                    error_count += 1;
                    errors.push(format!("{:?}", e));
                }
            }
        }

        if error_count > 0 || rotated_count != 1 {
            println!("FAILURE AT ITER {}: rotated={}, debounced={}, errors={}", iter, rotated_count, debounced_count, error_count);
            if !errors.is_empty() {
                println!("Sample error: {}", errors[0]);
            }
        }

        assert_eq!(error_count, 0, "Iteration {} failed with errors: {:?}", iter, errors);
        assert_eq!(rotated_count, 1, "Iteration {}: Exactly ONE slot failover should occur in a burst of 50, but got {}", iter, rotated_count);
        assert_eq!(debounced_count, num_tasks - 1, "Iteration {}: All other requests must be debounced", iter);
    }
}

/// Test 2: Barrier-synchronized 50 concurrent tasks when last failover was > 25s ago on multi-thread
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn test_high_concurrency_burst_after_debounce_expired_50() {
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

    // Set last_failover to 30 seconds ago
    let now_ms = chrono::Utc::now().timestamp_millis();
    failover.set_last_failover_epoch_ms(now_ms - 30_000);

    let num_tasks = 50;
    let barrier = Arc::new(Barrier::new(num_tasks));
    let mut handles = Vec::with_capacity(num_tasks);

    for _ in 0..num_tasks {
        let engine = failover.clone();
        let b = barrier.clone();
        handles.push(tokio::spawn(async move {
            b.wait().await;
            engine.on_quota_error().await
        }));
    }

    let mut rotated_count = 0;
    let mut debounced_count = 0;
    let mut error_count = 0;

    for handle in handles {
        match handle.await.unwrap() {
            Ok(FailoverDecision::Rotated { .. }) => rotated_count += 1,
            Ok(FailoverDecision::Debounced { .. }) => debounced_count += 1,
            Err(_e) => error_count += 1,
        }
    }

    println!("=== TEST 2 RESULTS (post-debounce burst) ===");
    println!("Rotated: {}, Debounced: {}, Errors: {}", rotated_count, debounced_count, error_count);

    assert_eq!(error_count, 0, "No errors should occur during concurrent failover burst");
    assert_eq!(rotated_count, 1, "Exactly ONE slot failover should occur in a burst of 50");
    assert_eq!(debounced_count, num_tasks - 1, "All other requests must be debounced");
}
