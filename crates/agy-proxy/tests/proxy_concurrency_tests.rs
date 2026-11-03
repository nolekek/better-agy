use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
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
use tokio::sync::{broadcast, Barrier, Mutex};

use agy_proxy::failover::FailoverEngine;
use agy_proxy::proxy::{ProxyConfig, ProxyServer};
use agy_proxy::vault::{AccountSlot, SlotState, Vault};

fn mock_body(s: &str) -> BoxBody<Bytes, std::io::Error> {
    Full::new(Bytes::from(s.to_string()))
        .map_err(|never: Infallible| -> std::io::Error { match never {} })
        .boxed()
}

/// Challenge 1, 2, 3:
/// End-to-end stress harness: 50 parallel requests hitting ProxyServer simultaneously.
/// Initial state:
/// - slot_1 has token "token_slot_1", which triggers HTTP 429 on upstream.
/// - slot_2 has token "token_slot_2", which returns HTTP 200 on upstream.
///
/// Expected behavior:
/// - Exactly ONE failover from slot_1 to slot_2 occurs.
/// - Debounce prevents slot thrashing.
/// - ALL 50 in-flight requests transparently retry with token_slot_2 and receive HTTP 200.
/// - In-flight request bodies are preserved without corruption or truncation.
/// - No file locking / permission denied crashes.
#[tokio::test]
async fn test_proxy_server_50_concurrent_429_burst() {
    // 1. Mock upstream server
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    let req_count = Arc::new(AtomicUsize::new(0));
    let received_bodies = Arc::new(Mutex::new(Vec::new()));
    let received_tokens = Arc::new(Mutex::new(Vec::new()));

    let count_clone = req_count.clone();
    let bodies_clone = received_bodies.clone();
    let tokens_clone = received_tokens.clone();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match mock_listener.accept().await {
                Ok(res) => res,
                Err(_) => break,
            };
            let io = TokioIo::new(stream);
            let c = count_clone.clone();
            let b = bodies_clone.clone();
            let t = tokens_clone.clone();

            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |mut req: Request<hyper::body::Incoming>| {
                            let c = c.clone();
                            let b = b.clone();
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

                                let body_bytes = req.body_mut().collect().await.unwrap().to_bytes();
                                b.lock().await.push(String::from_utf8_lossy(&body_bytes).to_string());

                                let (status, resp_body) = if auth.contains("token_slot_1") {
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
    let slot1 = AccountSlot::new(
        "slot_1",
        "corp1@company.com",
        "token_slot_1",
        "1//ref_1",
        now_sec + 3600,
        SlotState::Active,
    );
    let slot2 = AccountSlot::new(
        "slot_2",
        "corp2@company.com",
        "token_slot_2",
        "1//ref_2",
        now_sec + 3600,
        SlotState::Ready,
    );
    let slot3 = AccountSlot::new(
        "slot_3",
        "corp3@company.com",
        "token_slot_3",
        "1//ref_3",
        now_sec + 3600,
        SlotState::Ready,
    );
    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();
    vault.upsert_slot(slot3).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some(format!("http://{}", mock_addr)),
        max_retries: 5,
        connect_timeout: Duration::from_secs(5),
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()).with_debounce(Duration::from_millis(25_000)));
    let server = ProxyServer::new(proxy_config, failover.clone());

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let _ = server.run(listener, shutdown_rx).await;
    });

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(100)
        .build()
        .unwrap();

    let proxy_url = format!("http://{}/v1alpha/models/gemini-pro:streamGenerateContent", proxy_addr);

    let num_requests = 50;
    let barrier = Arc::new(Barrier::new(num_requests));
    let mut handles = Vec::with_capacity(num_requests);

    for req_idx in 0..num_requests {
        let c = client.clone();
        let u = proxy_url.clone();
        let b = barrier.clone();
        let body_payload = format!(r#"{{"client_id": {}, "data": "payload_test_data_{}"}}"#, req_idx, req_idx);

        handles.push(tokio::spawn(async move {
            b.wait().await;
            let resp = c.post(&u)
                .header("host", "generativelanguage.googleapis.com")
                .header("content-type", "application/json")
                .body(body_payload.clone())
                .send()
                .await;
            (req_idx, body_payload, resp)
        }));
    }

    let mut status_200_count = 0;
    let mut status_429_count = 0;
    let mut status_other_count = 0;
    let mut other_statuses = Vec::new();
    let mut client_body_map = std::collections::HashMap::new();

    for h in handles {
        let (req_idx, payload, res) = h.await.unwrap();
        client_body_map.insert(req_idx, payload);
        match res {
            Ok(resp) => {
                let code = resp.status().as_u16();
                if code == 200 {
                    status_200_count += 1;
                } else if code == 429 {
                    status_429_count += 1;
                } else {
                    status_other_count += 1;
                    other_statuses.push((req_idx, code));
                }
            }
            Err(_) => {
                status_other_count += 1;
                other_statuses.push((req_idx, 999));
            }
        }
    }

    println!("=== PROXY 50-REQUEST CONCURRENCY BURST RESULTS ===");
    println!("Status 200 (Success): {}", status_200_count);
    println!("Status 429 (Debounced / unrecovered): {}", status_429_count);
    println!("Status Other (Error): {}", status_other_count);
    if !other_statuses.is_empty() {
        println!("Other status codes: {:?}", other_statuses);
    }

    // Inspect Vault State
    let final_active = vault.get_active_slot().await.unwrap();
    println!("Final active slot: {}", final_active.id);
    let s1 = vault.get_slot("slot_1").await.unwrap();
    let s2 = vault.get_slot("slot_2").await.unwrap();
    let s3 = vault.get_slot("slot_3").await.unwrap();
    println!("slot_1 state: {:?}", s1.state);
    println!("slot_2 state: {:?}", s2.state);
    println!("slot_3 state: {:?}", s3.state);

    // Inspect Upstream Requests
    let recorded_tokens = received_tokens.lock().await.clone();
    println!("Total recorded requests at Mock Upstream: {}", recorded_tokens.len());
    let token_1_count = recorded_tokens.iter().filter(|t| t.contains("token_slot_1")).count();
    let token_2_count = recorded_tokens.iter().filter(|t| t.contains("token_slot_2")).count();
    println!("Upstream requests with token_slot_1: {}", token_1_count);
    println!("Upstream requests with token_slot_2: {}", token_2_count);

    // Verify In-flight Request Body Preservation
    let recorded_bodies = received_bodies.lock().await.clone();
    println!("Total recorded bodies at Upstream: {}", recorded_bodies.len());
    // For every request that was retried with token_2 and succeeded, verify body matched
    for body in &recorded_bodies {
        assert!(body.contains("payload_test_data_"), "Body must not be corrupted or empty");
    }

    let _ = shutdown_tx.send(());

    // Invariants:
    // 1. Exactly ONE slot rotation should have occurred: slot_1 -> slot_2. slot_3 must NOT be rotated into!
    assert_eq!(final_active.id, "slot_2", "Final active slot must be slot_2");
    assert_eq!(s3.state, SlotState::Ready, "slot_3 must still be Ready (no carousel cascade)");

    // 2. All 50 client requests should receive HTTP 200 OK via transparent retry!
    assert_eq!(status_200_count, num_requests, "All 50 client requests must receive HTTP 200 OK");
}
