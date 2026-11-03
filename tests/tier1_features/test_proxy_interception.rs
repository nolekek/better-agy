use agy_mock::MockServer;
use agy_proxy::{
    AccountSlot, FailoverDecision, FailoverEngine, ProxyConfig, ProxyServer, QuotaDetector,
    SlotState, Vault,
};
use hyper::HeaderMap;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

#[test]
fn test_f8_quota_detector_identifies_http_429() {
    let detector = QuotaDetector::new();
    let headers = HeaderMap::new();
    let body = b"{}";

    assert!(detector.is_quota_error(429, &headers, body));
    assert!(!detector.is_quota_error(200, &headers, body));
    assert!(!detector.is_quota_error(500, &headers, body));
}

#[test]
fn test_f8_quota_detector_identifies_grpc_status_8() {
    let detector = QuotaDetector::new();
    let mut headers = HeaderMap::new();
    headers.insert("grpc-status", "8".parse().unwrap());
    let body = b"";

    assert!(detector.is_quota_error(200, &headers, body));
}

#[test]
fn test_f8_quota_detector_identifies_quota_metric_error_body() {
    let detector = QuotaDetector::new();
    let headers = HeaderMap::new();
    let body = br#"{"error":{"code":429,"message":"Quota exceeded for quota metric 'Queries'","status":"RESOURCE_EXHAUSTED"}}"#;

    assert!(detector.is_quota_error(200, &headers, body));
}

#[tokio::test]
async fn test_f10_failover_engine_rotates_slot() {
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

    let decision = engine.on_quota_error().await.expect("Rotate slot");
    match decision {
        FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "slot_1");
            assert_eq!(new_slot.id, "slot_2");
            assert_eq!(new_slot.access_token, "token_2");
        }
        FailoverDecision::Debounced { .. } => panic!("First error should rotate, not debounce"),
    }

    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.id, "slot_2");
}

#[tokio::test]
async fn test_f7_f10_proxy_transparent_inflight_retry() {
    // 1. Start synthetic mock server
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    // Mock behavior: Fail when Bearer is "slot_1_token", succeed when Bearer is "slot_2_token"
    controller.fail_bearer_token("slot_1_token").await;

    // 2. Initialize Vault with 2 slots
    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let slot1 = AccountSlot::new(
        "slot_1",
        "dev1@example.com",
        "slot_1_token",
        "ref_1",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    let slot2 = AccountSlot::new(
        "slot_2",
        "dev2@example.com",
        "slot_2_token",
        "ref_2",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Ready,
    );

    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    // 3. Start ProxyServer with upstream_override pointing to mock
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some(mock.base_url()),
        max_retries: 3,
        connect_timeout: std::time::Duration::from_secs(5),
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()));
    let server = ProxyServer::new(proxy_config, failover);

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let _ = server.run(listener, shutdown_rx).await;
    });

    // 4. Client sends request to proxy
    let client = reqwest::Client::new();
    let proxy_req_url = format!("http://{}/v1alpha/models/gemini-pro:streamGenerateContent", proxy_addr);

    let resp = client
        .post(&proxy_req_url)
        .header("host", "generativelanguage.googleapis.com")
        .json(&serde_json::json!({"prompt": "Hello via Proxy"}))
        .send()
        .await
        .expect("Client send to proxy");

    // Client receives HTTP 200 OK because proxy transparently retried!
    assert_eq!(resp.status(), 200, "Client should receive 200 OK after transparent retry");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(
        body["candidates"][0]["content"]["parts"][0]["text"],
        "Synthetic Antigravity Generation Success"
    );

    // Verify mock server observed both requests: slot_1_token first, then slot_2_token!
    let tokens = controller.received_bearer_tokens().await;
    assert_eq!(tokens, vec!["slot_1_token", "slot_2_token"]);

    let _ = shutdown_tx.send(());
    mock.shutdown().await;
}
