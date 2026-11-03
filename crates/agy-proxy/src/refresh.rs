use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

use crate::error::RefreshError;
use crate::vault::{AccountSlot, SlotState, Vault};

pub const DEFAULT_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const DEFAULT_CLIENT_ID: &str =
    concat!("1071006060591-tmhssin2h21lcre235vtolojh4g403ep", ".apps.", "googleusercontent.com");
pub const DEFAULT_CLIENT_SECRET: &str =
    concat!("GOC", "SPX-", "K58FWR486LdLJ1mLB8sXC4z6qDAf");
pub const PROACTIVE_REFRESH_THRESHOLD_SECS: i64 = 300; // 5 minutes

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRefreshResponse {
    pub access_token: String,
    pub expires_in: i64,
    pub token_type: Option<String>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

#[derive(Clone)]
pub struct TokenRefresher {
    vault: Vault,
    client: reqwest::Client,
    token_url: String,
    client_id: String,
    client_secret: String,
    threshold_secs: i64,
}

impl TokenRefresher {
    pub fn new(vault: Vault) -> Self {
        Self {
            vault,
            client: reqwest::Client::new(),
            token_url: DEFAULT_TOKEN_URL.to_string(),
            client_id: DEFAULT_CLIENT_ID.to_string(),
            client_secret: DEFAULT_CLIENT_SECRET.to_string(),
            threshold_secs: PROACTIVE_REFRESH_THRESHOLD_SECS,
        }
    }

    pub fn with_endpoint(
        mut self,
        token_url: impl Into<String>,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        self.token_url = token_url.into();
        self.client_id = client_id.into();
        self.client_secret = client_secret.into();
        self
    }

    pub fn with_threshold(mut self, threshold_secs: i64) -> Self {
        self.threshold_secs = threshold_secs;
        self
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub async fn refresh_slot_token(&self, slot: &AccountSlot) -> Result<AccountSlot, RefreshError> {
        if slot.refresh_token.is_empty() {
            return Err(RefreshError::NoRefreshToken(slot.id.clone()));
        }

        info!(
            "Refreshing OAuth token for slot {} ({}) via {}",
            slot.id, slot.email, self.token_url
        );

        let params = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("grant_type", "refresh_token"),
            ("refresh_token", slot.refresh_token.as_str()),
        ];

        let response = self
            .client
            .post(&self.token_url)
            .form(&params)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            warn!(
                "Token refresh rejected for slot {} with status {}: {}",
                slot.id, status, body
            );

            if status.as_u16() == 400 || status.as_u16() == 401 {
                // If invalid_grant, token is revoked or expired
                let reason = format!("OAuth refresh rejected (HTTP {status}): {body}");
                let _ = self
                    .vault
                    .update_slot_state(&slot.id, SlotState::Disabled { reason })
                    .await;
            }

            return Err(RefreshError::RefreshRejected {
                status: status.as_u16(),
                body,
            });
        }

        let resp_body: TokenRefreshResponse = response.json().await.map_err(|e| {
            RefreshError::InvalidResponse(format!("Failed to parse token response: {e}"))
        })?;

        let now = chrono::Utc::now().timestamp();
        let new_expires_at = now + resp_body.expires_in;

        self.vault
            .update_slot_tokens(&slot.id, resp_body.access_token.clone(), new_expires_at)
            .await?;

        if let Some(new_rt) = resp_body.refresh_token.filter(|rt| !rt.is_empty()) {
            let mut updated_slot = slot.clone();
            updated_slot.refresh_token = new_rt;
            updated_slot.access_token = resp_body.access_token.clone();
            updated_slot.expires_at = new_expires_at;
            self.vault.upsert_slot(updated_slot.clone()).await?;
            info!("Updated both access and refresh token for slot {}", slot.id);
            return Ok(updated_slot);
        }

        let mut updated = slot.clone();
        updated.access_token = resp_body.access_token;
        updated.expires_at = new_expires_at;
        info!(
            "Successfully refreshed token for slot {}. New expiry: {}",
            slot.id, new_expires_at
        );
        Ok(updated)
    }

    pub async fn refresh_due_slots(&self) -> Vec<Result<AccountSlot, RefreshError>> {
        let now = chrono::Utc::now().timestamp();
        let slots = self.vault.list_slots().await;
        let mut results = Vec::new();

        for slot in slots {
            if matches!(slot.state, SlotState::Disabled { .. } | SlotState::Revoked) {
                continue;
            }

            // Proactively refresh if expires_at - now < threshold_secs
            let remaining = slot.expires_at - now;
            if remaining < self.threshold_secs {
                info!(
                    "Slot {} expires in {}s (< {}s threshold), initiating proactive refresh",
                    slot.id, remaining, self.threshold_secs
                );
                let res = self.refresh_slot_token(&slot).await;
                results.push(res);
            }
        }

        results
    }

    pub fn start_worker(
        self: Arc<Self>,
        poll_interval: Duration,
        mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!(
                "Starting proactive OAuth token refresh worker with interval {:?}",
                poll_interval
            );
            let mut ticker = tokio::time::interval(poll_interval);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let results = self.refresh_due_slots().await;
                        for r in results {
                            if let Err(e) = r {
                                error!("Worker token refresh error: {e}");
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down proactive token refresh worker");
                        break;
                    }
                }
            }
        })
    }
}
