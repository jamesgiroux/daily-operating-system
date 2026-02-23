//! User activity monitoring for background task throttling.
//!
//! Background tasks read `ActivityLevel` to decide poll intervals:
//! - `Active`: user is interacting with the app — throttle aggressively
//! - `Idle`: window focused but no interaction for 2+ minutes
//! - `Background`: window not focused — background work runs freely

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Tracks user activity state for background task throttling.
pub struct ActivityMonitor {
    window_focused: AtomicBool,
    last_interaction_at: AtomicU64,
    workflow_active: AtomicBool,
}

/// Activity level that background tasks use to select poll intervals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityLevel {
    /// User is actively using the app. Maximum throttle on background work.
    Active,
    /// Window focused but no interaction for 2+ minutes.
    Idle,
    /// Window not focused. Background work runs freely.
    Background,
}

/// Idle threshold: no interaction for this many seconds → Idle (not Active).
const IDLE_THRESHOLD_SECS: u64 = 120;

impl Default for ActivityMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityMonitor {
    pub fn new() -> Self {
        Self {
            window_focused: AtomicBool::new(false),
            last_interaction_at: AtomicU64::new(0),
            workflow_active: AtomicBool::new(false),
        }
    }

    /// Called when the app window gains or loses focus.
    pub fn set_window_focused(&self, focused: bool) {
        self.window_focused.store(focused, Ordering::Relaxed);
        if focused {
            self.touch();
        }
    }

    /// Called on user interaction (click, keypress, navigation).
    pub fn touch(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_interaction_at.store(now, Ordering::Relaxed);
    }

    /// Called when a workflow starts or finishes.
    pub fn set_workflow_active(&self, active: bool) {
        self.workflow_active.store(active, Ordering::Relaxed);
    }

    /// Returns the current activity level.
    pub fn level(&self) -> ActivityLevel {
        if !self.window_focused.load(Ordering::Relaxed) {
            return ActivityLevel::Background;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let last = self.last_interaction_at.load(Ordering::Relaxed);
        if now.saturating_sub(last) < IDLE_THRESHOLD_SECS {
            ActivityLevel::Active
        } else {
            ActivityLevel::Idle
        }
    }

    /// Whether a workflow (Today, Week, etc.) is currently running.
    pub fn is_workflow_active(&self) -> bool {
        self.workflow_active.load(Ordering::Relaxed)
    }
}

/// Returns an adaptive poll interval for background queue processors.
///
/// When work is queued, process promptly (2s). When idle and user is active,
/// back off significantly (30s). When app is in background, run at full speed.
pub fn adaptive_poll_interval(activity: &ActivityMonitor, queue_empty: bool) -> Duration {
    if !queue_empty {
        return Duration::from_secs(2);
    }
    match activity.level() {
        ActivityLevel::Active => Duration::from_secs(30),
        ActivityLevel::Idle => Duration::from_secs(15),
        ActivityLevel::Background => Duration::from_secs(5),
    }
}

/// Returns an adaptive poll interval for network pollers (calendar, email).
pub fn adaptive_network_interval(activity: &ActivityMonitor) -> Duration {
    match activity.level() {
        ActivityLevel::Active => Duration::from_secs(120),
        ActivityLevel::Idle => Duration::from_secs(60),
        ActivityLevel::Background => Duration::from_secs(30),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_when_not_focused() {
        let m = ActivityMonitor::new();
        assert_eq!(m.level(), ActivityLevel::Background);
    }

    #[test]
    fn test_active_after_focus_and_touch() {
        let m = ActivityMonitor::new();
        m.set_window_focused(true); // also calls touch()
        assert_eq!(m.level(), ActivityLevel::Active);
    }

    #[test]
    fn test_adaptive_intervals() {
        let m = ActivityMonitor::new();
        // Background + empty queue → 5s
        assert_eq!(adaptive_poll_interval(&m, true), Duration::from_secs(5));
        // Background + work queued → 2s
        assert_eq!(adaptive_poll_interval(&m, false), Duration::from_secs(2));
    }
}
