// Action status and priority constants — single source of truth.
//
// All action status/priority values used in SQL queries, Rust logic,
// and frontend communication reference these constants. Changing a
// value here propagates everywhere (DOS-55).

// ── Status values ──────────────────────────────────────────────────

/// AI-proposed items awaiting user triage.
pub const BACKLOG: &str = "backlog";

/// User-accepted items not yet actively worked.
pub const UNSTARTED: &str = "unstarted";

/// Actively being worked (set by push-to-Linear).
pub const STARTED: &str = "started";

/// Done.
pub const COMPLETED: &str = "completed";

/// Explicitly killed by the user.
pub const CANCELLED: &str = "cancelled";

/// Zero-guilt fadeout — stale items age into this.
pub const ARCHIVED: &str = "archived";

/// All valid action statuses for CHECK constraints and validation.
pub const ALL_STATUSES: &[&str] = &[BACKLOG, UNSTARTED, STARTED, COMPLETED, CANCELLED, ARCHIVED];

/// Statuses that represent "open" (active, not terminal).
pub const OPEN_STATUSES: &[&str] = &[BACKLOG, UNSTARTED, STARTED];

/// Statuses that represent "closed" (terminal).
pub const CLOSED_STATUSES: &[&str] = &[COMPLETED, CANCELLED, ARCHIVED];

// ── Priority values ────────────────────────────────────────────────

/// No priority set.
pub const PRIORITY_NONE: i32 = 0;

/// Urgent priority (Linear: Urgent).
pub const PRIORITY_URGENT: i32 = 1;

/// High priority (Linear: High).
pub const PRIORITY_HIGH: i32 = 2;

/// Medium priority — default (Linear: Medium).
pub const PRIORITY_MEDIUM: i32 = 3;

/// Low priority (Linear: Low).
pub const PRIORITY_LOW: i32 = 4;

/// Default priority for new actions.
pub const PRIORITY_DEFAULT: i32 = PRIORITY_MEDIUM;

/// All valid priority values.
pub const ALL_PRIORITIES: &[i32] = &[
    PRIORITY_NONE,
    PRIORITY_URGENT,
    PRIORITY_HIGH,
    PRIORITY_MEDIUM,
    PRIORITY_LOW,
];

/// Map old string priority to new integer.
pub fn migrate_priority(old: &str) -> i32 {
    match old {
        "P1" => PRIORITY_URGENT,
        "P2" => PRIORITY_MEDIUM,
        "P3" => PRIORITY_LOW,
        _ => PRIORITY_MEDIUM,
    }
}

/// Human-readable label for a priority integer.
pub fn priority_label(priority: i32) -> &'static str {
    match priority {
        0 => "None",
        1 => "Urgent",
        2 => "High",
        3 => "Medium",
        4 => "Low",
        _ => "Medium",
    }
}
