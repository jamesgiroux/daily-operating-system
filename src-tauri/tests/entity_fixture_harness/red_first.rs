//! Red-first no-bypass proof tests.
//!
//! Demonstrates the assertion library actually catches the regressions it
//! claims to. Two paired fixtures:
//!
//! - `__bad_account_legacy_bypass`  — a renderer SOURCE that imports the
//!   legacy `get_entity_context_entries` reader AND a rendered DOM whose
//!   claim text has no `[data-claim-id]` ancestor. Audit MUST fail.
//!
//! - `__good_account_envelope_only` — same surface, sourced from
//!   `get_entity_intelligence` envelope only, rendered DOM has
//!   `[data-claim-id]` ancestors on every claim-substantive text node.
//!   Audit MUST pass.
//!
//! The renderer SOURCE strings are inlined in the test (not on disk) — they
//! represent the rendering-layer code text the harness scans. The DOM
//! strings ARE rendered output. Both halves of AC-461.6a are exercised.

use crate::harness::assertions::audit_render;
use crate::harness::load_envelope;

// Renderer SOURCE — what the W2 block/PHP/React renderer's code text looks like.
// Harness scans this for bypass identifiers per AC-461.6a static check.

const LEGACY_BYPASS_RENDERER_SOURCE: &str = r#"
    // Simulated W2 block render — legacy bypass path.
    let entries = get_entity_context_entries(account_id);
    let rendered = compose_account_intelligence(entries);
    return rendered.html;
"#;

const ENVELOPE_ONLY_RENDERER_SOURCE: &str = r#"
    // Simulated W2 block render — envelope path (the contract).
    let envelope = get_entity_intelligence({
        entity_type: "account",
        entity_id: account_id,
        depth: "standard",
    });
    return render_envelope_to_dom(envelope);
"#;

// Rendered DOM strings. The "bad" version emits bare claim text without
// data-claim-id. The "good" version wraps each claim text in a
// data-claim-id ancestor matching the envelope.

const LEGACY_BYPASS_RENDERED_DOM: &str = r#"
<article class="account-detail">
  <section>
    <h2>Facts</h2>
    <div>
      <span>account-zero is the customer-zero workspace</span>
    </div>
  </section>
</article>
"#;

fn good_envelope_rendered_dom(claim_id: &str) -> String {
    format!(
        r#"
<article class="account-detail">
  <section>
    <h2>Facts</h2>
    <div data-claim-id="{claim_id}">
      <span>account-zero is the customer-zero workspace</span>
    </div>
  </section>
</article>
"#
    )
}

#[test]
fn red_first_bypass_fixture_fails_audit() {
    let env = load_envelope("__good_envelope_canonical.json")
        .unwrap_or_else(|e| panic!("canonical envelope load failed: {e}"));
    let report = audit_render(LEGACY_BYPASS_RENDERER_SOURCE, LEGACY_BYPASS_RENDERED_DOM, &env);
    assert!(
        !report.passes(),
        "red-first proof — bypass renderer source + unbound DOM MUST fail audit; report={report:?}"
    );
    assert!(
        report.bypass_count >= 2,
        "red-first proof — bypass scan must catch BOTH get_entity_context_entries AND compose_account_intelligence; matches={:?}",
        report.bypass_matches
    );
    assert!(
        report.unbound_text_count >= 1,
        "red-first proof — unbound claim text must surface as binding failure"
    );
    // The "good" envelope has no claim IDs in this DOM at all, so stale
    // count is 0 — bypass class is the failure mode here, not stale.
    assert_eq!(report.stale_render_count, 0);
}

#[test]
fn red_first_envelope_only_fixture_passes_audit() {
    let env = load_envelope("__good_envelope_canonical.json")
        .unwrap_or_else(|e| panic!("canonical envelope load failed: {e}"));
    let canonical_claim_id = env
        .facts
        .items
        .first()
        .map(|f| f.claim_id.clone())
        .expect("canonical envelope must include at least one fact");
    let dom = good_envelope_rendered_dom(&canonical_claim_id);
    let report = audit_render(ENVELOPE_ONLY_RENDERER_SOURCE, &dom, &env);
    assert!(
        report.passes(),
        "green-path proof — envelope-only renderer MUST pass audit; report={report:?}"
    );
}

#[test]
fn red_first_stale_vs_bypass_distinguishes_failure_modes() {
    // Cached DOM references a claim_id NOT present in the envelope.
    // Per AC-461.6b this is *stale*, NOT bypass.
    let env = load_envelope("account_claim_retracted_mid_render.json")
        .unwrap_or_else(|e| panic!("retraction fixture load failed: {e}"));
    let stale_dom = r#"
        <article>
            <div data-claim-id="account-claim-retracted-1">
                <span>cached render of a retracted claim</span>
            </div>
        </article>
    "#;
    let report = audit_render(ENVELOPE_ONLY_RENDERER_SOURCE, stale_dom, &env);
    assert_eq!(
        report.bypass_count, 0,
        "stale render with envelope-only source MUST NOT count as bypass"
    );
    assert_eq!(
        report.unbound_text_count, 0,
        "stale render's text has a data-claim-id ancestor — must NOT count as unbound bypass"
    );
    assert!(
        report.stale_render_count >= 1,
        "stale render MUST count as stale per AC-461.6b; report={report:?}"
    );
}
