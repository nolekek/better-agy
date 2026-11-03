use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use regex::Regex;
use tracing::{info, warn};

use crate::error::FailoverError;
use crate::vault::{AccountSlot, SlotState, Vault};

pub const DEFAULT_DEBOUNCE_MILLIS: i64 = 25_000; // 25.0 seconds
pub const COOLDOWN_DURATION_SECS: i64 = 300; // 300 seconds (5 minutes)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailoverDecision {
    Rotated {
        new_slot: AccountSlot,
        old_slot_id: String,
    },
    Debounced {
        active_slot: AccountSlot,
        elapsed_millis: i64,
    },
}

pub struct QuotaDetector {
    regex: Regex,
}

impl Default for QuotaDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaDetector {
    pub fn new() -> Self {
        let pattern = r"(?i)(?:rpc error: code = ResourceExhausted|status:\s*429|RESOURCE_EXHAUSTED.*quota|Quota exceeded for quota metric|generativelanguage.*429|RESOURCE_EXHAUSTED)";
        let regex = Regex::new(pattern).expect("Valid quota regex");
        Self { regex }
    }

    pub fn is_quota_error(
        &self,
        status_code: u16,
        headers: &hyper::HeaderMap,
        body_bytes: &[u8],
    ) -> bool {
        // 1. Direct HTTP 429
        if status_code == 429 {
            return true;
        }

        // 2. gRPC status header: grpc-status 8 is RESOURCE_EXHAUSTED
        if let Some(val) = headers.get("grpc-status") {
            if let Ok(s) = val.to_str() {
                if s.trim() == "8" {
                    return true;
                }
            }
        }

        // 3. gRPC message header
        if let Some(val) = headers.get("grpc-message") {
            if let Ok(s) = val.to_str() {
                if self.regex.is_match(s) {
                    return true;
                }
            }
        }

        // 4. Response body error string
        if !body_bytes.is_empty() {
            if let Ok(body_str) = std::str::from_utf8(body_bytes) {
                if self.regex.is_match(body_str) {
                    return true;
                }
            }
        }

        false
    }
}

pub struct FailoverEngine {
    vault: Vault,
    detector: QuotaDetector,
    last_failover_epoch_ms: Arc<AtomicI64>,
    mutex: Arc<tokio::sync::Mutex<()>>,
    debounce_millis: i64,
    cooldown_secs: i64,
}

impl FailoverEngine {
    pub fn new(vault: Vault) -> Self {
        Self {
            vault,
            detector: QuotaDetector::new(),
            last_failover_epoch_ms: Arc::new(AtomicI64::new(0)),
            mutex: Arc::new(tokio::sync::Mutex::new(())),
            debounce_millis: DEFAULT_DEBOUNCE_MILLIS,
            cooldown_secs: COOLDOWN_DURATION_SECS,
        }
    }

    pub fn with_debounce(mut self, debounce: Duration) -> Self {
        self.debounce_millis = debounce.as_millis() as i64;
        self
    }

    pub fn with_cooldown(mut self, cooldown_secs: i64) -> Self {
        self.cooldown_secs = cooldown_secs;
        self
    }

    pub fn detector(&self) -> &QuotaDetector {
        &self.detector
    }

    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    pub fn last_failover_epoch_ms(&self) -> i64 {
        self.last_failover_epoch_ms.load(Ordering::Acquire)
    }

    pub fn set_last_failover_epoch_ms(&self, ms: i64) {
        self.last_failover_epoch_ms.store(ms, Ordering::Release);
    }

    pub fn debounce_millis(&self) -> i64 {
        self.debounce_millis
    }

    /// Check if a response indicates quota exhaustion
    pub fn is_quota_error(
        &self,
        status_code: u16,
        headers: &hyper::HeaderMap,
        body_bytes: &[u8],
    ) -> bool {
        self.detector.is_quota_error(status_code, headers, body_bytes)
    }

    /// Handle quota error with 25-second debounce window.
    /// If within debounce window, return Debounced with current active slot.
    /// If debounce window has passed, perform slot rotation:
    /// - Mark failed slot Cooldown(now + cooldown_secs)
    /// - Select next available slot in carousel
    /// - Set next slot to Active
    /// - Return Rotated { new_slot, old_slot_id }
    pub async fn on_quota_error(&self) -> Result<FailoverDecision, FailoverError> {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let last_ms = self.last_failover_epoch_ms.load(Ordering::Acquire);
        let elapsed_ms = now_ms.saturating_sub(last_ms);

        // 1. Fast-path check: if debounce window is active, return Debounced immediately
        if last_ms > 0 && elapsed_ms < self.debounce_millis {
            let active_slot = self
                .vault
                .get_active_slot()
                .await
                .ok_or(FailoverError::NoSlotsConfigured)?;

            warn!(
                "Quota error burst detected! Debounce active (elapsed: {}ms < {}ms). Suppressing failover.",
                elapsed_ms, self.debounce_millis
            );
            return Ok(FailoverDecision::Debounced {
                active_slot,
                elapsed_millis: elapsed_ms,
            });
        }

        // 2. Slow-path: acquire serialization mutex
        let _guard = self.mutex.lock().await;

        // 3. Re-check debounce under lock
        let now_ms = chrono::Utc::now().timestamp_millis();
        let last_ms = self.last_failover_epoch_ms.load(Ordering::Acquire);
        let elapsed_ms = now_ms.saturating_sub(last_ms);

        let active_slot = self
            .vault
            .get_active_slot()
            .await
            .ok_or(FailoverError::NoSlotsConfigured)?;

        if last_ms > 0 && elapsed_ms < self.debounce_millis {
            warn!(
                "Quota error burst detected! Debounce active post-lock (elapsed: {}ms < {}ms). Suppressing failover.",
                elapsed_ms, self.debounce_millis
            );
            return Ok(FailoverDecision::Debounced {
                active_slot,
                elapsed_millis: elapsed_ms,
            });
        }

        // 4. Genuine failover: rotate slot and update timestamp inside the lock
        info!(
            "Debounce window passed (elapsed: {}ms >= {}ms). Executing slot failover rotation from slot {}.",
            elapsed_ms, self.debounce_millis, active_slot.id
        );

        let decision = self.rotate_slot(&active_slot.id, now_ms).await?;
        self.last_failover_epoch_ms.store(now_ms, Ordering::Release);
        Ok(decision)
    }

    pub async fn rotate_slot(
        &self,
        failed_slot_id: &str,
        now_ms: i64,
    ) -> Result<FailoverDecision, FailoverError> {
        let now_sec = now_ms / 1000;
        let mut vault_data = self.vault.get_data().await;

        if vault_data.slots.is_empty() {
            return Err(FailoverError::NoSlotsConfigured);
        }

        // Mark failed slot Cooldown(now + cooldown_secs)
        let cooldown_until = now_sec + self.cooldown_secs;
        if let Some(slot) = vault_data.slots.iter_mut().find(|s| s.id == failed_slot_id) {
            slot.state = SlotState::Cooldown {
                until: cooldown_until,
            };
            info!(
                "Slot {} ({}) placed in cooldown until {} ({}s duration)",
                slot.id, slot.email, cooldown_until, self.cooldown_secs
            );
        }

        // Locate next candidate in carousel
        let current_index = vault_data
            .slots
            .iter()
            .position(|s| s.id == failed_slot_id)
            .unwrap_or(0);

        let total_slots = vault_data.slots.len();
        let mut selected_idx = None;

        for offset in 1..total_slots {
            let idx = (current_index + offset) % total_slots;
            let candidate = &vault_data.slots[idx];

            // Verify state is available AND access_token is valid for > 60s
            if candidate.is_ready_for_use(now_sec) {
                selected_idx = Some(idx);
                break;
            }
        }

        let next_idx = match selected_idx {
            Some(idx) => idx,
            None => {
                // If all other slots are exhausted/cooling down, check if any slot has cooldown expiring soonest
                warn!("All candidate slots in carousel are cooling down or disabled!");
                self.vault.save_data(vault_data).await?;
                return Err(FailoverError::AllSlotsExhausted);
            }
        };

        // Activate the selected slot
        vault_data.slots[next_idx].state = SlotState::Active;
        let new_slot = vault_data.slots[next_idx].clone();
        vault_data.active_slot_id = Some(new_slot.id.clone());
        vault_data.updated_at = now_sec;

        self.vault.save_data(vault_data).await?;

        info!(
            "Failover successful: Switched to slot {} ({})",
            new_slot.id, new_slot.email
        );

        Ok(FailoverDecision::Rotated {
            new_slot,
            old_slot_id: failed_slot_id.to_string(),
        })
    }
}
