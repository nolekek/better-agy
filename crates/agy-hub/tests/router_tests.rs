use agy_hub::{
    create_router, AppState, HubConfig, MockAuthenticator, MockSystemdManager, ReverseProxyEngine,
    SessionManager,
};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tower::ServiceExt;

fn build_test_app() -> (axum::Router, Arc<SessionManager>, String) {
    let mut config = HubConfig::default();
    let candidates = [
        PathBuf::from("../../data/skills-library/presets.yaml"),
        PathBuf::from("data/skills-library/presets.yaml"),
        PathBuf::from("../data/skills-library/presets.yaml"),
    ];
    for c in &candidates {
        if c.exists() {
            config.presets_file = c.clone();
            break;
        }
    }
    let config = Arc::new(config);

    let auth = Arc::new(MockAuthenticator::new("agy-users"));
    auth.add_user("alice", "pass123", 1001, 1001, vec!["agy-users".to_string()]);

    let session = Arc::new(SessionManager::new("secret-key-for-test-purposes-123", 3600, false));
    let systemd = Arc::new(MockSystemdManager::new(&config.socket_dir_template));
    let proxy = Arc::new(ReverseProxyEngine::new(config.clone(), systemd.clone()));

    let user_info = agy_hub::auth::UserInfo {
        username: "alice".to_string(),
        uid: 1001,
        gid: 1001,
        groups: vec!["agy-users".to_string()],
        role: "developer".to_string(),
    };
    let token = session.create_token(&user_info).unwrap();
    let cookie_val = format!("agy_session={}", token);

    let state = AppState {
        config,
        auth,
        session: session.clone(),
        systemd,
        proxy,
        active_presets: Arc::new(RwLock::new(HashMap::new())),
    };

    let router = create_router(state);
    (router, session, cookie_val)
}

#[tokio::test]
async fn test_healthz_endpoint() {
    let (app, _, _) = build_test_app();
    let req = Request::builder()
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"OK");
}

#[tokio::test]
async fn test_login_flow() {
    let (app, _, _) = build_test_app();
    let login_payload = serde_json::json!({
        "username": "alice",
        "password": "pass123"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&login_payload).unwrap()))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify Set-Cookie header exists
    let set_cookie = resp.headers().get(header::SET_COOKIE).expect("Set-Cookie header must be present");
    let cookie_str = set_cookie.to_str().unwrap();
    assert!(cookie_str.contains("agy_session="));

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["ok"], true);
    assert_eq!(json_resp["user"]["username"], "alice");
    assert_eq!(json_resp["redirect"], "/dashboard");
}

#[tokio::test]
async fn test_protected_endpoints_reject_unauthenticated() {
    let (app, _, _) = build_test_app();
    let req = Request::builder()
        .uri("/api/hub/me")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_protected_endpoints_succeed_with_cookie() {
    let (app, _, cookie_val) = build_test_app();
    let req = Request::builder()
        .uri("/api/hub/me")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["ok"], true);
    assert_eq!(json_resp["username"], "alice");
    assert_eq!(json_resp["uid"], 1001);
}

#[tokio::test]
async fn test_systemd_lifecycle_api() {
    let (app, _, cookie_val) = build_test_app();

    // 1. Initial status -> inactive
    let req = Request::builder()
        .uri("/api/hub/status")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["service"]["active_state"], "inactive");

    // 2. Start unit
    let req = Request::builder()
        .method("POST")
        .uri("/api/hub/ide/start")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Status -> active
    let req = Request::builder()
        .uri("/api/hub/status")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["service"]["active_state"], "active");

    // 4. Stop unit
    let req = Request::builder()
        .method("POST")
        .uri("/api/hub/ide/stop")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_presets_api() {
    let (app, _, cookie_val) = build_test_app();

    // 1. List presets
    let req = Request::builder()
        .uri("/api/hub/presets")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["ok"], true);
    assert_eq!(json_resp["presets"].as_array().unwrap().len(), 16);
    assert_eq!(json_resp["always_active"].as_array().unwrap().len(), 5);

    // 2. Switch preset
    let switch_payload = serde_json::json!({
        "preset": "backend"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/api/hub/presets/switch")
        .header(header::COOKIE, &cookie_val)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&switch_payload).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. Verify status shows backend
    let req = Request::builder()
        .uri("/api/hub/status")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_resp: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_resp["active_preset"], "backend");
}

#[tokio::test]
async fn test_logout_flow() {
    let (app, _, cookie_val) = build_test_app();
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/logout")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let set_cookie = resp.headers().get(header::SET_COOKIE).expect("Set-Cookie must be present on logout");
    let cookie_str = set_cookie.to_str().unwrap();
    assert!(cookie_str.contains("Max-Age=0"));
}
