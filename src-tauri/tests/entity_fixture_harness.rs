//! DOS-461 — Entity fixture harness + no-bypass checks.
//!
//! Substrate-only test harness. Per `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` §5.3
//! this harness gates W2 entity-detail block ship: any rendering path that does
//! NOT consume `EntityIntelligenceEnvelope` through `get_entity_intelligence`
//! fails the no-bypass check.
//!
//! Structure:
//! - `entity_fixture_harness/fixtures/` — JSON envelope fixtures per
//!   AC-461.5b matrix (Account ✕ Project ✕ Person × fixture class).
//! - `entity_fixture_harness/matrix.rs` — per-subject expected-fixture set;
//!   absent expected fixture = harness fail.
//! - `entity_fixture_harness/dom.rs` — minimal HTML walker for `data-claim-id`
//!   ancestor / text extraction (no external HTML-parser dep).
//! - `entity_fixture_harness/assertions.rs` — `no_bypass`, `binding`,
//!   `stale_vs_bypass` assertion library (AC-461.6a + 6b).
//! - `entity_fixture_harness/accounts.rs` / `projects.rs` / `persons.rs` —
//!   per-subject proof suites (AC-461.2 / 461.3 / 461.4).
//! - `entity_fixture_harness/red_first.rs` — proof tests: bypass fixture
//!   MUST fail, envelope-only fixture MUST pass.
//!
//! Tests are pure-data: each fixture is a serialized
//! `EntityIntelligenceEnvelope` plus a small simulated-rendered-DOM string.
//! No live ability runtime is constructed — the goal is to validate the
//! contract layer (envelope ⇄ rendered surface) deterministically. W2
//! block PHP renderers will plug their actual DOM output into the same
//! assertion library when they ship.

#[path = "entity_fixture_harness/mod.rs"]
mod harness;

// `cargo test --test entity_fixture_harness` discovers `#[test]` functions in
// nested submodules through `mod harness;` above — no re-exports needed.
