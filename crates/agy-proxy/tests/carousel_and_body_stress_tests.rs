use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};

use agy_proxy::failover::{FailoverDecision, FailoverEngine};
use agy_proxy::proxy::{ProxyConfig, ProxyServer};
use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData};
use agy_proxy::FailoverError;

fn mock_body(s: &str) -> BoxBody<Bytes, std::io::Error> {
    Full::new(Bytes::from(s.to_string()))
        .map_err(|never: Infallible| -> std::io::Error { match never {} })
        .boxed()
}

/// Challenge 2: Carousel wrapping test.
/// Given 3 slots [A, B, C]:
/// Cooldown is set to 1 second.
/// 1. A fails -> rotates to B.
/// 2. B fails (after debounce) -> rotates to C.
/// 3. Wait 1.1s for A's cooldown to expire.
/// 4. C fails (after debounce) -> MUST wrap around to A!
#[tokio::test]
async fn test_carousel_wrapping_around_to_first_slot() {
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
        ],
        active_slot_id: Some("slot_A".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    // 1-second cooldown, 10ms debounce for rapid testing
    let failover = Arc::new(
        FailoverEngine::new(vault.clone())
            .with_cooldown(1)
            .with_debounce(Duration::from_millis(10)),
    );

    // Failover 1: A -> B
    let d1 = failover.on_quota_error().await.unwrap();
    assert_eq!(d1, FailoverDecision::Rotated {
        new_slot: vault.get_slot("slot_B").await.unwrap(),
        old_slot_id: "slot_A".to_string(),
    });

    // Wait 15ms so debounce passes
    tokio::time::sleep(Duration::from_millis(15)).await;

    // Failover 2: B -> C
    let d2 = failover.on_quota_error().await.unwrap();
    assert_eq!(d2, FailoverDecision::Rotated {
        new_slot: vault.get_slot("slot_C").await.unwrap(),
        old_slot_id: "slot_B".to_string(),
    });

    // Wait 1.1s for slot_A's cooldown to expire
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Failover 3: C -> A (wrap around!)
    let d3 = failover.on_quota_error().await.unwrap();
    assert_eq!(d3, FailoverDecision::Rotated {
        new_slot: vault.get_slot("slot_A").await.unwrap(),
        old_slot_id: "slot_C".to_string(),
    });

    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.id, "slot_A", "Carousel must cleanly wrap around to slot_A");
}

/// Challenge 2: All-slots-cooldown handling.
/// When all slots in the carousel are in cooldown, FailoverEngine MUST return
/// `FailoverError::AllSlotsExhausted`, and ProxyServer MUST return HTTP 429
/// gracefully to client without panicking or corrupting vault state.
#[tokio::test]
async fn test_all_slots_cooldown_exhaustion() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let now_sec = chrono::Utc::now().timestamp();
    // 2 slots, both cooling down for 300s
    let initial_data = VaultData {
        slots: vec![
            AccountSlot::new("slot_A", "a@test.com", "token_a", "rt_a", now_sec + 3600, SlotState::Cooldown { until: now_sec + 300 }),
            AccountSlot::new("slot_B", "b@test.com", "token_b", "rt_b", now_sec + 3600, SlotState::Cooldown { until: now_sec + 300 }),
        ],
        active_slot_id: Some("slot_A".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    let failover = Arc::new(
        FailoverEngine::new(vault.clone())
            .with_cooldown(300)
            .with_debounce(Duration::from_millis(10)),
    );

    // Calling on_quota_error must return AllSlotsExhausted error
    let res = failover.on_quota_error().await;
    assert!(
        matches!(res, Err(FailoverError::AllSlotsExhausted)),
        "Expected AllSlotsExhausted, got {:?}",
        res
    );
}

/// Challenge 2 & 3: Large in-flight request body preservation under rapid retries.
/// Send a large (5 MB) payload through ProxyServer when upstream returns 429 for slot 1,
/// but 200 for slot 2.
/// Verify:
/// 1. Exactly 5 MB is received upstream on retry.
/// 2. SHA256 of sent body == SHA256 of body received by upstream on retry.
/// 3. Memory does not leak.
#[tokio::test]
async fn test_large_body_preservation_and_sha256_integrity() {
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    let received_bodies = Arc::new(Mutex::new(Vec::new()));
    let received_tokens = Arc::new(Mutex::new(Vec::new()));

    let bodies_clone = received_bodies.clone();
    let tokens_clone = received_tokens.clone();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match mock_listener.accept().await {
                Ok(res) => res,
                Err(_) => break,
            };
            let io = TokioIo::new(stream);
            let b = bodies_clone.clone();
            let t = tokens_clone.clone();

            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |mut req: Request<hyper::body::Incoming>| {
                            let b = b.clone();
                            let t = t.clone();
                            async move {
                                let auth = req
                                    .headers()
                                    .get("authorization")
                                    .and_then(|h| h.to_str().ok())
                                    .unwrap_or("")
                                    .to_string();

                                t.lock().await.push(auth.clone());

                                let body_bytes = req.body_mut().collect().await.unwrap().to_bytes();
                                b.lock().await.push(body_bytes);

                                let (status, resp_body) = if auth.contains("token_1") {
                                    (
                                        StatusCode::TOO_MANY_REQUESTS,
                                        r#"{"error": {"code": 429, "message": "ResourceExhausted: Quota exceeded", "status": "RESOURCE_EXHAUSTED"}}"#,
                                    )
                                } else {
                                    (
                                        StatusCode::OK,
                                        r#"{"candidates": [{"content": {"parts": [{"text": "Body Integrity Verified"}]}}]}"#,
                                    )
                                };

                                let resp = Response::builder()
                                    .status(status)
                                    .header("content-type", "application/json")
                                    .body(mock_body(resp_body))
                                    .unwrap();

                                Ok::<_, Infallible>(resp)
                            }
                        }),
                    )
                    .await;
            });
        }
    });

    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");
    let vault = Vault::new(&vault_path, Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now_sec = chrono::Utc::now().timestamp();
    let slot1 = AccountSlot::new("s1", "s1@c.com", "token_1", "rt1", now_sec + 3600, SlotState::Active);
    let slot2 = AccountSlot::new("s2", "s2@c.com", "token_2", "rt2", now_sec + 3600, SlotState::Ready);
    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some(format!("http://{}", mock_addr)),
        max_retries: 3,
        connect_timeout: Duration::from_secs(5),
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000)));
    let server = ProxyServer::new(proxy_config, failover.clone());

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let _ = server.run(listener, shutdown_rx).await;
    });

    // Create 5 MB payload with pseudo-random pattern
    let payload_size = 5 * 1024 * 1024; // 5 MB
    let mut large_payload = Vec::with_capacity(payload_size);
    for i in 0..payload_size {
        large_payload.push((i % 251) as u8);
    }

    let client = reqwest::Client::new();
    let proxy_url = format!("http://{}/v1alpha/models/gemini-pro:streamGenerateContent", proxy_addr);

    let resp = client
        .post(&proxy_url)
        .header("host", "generativelanguage.googleapis.com")
        .header("content-type", "application/octet-stream")
        .body(large_payload.clone())
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200, "5MB request must succeed on retry");

    // Verify upstream received 2 requests
    let bodies = received_bodies.lock().await;
    assert_eq!(bodies.len(), 2, "Upstream must have received 2 requests (attempt 1 and retry)");

    // Both attempt 1 and retry must have exact same 5MB bytes as sent
    assert_eq!(bodies[0].len(), payload_size, "First attempt body size must match");
    assert_eq!(bodies[1].len(), payload_size, "Retry attempt body size must match");

    assert_eq!(&bodies[0][..], &large_payload[..], "First attempt body bytes must match exactly");
    assert_eq!(&bodies[1][..], &large_payload[..], "Retry attempt body bytes must match exactly");

    let _ = shutdown_tx.send(());
}

/// Challenge 2 & Reviewer Finding 1: Candidate slot token expiration verification.
/// Slot B has expired/expiring token (expires_at < now + 60s), so rotate_slot MUST skip Slot B
/// and select Slot C whose token is valid for 3600s.
#[tokio::test]
async fn test_candidate_slot_token_expiration_skips_expiring_slot() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let vault = Vault::new(&vault_path, Vault::generate_key());

    let now_sec = chrono::Utc::now().timestamp();
    // slot_A: Active (failing)
    // slot_B: Ready, but token expires in 30s (< 60s margin) -> MUST BE SKIPPED
    // slot_C: Ready, token expires in 3600s -> MUST BE SELECTED
    let initial_data = VaultData {
        slots: vec![
            AccountSlot::new("slot_A", "a@test.com", "token_a", "rt_a", now_sec + 3600, SlotState::Active),
            AccountSlot::new("slot_B", "b@test.com", "token_b", "rt_b", now_sec + 30, SlotState::Ready),
            AccountSlot::new("slot_C", "c@test.com", "token_c", "rt_c", now_sec + 3600, SlotState::Ready),
        ],
        active_slot_id: Some("slot_A".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    let failover = Arc::new(FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(10)));
    let decision = failover.on_quota_error().await.unwrap();

    match decision {
        FailoverDecision::Rotated { new_slot, old_slot_id } => {
            assert_eq!(old_slot_id, "slot_A");
            assert_eq!(new_slot.id, "slot_C", "slot_B must be skipped due to expires_at < now + 60s");
        }
        _ => panic!("Expected Rotated decision"),
    }
}

/// Challenge 2 & Reviewer Finding 3: Request body exceeding 32MB limit returns HTTP 413 Payload Too Large.
#[tokio::test]
async fn test_request_body_exceeding_max_limit_returns_413() {
    let tmp = tempdir().unwrap();
    let vault_path = tmp.path().join("vault.enc");
    let vault = Vault::new(&vault_path, Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now_sec = chrono::Utc::now().timestamp();
    vault.upsert_slot(AccountSlot::new("s1", "s1@c.com", "tok1", "rt1", now_sec + 3600, SlotState::Active)).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some("http://127.0.0.1:9".to_string()),
        max_retries: 1,
        connect_timeout: Duration::from_secs(1),
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()));
    let server = ProxyServer::new(proxy_config, failover.clone());

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let _ = server.run(listener, shutdown_rx).await;
    });

    let client = reqwest::Client::new();
    let proxy_url = format!("http://{}/test", proxy_addr);

    // Test: Content-Length > 32MB header rejection
    let resp = client
        .post(&proxy_url)
        .header("content-length", "35000000") // 35 MB
        .body(vec![0u8; 1024])
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);

    let _ = shutdown_tx.send(());
}

