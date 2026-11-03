use std::sync::Arc;
use std::time::Duration;

use agy_proxy::failover::FailoverEngine;
use agy_proxy::proxy::{ProxyConfig, ProxyServer};
use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData};
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

#[tokio::test]
async fn test_proxy_status_endpoint_empty_vault() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let failover = Arc::new(FailoverEngine::new(vault.clone()));

    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        ..Default::default()
    };

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let proxy_server = ProxyServer::new(proxy_config, failover);

    tokio::spawn(async move {
        let _ = proxy_server.run(proxy_listener, shutdown_rx).await;
    });

    let client = reqwest::Client::new();
    let status_url = format!("http://{}/api/proxy/status", proxy_addr);

    let res = client.get(&status_url).send().await.expect("Failed to send request");
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let json: serde_json::Value = res.json().await.expect("Failed to parse JSON");
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["total_slots"], 0);
    assert_eq!(json["available_slots"], 0);
    assert!(json["active_slot"].is_null());
    assert_eq!(json["slots"].as_array().unwrap().len(), 0);
    assert_eq!(json["quotas"]["total_slots"], 0);

    let _ = shutdown_tx.send(());
}

#[tokio::test]
async fn test_proxy_status_endpoint_live_telemetry_and_cooldown() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let now_sec = chrono::Utc::now().timestamp();
    let initial_data = VaultData {
        slots: vec![
            AccountSlot::new(
                "slot_1",
                "dev1@google.com",
                "ya29.abcdefghijk123456",
                "rt_1",
                now_sec + 3600,
                SlotState::Active,
            ),
            AccountSlot::new(
                "slot_2",
                "dev2@google.com",
                "ya29.lmnopqrstuv789012",
                "rt_2",
                now_sec + 3600,
                SlotState::Ready,
            ),
        ],
        active_slot_id: Some("slot_1".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    let failover = Arc::new(FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000)));

    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        ..Default::default()
    };

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let proxy_server = ProxyServer::new(proxy_config, failover.clone());

    tokio::spawn(async move {
        let _ = proxy_server.run(proxy_listener, shutdown_rx).await;
    });

    let client = reqwest::Client::new();
    let status_url = format!("http://{}/api/proxy/status", proxy_addr);

    // Initial check: slot_1 is active
    let res = client.get(&status_url).send().await.expect("Failed to send request");
    assert_eq!(res.status(), reqwest::StatusCode::OK);

    let json: serde_json::Value = res.json().await.expect("Failed to parse JSON");
    assert_eq!(json["status"], "healthy");
    assert_eq!(json["total_slots"], 2);
    assert_eq!(json["available_slots"], 2);
    assert_eq!(json["active_slot_id"], "slot_1");
    assert_eq!(json["debounce_window_sec"], 25);
    assert_eq!(json["quotas"]["cooldown_slots"], 0);

    let slots = json["slots"].as_array().expect("slots must be array");
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0]["id"], "slot_1");
    assert_eq!(slots[0]["email"], "dev1@google.com");
    assert_eq!(slots[0]["token_valid"], true);
    assert_eq!(slots[0]["ready_for_use"], true);
    assert!(slots[0]["token_preview"].as_str().unwrap().contains("..."));

    // Trigger quota failover rotation
    let decision = failover.on_quota_error().await.expect("Failover should succeed");
    match decision {
        agy_proxy::failover::FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "slot_1");
            assert_eq!(new_slot.id, "slot_2");
        }
        _ => panic!("Expected rotation"),
    }

    // Verify status endpoint reflects rotation and cooldown state in real time
    let res2 = client.get(&status_url).send().await.expect("Failed to send request");
    assert_eq!(res2.status(), reqwest::StatusCode::OK);

    let json2: serde_json::Value = res2.json().await.expect("Failed to parse JSON");
    assert_eq!(json2["active_slot_id"], "slot_2");
    assert_eq!(json2["quotas"]["cooldown_slots"], 1);
    assert_eq!(json2["available_slots"], 1);
    assert!(json2["last_failover_epoch_ms"].as_i64().unwrap() > 0);

    let _ = shutdown_tx.send(());
}
