use agy_mock::{GoogleApiErrorResponse, MockServer, OAuthTokenResponse};
use reqwest::header::AUTHORIZATION;
use std::time::Duration;

#[tokio::test]
async fn test_f24_mock_200_generation() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let client = reqwest::Client::new();
    let resp = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer test_token")
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": "Hello Antigravity"}]}]
        }))
        .send()
        .await
        .expect("Send generate request");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("Valid JSON");
    assert_eq!(
        body["candidates"][0]["content"]["parts"][0]["text"],
        "Synthetic Antigravity Generation Success"
    );
    assert_eq!(controller.generate_count().await, 1);

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f24_mock_chunked_streaming() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let tokens = vec![
        "fn ".to_string(),
        "solve() ".to_string(),
        "-> Result<()> ".to_string(),
        "{ Ok(()) }".to_string(),
    ];
    controller
        .set_stream_chunks(tokens.clone(), Duration::from_millis(10))
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(mock.generate_url())
        .send()
        .await
        .expect("Send streaming request");

    assert_eq!(resp.status(), 200);
    let body_text = resp.text().await.expect("Read stream text");
    for tok in &tokens {
        assert!(body_text.contains(tok));
    }

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f24_mock_429_quota_metric_body() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    controller.fail_first_n_with_429(1).await;

    let client = reqwest::Client::new();
    let resp = client
        .post(mock.generate_url())
        .send()
        .await
        .expect("Send request");

    assert_eq!(resp.status(), 429);
    assert_eq!(
        resp.headers().get("retry-after").unwrap().to_str().unwrap(),
        "25"
    );

    let err: GoogleApiErrorResponse = resp.json().await.expect("Parse GoogleApiErrorResponse");
    assert_eq!(err.error.code, 429);
    assert_eq!(err.error.status, "RESOURCE_EXHAUSTED");
    assert!(err.error.message.contains("Quota exceeded for quota metric"));

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f24_mock_grpc_resource_exhausted() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    controller.set_grpc_resource_exhausted().await;

    let client = reqwest::Client::new();
    let resp = client.post(mock.generate_url()).send().await.unwrap();

    assert_eq!(
        resp.headers().get("grpc-status").unwrap().to_str().unwrap(),
        "8"
    );
    let grpc_msg = resp
        .headers()
        .get("grpc-message")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(grpc_msg.contains("Resource has been exhausted"));

    let body = resp.text().await.unwrap();
    assert!(body.contains("ResourceExhausted"));

    mock.shutdown().await;
}

#[tokio::test]
async fn test_f24_mock_oauth_refresh_endpoint() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", "1//04refreshtoken"),
        ("client_id", "test-client-id"),
        ("client_secret", "test-client-secret"),
    ];

    let resp = client
        .post(mock.oauth_url())
        .form(&params)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let token_data: OAuthTokenResponse = resp.json().await.unwrap();
    assert!(token_data.access_token.starts_with("ya29."));
    assert_eq!(token_data.expires_in, 3600);
    assert_eq!(token_data.token_type, "Bearer");

    assert_eq!(controller.oauth_refresh_count().await, 1);
    mock.shutdown().await;
}

#[tokio::test]
async fn test_f24_mock_programmable_failover_sequence() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    // Sequence: First request fails with 429, second request succeeds with 200
    controller.fail_first_n_with_429(1).await;

    let client = reqwest::Client::new();

    // Request 1: 429
    let r1 = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer slot_1_token")
        .send()
        .await
        .unwrap();
    assert_eq!(r1.status(), 429);

    // Request 2: 200 with swapped token
    let r2 = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer slot_2_rotated_token")
        .send()
        .await
        .unwrap();
    assert_eq!(r2.status(), 200);

    let observed_tokens = controller.received_bearer_tokens().await;
    assert_eq!(
        observed_tokens,
        vec!["slot_1_token", "slot_2_rotated_token"]
    );

    mock.shutdown().await;
}
