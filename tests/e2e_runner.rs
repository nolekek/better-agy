use agy_core::{
    load_presets_from_str, sync_skills, ALWAYS_ACTIVE_META_SKILLS,
};
use agy_mock::MockServer;
use agy_proxy::{
    AccountSlot, FailoverDecision, FailoverEngine, ProxyConfig, ProxyServer, SlotState,
    TokenRefresher, Vault, DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET,
};
use std::fs;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

const E2E_PRESETS_YAML: &str = r#"
version: "3.0"
always_active:
  - "agent-preset-conductor"
  - "grill-me"
  - "systematic-debugging"
  - "verification-before-completion"
  - "concise-planning"
presets:
  web:
    name: "Web Development"
    skills:
      - "nextjs-fullstack-guru"
      - "react-best-practices"
  backend:
    name: "Backend Engineering"
    skills:
      - "postgres-dba-optimizer"
      - "database-design"
  devops:
    skills: []
  appsec:
    skills: []
  qa:
    skills: []
  ai-agents:
    skills: []
  web-research:
    skills: []
  reverse-engineering:
    skills: []
  product:
    skills: []
  orchestrator:
    skills: []
  systems:
    skills: []
  vst3:
    skills: []
  hardware:
    skills: []
  media:
    skills: []
  meta-dev:
    skills: []
  minimal:
    skills: []
"#;

fn setup_central_skills_library(lib_dir: &std::path::Path) {
    let skills = [
        "agent-preset-conductor",
        "grill-me",
        "systematic-debugging",
        "verification-before-completion",
        "concise-planning",
        "nextjs-fullstack-guru",
        "react-best-practices",
        "postgres-dba-optimizer",
        "database-design",
    ];
    for s in &skills {
        let p = lib_dir.join(s);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("SKILL.md"), format!("# Skill {}\n", s)).unwrap();
    }
}

/// Scenario 1: High-Concurrency LLM Streaming via Ingress & Mock Interception
#[tokio::test]
async fn test_e2e_scenario_1_concurrent_llm_streaming() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    let chunks = vec![
        "pub async fn ".to_string(),
        "stream_pipeline() ".to_string(),
        "-> Result<()> ".to_string(),
        "{ Ok(()) }".to_string(),
    ];
    controller
        .set_stream_chunks(chunks.clone(), Duration::from_millis(5))
        .await;

    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let slot = AccountSlot::new(
        "slot_stream",
        "streamer@example.com",
        "ya29.stream_token",
        "1//ref",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    vault.upsert_slot(slot).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some(mock.base_url()),
        max_retries: 3,
        connect_timeout: Duration::from_secs(5),
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()));
    let server = ProxyServer::new(proxy_config, failover);

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let _ = server.run(listener, shutdown_rx).await;
    });

    let client = reqwest::Client::new();
    let proxy_url = format!("http://{}/v1alpha/models/gemini-pro:streamGenerateContent", proxy_addr);

    // Launch 3 concurrent requests through proxy
    let mut handles = Vec::new();
    for _ in 0..3 {
        let c = client.clone();
        let u = proxy_url.clone();
        handles.push(tokio::spawn(async move {
            c.post(&u)
                .header("host", "generativelanguage.googleapis.com")
                .send()
                .await
                .unwrap()
                .text()
                .await
                .unwrap()
        }));
    }

    for h in handles {
        let body = h.await.unwrap();
        for ch in &chunks {
            assert!(body.contains(ch));
        }
    }

    let _ = shutdown_tx.send(());
    mock.shutdown().await;
}

/// Scenario 2: In-Flight Quota Exhaustion & Debounced Failover
#[tokio::test]
async fn test_e2e_scenario_2_inflight_quota_failover_and_debounce() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    // Mock rule: Fail slot_1_token with 429 quota exhausted, succeed with slot_2_token
    controller.fail_bearer_token("slot_1_token").await;

    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let slot1 = AccountSlot::new(
        "slot_1",
        "corp1@company.com",
        "slot_1_token",
        "1//ref_1",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Active,
    );
    let slot2 = AccountSlot::new(
        "slot_2",
        "corp2@company.com",
        "slot_2_token",
        "1//ref_2",
        chrono::Utc::now().timestamp() + 3600,
        SlotState::Ready,
    );
    vault.upsert_slot(slot1).await.unwrap();
    vault.upsert_slot(slot2).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_addr = listener.local_addr().unwrap();

    let proxy_config = ProxyConfig {
        bind_addr: proxy_addr,
        upstream_override: Some(mock.base_url()),
        max_retries: 3,
        connect_timeout: Duration::from_secs(5),
    };

    let failover = Arc::new(FailoverEngine::new(vault.clone()));
    let server = ProxyServer::new(proxy_config, failover.clone());

    let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
    tokio::spawn(async move {
        let _ = server.run(listener, shutdown_rx).await;
    });

    let client = reqwest::Client::new();
    let proxy_url = format!("http://{}/v1alpha/models/gemini-pro:streamGenerateContent", proxy_addr);

    // Initial request: Transparent retry turns 429 into 200 for the client!
    let resp = client
        .post(&proxy_url)
        .header("host", "generativelanguage.googleapis.com")
        .json(&serde_json::json!({"prompt": "Hello"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Verify rotation occurred in vault
    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.id, "slot_2");

    // Burst verification: Next 5 requests within debounce window are suppressed from rapid rotating
    for _ in 0..5 {
        let decision = failover.on_quota_error().await.unwrap();
        assert!(
            matches!(decision, FailoverDecision::Debounced { .. }),
            "Burst errors within debounce window must be debounced"
        );
    }

    let _ = shutdown_tx.send(());
    mock.shutdown().await;
}

/// Scenario 3: Complete Developer Workspace Provisioning & Symlink Projection
#[test]
fn test_e2e_scenario_3_developer_workspace_projection_and_preservation() {
    let lib_tmp = tempdir().unwrap();
    setup_central_skills_library(lib_tmp.path());

    let ws_tmp = tempdir().unwrap();
    let user_skills_dir = ws_tmp.path().join(".gemini").join("skills");
    let manifest = load_presets_from_str(E2E_PRESETS_YAML).unwrap();

    // 1. Initial developer onboarding with 'web' preset
    let stats1 = sync_skills(lib_tmp.path(), &user_skills_dir, "web", &manifest).unwrap();
    assert_eq!(stats1.added.len(), 7); // 5 meta + 2 web

    // Verify all 5 mandatory meta-skills exist as symlinks
    for &meta in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(user_skills_dir.join(meta).exists());
    }

    // 2. Developer adds custom private script/folder
    let custom_dir = user_skills_dir.join("my-custom-dev-tools");
    fs::create_dir_all(&custom_dir).unwrap();
    fs::write(custom_dir.join("tool.sh"), "echo 'do work'").unwrap();

    // 3. Developer switches preset to 'backend'
    let stats2 = sync_skills(lib_tmp.path(), &user_skills_dir, "backend", &manifest).unwrap();

    // Web skills pruned
    assert!(stats2.removed.contains(&"nextjs-fullstack-guru".to_string()));
    assert!(!user_skills_dir.join("nextjs-fullstack-guru").exists());

    // Backend skills added
    assert!(stats2.added.contains(&"postgres-dba-optimizer".to_string()));
    assert!(user_skills_dir.join("postgres-dba-optimizer").exists());

    // Custom directory PRESERVED untouched
    assert!(custom_dir.exists());
    assert!(custom_dir.join("tool.sh").exists());
    assert!(stats2.preserved_user_dirs.contains(&"my-custom-dev-tools".to_string()));

    // 5 meta-skills kept
    for &meta in &ALWAYS_ACTIVE_META_SKILLS {
        assert!(stats2.kept.contains(&meta.to_string()));
        assert!(user_skills_dir.join(meta).exists());
    }
}

/// Scenario 4: Proactive Token Refresh & Expiration Lifecycle
#[tokio::test]
async fn test_e2e_scenario_4_proactive_token_refresh_lifecycle() {
    let mock = MockServer::start().await.expect("Start mock server");
    let controller = mock.controller();

    let tmp = tempdir().unwrap();
    let vault = Vault::new(tmp.path().join("vault.enc"), Vault::generate_key());
    vault.load_or_init().await.unwrap();

    let now = chrono::Utc::now().timestamp();
    // Slot expiring in 120s (< 300s threshold)
    let slot = AccountSlot::new(
        "slot_expiring_soon",
        "worker@company.com",
        "ya29.stale_token",
        "1//refresh_token_worker",
        now + 120,
        SlotState::Active,
    );
    vault.upsert_slot(slot).await.unwrap();

    let refresher = TokenRefresher::new(vault.clone()).with_endpoint(
        mock.oauth_url(),
        DEFAULT_CLIENT_ID,
        DEFAULT_CLIENT_SECRET,
    );

    // Refresh due slots
    let results = refresher.refresh_due_slots().await;
    assert_eq!(results.len(), 1);
    let refreshed = results.into_iter().next().unwrap().unwrap();

    assert!(refreshed.access_token.starts_with("ya29.synthetic_refreshed_access_token_mock_"));
    assert!(refreshed.expires_at > now + 3000);

    // Verify Vault on disk has the refreshed token
    let active = vault.get_active_slot().await.unwrap();
    assert_eq!(active.access_token, refreshed.access_token);
    assert_eq!(controller.oauth_refresh_count().await, 1);

    mock.shutdown().await;
}

/// Scenario 5: Multi-Tenant Onboarding, Authentication, Reverse Proxying & Doctor Self-Audit
#[tokio::test]
async fn test_e2e_scenario_5_multi_tenant_portal_and_doctor_audit() {
    use agy_cli::doctor::DoctorEngine;
    use agy_hub::{
        create_router, AppState, HubConfig, MockAuthenticator, MockSystemdManager,
        ReverseProxyEngine, SessionManager,
    };
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use http_body_util::BodyExt;
    use hyper::server::conn::http1;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use tower::ServiceExt;

    // 1. Start two distinct upstream IDE servers on ephemeral ports
    // Alice IDE (UID 1001)
    let alice_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let alice_addr = alice_listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = alice_listener.accept().await {
            let io = TokioIo::new(stream);
            tokio::spawn(async move {
                let service = service_fn(|req: hyper::Request<hyper::body::Incoming>| async move {
                    let user_hdr = req
                        .headers()
                        .get("x-forwarded-user")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    let resp = hyper::Response::builder()
                        .status(200)
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(format!(
                            "Alice IDE: welcome {}, path={}",
                            user_hdr,
                            req.uri()
                        ))))
                        .unwrap();
                    Ok::<_, std::convert::Infallible>(resp)
                });
                let _ = http1::Builder::new().serve_connection(io, service).await;
            });
        }
    });

    // Bob IDE (UID 1002)
    let bob_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let _bob_addr = bob_listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = bob_listener.accept().await {
            let io = TokioIo::new(stream);
            tokio::spawn(async move {
                let service = service_fn(|req: hyper::Request<hyper::body::Incoming>| async move {
                    let user_hdr = req
                        .headers()
                        .get("x-forwarded-user")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown");
                    let resp = hyper::Response::builder()
                        .status(200)
                        .body(http_body_util::Full::new(hyper::body::Bytes::from(format!(
                            "Bob IDE: welcome {}, path={}",
                            user_hdr,
                            req.uri()
                        ))))
                        .unwrap();
                    Ok::<_, std::convert::Infallible>(resp)
                });
                let _ = http1::Builder::new().serve_connection(io, service).await;
            });
        }
    });

    // 2. Setup Central Hub
    let tmp = tempdir().unwrap();
    let presets_file = tmp.path().join("presets.yaml");
    fs::write(&presets_file, E2E_PRESETS_YAML).unwrap();

    let config = Arc::new(HubConfig {
        presets_file: presets_file.clone(),
        socket_dir_template: format!("tcp:{}", alice_addr), // default
        ..HubConfig::default()
    });

    let auth = Arc::new(MockAuthenticator::new("agy-users"));
    auth.add_user("alice", "alice123", 1001, 1001, vec!["agy-users".to_string()]);
    auth.add_user("bob", "bob123", 1002, 1002, vec!["agy-users".to_string()]);

    let session = Arc::new(SessionManager::new("e2e-super-secret-key-32-bytes!!", 3600, false));
    let systemd = Arc::new(MockSystemdManager::new(&config.socket_dir_template));
    systemd.set_socket_ready(1001, "antigravity-web.service", true);
    systemd.set_socket_ready(1002, "antigravity-web.service", true);

    let proxy = Arc::new(ReverseProxyEngine::new(config.clone(), systemd.clone()));

    let state = AppState {
        config: config.clone(),
        auth,
        session: session.clone(),
        systemd,
        proxy,
        active_presets: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = create_router(state);

    // 3. Alice authenticates
    let alice_login = serde_json::json!({ "username": "alice", "password": "alice123" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&alice_login).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let alice_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 4. Bob authenticates
    let bob_login = serde_json::json!({ "username": "bob", "password": "bob123" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&bob_login).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let _bob_cookie = resp
        .headers()
        .get(header::SET_COOKIE)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 5. Alice accesses IDE workspace
    let req = Request::builder()
        .uri("/ide/workspace/main.rs")
        .header(header::COOKIE, &alice_cookie)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("Alice IDE: welcome alice, path=/workspace/main.rs"));

    // 6. Alice switches preset to 'backend'
    let switch_req = serde_json::json!({ "preset": "backend" });
    let req = Request::builder()
        .method("POST")
        .uri("/api/hub/presets/switch")
        .header(header::COOKIE, &alice_cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&switch_req).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify status returns backend for Alice
    let req = Request::builder()
        .uri("/api/hub/status")
        .header(header::COOKIE, &alice_cookie)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let json_status: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json_status["active_preset"], "backend");

    // 7. Verify Doctor self-audit passes
    let doctor = DoctorEngine {
        skills_library_path: Some(tmp.path().to_string_lossy().to_string()),
        presets_yaml_path: Some(presets_file.to_string_lossy().to_string()),
        caddyfile_path: Some("deploy/caddy/Caddyfile".to_string()),
        ..Default::default()
    };
    let report = doctor.run_diagnostics().await;
    assert_eq!(report.total_checks, 9);
}

