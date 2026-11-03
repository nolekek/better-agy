use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full, LengthLimitError, Limited};
use hyper::header::{HeaderName, HeaderValue, AUTHORIZATION, HOST};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info, warn};

use crate::error::ProxyError;
use crate::failover::{FailoverDecision, FailoverEngine};

pub const DEFAULT_PROXY_ADDR: &str = "127.0.0.1:51128";
pub const MAX_BODY_BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32 MB

pub struct ProxyConfig {
    pub bind_addr: SocketAddr,
    pub upstream_override: Option<String>,
    pub max_retries: usize,
    pub connect_timeout: Duration,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            bind_addr: DEFAULT_PROXY_ADDR.parse().expect("Valid socket address"),
            upstream_override: None,
            max_retries: 5,
            connect_timeout: Duration::from_secs(10),
        }
    }
}

pub struct ProxyService {
    failover: Arc<FailoverEngine>,
    http_client: reqwest::Client,
    config: Arc<ProxyConfig>,
}

impl ProxyService {
    pub fn new(failover: Arc<FailoverEngine>, config: Arc<ProxyConfig>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .connect_timeout(config.connect_timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            failover,
            http_client,
            config,
        }
    }

    pub async fn handle_request(
        self: Arc<Self>,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        if req.method() == Method::CONNECT {
            return self.handle_connect(req).await;
        }

        self.handle_http_request(req).await
    }

    async fn handle_connect(
        self: Arc<Self>,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        let authority = match req.uri().authority().map(|a| a.to_string()) {
            Some(auth) => auth,
            None => {
                if let Some(host) = req.headers().get(HOST).and_then(|h| h.to_str().ok()) {
                    host.to_string()
                } else {
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(full_box_body(Bytes::from("Missing host for CONNECT")))
                        .unwrap());
                }
            }
        };

        info!("Handling HTTP CONNECT tunnel for {}", authority);

        // Check if target is a Google domain or custom upstream
        let _is_google = is_google_domain(&authority);
        let target_authority = if let Some(ref override_url) = self.config.upstream_override {
            if let Ok(parsed) = reqwest::Url::parse(override_url) {
                if let Some(host_str) = parsed.host_str() {
                    let port = parsed.port_or_known_default().unwrap_or(80);
                    format!("{}:{}", host_str, port)
                } else {
                    authority.clone()
                }
            } else {
                authority.clone()
            }
        } else {
            authority.clone()
        };

        tokio::spawn(async move {
            match hyper::upgrade::on(req).await {
                Ok(upgraded) => {
                    let mut upgraded_io = TokioIo::new(upgraded);
                    match TcpStream::connect(&target_authority).await {
                        Ok(mut target_stream) => {
                            let _ = tokio::io::copy_bidirectional(&mut upgraded_io, &mut target_stream).await;
                        }
                        Err(e) => {
                            error!("Failed to connect to target {}: {}", target_authority, e);
                        }
                    }
                }
                Err(e) => {
                    warn!("CONNECT upgrade failed for {}: {}", target_authority, e);
                }
            }
        });

        // 200 Connection Established
        let resp = Response::builder()
            .status(StatusCode::OK)
            .body(empty_box_body())
            .unwrap();

        Ok(resp)
    }

    async fn handle_status_request(
        &self,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        let now_sec = chrono::Utc::now().timestamp();
        let vault_data = self.failover.vault().get_data().await;
        let active_slot = self.failover.vault().get_active_slot().await;
        let total_slots = vault_data.slots.len();

        let mut available_slots = 0;
        let mut cooldown_slots = 0;
        let mut slots_json = Vec::new();

        for slot in &vault_data.slots {
            let is_ready = slot.is_ready_for_use(now_sec);
            let is_valid_token = slot.is_token_valid(now_sec, 0);
            if is_ready {
                available_slots += 1;
            }
            if matches!(slot.state, crate::vault::SlotState::Cooldown { .. }) {
                cooldown_slots += 1;
            }

            let token_preview = if slot.access_token.len() > 10 {
                format!(
                    "{}...{}",
                    &slot.access_token[..6],
                    &slot.access_token[slot.access_token.len() - 4..]
                )
            } else {
                "***".to_string()
            };

            slots_json.push(serde_json::json!({
                "id": slot.id,
                "email": slot.email,
                "state": slot.state,
                "expires_at": slot.expires_at,
                "token_valid": is_valid_token,
                "ready_for_use": is_ready,
                "token_preview": token_preview,
            }));
        }

        let active_slot_val = match &active_slot {
            Some(s) => {
                if let Ok(num) = s.id.parse::<u64>() {
                    serde_json::json!(num)
                } else {
                    serde_json::json!(s.id)
                }
            }
            None => serde_json::Value::Null,
        };

        let debounce_window_sec = (self.failover.debounce_millis() / 1000) as u64;
        let last_failover_epoch_ms = self.failover.last_failover_epoch_ms();

        let status_json = serde_json::json!({
            "status": "healthy",
            "active_slot": active_slot_val,
            "active_slot_id": active_slot.as_ref().map(|s| s.id.as_str()),
            "total_slots": total_slots,
            "available_slots": available_slots,
            "auto_failover": vault_data.auto_failover,
            "debounce_window_sec": debounce_window_sec,
            "last_failover_epoch_ms": last_failover_epoch_ms,
            "slots": slots_json,
            "quotas": {
                "active_slot_id": active_slot.as_ref().map(|s| s.id.as_str()),
                "total_slots": total_slots,
                "available_slots": available_slots,
                "cooldown_slots": cooldown_slots,
                "debounce_window_sec": debounce_window_sec,
            }
        });

        let body_bytes = serde_json::to_vec_pretty(&status_json).unwrap_or_default();

        let resp = Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .header("cache-control", "no-cache, no-store")
            .body(full_box_body(Bytes::from(body_bytes)))
            .unwrap();

        Ok(resp)
    }

    async fn handle_http_request(
        self: Arc<Self>,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, Infallible>>, Infallible> {
        let method = req.method().clone();
        let uri = req.uri().clone();
        let headers = req.headers().clone();

        // Direct handling for proxy telemetry/status endpoint
        let path = uri.path();
        if (method == Method::GET || method == Method::HEAD)
            && (path == "/api/proxy/status" || path == "/api/proxy/status/" || path.ends_with("/api/proxy/status"))
        {
            return self.handle_status_request().await;
        }

        // 1. Fast rejection if Content-Length header > MAX_BODY_BUFFER_SIZE
        if let Some(cl_val) = headers.get(hyper::header::CONTENT_LENGTH) {
            if let Ok(cl_str) = cl_val.to_str() {
                if let Ok(content_length) = cl_str.trim().parse::<usize>() {
                    if content_length > MAX_BODY_BUFFER_SIZE {
                        warn!(
                            "Request Content-Length {} exceeds MAX_BODY_BUFFER_SIZE {}",
                            content_length, MAX_BODY_BUFFER_SIZE
                        );
                        return Ok(Response::builder()
                            .status(StatusCode::PAYLOAD_TOO_LARGE)
                            .header("content-type", "text/plain")
                            .body(full_box_body(Bytes::from("413 Payload Too Large")))
                            .unwrap());
                    }
                }
            }
        }

        // 2. Read and buffer request body in memory, enforcing MAX_BODY_BUFFER_SIZE
        let limited_body = Limited::new(req.into_body(), MAX_BODY_BUFFER_SIZE);
        let body_bytes = match limited_body.collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                if e.downcast_ref::<LengthLimitError>().is_some() {
                    warn!(
                        "Request body exceeded MAX_BODY_BUFFER_SIZE ({} bytes)",
                        MAX_BODY_BUFFER_SIZE
                    );
                    return Ok(Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .header("content-type", "text/plain")
                        .body(full_box_body(Bytes::from("413 Payload Too Large")))
                        .unwrap());
                }
                error!("Failed to read request body: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(full_box_body(Bytes::from("Failed to read body")))
                    .unwrap());
            }
        };

        let target_url = self.resolve_target_url(&uri, &headers);
        let mut retries = 0;
        let mut last_used_token: Option<String> = None;

        loop {
            // Get current active slot token
            let active_slot = match self.failover.vault().get_active_slot().await {
                Some(slot) => slot,
                None => {
                    error!("No active slot available in vault");
                    return Ok(Response::builder()
                        .status(StatusCode::SERVICE_UNAVAILABLE)
                        .body(full_box_body(Bytes::from("No account slots configured")))
                        .unwrap());
                }
            };

            let current_token = active_slot.access_token.clone();
            let _prev_token = last_used_token.replace(current_token.clone());

            info!(
                "Dispatching upstream {} {} (Slot: {}, Token: ya29...{})",
                method,
                target_url,
                active_slot.id,
                &current_token[current_token.len().saturating_sub(6)..]
            );

            // Build upstream request
            let mut req_builder = self.http_client.request(
                reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::GET),
                &target_url,
            );

            // Copy headers excluding hop-by-hop headers
            for (name, value) in &headers {
                let name_str = name.as_str().to_lowercase();
                if is_hop_by_hop_header(&name_str) {
                    continue;
                }
                if name == AUTHORIZATION {
                    continue; // Will inject active slot token below
                }
                if let Ok(v) = value.to_str() {
                    req_builder = req_builder.header(name.as_str(), v);
                }
            }

            // Inject Authorization: Bearer <token>
            let bearer_val = format!("Bearer {}", current_token);
            req_builder = req_builder.header("authorization", bearer_val);

            // Attach buffered body
            req_builder = req_builder.body(body_bytes.clone());

            // Dispatch upstream
            let upstream_res = match req_builder.send().await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Upstream connection error to {}: {}", target_url, e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(full_box_body(Bytes::from(format!("Upstream error: {e}"))))
                        .unwrap());
                }
            };

            let status = upstream_res.status().as_u16();
            let res_headers = convert_reqwest_headers(upstream_res.headers());

            // Read response body bytes for quota detection and forwarding
            let res_body_bytes = match upstream_res.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    error!("Failed to read upstream response body: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(full_box_body(Bytes::from("Failed to read upstream response")))
                        .unwrap());
                }
            };

            // Check if this response indicates quota exhaustion (429 / ResourceExhausted)
            let is_quota = self.failover.is_quota_error(status, &res_headers, &res_body_bytes);

            if is_quota && retries < self.config.max_retries {
                retries += 1;
                warn!(
                    "Quota exhaustion detected from upstream (status: {}, retry {}/{})",
                    status, retries, self.config.max_retries
                );

                match self.failover.on_quota_error().await {
                    Ok(FailoverDecision::Rotated { new_slot, old_slot_id }) => {
                        info!(
                            "In-flight failover: Rotated from slot {} to {}! Retrying upstream transparently...",
                            old_slot_id, new_slot.id
                        );
                        continue;
                    }
                    Ok(FailoverDecision::Debounced { active_slot: current_active, elapsed_millis }) => {
                        if Some(&current_active.access_token) != last_used_token.as_ref() {
                            info!(
                                "Debounce active ({}ms), but active slot changed to {}. Retrying with new token...",
                                elapsed_millis, current_active.id
                            );
                            continue;
                        } else {
                            warn!(
                                "Debounce active ({}ms) and token unchanged. Returning 429 to client.",
                                elapsed_millis
                            );
                            return Ok(build_hyper_response(status, res_headers, res_body_bytes));
                        }
                    }
                    Err(e) => {
                        warn!("Failover failed: {e}. Returning upstream response.");
                        return Ok(build_hyper_response(status, res_headers, res_body_bytes));
                    }
                }
            }

            // Normal success or non-retryable response: forward directly to client
            return Ok(build_hyper_response(status, res_headers, res_body_bytes));
        }
    }

    fn resolve_target_url(&self, uri: &hyper::Uri, headers: &hyper::HeaderMap) -> String {
        if let Some(ref base) = self.config.upstream_override {
            let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
            return format!("{}{}", base.trim_end_matches('/'), path_and_query);
        }

        if let Some(scheme) = uri.scheme_str() {
            if let Some(authority) = uri.authority() {
                let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
                return format!("{}://{}{}", scheme, authority, path_and_query);
            }
        }

        if let Some(host) = headers.get(HOST).and_then(|h| h.to_str().ok()) {
            let path_and_query = uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/");
            return format!("https://{}{}", host, path_and_query);
        }

        uri.to_string()
    }
}

pub struct ProxyServer {
    config: Arc<ProxyConfig>,
    failover: Arc<FailoverEngine>,
}

impl ProxyServer {
    pub fn new(config: ProxyConfig, failover: Arc<FailoverEngine>) -> Self {
        Self {
            config: Arc::new(config),
            failover,
        }
    }

    pub fn bind_addr(&self) -> SocketAddr {
        self.config.bind_addr
    }

    pub async fn run(
        self,
        listener: TcpListener,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> Result<(), ProxyError> {
        let service = Arc::new(ProxyService::new(self.failover.clone(), self.config.clone()));
        info!("Proxy server listening on {}", listener.local_addr()?);

        loop {
            tokio::select! {
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, client_addr)) => {
                            let service_clone = service.clone();
                            tokio::spawn(async move {
                                let io = TokioIo::new(stream);
                                let conn = http1::Builder::new().serve_connection(
                                    io,
                                    service_fn(move |req| {
                                        let s = service_clone.clone();
                                        async move { s.handle_request(req).await }
                                    }),
                                );

                                if let Err(err) = conn.with_upgrades().await {
                                    warn!("Connection error with {}: {}", client_addr, err);
                                }
                            });
                        }
                        Err(e) => {
                            error!("Accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("Proxy server received shutdown signal");
                    break;
                }
            }
        }

        Ok(())
    }
}

fn full_box_body(b: Bytes) -> BoxBody<Bytes, Infallible> {
    Full::new(b).map_err(|never| match never {}).boxed()
}

fn empty_box_body() -> BoxBody<Bytes, Infallible> {
    Empty::new().map_err(|never| match never {}).boxed()
}

fn is_google_domain(authority: &str) -> bool {
    let host = authority.split(':').next().unwrap_or(authority);
    host.ends_with(".googleapis.com")
        || host.ends_with("google.com")
        || host == "generativelanguage.googleapis.com"
        || host == "cloudcode-pa.googleapis.com"
        || host == "daily-cloudcode-pa.googleapis.com"
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name,
        "connection"
            | "proxy-connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn convert_reqwest_headers(req_headers: &reqwest::header::HeaderMap) -> hyper::HeaderMap {
    let mut map = hyper::HeaderMap::new();
    for (k, v) in req_headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(k.as_str().as_bytes()),
            HeaderValue::from_bytes(v.as_bytes()),
        ) {
            map.insert(name, val);
        }
    }
    map
}

fn build_hyper_response(
    status: u16,
    headers: hyper::HeaderMap,
    body: Bytes,
) -> Response<BoxBody<Bytes, Infallible>> {
    let mut builder = Response::builder().status(status);
    for (name, value) in &headers {
        let name_str = name.as_str().to_lowercase();
        if !is_hop_by_hop_header(&name_str) && name_str != "content-length" {
            builder = builder.header(name, value);
        }
    }
    builder = builder.header("content-length", body.len().to_string());
    builder.body(full_box_body(body)).unwrap()
}
