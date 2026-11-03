use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HubError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("D-Bus systemd error: {0}")]
    DbusError(String),

    #[error("Service timeout: socket not ready after {0}s")]
    ServiceTimeout(u64),

    #[error("Upstream socket error: {0}")]
    UpstreamError(String),

    #[error("Preset error: {0}")]
    PresetError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Internal I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl IntoResponse for HubError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            HubError::AuthFailed(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            HubError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            HubError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg.clone()),
            HubError::SessionError(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            HubError::DbusError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            HubError::ServiceTimeout(secs) => (
                StatusCode::GATEWAY_TIMEOUT,
                format!("Web IDE socket failed to become ready within {}s", secs),
            ),
            HubError::UpstreamError(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            HubError::PresetError(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            HubError::ConfigError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            HubError::IoError(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            HubError::JsonError(err) => (StatusCode::BAD_REQUEST, err.to_string()),
        };

        let body = Json(json!({
            "ok": false,
            "error": error_message
        }));

        (status, body).into_response()
    }
}
