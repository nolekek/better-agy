use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use agy_proxy::failover::FailoverEngine;
use agy_proxy::proxy::{ProxyConfig, ProxyServer};
use agy_proxy::vault::{AccountSlot, SlotState, Vault, VaultData};
use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Full};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

fn mock_body(s: &str) -> BoxBody<Bytes, std::io::Error> {
    Full::new(Bytes::from(s.to_string()))
        .map_err(|never: Infallible| -> std::io::Error { match never {} })
        .boxed()
}

#[tokio::test]
async fn test_in_flight_retry_with_bearer_token_substitution() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let now_sec = chrono::Utc::now().timestamp();
    let initial_data = VaultData {
        slots: vec![
            AccountSlot::new(
                "slot_exhausted",
                "exhausted@google.com",
                "token_slot_1",
                "rt1",
                now_sec + 3600,
                SlotState::Active,
            ),
            AccountSlot::new(
                "slot_fresh",
                "fresh@google.com",
                "token_slot_2",
                "rt2",
                now_sec + 3600,
                SlotState::Ready,
            ),
        ],
        active_slot_id: Some("slot_exhausted".to_string()),
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    let failover = Arc::new(FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000)));

    // 1. Start Mock Upstream Google Server
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    let req_count = Arc::new(AtomicUsize::new(0));
    let received_auth_tokens = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let count_clone = req_count.clone();
    let tokens_clone = received_auth_tokens.clone();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match mock_listener.accept().await {
                Ok(res) => res,
                Err(_) => break,
            };
            let io = TokioIo::new(stream);
            let c = count_clone.clone();
            let t = tokens_clone.clone();

            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req: Request<hyper::body::Incoming>| {
                            let c = c.clone();
                            let t = t.clone();
                            async move {
                                c.fetch_add(1, Ordering::SeqCst);
                                let auth = req
                                    .headers()
                                    .get("authorization")
                                    .and_then(|h| h.to_str().ok())
                                    .unwrap_or("")
                                    .to_string();

                                t.lock().await.push(auth.clone());

                                let (status, body) = if auth.contains("token_slot_1") {
                                    // Slot 1 is exhausted -> return 429 ResourceExhausted
                                    (
                                        StatusCode::TOO_MANY_REQUESTS,
                                        r#"{"error": {"code": 429, "message": "ResourceExhausted: Quota exceeded for quota metric", "status": "RESOURCE_EXHAUSTED"}}"#,
                                    )
                                } else if auth.contains("token_slot_2") {
                                    // Slot 2 has fresh quota -> return 200 OK
                                    (
                                        StatusCode::OK,
                                        r#"{"candidates": [{"content": {"parts": [{"text": "Success from Antigravity LLM"}]}}]}"#,
                                    )
                                } else {
                                    (StatusCode::UNAUTHORIZED, r#"{"error": "Unauthorized"}"#)
                                };

                                let resp = Response::builder()
                                    .status(status)
                                    .header("content-type", "application/json")
                                    .body(mock_body(body))
                                    .unwrap();

                                Ok::<_, Infallible>(resp)
                            }
                        }),
                    )
                    .await;
            });
        }
    });

    // 2. Start Proxy Server with upstream_override pointing to Mock Upstream
    let proxy_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = proxy_listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some(format!("http://{}", mock_addr)),
        ..Default::default()
    };

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    let proxy_server = ProxyServer::new(proxy_config, failover);

    tokio::spawn(async move {
        let _ = proxy_server.run(proxy_listener, shutdown_rx).await;
    });

    // 3. Client sends single request to Proxy Server
    let client = reqwest::Client::new();
    let client_req_url = format!("http://{}/v1beta/models/antigravity:generateContent", proxy_addr);

    let client_payload = r#"{"contents": [{"parts": [{"text": "Hello AI"}]}]}"#;

    // Client initially sends whatever Authorization header (e.g. dummy or stale token)
    let response = client
        .post(&client_req_url)
        .header("content-type", "application/json")
        .header("authorization", "Bearer stale_client_token")
        .body(client_payload)
        .send()
        .await
        .unwrap();

    // 4. VERIFICATION: Client receives HTTP 200 OK with successful stream!
    assert_eq!(
        response.status(),
        reqwest::StatusCode::OK,
        "Client must transparently receive HTTP 200 OK"
    );

    let body_text = response.text().await.unwrap();
    assert!(
        body_text.contains("Success from Antigravity LLM"),
        "Client received response body: {}",
        body_text
    );

    // 5. Verify upstream request trace
    // Mock upstream should have received:
    // Request 1: with Bearer token_slot_1 (which returned 429)
    // Request 2: with substituted Bearer token_slot_2 (which returned 200)
    assert_eq!(
        req_count.load(Ordering::SeqCst),
        2,
        "Mock upstream should have received exactly 2 requests (1st failed, 2nd retried)"
    );

    let tokens = received_auth_tokens.lock().await;
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0], "Bearer token_slot_1");
    assert_eq!(tokens[1], "Bearer token_slot_2");

    // 6. Verify Vault state
    let slot1 = vault.get_slot("slot_exhausted").await.unwrap();
    assert!(
        matches!(slot1.state, SlotState::Cooldown { .. }),
        "Exhausted slot must be placed in Cooldown"
    );

    let active_slot = vault.get_active_slot().await.unwrap();
    assert_eq!(
        active_slot.id, "slot_fresh",
        "Fresh slot must now be active"
    );

    let _ = shutdown_tx.send(());
}
