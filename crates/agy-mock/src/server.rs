use crate::types::*;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use bytes::Bytes;
use futures_util::stream;
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{oneshot, RwLock};

#[derive(Debug, Clone)]
pub struct MockInner {
    pub behavior: MockBehavior,
    pub request_count: usize,
    pub oauth_refresh_count: usize,
    pub generate_count: usize,
    pub recorded_requests: Vec<RecordedRequest>,
    pub default_service_name: String,
    pub default_stream_chunks: Vec<String>,
    pub default_chunk_delay: Duration,
}

impl Default for MockInner {
    fn default() -> Self {
        Self {
            behavior: MockBehavior::Default,
            request_count: 0,
            oauth_refresh_count: 0,
            generate_count: 0,
            recorded_requests: Vec::new(),
            default_service_name: "generativelanguage.googleapis.com".to_string(),
            default_stream_chunks: vec![
                "def ".to_string(),
                "antigravity_optimize():\n".to_string(),
                "    return True\n".to_string(),
            ],
            default_chunk_delay: Duration::from_millis(5),
        }
    }
}

/// Thread-safe controller handle to configure and inspect the mock server at runtime.
#[derive(Debug, Clone)]
pub struct MockController {
    inner: Arc<RwLock<MockInner>>,
}

impl MockController {
    pub fn new(inner: Arc<RwLock<MockInner>>) -> Self {
        Self { inner }
    }

    /// Sets the general programmable behavior.
    pub async fn set_behavior(&self, behavior: MockBehavior) {
        let mut w = self.inner.write().await;
        w.behavior = behavior;
    }

    /// Configures the mock to fail the first `count` requests with HTTP 429 Quota Exhaustion.
    pub async fn fail_first_n_with_429(&self, count: usize) {
        self.set_behavior(MockBehavior::FailFirstNWith429 { count }).await;
    }

    /// Configures the mock to fail when the incoming Bearer token matches `bad_token`.
    pub async fn fail_bearer_token(&self, bad_token: impl Into<String>) {
        self.set_behavior(MockBehavior::FailBearerToken {
            bad_token: bad_token.into(),
        })
        .await;
    }

    /// Configures the mock to simulate gRPC ResourceExhausted status.
    pub async fn set_grpc_resource_exhausted(&self) {
        self.set_behavior(MockBehavior::GrpcResourceExhausted).await;
    }

    /// Configures the mock to return `invalid_grant` on OAuth token refresh.
    pub async fn set_oauth_invalid_grant(&self) {
        self.set_behavior(MockBehavior::OAuthInvalidGrant).await;
    }

    /// Configures token chunks and delay for LLM streaming emulation.
    pub async fn set_stream_chunks(&self, chunks: Vec<String>, chunk_delay: Duration) {
        self.set_behavior(MockBehavior::StreamChunks {
            chunks,
            chunk_delay,
        })
        .await;
    }

    /// Pushes a custom sequence of programmed responses.
    pub async fn set_custom_sequence(&self, responses: Vec<MockResponse>) {
        self.set_behavior(MockBehavior::CustomSequence { responses }).await;
    }

    /// Returns a snapshot of all recorded requests received so far.
    pub async fn recorded_requests(&self) -> Vec<RecordedRequest> {
        self.inner.read().await.recorded_requests.clone()
    }

    /// Returns all Bearer tokens observed in incoming requests in order.
    pub async fn received_bearer_tokens(&self) -> Vec<String> {
        let r = self.inner.read().await;
        r.recorded_requests
            .iter()
            .filter_map(|req| req.bearer_token.clone())
            .collect()
    }

    /// Returns total count of all incoming requests.
    pub async fn request_count(&self) -> usize {
        self.inner.read().await.request_count
    }

    /// Returns count of OAuth token refresh requests.
    pub async fn oauth_refresh_count(&self) -> usize {
        self.inner.read().await.oauth_refresh_count
    }

    /// Returns count of content generation requests.
    pub async fn generate_count(&self) -> usize {
        self.inner.read().await.generate_count
    }

    /// Resets all counters and recorded request history.
    pub async fn reset(&self) {
        let mut w = self.inner.write().await;
        w.request_count = 0;
        w.oauth_refresh_count = 0;
        w.generate_count = 0;
        w.recorded_requests.clear();
        w.behavior = MockBehavior::Default;
    }
}

/// Running Synthetic Antigravity API and Google OAuth Mock Server.
pub struct MockServer {
    addr: SocketAddr,
    controller: MockController,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl MockServer {
    /// Starts the synthetic mock server on an ephemeral loopback port (`127.0.0.1:0`).
    pub async fn start() -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        Self::start_with_listener(listener).await
    }

    /// Starts the mock server bound to a specific socket address.
    pub async fn bind(addr: SocketAddr) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr).await?;
        Self::start_with_listener(listener).await
    }

    async fn start_with_listener(listener: TcpListener) -> Result<Self, std::io::Error> {
        let addr = listener.local_addr()?;
        let inner = Arc::new(RwLock::new(MockInner::default()));
        let controller = MockController::new(inner.clone());

        let app = create_mock_router(inner);
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        tokio::spawn(async move {
            let server = axum::serve(listener, app);
            tokio::select! {
                res = server => {
                    if let Err(e) = res {
                        tracing::error!("Synthetic Mock Server terminated with error: {e}");
                    }
                }
                _ = shutdown_rx => {
                    tracing::debug!("Synthetic Mock Server received shutdown signal");
                }
            }
        });

        tracing::info!("Synthetic Antigravity Mock Server listening on http://{addr}");
        Ok(Self {
            addr,
            controller,
            shutdown_tx: Some(shutdown_tx),
        })
    }

    /// Returns the bound socket address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns the bound TCP port.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Returns the root base URL (e.g. `http://127.0.0.1:54321`).
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Formats a full URL for the given path.
    pub fn url(&self, path: &str) -> String {
        let path = path.strip_prefix('/').unwrap_or(path);
        format!("http://{}/{}", self.addr, path)
    }

    /// Standard Google OAuth token refresh endpoint URL.
    pub fn oauth_url(&self) -> String {
        self.url("oauth2/v4/token")
    }

    /// Standard Antigravity content generation endpoint URL.
    pub fn generate_url(&self) -> String {
        self.url("v1alpha/models/gemini-pro:streamGenerateContent")
    }

    /// Access the controller to modify behavior or inspect received requests.
    pub fn controller(&self) -> MockController {
        self.controller.clone()
    }

    /// Gracefully stops the mock server.
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Creates the full Axum router for the synthetic mock server.
pub fn create_mock_router(inner: Arc<RwLock<MockInner>>) -> Router {
    Router::new()
        // Health check
        .route("/healthz", get(handle_healthz))
        // Google OAuth Token Refresh endpoints
        .route("/token", post(handle_oauth_token))
        .route("/oauth2/token", post(handle_oauth_token))
        .route("/oauth2/v3/token", post(handle_oauth_token))
        .route("/oauth2/v4/token", post(handle_oauth_token))
        // Antigravity & Gemini API Endpoints
        .route(
            "/v1internal:action",
            post(handle_generate_content_with_param),
        )
        .route(
            "/v1alpha/models/:action",
            post(handle_generate_content_with_param),
        )
        .route(
            "/v1beta/models/:action",
            post(handle_generate_content_with_param),
        )
        .route(
            "/v1beta2/models/:action",
            post(handle_generate_content_with_param),
        )
        // Control & Inspection Endpoints
        .route("/_mock/requests", get(handle_inspect_requests))
        .route("/_mock/status", get(handle_inspect_status))
        .route("/_mock/reset", post(handle_control_reset))
        .fallback(handle_fallback)
        .with_state(inner)
}

async fn record_request(
    inner: &Arc<RwLock<MockInner>>,
    uri: &Uri,
    method: &str,
    headers: &HeaderMap,
    body_bytes: &[u8],
) -> (RecordedRequest, usize) {
    let mut header_map = HashMap::new();
    let mut bearer_token = None;

    for (k, v) in headers.iter() {
        let key_str = k.as_str().to_lowercase();
        if let Ok(val_str) = v.to_str() {
            if key_str == "authorization" {
                if let Some(stripped) = val_str.strip_prefix("Bearer ") {
                    bearer_token = Some(stripped.trim().to_string());
                } else if let Some(stripped) = val_str.strip_prefix("bearer ") {
                    bearer_token = Some(stripped.trim().to_string());
                } else {
                    bearer_token = Some(val_str.to_string());
                }
            }
            header_map.insert(key_str, val_str.to_string());
        }
    }

    let recorded = RecordedRequest {
        method: method.to_string(),
        uri: uri.to_string(),
        path: uri.path().to_string(),
        query: uri.query().map(|q| q.to_string()),
        headers: header_map,
        bearer_token,
        body: body_bytes.to_vec(),
        timestamp: Instant::now(),
    };

    let mut w = inner.write().await;
    w.request_count += 1;
    w.recorded_requests.push(recorded.clone());
    let current_count = w.request_count;
    (recorded, current_count)
}

async fn handle_healthz() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn handle_oauth_token(
    State(inner): State<Arc<RwLock<MockInner>>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (_req, _) = record_request(&inner, &uri, "POST", &headers, &body).await;

    let behavior = {
        let mut w = inner.write().await;
        w.oauth_refresh_count += 1;
        w.behavior.clone()
    };

    if matches!(behavior, MockBehavior::OAuthInvalidGrant) {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            json!({
                "error": "invalid_grant",
                "error_description": "Token has been expired or revoked."
            })
            .to_string(),
        )
            .into_response();
    }

    let refresh_count = inner.read().await.oauth_refresh_count;
    let token_resp = OAuthTokenResponse {
        access_token: format!("ya29.synthetic_refreshed_access_token_mock_{refresh_count}"),
        expires_in: 3600,
        token_type: "Bearer".to_string(),
        scope: Some("https://www.googleapis.com/auth/userinfo.email openid".to_string()),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(token_resp),
    )
        .into_response()
}

async fn handle_generate_content_with_param(
    State(inner): State<Arc<RwLock<MockInner>>>,
    Path(_action): Path<String>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    handle_generate_core(inner, uri, headers, body).await
}


async fn handle_generate_core(
    inner: Arc<RwLock<MockInner>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let (recorded, _) = record_request(&inner, &uri, "POST", &headers, &body).await;

    let (behavior, generate_index, service_name, _default_chunks, _chunk_delay) = {
        let mut w = inner.write().await;
        let index = w.generate_count;
        w.generate_count += 1;
        (
            w.behavior.clone(),
            index,
            w.default_service_name.clone(),
            w.default_stream_chunks.clone(),
            w.default_chunk_delay,
        )
    };

    match behavior {
        MockBehavior::Default => build_standard_200_response(),

        MockBehavior::FailFirstNWith429 { count } => {
            if generate_index < count {
                build_quota_429_response(&service_name, 25)
            } else {
                build_standard_200_response()
            }
        }

        MockBehavior::FailBearerToken { bad_token } => {
            let matches_bad = recorded
                .bearer_token
                .as_ref()
                .map(|t| t == &bad_token || t.contains(&bad_token))
                .unwrap_or(false);

            if matches_bad {
                build_quota_429_response(&service_name, 25)
            } else {
                build_standard_200_response()
            }
        }

        MockBehavior::GrpcResourceExhausted => build_grpc_resource_exhausted_response(),

        MockBehavior::StreamChunks {
            chunks,
            chunk_delay,
        } => build_stream_response(chunks, chunk_delay),

        MockBehavior::CustomSequence { mut responses } => {
            if !responses.is_empty() {
                let resp = responses.remove(0);
                let mut w = inner.write().await;
                w.behavior = MockBehavior::CustomSequence { responses };
                build_custom_mock_response(resp)
            } else {
                build_standard_200_response()
            }
        }

        MockBehavior::OAuthInvalidGrant => build_standard_200_response(),
    }
}

async fn handle_fallback(
    State(inner): State<Arc<RwLock<MockInner>>>,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let _ = record_request(&inner, &uri, "ANY", &headers, &body).await;
    build_standard_200_response()
}

fn build_standard_200_response() -> Response {
    let resp = GenerateContentResponse {
        candidates: vec![Candidate {
            content: Content {
                parts: vec![Part {
                    text: "Synthetic Antigravity Generation Success".to_string(),
                }],
                role: Some("model".to_string()),
            },
            finish_reason: Some("STOP".to_string()),
            index: Some(0),
        }],
        usage_metadata: Some(UsageMetadata {
            prompt_token_count: 10,
            candidates_token_count: 5,
            total_token_count: 15,
        }),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(resp),
    )
        .into_response()
}

fn build_quota_429_response(service: &str, retry_after: u32) -> Response {
    let err = GoogleApiErrorResponse::standard_quota_exhausted(service);
    (
        StatusCode::TOO_MANY_REQUESTS,
        [
            (header::CONTENT_TYPE, "application/json"),
            (
                header::RETRY_AFTER,
                header::HeaderValue::from_str(&retry_after.to_string())
                    .unwrap_or(header::HeaderValue::from_static("25"))
                    .to_str()
                    .unwrap_or("25"),
            ),
        ],
        Json(err),
    )
        .into_response()
}

fn build_grpc_resource_exhausted_response() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "application/grpc"),
            ("grpc-status", "8"),
            (
                "grpc-message",
                "Resource has been exhausted (e.g. check quota).",
            ),
        ],
        "rpc error: code = ResourceExhausted desc = Quota exceeded for quota metric",
    )
        .into_response()
}

fn build_stream_response(chunks: Vec<String>, delay: Duration) -> Response {
    let stream = stream::unfold(
        (chunks, 0usize, delay),
        move |(chunks, idx, delay)| async move {
            if idx >= chunks.len() {
                None
            } else {
                if delay.as_millis() > 0 && idx > 0 {
                    tokio::time::sleep(delay).await;
                }
                let chunk_text = &chunks[idx];
                let json_chunk = json!({
                    "candidates": [{
                        "content": {
                            "parts": [{"text": chunk_text}]
                        }
                    }]
                });
                let payload = format!("{}\n", json_chunk);
                Some((Ok::<Bytes, std::convert::Infallible>(Bytes::from(payload)), (chunks, idx + 1, delay)))
            }
        },
    );

    let body = Body::from_stream(stream);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::TRANSFER_ENCODING, "chunked")
        .body(body)
        .unwrap_or_else(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Stream building error").into_response())
}

fn build_custom_mock_response(mock_resp: MockResponse) -> Response {
    let status = StatusCode::from_u16(mock_resp.status).unwrap_or(StatusCode::OK);
    let mut builder = Response::builder().status(status);

    for (k, v) in mock_resp.headers {
        builder = builder.header(k, v);
    }

    match mock_resp.body {
        MockBody::Json(val) => builder
            .body(Body::from(val.to_string()))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        MockBody::Bytes(b) => builder
            .body(Body::from(b))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
        MockBody::StreamChunks {
            chunks,
            chunk_delay,
        } => build_stream_response(chunks, chunk_delay),
    }
}

async fn handle_inspect_requests(
    State(inner): State<Arc<RwLock<MockInner>>>,
) -> impl IntoResponse {
    let r = inner.read().await;
    let list: Vec<serde_json::Value> = r
        .recorded_requests
        .iter()
        .map(|req| {
            json!({
                "method": req.method,
                "uri": req.uri,
                "path": req.path,
                "bearer_token": req.bearer_token,
                "headers": req.headers,
                "body_len": req.body.len(),
            })
        })
        .collect();

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(list),
    )
}

async fn handle_inspect_status(State(inner): State<Arc<RwLock<MockInner>>>) -> impl IntoResponse {
    let r = inner.read().await;
    let status = json!({
        "request_count": r.request_count,
        "oauth_refresh_count": r.oauth_refresh_count,
        "generate_count": r.generate_count,
        "recorded_requests_len": r.recorded_requests.len(),
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(status),
    )
}

async fn handle_control_reset(State(inner): State<Arc<RwLock<MockInner>>>) -> impl IntoResponse {
    let mut w = inner.write().await;
    w.request_count = 0;
    w.oauth_refresh_count = 0;
    w.generate_count = 0;
    w.recorded_requests.clear();
    w.behavior = MockBehavior::Default;
    (StatusCode::OK, "Reset OK")
}
