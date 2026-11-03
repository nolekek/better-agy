use std::convert::Infallible;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use agy_proxy::refresh::{TokenRefresher, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET};
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

fn mock_body(s: &str) -> BoxBody<Bytes, std::io::Error> {
    Full::new(Bytes::from(s.to_string()))
        .map_err(|never: Infallible| -> std::io::Error { match never {} })
        .boxed()
}

#[tokio::test]
async fn test_proactive_oauth_refresh_within_300s() {
    let dir = tempdir().unwrap();
    let vault_path = dir.path().join("vault.enc");
    let key = Vault::generate_key();
    let vault = Vault::new(&vault_path, key);

    let now_sec = chrono::Utc::now().timestamp();
    let initial_data = VaultData {
        slots: vec![
            // Slot 1: Expires in 120s (< 300s) -> MUST BE REFRESHED
            AccountSlot::new(
                "slot_due",
                "due@corp.com",
                "old_token_due",
                "valid_refresh_token_1",
                now_sec + 120,
                SlotState::Active,
            ),
            // Slot 2: Expires in 1000s (> 300s) -> MUST NOT BE REFRESHED
            AccountSlot::new(
                "slot_ok",
                "ok@corp.com",
                "token_ok",
                "valid_refresh_token_2",
                now_sec + 1000,
                SlotState::Ready,
            ),
            // Slot 3: Revoked token -> returns 400 invalid_grant -> MARKS DISABLED
            AccountSlot::new(
                "slot_revoked",
                "revoked@corp.com",
                "token_revoked",
                "revoked_refresh_token",
                now_sec + 50,
                SlotState::Ready,
            ),
        ],
        ..Default::default()
    };
    vault.save_data(initial_data).await.unwrap();

    // 1. Mock OAuth Token Endpoint
    let mock_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = mock_listener.local_addr().unwrap();

    let refresh_count = Arc::new(AtomicUsize::new(0));
    let count_clone = refresh_count.clone();

    tokio::spawn(async move {
        loop {
            let (stream, _) = match mock_listener.accept().await {
                Ok(res) => res,
                Err(_) => break,
            };
            let io = TokioIo::new(stream);
            let c = count_clone.clone();

            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(
                        io,
                        service_fn(move |req: Request<hyper::body::Incoming>| {
                            let c = c.clone();
                            async move {
                                let body_bytes = req.into_body().collect().await.unwrap().to_bytes();
                                let body_str = String::from_utf8_lossy(&body_bytes);

                                c.fetch_add(1, Ordering::SeqCst);

                                assert!(body_str.contains("grant_type=refresh_token"));
                                assert!(body_str.contains(DEFAULT_CLIENT_ID));

                                if body_str.contains("revoked_refresh_token") {
                                    let resp = Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .header("content-type", "application/json")
                                        .body(mock_body(
                                            r#"{"error": "invalid_grant", "error_description": "Token has been expired or revoked."}"#,
                                        ))
                                        .unwrap();
                                    return Ok::<_, Infallible>(resp);
                                }

                                let resp_json = r#"{
                                    "access_token": "ya29.proactively_refreshed_token",
                                    "expires_in": 3600,
                                    "token_type": "Bearer"
                                }"#;

                                let resp = Response::builder()
                                    .status(StatusCode::OK)
                                    .header("content-type", "application/json")
                                    .body(mock_body(resp_json))
                                    .unwrap();

                                Ok::<_, Infallible>(resp)
                            }
                        }),
                    )
                    .await;
            });
        }
    });

    let token_url = format!("http://{}/token", mock_addr);
    let refresher = TokenRefresher::new(vault.clone())
        .with_endpoint(token_url, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET)
        .with_threshold(300);

    // 2. Trigger proactive refresh scan
    let results = refresher.refresh_due_slots().await;
    // slot_due (<300s) and slot_revoked (<300s) should both be evaluated
    assert_eq!(results.len(), 2);

    // 3. Verify Slot 1 was updated
    let slot_due_updated = vault.get_slot("slot_due").await.unwrap();
    assert_eq!(
        slot_due_updated.access_token,
        "ya29.proactively_refreshed_token"
    );
    assert!(slot_due_updated.expires_at >= now_sec + 3500);

    // 4. Verify Slot 2 was untouched
    let slot_ok = vault.get_slot("slot_ok").await.unwrap();
    assert_eq!(slot_ok.access_token, "token_ok");
    assert_eq!(slot_ok.expires_at, now_sec + 1000);

    // 5. Verify Slot 3 was marked Disabled on invalid_grant
    let slot_revoked = vault.get_slot("slot_revoked").await.unwrap();
    assert!(
        matches!(slot_revoked.state, SlotState::Disabled { .. }),
        "Revoked slot must be marked Disabled"
    );
}
