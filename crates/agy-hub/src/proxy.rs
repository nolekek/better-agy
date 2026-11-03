use crate::config::HubConfig;
use crate::dbus::SystemdManager;
use crate::error::HubError;
use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderValue, StatusCode, Uri};
use axum::response::Response;
use hyper_util::rt::TokioIo;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info};

pub struct ReverseProxyEngine {
    config: Arc<HubConfig>,
    systemd: Arc<dyn SystemdManager>,
}

impl ReverseProxyEngine {
    pub fn new(config: Arc<HubConfig>, systemd: Arc<dyn SystemdManager>) -> Self {
        Self { config, systemd }
    }

    /// Ensures that the user's Web IDE socket is responsive, starting it if necessary
    pub async fn ensure_socket_ready(&self, uid: u32) -> Result<String, HubError> {
        let socket_path = self.config.socket_path_for_uid(uid);
        let socket_str = socket_path.to_string_lossy().to_string();

        if self.probe_socket(&socket_str).await {
            return Ok(socket_str);
        }

        info!("Socket '{}' not ready; triggering systemd start for UID {}", socket_str, uid);
        self.systemd.start_unit(uid, "antigravity-web.service").await?;

        let timeout = Duration::from_secs(self.config.socket_poll_timeout_secs);
        let start_time = Instant::now();

        while start_time.elapsed() < timeout {
            if self.probe_socket(&socket_str).await {
                info!("Socket '{}' is now ready for UID {}", socket_str, uid);
                return Ok(socket_str);
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Err(HubError::ServiceTimeout(self.config.socket_poll_timeout_secs))
    }

    async fn probe_socket(&self, target: &str) -> bool {
        if target.starts_with("tcp:") || target.starts_with("127.0.0.1:") {
            let addr = target.trim_start_matches("tcp:");
            tokio::net::TcpStream::connect(addr).await.is_ok()
        } else {
            #[cfg(unix)]
            {
                tokio::net::UnixStream::connect(target).await.is_ok()
            }
            #[cfg(not(unix))]
            {
                std::path::Path::new(target).exists()
            }
        }
    }

    /// Handles an incoming /ide/* request by proxying to the user's socket
    pub async fn proxy_request(
        &self,
        uid: u32,
        username: &str,
        mut req: Request,
    ) -> Result<Response, HubError> {
        let socket_str = self.ensure_socket_ready(uid).await?;

        // 1. Rewrite URI path: strip /ide prefix
        let original_uri = req.uri().clone();
        let path = original_uri.path();
        let stripped_path = if path == "/ide" || path == "/ide/" {
            "/"
        } else if let Some(rest) = path.strip_prefix("/ide") {
            rest
        } else {
            path
        };

        let new_uri_str = if let Some(query) = original_uri.query() {
            format!("{}?{}", stripped_path, query)
        } else {
            stripped_path.to_string()
        };

        let new_uri: Uri = new_uri_str.parse().map_err(|e| {
            HubError::UpstreamError(format!("Invalid rewritten URI: {}", e))
        })?;

        *req.uri_mut() = new_uri;

        // 2. Normalize and inject headers
        req.headers_mut().insert(
            header::HeaderName::from_static("x-forwarded-user"),
            HeaderValue::from_str(username).unwrap(),
        );
        req.headers_mut().insert(
            header::HeaderName::from_static("x-forwarded-proto"),
            HeaderValue::from_static("https"),
        );
        req.headers_mut().insert(
            header::HeaderName::from_static("x-forwarded-prefix"),
            HeaderValue::from_static("/ide"),
        );
        // Remove internal session cookie to avoid leaking to upstream IDE
        req.headers_mut().remove(header::COOKIE);

        // 3. Check for WebSocket Upgrade
        let is_websocket = req
            .headers()
            .get(header::UPGRADE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.eq_ignore_ascii_case("websocket"))
            .unwrap_or(false);

        if is_websocket {
            return self.handle_websocket_upgrade(req, socket_str).await;
        }

        // 4. Standard HTTP Reverse Proxy
        self.forward_http_request(req, socket_str).await
    }

    async fn forward_http_request(
        &self,
        req: Request,
        socket_target: String,
    ) -> Result<Response, HubError> {
        if socket_target.starts_with("tcp:") || socket_target.starts_with("127.0.0.1:") {
            let addr = socket_target.trim_start_matches("tcp:");
            let stream = tokio::net::TcpStream::connect(addr)
                .await
                .map_err(|e| HubError::UpstreamError(format!("TCP connect error: {}", e)))?;
            let io = TokioIo::new(stream);
            Self::dispatch_hyper_request(req, io).await
        } else {
            #[cfg(unix)]
            {
                let stream = tokio::net::UnixStream::connect(&socket_target)
                    .await
                    .map_err(|e| HubError::UpstreamError(format!("Unix socket connect error: {}", e)))?;
                let io = TokioIo::new(stream);
                Self::dispatch_hyper_request(req, io).await
            }
            #[cfg(not(unix))]
            {
                let stream = tokio::net::TcpStream::connect(&socket_target)
                    .await
                    .map_err(|e| HubError::UpstreamError(format!("Connect error: {}", e)))?;
                let io = TokioIo::new(stream);
                Self::dispatch_hyper_request(req, io).await
            }
        }
    }

    async fn dispatch_hyper_request<T>(
        req: Request,
        io: TokioIo<T>,
    ) -> Result<Response, HubError>
    where
        T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
    {
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(|e| HubError::UpstreamError(format!("Handshake failed: {}", e)))?;

        tokio::spawn(async move {
            if let Err(err) = conn.await {
                debug!("Upstream connection closed: {:?}", err);
            }
        });

        // Convert Axum body to Hyper body
        let (parts, body) = req.into_parts();
        let hyper_req = hyper::Request::from_parts(parts, body);

        let upstream_resp = sender
            .send_request(hyper_req)
            .await
            .map_err(|e| HubError::UpstreamError(format!("Request failed: {}", e)))?;

        let (resp_parts, resp_body) = upstream_resp.into_parts();
        let axum_body = Body::new(resp_body);
        let resp = Response::from_parts(resp_parts, axum_body);

        Ok(resp)
    }

    async fn handle_websocket_upgrade(
        &self,
        mut req: Request,
        socket_target: String,
    ) -> Result<Response, HubError> {
        let key = req
            .headers()
            .get(header::SEC_WEBSOCKET_KEY)
            .cloned()
            .unwrap_or_else(|| HeaderValue::from_static(""));

        let accept_val = derive_accept_key(key.as_bytes());

        tokio::spawn(async move {
            match hyper::upgrade::on(&mut req).await {
                Ok(upgraded) => {
                    let mut client_io = TokioIo::new(upgraded);
                    if socket_target.starts_with("tcp:") || socket_target.starts_with("127.0.0.1:") {
                        let addr = socket_target.trim_start_matches("tcp:");
                        if let Ok(mut upstream_tcp) = tokio::net::TcpStream::connect(addr).await {
                            let _ = tokio::io::copy_bidirectional(&mut client_io, &mut upstream_tcp).await;
                        }
                    } else {
                        #[cfg(unix)]
                        {
                            if let Ok(mut upstream_unix) = tokio::net::UnixStream::connect(&socket_target).await {
                                let _ = tokio::io::copy_bidirectional(&mut client_io, &mut upstream_unix).await;
                            }
                        }
                        #[cfg(not(unix))]
                        {
                            if let Ok(mut upstream_tcp) = tokio::net::TcpStream::connect(&socket_target).await {
                                let _ = tokio::io::copy_bidirectional(&mut client_io, &mut upstream_tcp).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket upgrade error: {}", e);
                }
            }
        });

        let mut resp = Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .body(Body::empty())
            .unwrap();

        resp.headers_mut().insert(header::UPGRADE, HeaderValue::from_static("websocket"));
        resp.headers_mut().insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
        resp.headers_mut().insert(
            header::HeaderName::from_static("sec-websocket-accept"),
            HeaderValue::from_str(&accept_val).unwrap(),
        );

        Ok(resp)
    }
}

fn derive_accept_key(key: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(key);
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, hasher.finalize())
}
