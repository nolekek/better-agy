use agy_hub::{
    create_router, AppState, HubConfig, MockAuthenticator, MockSystemdManager, ReverseProxyEngine,
    SessionManager,
};
use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use tower::ServiceExt;

#[derive(Clone, Default)]
struct CapturedRequest {
    path: String,
    headers: HashMap<String, String>,
}

#[tokio::test]
async fn test_reverse_proxy_http_forwarding_and_sanitization() {
    let captured = Arc::new(Mutex::new(CapturedRequest::default()));
    let captured_clone = captured.clone();

    // 1. Start upstream mock server on ephemeral port
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let io = TokioIo::new(stream);
                let cap = captured_clone.clone();
                tokio::spawn(async move {
                    let service = service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                        let cap = cap.clone();
                        async move {
                            let mut c = cap.lock().unwrap();
                            c.path = req.uri().to_string();
                            for (name, val) in req.headers() {
                                if let Ok(v) = val.to_str() {
                                    c.headers.insert(name.as_str().to_lowercase(), v.to_string());
                                }
                            }
                            let resp = hyper::Response::builder()
                                .status(200)
                                .body(http_body_util::Full::new(hyper::body::Bytes::from("hello from upstream ide")))
                                .unwrap();
                            Ok::<_, std::convert::Infallible>(resp)
                        }
                    });
                    let _ = http1::Builder::new().serve_connection(io, service).await;
                });
            }
        }
    });

    // 2. Configure agy-hub with socket template pointing to the upstream TCP address
    let config = Arc::new(HubConfig {
        socket_dir_template: format!("tcp:{}", upstream_addr),
        ..Default::default()
    });

    let auth = Arc::new(MockAuthenticator::new("agy-users"));
    auth.add_user("alice", "pass123", 1001, 1001, vec!["agy-users".to_string()]);

    let session = Arc::new(SessionManager::new("secret-key-for-test-purposes-123", 3600, false));
    let systemd = Arc::new(MockSystemdManager::new(&config.socket_dir_template));
    systemd.set_socket_ready(1001, "antigravity-web.service", true);

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
        session,
        systemd,
        proxy,
        active_presets: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = create_router(state);

    // 3. Send request through /ide/workspace/file.rs?line=42
    let req = Request::builder()
        .uri("/ide/workspace/file.rs?line=42")
        .header(header::COOKIE, &cookie_val)
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"hello from upstream ide");

    // 4. Inspect captured upstream request
    let cap = captured.lock().unwrap();
    // Path rewriting: /ide was stripped
    assert_eq!(cap.path, "/workspace/file.rs?line=42");
    // Headers injected
    assert_eq!(cap.headers.get("x-forwarded-user").map(|s| s.as_str()), Some("alice"));
    assert_eq!(cap.headers.get("x-forwarded-proto").map(|s| s.as_str()), Some("https"));
    assert_eq!(cap.headers.get("x-forwarded-prefix").map(|s| s.as_str()), Some("/ide"));
    // Cookie sanitized: agy_session not forwarded upstream
    assert_eq!(cap.headers.get("cookie"), None);
}

#[tokio::test]
async fn test_reverse_proxy_websocket_handshake() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_addr = listener.local_addr().unwrap();

    let config = Arc::new(HubConfig {
        socket_dir_template: format!("tcp:{}", upstream_addr),
        ..Default::default()
    });

    let auth = Arc::new(MockAuthenticator::new("agy-users"));
    auth.add_user("alice", "pass123", 1001, 1001, vec!["agy-users".to_string()]);

    let session = Arc::new(SessionManager::new("secret-key-for-test-purposes-123", 3600, false));
    let systemd = Arc::new(MockSystemdManager::new(&config.socket_dir_template));
    systemd.set_socket_ready(1001, "antigravity-web.service", true);

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
        session,
        systemd,
        proxy,
        active_presets: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = create_router(state);

    let req = Request::builder()
        .uri("/ide/terminal/ws")
        .header(header::COOKIE, &cookie_val)
        .header(header::CONNECTION, "Upgrade")
        .header(header::UPGRADE, "websocket")
        .header(header::SEC_WEBSOCKET_VERSION, "13")
        .header(header::SEC_WEBSOCKET_KEY, "dGhlIHNhbXBsZSBub25jZQ==")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(resp.headers().get(header::UPGRADE).unwrap(), "websocket");
    assert!(resp.headers().get("sec-websocket-accept").is_some());
}
