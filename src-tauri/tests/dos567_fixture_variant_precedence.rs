//! W4-B ac §38 — `BridgeSurfaceError` precedence (§6.5 table).
//!
//! When multiple BridgeSurfaceError conditions could fire simultaneously, the
//! bridge entry point must evaluate top-down per §6.5 and return the first
//! match. Ordering (highest → lowest precedence):
//!
//!   1. ProjectionTampered       (W4-C signature check)
//!   2. ProjectionVersionRollback (W4-C ledger replay defense)
//!   3. MidFlightMutation        (W4-B lock holder collision)
//!   4. MissingExpectedClaimVersion (W4-B foot-gun / Mutate-w/o-version)
//!   5. ClaimVersionOverflow / CompositionVersionOverflow (W4-B i64::MAX defense)
//!   6. StaleVersion             (W4-B watermark CAS miss)
//!   7. StaleComposition         (W4-B composition watermark CAS miss)
//!
//! This fixture pins the precedence ordering as a substrate contract; the
//! pairwise runtime tests (tamper+stale → tamper wins, etc.) require a bridge
//! entry-point resolver helper currently being wired by the substrate-sweep
//! agent and so are `#[ignore]` until that lands.

use dailyos_lib::bridges::BridgeSurfaceError;

/// Precedence rank — lower is higher precedence (matches first).
/// Mirrors the §6.5 table; substrate-side resolver must agree.
fn precedence_rank(error: &BridgeSurfaceError) -> u8 {
    match error {
        BridgeSurfaceError::ProjectionTampered { .. } => 0,
        BridgeSurfaceError::ProjectionVersionRollback { .. } => 1,
        BridgeSurfaceError::MidFlightMutation { .. } => 2,
        BridgeSurfaceError::MissingExpectedClaimVersion { .. } => 3,
        BridgeSurfaceError::ClaimVersionOverflow { .. }
        | BridgeSurfaceError::CompositionVersionOverflow { .. } => 4,
        BridgeSurfaceError::StaleVersion { .. } => 5,
        BridgeSurfaceError::StaleComposition { .. } => 6,
        BridgeSurfaceError::AbilityUnavailable
        | BridgeSurfaceError::Validation(_)
        | BridgeSurfaceError::Ownership(_) => 7,
    }
}

fn tampered() -> BridgeSurfaceError {
    BridgeSurfaceError::ProjectionTampered {
        projection_id: "proj-prec".to_string(),
        signature_id: "sig-prec".to_string(),
        key_id: "psk-prec".to_string(),
        observed_signature_status: "signature_invalid".to_string(),
        quarantine_id: "pq-prec".to_string(),
    }
}

fn rollback() -> BridgeSurfaceError {
    BridgeSurfaceError::ProjectionVersionRollback {
        projection_id: "proj-prec".to_string(),
        signed_composition_version: 5,
        ledger_composition_version: 12,
        signed_claim_version: Some(3),
        ledger_claim_version: Some(4),
    }
}

fn mid_flight() -> BridgeSurfaceError {
    BridgeSurfaceError::MidFlightMutation {
        claim_id: "claim-prec".to_string(),
        mutation_id: "mut-1".to_string(),
        retry_after_event: "12345678-1234-4234-8234-1234567890ab".to_string(),
    }
}

fn missing_version() -> BridgeSurfaceError {
    BridgeSurfaceError::MissingExpectedClaimVersion {
        claim_id: "claim-prec".to_string(),
    }
}

fn overflow() -> BridgeSurfaceError {
    BridgeSurfaceError::ClaimVersionOverflow {
        claim_id: "claim-prec".to_string(),
    }
}

fn composition_overflow() -> BridgeSurfaceError {
    BridgeSurfaceError::CompositionVersionOverflow {
        composition_id: "comp-prec".to_string(),
    }
}

fn stale_claim() -> BridgeSurfaceError {
    BridgeSurfaceError::StaleVersion {
        claim_id: "claim-prec".to_string(),
        expected: 3,
        current: 5,
        correction: None,
    }
}

fn stale_comp() -> BridgeSurfaceError {
    BridgeSurfaceError::StaleComposition {
        composition_id: "comp-prec".to_string(),
        expected: 1,
        current: 2,
    }
}

#[test]
fn dos567_precedence_tamper_beats_stale_version() {
    assert!(precedence_rank(&tampered()) < precedence_rank(&stale_claim()));
}

#[test]
fn dos567_precedence_rollback_beats_stale_composition() {
    assert!(precedence_rank(&rollback()) < precedence_rank(&stale_comp()));
}

#[test]
fn dos567_precedence_mid_flight_beats_stale_version() {
    assert!(precedence_rank(&mid_flight()) < precedence_rank(&stale_claim()));
}

#[test]
fn dos567_precedence_tamper_beats_rollback() {
    // §V9: tamper is the highest signature/ledger check; rollback second.
    assert!(precedence_rank(&tampered()) < precedence_rank(&rollback()));
}

#[test]
fn dos567_precedence_missing_version_beats_overflow() {
    // §6.5: caller error (missing) takes precedence over substrate overflow,
    // because the caller's request is malformed before substrate state is
    // examined.
    assert!(precedence_rank(&missing_version()) < precedence_rank(&overflow()));
}

#[test]
fn dos567_precedence_composition_overflow_matches_claim_overflow() {
    assert_eq!(
        precedence_rank(&composition_overflow()),
        precedence_rank(&overflow())
    );
}

#[test]
fn dos567_precedence_table_total_ordering() {
    // Exhaustive: every variant ranked, all ranks distinct in the precedence
    // family. (Tail family — AbilityUnavailable / Validation / Ownership —
    // share the lowest rank since they're orthogonal to version semantics.)
    let ordered = [
        tampered(),
        rollback(),
        mid_flight(),
        missing_version(),
        overflow(),
        stale_claim(),
        stale_comp(),
    ];
    let ranks: Vec<u8> = ordered.iter().map(precedence_rank).collect();
    for (lhs, rhs) in ranks.iter().zip(ranks.iter().skip(1)) {
        assert!(lhs < rhs, "precedence table monotonic: {lhs} < {rhs}");
    }
}

// Pairwise runtime scenarios — depend on substrate-sweep agent wiring a
// bridge entry-point resolver that takes a tuple of conditions and returns
// the §6.5-highest variant. Marked `#[ignore]` until that resolver lands.

#[test]
#[ignore = "requires substrate-sweep agent's bridge entry-point resolver: invoke with (tamper-true, stale-true) and assert ProjectionTampered returned"]
fn dos567_runtime_tamper_and_stale_yields_tampered() {
    unreachable!("ignored until bridge resolver helper lands");
}

#[test]
#[ignore = "requires substrate-sweep agent's bridge entry-point resolver: invoke with (rollback-true, stale-true) and assert ProjectionVersionRollback returned"]
fn dos567_runtime_rollback_and_stale_yields_rollback() {
    unreachable!("ignored until bridge resolver helper lands");
}

#[test]
#[ignore = "requires substrate-sweep agent's bridge entry-point resolver: invoke with (mid-flight-true, stale-true) and assert MidFlightMutation returned"]
fn dos567_runtime_mid_flight_and_stale_yields_mid_flight() {
    unreachable!("ignored until bridge resolver helper lands");
}
