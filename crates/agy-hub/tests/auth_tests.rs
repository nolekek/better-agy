use agy_hub::auth::{Authenticator, MockAuthenticator};
use agy_hub::session::SessionManager;
use agy_hub::SystemdManager;

#[tokio::test]
async fn test_mock_authenticator_valid_credentials() {
    let auth = MockAuthenticator::new("agy-users");
    auth.add_user("alice", "secret123", 1001, 1001, vec!["agy-users".to_string()]);

    let user = auth.authenticate("alice", "secret123").await.expect("Authentication should succeed");
    assert_eq!(user.username, "alice");
    assert_eq!(user.uid, 1001);
    assert_eq!(user.gid, 1001);
    assert!(user.groups.contains(&"agy-users".to_string()));
}

#[tokio::test]
async fn test_mock_authenticator_invalid_credentials() {
    let auth = MockAuthenticator::new("agy-users");
    auth.add_user("alice", "secret123", 1001, 1001, vec!["agy-users".to_string()]);

    let err = auth.authenticate("alice", "wrongpass").await.unwrap_err();
    assert!(err.to_string().contains("Invalid username or password"));

    let err_nonexistent = auth.authenticate("unknown", "secret123").await.unwrap_err();
    assert!(err_nonexistent.to_string().contains("Invalid username or password"));
}

#[tokio::test]
async fn test_mock_authenticator_wrong_group_rejected() {
    let auth = MockAuthenticator::new("agy-users");
    // Bob is not in agy-users
    auth.add_user("bob", "secret123", 1002, 1002, vec!["other-group".to_string()]);

    let err = auth.authenticate("bob", "secret123").await.unwrap_err();
    assert!(err.to_string().contains("not a member of authorized group 'agy-users'"));
}

#[tokio::test]
async fn test_jwt_session_token_roundtrip() {
    let session_mgr = SessionManager::new("test-secret-key-32-bytes-long!!", 3600, true);
    let auth = MockAuthenticator::new("agy-users");
    auth.add_user("alice", "secret123", 1001, 1001, vec!["agy-users".to_string()]);
    let user = auth.authenticate("alice", "secret123").await.unwrap();

    let token = session_mgr.create_token(&user).expect("Token creation should succeed");
    assert!(!token.is_empty());

    let claims = session_mgr.verify_token(&token).expect("Token verification should succeed");
    assert_eq!(claims.sub, "alice");
    assert_eq!(claims.uid, 1001);
    assert_eq!(claims.gid, 1001);
    assert_eq!(claims.iss, "antigravity-hub");
    assert!(claims.exp > claims.iat);
}

#[tokio::test]
async fn test_jwt_session_tampering_rejected() {
    let session_mgr = SessionManager::new("test-secret-key-32-bytes-long!!", 3600, true);
    let auth = MockAuthenticator::new("agy-users");
    auth.add_user("alice", "secret123", 1001, 1001, vec!["agy-users".to_string()]);
    let user = auth.authenticate("alice", "secret123").await.unwrap();

    let token = session_mgr.create_token(&user).unwrap();
    // Tamper with the token
    let tampered = format!("{}tampered", token);

    let err = session_mgr.verify_token(&tampered).unwrap_err();
    assert!(err.to_string().contains("Invalid token"));
}

#[tokio::test]
async fn test_cookie_header_creation_and_extraction() {
    let session_mgr = SessionManager::new("test-secret-key-32-bytes-long!!", 86400, true);
    let cookie = session_mgr.build_cookie_header("jwt-token-sample-value");

    assert!(cookie.contains("agy_session=jwt-token-sample-value"));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("Max-Age=86400"));

    let extracted = session_mgr.extract_token_from_cookie(&cookie);
    assert_eq!(extracted.as_deref(), Some("jwt-token-sample-value"));

    let logout_cookie = session_mgr.build_logout_cookie_header();
    assert!(logout_cookie.contains("agy_session=deleted"));
    assert!(logout_cookie.contains("Max-Age=0"));
}

#[tokio::test]
async fn test_pam_authenticator_cross_platform_behavior() {
    let pam_auth = agy_hub::PamAuthenticator::new("antigravity-hub", "agy-users");

    #[cfg(not(target_os = "linux"))]
    {
        let res = pam_auth.authenticate("developer1", "secret").await;
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("PAM authentication requires Linux host"));
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux without real user developer1 configured in PAM, authentication should fail with AuthFailed
        let res = pam_auth.authenticate("nonexistent_test_user", "invalid_password").await;
        assert!(res.is_err());
    }
}

#[tokio::test]
async fn test_linux_systemd_manager_cross_platform_contract() {
    let mgr = agy_hub::LinuxSystemdManager::new("/run/user/{uid}/antigravity.sock");

    #[cfg(not(target_os = "linux"))]
    {
        let start_res = mgr.start_unit(1001, "antigravity-web.service").await;
        assert!(start_res.is_err());
        assert!(start_res.unwrap_err().to_string().contains("requires Linux systemd user manager"));

        let stop_res = mgr.stop_unit(1001, "antigravity-web.service").await;
        assert!(stop_res.is_err());

        let state = mgr.get_unit_state(1001, "antigravity-web.service").await.unwrap();
        assert_eq!(state.active_state, "inactive");
        assert_eq!(state.sub_state, "dead");
        assert_eq!(state.socket_path, "/run/user/1001/antigravity.sock");
        assert!(!state.socket_ready);
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, get_unit_state should query socket path and return valid ServiceState
        let state = mgr.get_unit_state(1001, "antigravity-web.service").await.unwrap();
        assert_eq!(state.socket_path, "/run/user/1001/antigravity.sock");
    }
}

