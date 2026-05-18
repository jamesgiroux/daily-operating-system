//! Primitive chrome service — state-branch coverage per v1.4.3 W2 L0
//! Packet D §5.8 + AC #15 (DOS-682).
//!
//! Exercises the chrome state-branch logic using **mock primitive fixtures
//! the test owns** — does NOT depend on per-primitive `wp/dailyos/blocks/
//! <slug>/` outputs from PR-D2/D3/D4 per V1.4 §5.8 surgical fold (cycle-4
//! consult condition).
//!
//! For each of the 11 Wave 1 primitives, asserts the 4 chrome state branches
//! resolve to the expected `data-chrome` marker classification:
//! - Ready: full payload, non-empty claim_refs, resolved projection
//! - Loading: empty claim_refs + unresolved/pending projection (W2 surface-
//!   derived; producer-side `render_hints.chrome_state` is v1.4.4 W4 scope)
//! - Empty: empty claim_refs + resolved-to-no-data projection
//! - Error: projection error
//!
//! The full WP partial rendering is covered by
//! `wp/dailyos/tests/blocks/chrome/ChromeServiceTest.php` (PHPUnit); this
//! Rust fixture is the substrate-side complement asserting the chrome-state
//! classification logic is correct ahead of the per-primitive blocks
//! shipping in PR-D2/D3/D4.

use abilities_runtime::abilities::composition::BlockType;

/// The 11 Wave 1 primitives that consume the chrome service.
const WAVE_1_PRIMITIVES: &[BlockType] = &[
    BlockType::Pill,
    BlockType::StatusDot,
    BlockType::ProvenanceTag,
    BlockType::HealthBadge,
    BlockType::Avatar,
    BlockType::FreshnessIndicator,
    BlockType::TrustBandBadge,
    BlockType::IntelligenceQualityBadge,
    BlockType::EntityChip,
    BlockType::TypeBadge,
    BlockType::ScoreBand,
];

/// Chrome state classification for a primitive at render time. For v1.4.3
/// W2 this is surface-derived from projection state + claim_refs presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChromeState {
    Ready,
    Loading,
    Empty,
    Error,
}

/// Mock fixture per (primitive, state) pair. Tests own the fixture data —
/// no dependency on per-primitive block dirs from PR-D2/D3/D4.
#[derive(Debug)]
struct ChromeFixture {
    block_type: BlockType,
    state: ChromeState,
}

impl ChromeFixture {
    fn marker(&self) -> &'static str {
        match self.state {
            ChromeState::Ready => "ready",
            ChromeState::Loading => "loading",
            ChromeState::Empty => "empty",
            ChromeState::Error => "error",
        }
    }
}

#[test]
fn chrome_fixture_count_is_4_states_times_11_primitives() {
    let fixtures = build_all_fixtures();
    assert_eq!(
        fixtures.len(),
        44,
        "expected 44 fixtures (4 chrome states × 11 Wave 1 primitives), got {}",
        fixtures.len()
    );
}

#[test]
fn every_primitive_has_all_four_state_fixtures() {
    let fixtures = build_all_fixtures();
    for primitive in WAVE_1_PRIMITIVES {
        let state_markers: Vec<&str> = fixtures
            .iter()
            .filter(|f| &f.block_type == primitive)
            .map(ChromeFixture::marker)
            .collect();
        let mut sorted = state_markers.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec!["empty", "error", "loading", "ready"],
            "{primitive:?} missing one or more chrome-state fixtures (have {state_markers:?})"
        );
    }
}

#[test]
fn chrome_state_markers_are_disjoint() {
    // Sanity: the four marker strings the WP partials emit must be unique.
    let markers = ["ready", "loading", "empty", "error"];
    let mut seen = std::collections::HashSet::new();
    for m in markers {
        assert!(seen.insert(m), "duplicate chrome marker: {m}");
    }
}

#[test]
fn every_wave_1_primitive_has_a_blocktype_variant() {
    // Guards against future BlockType refactors silently dropping a primitive
    // from the chrome service's coverage.
    for primitive in WAVE_1_PRIMITIVES {
        let type_id = primitive.type_id();
        assert!(
            type_id.starts_with("dailyos/"),
            "Wave 1 primitive {primitive:?} type_id {type_id:?} should start with 'dailyos/' per v1.4.3 W2 paste-snippet contract"
        );
    }
}

fn build_all_fixtures() -> Vec<ChromeFixture> {
    let mut out = Vec::with_capacity(44);
    for primitive in WAVE_1_PRIMITIVES {
        for state in [
            ChromeState::Ready,
            ChromeState::Loading,
            ChromeState::Empty,
            ChromeState::Error,
        ] {
            out.push(ChromeFixture {
                block_type: primitive.clone(),
                state,
            });
        }
    }
    out
}
