use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Recorded incoming HTTP request for test inspection and assertions.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    pub method: String,
    pub uri: String,
    pub path: String,
    pub query: Option<String>,
    pub headers: HashMap<String, String>,
    pub bearer_token: Option<String>,
    pub body: Vec<u8>,
    pub timestamp: Instant,
}

impl RecordedRequest {
    /// Attempts to parse the request body as JSON.
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Attempts to parse the request body as a UTF-8 string.
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }
}

/// Google OAuth token response schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub token_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

/// Google OAuth error response schema (e.g. invalid_grant).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthErrorResponse {
    pub error: String,
    pub error_description: String,
}

/// Synthetic LLM generation candidate schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GenerateContentResponse {
    pub candidates: Vec<Candidate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Candidate {
    pub content: Content,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Content {
    pub parts: Vec<Part>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Part {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageMetadata {
    pub prompt_token_count: u32,
    pub candidates_token_count: u32,
    pub total_token_count: u32,
}

/// Google API Quota / 429 Error Response schema.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleApiErrorResponse {
    pub error: GoogleApiErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GoogleApiErrorDetail {
    pub code: u16,
    pub message: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<serde_json::Value>,
}

impl GoogleApiErrorResponse {
    /// Creates a standard Google Cloud quota exhaustion 429 error.
    pub fn standard_quota_exhausted(service: &str) -> Self {
        Self {
            error: GoogleApiErrorDetail {
                code: 429,
                message: format!(
                    "Quota exceeded for quota metric 'Queries' and limit 'Queries per minute' of service '{service}'"
                ),
                status: "RESOURCE_EXHAUSTED".to_string(),
                details: vec![serde_json::json!({
                    "@type": "type.googleapis.com/google.rpc.QuotaFailure",
                    "violations": [
                        {
                            "subject": "project:antigravity-dev-pool",
                            "description": "Quota exceeded for quota metric 'Queries' and limit 'Queries per minute'"
                        }
                    ]
                })],
            },
        }
    }
}

/// Programmable response body representation.
#[derive(Debug, Clone)]
pub enum MockBody {
    Json(serde_json::Value),
    Bytes(Bytes),
    StreamChunks {
        chunks: Vec<String>,
        chunk_delay: Duration,
    },
}

/// Fully customizable mock response.
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: MockBody,
}

impl MockResponse {
    pub fn ok_json(value: serde_json::Value) -> Self {
        Self {
            status: 200,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: MockBody::Json(value),
        }
    }

    pub fn ok_stream(chunks: Vec<String>, delay: Duration) -> Self {
        Self {
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("transfer-encoding".to_string(), "chunked".to_string()),
            ],
            body: MockBody::StreamChunks {
                chunks,
                chunk_delay: delay,
            },
        }
    }

    pub fn too_many_requests_quota(service: &str, retry_after_secs: u32) -> Self {
        let err = GoogleApiErrorResponse::standard_quota_exhausted(service);
        Self {
            status: 429,
            headers: vec![
                ("content-type".to_string(), "application/json".to_string()),
                ("retry-after".to_string(), retry_after_secs.to_string()),
            ],
            body: MockBody::Json(serde_json::to_value(err).unwrap()),
        }
    }

    pub fn grpc_resource_exhausted() -> Self {
        Self {
            status: 200,
            headers: vec![
                ("content-type".to_string(), "application/grpc".to_string()),
                ("grpc-status".to_string(), "8".to_string()),
                (
                    "grpc-message".to_string(),
                    "Resource has been exhausted (e.g. check quota).".to_string(),
                ),
            ],
            body: MockBody::Bytes(Bytes::from_static(
                b"rpc error: code = ResourceExhausted desc = Quota exceeded for quota metric",
            )),
        }
    }

    pub fn oauth_invalid_grant() -> Self {
        Self {
            status: 400,
            headers: vec![("content-type".to_string(), "application/json".to_string())],
            body: MockBody::Json(serde_json::json!({
                "error": "invalid_grant",
                "error_description": "Token has been expired or revoked."
            })),
        }
    }
}

/// Programmable behavior configuration for the synthetic mock server.
#[derive(Debug, Clone, Default)]
pub enum MockBehavior {
    /// Always return standard successful responses.
    #[default]
    Default,
    /// Return 429 Quota Exhaustion for the first N requests, then return 200 OK.
    FailFirstNWith429 {
        count: usize,
    },
    /// Return 429 Quota Exhaustion if Bearer token matches the bad token; 200 OK otherwise.
    FailBearerToken {
        bad_token: String,
    },
    /// Simulate gRPC ResourceExhausted status response.
    GrpcResourceExhausted,
    /// Stream LLM tokens in chunks with specified delay.
    StreamChunks {
        chunks: Vec<String>,
        chunk_delay: Duration,
    },
    /// Return invalid_grant on OAuth refresh endpoint.
    OAuthInvalidGrant,
    /// Pop programmed responses from an explicit sequential queue.
    CustomSequence {
        responses: Vec<MockResponse>,
    },
}
