use agy_mock::{
    GoogleApiErrorResponse, MockResponse, MockServer, OAuthErrorResponse, OAuthTokenResponse,
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::time::Duration;

#[tokio::test]
async fn test_mock_server_lifecycle_and_healthz() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    assert!(mock.port() > 0);

    let client = reqwest::Client::new();
    let resp = client
        .get(mock.url("healthz"))
        .send()
        .await
        .expect("Failed to query healthz");

    assert_eq!(resp.status(), 200);
    let text = resp.text().await.unwrap();
    assert_eq!(text, "OK");

    mock.shutdown().await;
}

#[tokio::test]
async fn test_oauth_token_refresh_success() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let client = reqwest::Client::new();
    let params = [
        ("client_id", "test-client-id"),
        ("client_secret", "test-client-secret"),
        ("grant_type", "refresh_token"),
        ("refresh_token", "test-refresh-token-12345"),
    ];

    let resp = client
        .post(mock.oauth_url())
        .form(&params)
        .send()
        .await
        .expect("Failed to send token request");

    assert_eq!(resp.status(), 200);
    let oauth_resp: OAuthTokenResponse = resp.json().await.expect("Failed to parse token response");
    assert!(oauth_resp.access_token.starts_with("ya29.synthetic_refreshed_access_token_mock_"));
    assert_eq!(oauth_resp.expires_in, 3600);
    assert_eq!(oauth_resp.token_type, "Bearer");

    assert_eq!(controller.oauth_refresh_count().await, 1);
    let requests = controller.recorded_requests().await;
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert!(requests[0].path.contains("token"));

    mock.shutdown().await;
}

#[tokio::test]
async fn test_oauth_token_refresh_invalid_grant() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();
    controller.set_oauth_invalid_grant().await;

    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", "revoked-token-xyz"),
    ];

    let resp = client
        .post(mock.oauth_url())
        .form(&params)
        .send()
        .await
        .expect("Failed to send token request");

    assert_eq!(resp.status(), 400);
    let err_resp: OAuthErrorResponse = resp.json().await.expect("Failed to parse error response");
    assert_eq!(err_resp.error, "invalid_grant");
    assert_eq!(err_resp.error_description, "Token has been expired or revoked.");

    mock.shutdown().await;
}

#[tokio::test]
async fn test_standard_generation_200() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let client = reqwest::Client::new();
    let resp = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer slot_1_token")
        .json(&serde_json::json!({
            "contents": [{"parts": [{"text": "Hello"}]}]
        }))
        .send()
        .await
        .expect("Failed to send generate request");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.expect("Valid JSON");
    assert_eq!(
        body["candidates"][0]["content"]["parts"][0]["text"],
        "Synthetic Antigravity Generation Success"
    );

    let tokens = controller.received_bearer_tokens().await;
    assert_eq!(tokens, vec!["slot_1_token"]);

    mock.shutdown().await;
}

#[tokio::test]
async fn test_streaming_chunked_response() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let chunks = vec![
        "fn ".to_string(),
        "hello() ".to_string(),
        "-> bool ".to_string(),
        "{ true }".to_string(),
    ];
    controller
        .set_stream_chunks(chunks.clone(), Duration::from_millis(5))
        .await;

    let client = reqwest::Client::new();
    let resp = client
        .post(mock.generate_url())
        .send()
        .await
        .expect("Failed to send generate request");

    assert_eq!(resp.status(), 200);
    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(content_type.contains("application/json"));

    let full_text = resp.text().await.expect("Read stream text");
    for chunk in &chunks {
        assert!(full_text.contains(chunk));
    }

    mock.shutdown().await;
}

#[tokio::test]
async fn test_fail_first_n_with_429_quota() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    // Fail first 2 requests, 3rd request should succeed
    controller.fail_first_n_with_429(2).await;

    let client = reqwest::Client::new();

    // 1st request -> 429
    let resp1 = client.post(mock.generate_url()).send().await.unwrap();
    assert_eq!(resp1.status(), 429);
    assert_eq!(
        resp1.headers().get("retry-after").unwrap().to_str().unwrap(),
        "25"
    );
    let err1: GoogleApiErrorResponse = resp1.json().await.unwrap();
    assert_eq!(err1.error.code, 429);
    assert_eq!(err1.error.status, "RESOURCE_EXHAUSTED");
    assert!(err1.error.message.contains("Quota exceeded for quota metric"));

    // 2nd request -> 429
    let resp2 = client.post(mock.generate_url()).send().await.unwrap();
    assert_eq!(resp2.status(), 429);

    // 3rd request -> 200 OK
    let resp3 = client.post(mock.generate_url()).send().await.unwrap();
    assert_eq!(resp3.status(), 200);

    assert_eq!(controller.generate_count().await, 3);
    mock.shutdown().await;
}

#[tokio::test]
async fn test_fail_bearer_token_conditional() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    controller.fail_bearer_token("exhausted_token").await;

    let client = reqwest::Client::new();

    // Request with bad token -> 429
    let resp_bad = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer exhausted_token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp_bad.status(), 429);

    // Request with rotated valid token -> 200
    let resp_good = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer healthy_slot_token")
        .send()
        .await
        .unwrap();
    assert_eq!(resp_good.status(), 200);

    let tokens = controller.received_bearer_tokens().await;
    assert_eq!(tokens, vec!["exhausted_token", "healthy_slot_token"]);

    mock.shutdown().await;
}

#[tokio::test]
async fn test_grpc_resource_exhausted() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();
    controller.set_grpc_resource_exhausted().await;

    let client = reqwest::Client::new();
    let resp = client.post(mock.generate_url()).send().await.unwrap();

    assert_eq!(
        resp.headers().get("grpc-status").unwrap().to_str().unwrap(),
        "8"
    );
    assert!(resp
        .headers()
        .get("grpc-message")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("Resource has been exhausted"));

    let body = resp.text().await.unwrap();
    assert!(body.contains("ResourceExhausted"));

    mock.shutdown().await;
}

#[tokio::test]
async fn test_custom_response_sequence() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let seq = vec![
        MockResponse::too_many_requests_quota("test.googleapis.com", 30),
        MockResponse::grpc_resource_exhausted(),
        MockResponse::ok_json(serde_json::json!({"custom_key": "custom_val"})),
    ];
    controller.set_custom_sequence(seq).await;

    let client = reqwest::Client::new();

    // 1st in sequence: 429
    let r1 = client.post(mock.generate_url()).send().await.unwrap();
    assert_eq!(r1.status(), 429);

    // 2nd in sequence: gRPC ResourceExhausted
    let r2 = client.post(mock.generate_url()).send().await.unwrap();
    assert_eq!(r2.headers().get("grpc-status").unwrap().to_str().unwrap(), "8");

    // 3rd in sequence: custom JSON 200
    let r3 = client.post(mock.generate_url()).send().await.unwrap();
    assert_eq!(r3.status(), 200);
    let body: serde_json::Value = r3.json().await.unwrap();
    assert_eq!(body["custom_key"], "custom_val");

    mock.shutdown().await;
}

#[tokio::test]
async fn test_inspection_and_reset_endpoints() {
    let mock = MockServer::start().await.expect("Failed to start mock server");
    let controller = mock.controller();

    let client = reqwest::Client::new();
    let _ = client
        .post(mock.generate_url())
        .header(AUTHORIZATION, "Bearer token_inspect_1")
        .send()
        .await
        .unwrap();

    // Query status endpoint
    let status_resp: serde_json::Value = client
        .get(mock.url("_mock/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(status_resp["request_count"], 1);
    assert_eq!(status_resp["generate_count"], 1);

    // Query requests endpoint
    let reqs_resp: serde_json::Value = client
        .get(mock.url("_mock/requests"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(reqs_resp.as_array().unwrap().len(), 1); // 1 recorded generate request

    // Call reset
    let reset_resp = client
        .post(mock.url("_mock/reset"))
        .send()
        .await
        .unwrap();
    assert_eq!(reset_resp.status(), 200);

    assert_eq!(controller.request_count().await, 0);
    assert_eq!(controller.recorded_requests().await.len(), 0);

    mock.shutdown().await;
}
