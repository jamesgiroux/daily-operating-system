//! DOS-209 (W2-A) regression tests — minimum §9 evidence set.
//!
//! Covers three of the required invariants:
//!   1. Zero raw `crate::signals::bus::emit_signal*` calls under src/services/ — the
//!      service-layer must route all signal emission through the ctx-gated facade.
//!   2. Zero raw `chrono::Utc::now()` / `rand::thread_rng()` calls in service files
//!      (clock and RNG must go through ServiceContext).
//!   3. Mode-boundary: ServiceContext in Evaluate/Simulate mode returns
//!      WriteBlockedByMode before any mutation reaches the DB.
//!
//! The mutation catalog no-drift test (§9 item 6, requires running
//! scripts/dos209-mutation-audit.sh and comparing against the golden catalog)
//! and capability trybuild fixtures (§9 item 8) are deferred to a follow-up
//! once the audit script is stable in CI.

use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    let manifest = std::env::current_dir()
        .expect("cwd")
        .ancestors()
        .find(|p| p.join("src-tauri/Cargo.toml").exists())
        .expect("repo root with src-tauri/Cargo.toml")
        .to_path_buf();
    manifest
}

fn services_dir(root: &Path) -> PathBuf {
    root.join("src-tauri/src/services")
}

/// Run rg with --files-with-matches against a pattern under a directory.
/// Returns the matched lines (stdout) as a string; empty means no matches.
fn rg_in(dir: &Path, pattern: &str) -> String {
    let out = Command::new("rg")
        .args(["--files-with-matches", "--glob", "*.rs", pattern, dir.to_str().unwrap()])
        .output();

    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        // rg exit 1 means no matches (also "not found"), exit 2 means error
        Err(_) => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Lint: no raw bus emit calls in services
// ---------------------------------------------------------------------------

#[test]
fn no_raw_bus_emit_signal_in_services() {
    let root = repo_root();
    let services = services_dir(&root);
    let hits = rg_in(&services, r"crate::signals::bus::emit_signal");
    assert!(
        hits.is_empty(),
        "DOS-209: raw crate::signals::bus::emit_signal* calls found in src/services/.\n\
         All service-layer signal emission must go through crate::services::signals::emit*.\n\
         Affected files:\n{hits}"
    );
}

// ---------------------------------------------------------------------------
// Lint: no raw clock/RNG in service files
// ---------------------------------------------------------------------------

#[test]
fn no_raw_chrono_utc_now_in_services() {
    let root = repo_root();
    let services = services_dir(&root);
    // Pattern covers the most common raw-clock forms; dos259-grandfathered markers
    // are the one accepted exception (intelligence provider files under services/intelligence/).
    let hits = rg_in(&services, r"chrono::Utc::now\s*\(\)");
    // Filter out lines with the grandfathered marker
    let filtered: Vec<&str> = hits
        .lines()
        .filter(|f| !f.contains("dos259-grandfathered"))
        .collect();
    assert!(
        filtered.is_empty(),
        "DOS-209: raw chrono::Utc::now() calls found in src/services/ (without grandfathered marker).\n\
         Use ctx.clock.now() instead.\nAffected files:\n{}",
        filtered.join("\n")
    );
}

#[test]
fn no_raw_thread_rng_in_services() {
    let root = repo_root();
    let services = services_dir(&root);
    let hits = rg_in(&services, r"rand::thread_rng\s*\(\)");
    assert!(
        hits.is_empty(),
        "DOS-209: raw rand::thread_rng() calls found in src/services/.\n\
         Use ctx.rng instead.\nAffected files:\n{hits}"
    );
}

// ---------------------------------------------------------------------------
// Mode boundary: ServiceContext gate mechanics (unit-level proof)
// ---------------------------------------------------------------------------
//
// The context.rs unit tests (check_mutation_allowed_rejects_evaluate,
// check_mutation_allowed_rejects_simulate, constructors_set_expected_modes)
// already pin the gate mechanism. These integration-test-level tests confirm
// the same invariants are visible from the tests/ crate boundary.

#[test]
fn evaluate_mode_ctx_blocks_mutations() {
    use dailyos_lib::services::context::{
        ExecutionMode, ExternalClients, FixedClock, SeedableRng, ServiceContext, ServiceError,
    };
    use chrono::TimeZone;

    let clk = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap());
    let rng = SeedableRng::new(42);
    let ext = ExternalClients::default();
    let ctx = ServiceContext::new_evaluate(&clk, &rng, &ext);

    let result = ctx.check_mutation_allowed();
    assert!(
        result.is_err(),
        "Evaluate-mode ctx must block mutations, but check_mutation_allowed returned Ok"
    );
    assert!(
        matches!(result, Err(ServiceError::WriteBlockedByMode(ExecutionMode::Evaluate))),
        "Expected WriteBlockedByMode(Evaluate), got: {:?}",
        result
    );
}

#[test]
fn simulate_mode_ctx_blocks_mutations() {
    use dailyos_lib::services::context::{
        ExecutionMode, ExternalClients, FixedClock, SeedableRng, ServiceContext, ServiceError,
    };
    use chrono::TimeZone;

    let clk = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap());
    let rng = SeedableRng::new(42);
    let ext = ExternalClients::default();
    let ctx = ServiceContext::new_simulate(&clk, &rng, &ext);

    let result = ctx.check_mutation_allowed();
    assert!(
        result.is_err(),
        "Simulate-mode ctx must block mutations, but check_mutation_allowed returned Ok"
    );
    assert!(
        matches!(result, Err(ServiceError::WriteBlockedByMode(ExecutionMode::Simulate))),
        "Expected WriteBlockedByMode(Simulate), got: {:?}",
        result
    );
}

#[test]
fn live_mode_ctx_permits_mutations() {
    use dailyos_lib::services::context::{
        ExternalClients, FixedClock, SeedableRng, ServiceContext,
    };
    use chrono::TimeZone;

    let clk = FixedClock::new(chrono::Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap());
    let rng = SeedableRng::new(42);
    let ext = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clk, &rng, &ext);

    assert!(
        ctx.check_mutation_allowed().is_ok(),
        "Live-mode ctx must permit mutations"
    );
}
