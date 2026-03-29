//! `AmpCode` quota snapshot storage — caches free-tier status and balance info
//! intercepted from `/api/internal` responses.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use utoipa::ToSchema;

/// Cached `AmpCode` quota data, assembled from two upstream responses:
/// - `getUserFreeTierStatus` → free-tier flags
/// - `userDisplayBalanceInfo` → balance display text
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AmpQuotaSnapshot {
    /// Whether the user can use Amp's free tier.
    pub can_use_amp_free: Option<bool>,
    /// Whether the daily free grant is enabled.
    pub is_daily_grant_enabled: Option<bool>,
    /// Raw `displayText` array from `userDisplayBalanceInfo`.
    pub balance_display: Option<serde_json::Value>,
    /// Unix timestamp (seconds) of the last `getUserFreeTierStatus` capture.
    pub free_tier_captured_at: Option<u64>,
    /// Unix timestamp (seconds) of the last `userDisplayBalanceInfo` capture.
    pub balance_captured_at: Option<u64>,
}

/// Thread-safe single-value cache for `AmpCode` quota data.
pub struct AmpQuotaStore {
    inner: Mutex<AmpQuotaSnapshot>,
}

impl AmpQuotaStore {
    /// Creates a new empty store.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AmpQuotaSnapshot {
                can_use_amp_free: None,
                is_daily_grant_enabled: None,
                balance_display: None,
                free_tier_captured_at: None,
                balance_captured_at: None,
            }),
        }
    }

    /// Update free-tier status fields from a `getUserFreeTierStatus` response.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn update_free_tier(&self, can_use_amp_free: bool, is_daily_grant_enabled: bool) {
        let mut snap = self.inner.lock().unwrap();
        snap.can_use_amp_free = Some(can_use_amp_free);
        snap.is_daily_grant_enabled = Some(is_daily_grant_enabled);
        snap.free_tier_captured_at = Some(now_secs());
    }

    /// Update balance display from a `userDisplayBalanceInfo` response.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    pub fn update_balance(&self, display_text: serde_json::Value) {
        let mut snap = self.inner.lock().unwrap();
        snap.balance_display = Some(display_text);
        snap.balance_captured_at = Some(now_secs());
    }

    /// Returns a clone of the current snapshot.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned.
    #[must_use]
    pub fn snapshot(&self) -> AmpQuotaSnapshot {
        self.inner.lock().unwrap().clone()
    }
}

impl Default for AmpQuotaStore {
    fn default() -> Self {
        Self::new()
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot() {
        let store = AmpQuotaStore::new();
        let snap = store.snapshot();
        assert!(snap.can_use_amp_free.is_none());
        assert!(snap.balance_display.is_none());
    }

    #[test]
    fn test_update_free_tier() {
        let store = AmpQuotaStore::new();
        store.update_free_tier(true, false);
        let snap = store.snapshot();
        assert_eq!(snap.can_use_amp_free, Some(true));
        assert_eq!(snap.is_daily_grant_enabled, Some(false));
        assert!(snap.free_tier_captured_at.is_some());
    }

    #[test]
    fn test_update_balance() {
        let store = AmpQuotaStore::new();
        store.update_balance(serde_json::json!([{"type": "text", "content": "test"}]));
        let snap = store.snapshot();
        assert!(snap.balance_display.is_some());
        assert!(snap.balance_captured_at.is_some());
    }

    #[test]
    fn test_partial_updates_preserve_other_fields() {
        let store = AmpQuotaStore::new();
        store.update_free_tier(true, true);
        store.update_balance(serde_json::json!("balance"));
        let snap = store.snapshot();
        assert_eq!(snap.can_use_amp_free, Some(true));
        assert!(snap.balance_display.is_some());
    }
}
