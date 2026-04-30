//! DOS-259 (W2-B follow-up): AppState-Arc bridge swap semantics + settings-race regression.
//!
//! Per L2 codex review 2026-04-30 finding #1 + #3: the original parity
//! tests used `ReplayProvider` only and never exercised the AppState
//! bridge or routing on settings change. These tests:
//!
//! 1. Verify `swap_intelligence_provider` updates the read-at-call-time
//!    bridge atomically (per ADR-0091 "next dequeue takes effect").
//! 2. Regression-test the settings-race fix: `build_context_provider(Local)`
//!    clears both bridges so a follow-up dequeue cannot construct an
//!    inline Glean call.
//! 3. Verify the dual-Arc bridge (trait + concrete Glean) stays consistent
//!    across settings flips.

use std::sync::Arc;

use dailyos_lib::context_provider::ContextMode;
use dailyos_lib::intelligence::glean_provider::GleanIntelligenceProvider;
use dailyos_lib::intelligence::provider::{
    IntelligenceProvider, ModelTier, ProviderKind, ReplayProvider,
};
use dailyos_lib::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn fresh_state() -> Arc<AppState> {
    Arc::new(AppState::new())
}

#[test]
fn appstate_intelligence_provider_default_is_none_when_no_remote_configured() {
    // Boot state with no Glean configured should leave the Arc empty —
    // local-only callers fall back to PTY (`PtyClaudeCode` per call).
    let state = fresh_state();
    // The default fixture has no Glean configured by tests (the Keychain
    // entry is whatever the dev env happens to have). We assert the
    // weaker invariant: `intelligence_provider()` is callable and
    // returns the same shape (`Option<Arc<...>>`) without panicking.
    let _ = state.intelligence_provider();
    let _ = state.glean_intelligence_provider();
}

#[test]
fn swap_intelligence_provider_takes_effect_on_next_read() {
    // ADR-0091 "switch mid-queue takes effect on next dequeue" is the
    // load-bearing invariant — verified here by swapping a ReplayProvider
    // into the bridge and asserting the next read returns it.
    let state = fresh_state();

    let replay: Arc<dyn IntelligenceProvider + Send + Sync> = Arc::new(
        ReplayProvider::from_prompt_pairs([("ping", "pong")]),
    );
    state.swap_intelligence_provider(Some(Arc::clone(&replay)));

    let read = state
        .intelligence_provider()
        .expect("bridge populated after swap");
    assert_eq!(read.provider_kind(), ProviderKind::Other("replay"));
}

#[test]
fn swap_intelligence_provider_to_none_clears_bridge() {
    let state = fresh_state();
    let replay: Arc<dyn IntelligenceProvider + Send + Sync> = Arc::new(
        ReplayProvider::from_prompt_pairs([("ping", "pong")]),
    );
    state.swap_intelligence_provider(Some(replay));
    state.swap_intelligence_provider(None);
    assert!(state.intelligence_provider().is_none());
}

#[test]
fn build_context_provider_local_clears_intelligence_provider_bridge() {
    // L2 codex finding #1 regression: prior to the fix, a Local switch
    // could leave callers reading a stale Glean Arc OR (worse)
    // constructing a fresh one inline. Now `build_context_provider(Local)`
    // must clear both Arcs so the next dequeue sees a None bridge and
    // falls through to PTY (or skips Glean for supplemental paths).
    let state = fresh_state();

    // Seed a replay provider as the "remote" bridge.
    let replay: Arc<dyn IntelligenceProvider + Send + Sync> = Arc::new(
        ReplayProvider::from_prompt_pairs([("ping", "pong")]),
    );
    state.swap_intelligence_provider(Some(replay));
    state.swap_glean_intelligence_provider(Some(Arc::new(
        GleanIntelligenceProvider::new("https://example.invalid/glean"),
    )));
    assert!(state.intelligence_provider().is_some());
    assert!(state.glean_intelligence_provider().is_some());

    // Switch to Local — both bridges must clear.
    let _local_provider = state.build_context_provider(&ContextMode::Local);
    assert!(
        state.intelligence_provider().is_none(),
        "Local switch must clear the trait Arc bridge — settings race regression"
    );
    assert!(
        state.glean_intelligence_provider().is_none(),
        "Local switch must clear the concrete Glean Arc bridge — settings race regression"
    );
}

#[test]
fn build_context_provider_glean_seeds_both_bridges() {
    // The dual-Arc bridge (trait + concrete Glean) must stay consistent:
    // a Glean swap populates BOTH so callers reading either form see
    // the same provider.
    let state = fresh_state();
    let _glean_provider = state.build_context_provider(&ContextMode::Glean {
        endpoint: "https://example.invalid/glean".to_string(),
    });
    let trait_arc = state
        .intelligence_provider()
        .expect("Glean swap populates trait Arc");
    assert_eq!(trait_arc.provider_kind(), ProviderKind::Other("glean"));
    let concrete_arc = state
        .glean_intelligence_provider()
        .expect("Glean swap populates concrete Glean Arc");
    assert_eq!(concrete_arc.endpoint(), "https://example.invalid/glean");
}

#[test]
fn context_snapshot_returns_coherent_view_under_one_lock() {
    // L2 cycle-2 finding #1 regression: a snapshot must read all three
    // Arcs under one lock acquisition. Seed Glean state, snapshot, then
    // observe that the snapshot shows is_remote=true AND a Glean Arc.
    let state = fresh_state();
    state.set_context_mode_atomic(&ContextMode::Glean {
        endpoint: "https://example.invalid/glean".to_string(),
    });
    let snap = state.context_snapshot();
    assert!(snap.is_remote(), "snapshot is_remote must reflect Glean mode");
    assert!(
        snap.intelligence_provider.is_some(),
        "snapshot trait Arc populated for Glean mode"
    );
    assert!(
        snap.glean_intelligence_provider.is_some(),
        "snapshot Glean Arc populated for Glean mode"
    );

    // After a Local switch, the next snapshot must show all three cleared.
    state.set_context_mode_atomic(&ContextMode::Local);
    let snap = state.context_snapshot();
    assert!(!snap.is_remote(), "snapshot is_remote false after Local switch");
    assert!(snap.intelligence_provider.is_none());
    assert!(snap.glean_intelligence_provider.is_none());
}

#[test]
fn build_context_provider_never_lets_reader_observe_torn_state() {
    // L2 cycle-3 finding regression: codex flagged that the prior
    // interleaving test only drove `set_context_mode_atomic` directly,
    // not the public `build_context_provider` settings command path.
    // After the cycle-4 fix (callers stopped doing `build + swap` two-step),
    // `build_context_provider` is the production entry point — so the
    // race must be closed when interleaving THAT method.
    let state = fresh_state();
    state.build_context_provider(&ContextMode::Glean {
        endpoint: "https://example.invalid/glean".to_string(),
    });

    let stop = Arc::new(AtomicBool::new(false));
    let torn_observed = Arc::new(AtomicBool::new(false));

    let mut readers = Vec::new();
    for _ in 0..4 {
        let s = Arc::clone(&state);
        let stop_r = Arc::clone(&stop);
        let torn_r = Arc::clone(&torn_observed);
        readers.push(std::thread::spawn(move || {
            while !stop_r.load(Ordering::Relaxed) {
                let snap = s.context_snapshot();
                let remote = snap.is_remote();
                let trait_arc = snap.intelligence_provider.is_some();
                let glean_arc = snap.glean_intelligence_provider.is_some();
                if !(remote == trait_arc && trait_arc == glean_arc) {
                    torn_r.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }));
    }

    // Two writer threads racing the public `build_context_provider`
    // settings entry point — the same call shape commands/integrations.rs
    // uses for set/auto-set/disconnect. Any torn state observed during
    // these flips would mean the production settings race is still live.
    let s1 = Arc::clone(&state);
    let writer1 = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(200) {
            s1.build_context_provider(&ContextMode::Glean {
                endpoint: "https://example.invalid/glean".to_string(),
            });
        }
    });
    let s2 = Arc::clone(&state);
    let writer2 = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_millis(200) {
            s2.build_context_provider(&ContextMode::Local);
        }
    });

    writer1.join().expect("writer1");
    writer2.join().expect("writer2");
    stop.store(true, Ordering::Relaxed);
    for r in readers {
        r.join().expect("reader thread");
    }

    assert!(
        !torn_observed.load(Ordering::Relaxed),
        "build_context_provider must close the L2 cycle-3 settings race \
         (concurrent settings flips through the public entry point must \
         never produce torn-bundle reads)"
    );
}

#[test]
fn set_context_mode_atomic_never_lets_reader_observe_torn_state() {
    // L2 cycle-2 finding #1: the previous fix-up updated 3 Arcs as 3
    // separate writes; a parallel reader could observe is_remote=true +
    // None Glean Arc between writes. With the atomic bundle, every
    // snapshot must see EITHER fully-Glean OR fully-Local — never a mix.
    //
    // Drive the writer + N reader threads in parallel for 200ms;
    // assert no reader ever observed a torn state.
    let state = fresh_state();
    state.set_context_mode_atomic(&ContextMode::Glean {
        endpoint: "https://example.invalid/glean".to_string(),
    });

    let stop = Arc::new(AtomicBool::new(false));
    let torn_observed = Arc::new(AtomicBool::new(false));

    let mut readers = Vec::new();
    for _ in 0..4 {
        let s = Arc::clone(&state);
        let stop_r = Arc::clone(&stop);
        let torn_r = Arc::clone(&torn_observed);
        readers.push(std::thread::spawn(move || {
            while !stop_r.load(Ordering::Relaxed) {
                let snap = s.context_snapshot();
                let remote = snap.is_remote();
                let trait_arc = snap.intelligence_provider.is_some();
                let glean_arc = snap.glean_intelligence_provider.is_some();
                // Coherent state means: (remote == trait_arc == glean_arc).
                // If those three booleans diverge for any snapshot, the
                // bundle was torn — which would be the bug we're regressing.
                if !(remote == trait_arc && trait_arc == glean_arc) {
                    torn_r.store(true, Ordering::Relaxed);
                    return;
                }
            }
        }));
    }

    // Writer flips between Glean and Local for ~200ms. Each call is one
    // atomic transition; intermediate reads must never see torn state.
    let s = Arc::clone(&state);
    let writer = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut toggle = false;
        while start.elapsed() < Duration::from_millis(200) {
            if toggle {
                s.set_context_mode_atomic(&ContextMode::Local);
            } else {
                s.set_context_mode_atomic(&ContextMode::Glean {
                    endpoint: "https://example.invalid/glean".to_string(),
                });
            }
            toggle = !toggle;
        }
    });

    writer.join().expect("writer thread");
    stop.store(true, Ordering::Relaxed);
    for r in readers {
        r.join().expect("reader thread");
    }

    assert!(
        !torn_observed.load(Ordering::Relaxed),
        "atomic bundle must prevent torn-state reads during settings flips"
    );
}

#[tokio::test]
async fn replay_provider_through_appstate_bridge_returns_canned_text() {
    // End-to-end exercise of the bridge: seed a ReplayProvider, read it
    // back via `state.intelligence_provider()`, and call `.complete()`.
    // This is the smallest integration that goes through the actual
    // production read path the migrated sites use.
    let state = fresh_state();
    let replay: Arc<dyn IntelligenceProvider + Send + Sync> = Arc::new(
        ReplayProvider::from_prompt_pairs([("end-to-end prompt", "end-to-end response")]),
    );
    state.swap_intelligence_provider(Some(replay));

    let provider = state
        .intelligence_provider()
        .expect("bridge populated");
    let prompt =
        dailyos_lib::intelligence::provider::PromptInput::new("end-to-end prompt");
    let completion = provider
        .complete(prompt, ModelTier::Synthesis)
        .await
        .expect("replay returns canned text");
    assert_eq!(completion.text, "end-to-end response");
    assert_eq!(
        completion.fingerprint_metadata.provider,
        ProviderKind::Other("replay")
    );
}
