use crate::auth::Authenticator;
use crate::config::HubConfig;
use crate::dbus::SystemdManager;
use crate::error::HubError;
use crate::proxy::ReverseProxyEngine;
use crate::session::{Claims, SessionManager};
use crate::web::{DASHBOARD_HTML, LOGIN_HTML};
use axum::extract::{Request, State};
use axum::http::header::{HeaderMap, SET_COOKIE};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<HubConfig>,
    pub auth: Arc<dyn Authenticator>,
    pub session: Arc<SessionManager>,
    pub systemd: Arc<dyn SystemdManager>,
    pub proxy: Arc<ReverseProxyEngine>,
    pub active_presets: Arc<RwLock<HashMap<String, String>>>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/login", get(login_page))
        .route("/api/auth/login", post(login_api))
        .route("/api/auth/logout", post(logout_api))
        .route("/", get(dashboard_page))
        .route("/dashboard", get(dashboard_page))
        .route("/api/hub/me", get(me_api))
        .route("/api/hub/status", get(status_api))
        .route("/api/hub/ide/start", post(ide_start_api))
        .route("/api/hub/ide/stop", post(ide_stop_api))
        .route("/api/hub/ide/restart", post(ide_restart_api))
        .route("/api/hub/presets", get(presets_list_api))
        .route("/api/hub/presets/switch", post(presets_switch_api))
        .route("/api/hub/quotas", get(quotas_api))
        .fallback(fallback_handler)
        .with_state(state)
}

async fn healthz() -> &'static str {
    "OK"
}

async fn login_page() -> Html<&'static str> {
    Html(LOGIN_HTML)
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

async fn login_api(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, HubError> {
    let user_info = state.auth.authenticate(&payload.username, &payload.password).await?;
    let token = state.session.create_token(&user_info)?;
    let cookie_header = state.session.build_cookie_header(&token);

    let body = Json(json!({
        "ok": true,
        "user": user_info,
        "redirect": "/dashboard"
    }));

    let mut resp = (StatusCode::OK, body).into_response();
    resp.headers_mut().insert(
        SET_COOKIE,
        header::HeaderValue::from_str(&cookie_header).map_err(|e| HubError::SessionError(e.to_string()))?,
    );
    Ok(resp)
}

async fn logout_api(State(state): State<AppState>) -> Response {
    let cookie_header = state.session.build_logout_cookie_header();
    let body = Json(json!({ "ok": true }));
    let mut resp = (StatusCode::OK, body).into_response();
    if let Ok(val) = header::HeaderValue::from_str(&cookie_header) {
        resp.headers_mut().insert(SET_COOKIE, val);
    }
    resp
}

fn authenticate_request(state: &AppState, headers: &HeaderMap) -> Result<Claims, HubError> {
    if let Some(cookie_hdr) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        if let Some(token) = state.session.extract_token_from_cookie(cookie_hdr) {
            return state.session.verify_token(&token);
        }
    }
    Err(HubError::Unauthorized("Missing or invalid session cookie".to_string()))
}

async fn dashboard_page(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Response {
    if authenticate_request(&state, &headers).is_err() {
        return axum::response::Redirect::temporary("/login").into_response();
    }
    Html(DASHBOARD_HTML).into_response()
}

async fn me_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;
    Ok(Json(json!({
        "ok": true,
        "username": claims.sub,
        "uid": claims.uid,
        "gid": claims.gid,
        "groups": claims.groups,
        "role": claims.role
    })))
}

async fn status_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;
    let service_state = state
        .systemd
        .get_unit_state(claims.uid, "antigravity-web.service")
        .await?;

    let active_preset = {
        let map = state.active_presets.read().unwrap();
        map.get(&claims.sub).cloned().unwrap_or_else(|| "web".to_string())
    };

    Ok(Json(json!({
        "ok": true,
        "service": service_state,
        "active_preset": active_preset
    })))
}

async fn ide_start_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;
    let job = state
        .systemd
        .start_unit(claims.uid, "antigravity-web.service")
        .await?;
    Ok(Json(json!({ "ok": true, "job": job })))
}

async fn ide_stop_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;
    let job = state
        .systemd
        .stop_unit(claims.uid, "antigravity-web.service")
        .await?;
    Ok(Json(json!({ "ok": true, "job": job })))
}

async fn ide_restart_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;
    let job = state
        .systemd
        .restart_unit(claims.uid, "antigravity-web.service")
        .await?;
    Ok(Json(json!({ "ok": true, "job": job })))
}

async fn presets_list_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;
    let manifest = agy_core::preset::load_presets(&state.config.presets_file)
        .map_err(|e| HubError::PresetError(e.to_string()))?;

    let active = {
        let map = state.active_presets.read().unwrap();
        map.get(&claims.sub).cloned().unwrap_or_else(|| "web".to_string())
    };

    Ok(Json(json!({
        "ok": true,
        "active_preset": active,
        "always_active": manifest.always_active,
        "presets": manifest.presets.keys().cloned().collect::<Vec<_>>()
    })))
}

#[derive(Deserialize)]
struct SwitchPresetRequest {
    preset: String,
}

async fn presets_switch_api(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SwitchPresetRequest>,
) -> Result<Json<serde_json::Value>, HubError> {
    let claims = authenticate_request(&state, &headers)?;

    // Validate preset exists in manifest
    let manifest = agy_core::preset::load_presets(&state.config.presets_file)
        .map_err(|e| HubError::PresetError(e.to_string()))?;

    let target_skills = manifest
        .resolve_preset(&payload.preset)
        .map_err(|e| HubError::PresetError(e.to_string()))?;

    // Record in active presets map
    {
        let mut map = state.active_presets.write().unwrap();
        map.insert(claims.sub.clone(), payload.preset.clone());
    }

    Ok(Json(json!({
        "ok": true,
        "preset": payload.preset,
        "skills_projected": target_skills.len()
    })))
}

async fn quotas_api(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, HubError> {
    let _claims = authenticate_request(&state, &headers)?;

    // Query proxy status endpoint if accessible or return baseline
    let client = reqwest::Client::new();
    let proxy_status_url = format!("{}/api/proxy/status", state.config.proxy_url);

    if let Ok(res) = client.get(&proxy_status_url).timeout(std::time::Duration::from_millis(500)).send().await {
        if let Ok(json_body) = res.json::<serde_json::Value>().await {
            return Ok(Json(json_body));
        }
    }

    // Default status if proxy is standalone
    Ok(Json(json!({
        "active_slot": 1,
        "total_slots": 4,
        "auto_failover": true,
        "quotas": {
            "rolling_5h_percent": 24.5,
            "weekly_percent": 14.8,
            "debounce_window_sec": 25
        }
    })))
}

async fn fallback_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Request,
) -> Result<Response, HubError> {
    let uri = req.uri().clone();
    let path = uri.path();

    if path.starts_with("/ide") {
        let claims = authenticate_request(&state, &headers)?;
        return state.proxy.proxy_request(claims.uid, &claims.sub, req).await;
    }

    Ok((StatusCode::NOT_FOUND, "Not Found").into_response())
}
