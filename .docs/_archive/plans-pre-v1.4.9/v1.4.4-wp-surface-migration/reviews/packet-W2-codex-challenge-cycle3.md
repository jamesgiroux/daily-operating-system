src-tauri/src/services/claim_receipt/feedback.rs:1035:        let err = validate_and_sanitize_metadata(FeedbackAction::MergeIntent, None).unwrap_err();
src-tauri/src/services/claim_receipt/feedback.rs:1038:            FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1039:            Some(&serde_json::json!({"merge_target": {"person": "person-canonical-1"}})),
src-tauri/src/services/claim_receipt/feedback.rs:1041:        .expect("merge_intent with merge_target only");
src-tauri/src/services/claim_receipt/feedback.rs:1043:            FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1045:                "merge_target": {"person": "person-canonical-1"},
src-tauri/src/services/claim_receipt/feedback.rs:1049:        .expect("merge_intent with merge_target + supporting_evidence");
src-tauri/src/services/claim_receipt/feedback.rs:1050:        // Reject malformed merge_target.
src-tauri/src/services/claim_receipt/feedback.rs:1052:            FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1053:            Some(&serde_json::json!({"merge_target": 42})),
src-tauri/src/services/claim_receipt/feedback.rs:1056:        assert!(matches!(err, FeedbackError::BadRequest(message) if message.contains("merge_target")));
src-tauri/src/services/claim_receipt/feedback.rs:1062:            FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1064:                "merge_target": {"person": "person-canonical-1"},
src-tauri/src/services/claim_receipt/feedback.rs:1083:            FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1085:                "merge_target": {"person": "person-1"},
src-tauri/src/services/claim_receipt/feedback.rs:1096:            FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1098:                "merge_target": {"person": "person-1"},
src-tauri/src/services/claim_receipt/feedback.rs:1294:        // AC-8.13 extension (ADR-0123 V1.1): MergeIntent is user-only.
src-tauri/src/services/claim_receipt/feedback.rs:1318:                    action: FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1321:                        "merge_target": {"person": "person-canonical-1"}
src-tauri/src/services/claim_receipt/feedback.rs:1330:                "surface {:?} must deny agent for MergeIntent",
src-tauri/src/services/claim_receipt/feedback.rs:1338:        // MergeIntent persists the typed proposal as a claim_feedback row
src-tauri/src/services/claim_receipt/feedback.rs:1356:                action: FeedbackAction::MergeIntent,
src-tauri/src/services/claim_receipt/feedback.rs:1359:                    "merge_target": {"person": "person-canonical-1"},
src-tauri/src/services/claim_receipt/feedback.rs:1370:            "MergeIntent must not mutate claim verification_state");
src-tauri/src/services/claim_receipt/feedback.rs:1372:            "MergeIntent must not enqueue a repair job");

exec
/bin/zsh -lc "nl -ba .docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md | sed -n '873,893p'; nl -ba .docs/decisions/0123-typed-claim-feedback-semantics.md | sed -n '108,130p'" in /Users/jamesgiroux/Documents/dailyos-repo
 succeeded in 0ms:
   873	## 10. Architecture invariants
   874	
   875	W2 inherits ALL wave-level invariants from `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` V1.1 §10. Notable for W2:
   876	
   877	| Invariant (wave §10 source) | W2 application |
   878	|---|---|
   879	| **Outer/inner block contract** | Every entity-detail composite is 1 outer + N inner per §5; primitives stay inserter-global (no `parent`); context via `providesContext`/`usesContext`; render.php calls `do_blocks($content)` for inner-block reordering. |
   880	| **Entity list pagination contract** | List shells consume `executeAbility()` (WP 7.0) via `useAbilityCursor()` shared hook at `wp/dailyos/blocks/_shared/hooks/`; NO `useEntityRecords`. Cursor opaque + server-encoded. Reset on watermark change. Outer envelope carries first-page slice per wave §13 Q1 sub-clause. |
   881	| **Per-project tint via CSS custom property** | `dailyos/project-detail` wrapper carries `--dailyos-project-tint: var(--color-garden-olive)` (DOS-725 locked); narrow exception to `no inline CSS` rule per memory `feedback_no_inline_css`. |
   882	| **Refresh model: pull-on-render + user refresh** | All 4 composites + 3 list shells + metadata-proposal inner blocks invoke producer on render; staleness via `FreshnessIndicator` + trust-band downgrade. NO push-invalidation bus at v1.4.4. **Exception:** Meeting Detail's prep-status surfaces a `MeetingPrepStatusChanged` signal via chrome.js — chrome-lane primitive, not a W2-novel push. |
   883	| **Block apiVersion 3 mandatory** | Every new W2 block.json declares `apiVersion: 3`. CI gate optional but lint-checkable. |
   884	| **Substrate-in-same-wave (C4)** | If L0 review surfaces a substrate gap blocking a named W2 AC, W1 reopens — NOT deferred to v1.4.5+. C4 supersedes path-α. |
   885	| **AgentMcp audience filter** | Every claim-bearing inner block routes receipt through `build_receipt_for_audience(target, audience, conn)` (W1 DOS-341); AgentMcp audience row's allowlist (trust band, coarsened freshness, redaction level, lifecycle, sanitized evidence_summary, subject_type) enforced. Negative fixture per W1 AC-341.12. |
   886	| **Inline-edit-affordance contract (anchored decision #2)** | Every correction emits `FeedbackAction` through `record_claim_feedback`; NO direct PHP/JS DB writes. |
   887	| **Chrome runtime-injection scope** | Body content stays Gutenberg blocks. Chrome (FolioBar / FloatingNavIsland / AtmosphereLayer / MagazinePageLayout) is the only runtime-injection lane. W2 introduces NO new runtime-injection module. |
   888	| **Design-system canonicity** | Every chapter inner block translates an existing canonical design (`src/pages/AccountDetailPage.tsx`, `ProjectDetailEditorial.tsx`, `PersonDetailEditorial.tsx`, `MeetingDetailPage.tsx`, `.docs/design/patterns/*`) — NO new visual patterns invented in `wp/dailyos/blocks/`. New patterns land canonical-first with `ce-design-lens-reviewer` approval. |
   889	| **L2 bounded by acceptance criteria** | Path-α findings → maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. C4-blocking findings reopen W1. |
   890	| **Code-shape sketch obligation (NEW V1.1, per architecture A1 + wp-skill H1)** | Every block.json declaration named in §5 ships with a concrete code-shape sketch — block.json + render.php skeleton — alongside the prose. No spec-named-in-prose-only patterns. If a third spec-vs-sketch gap surfaces in cycle 2, switch to a class-wide sweep over §5. |
   891	| **Empty-state pattern (NEW V1.1, per design-lens F1)** | Every inner block renders empty as a quiet chip with `data-empty-reason="<reason>"` attribute. NEVER silent-hidden (`return ''` from render.php is non-conforming). Reasons inherit envelope's `SectionState::Empty { reason: EmptyReason }` where applicable, surface-local otherwise. Anchors a stable QA target across 60+ inner blocks. |
   892	| **AgentMcp touchpoint aggregate render (NEW V1.1, per design-lens F2 + codex-challenge F4)** | For AgentMcp audience, `dailyos/touchpoints-feed` (and any other touchpoint-bearing inner block) emits `{ count: u64, recency: Recent | Aging | Stale, content: redacted }` only. NO per-item rows, NO titles, NO `source_asof` timestamps, NO per-row trust bands. Recency tier resolves coarsely server-side from the touchpoint set; no timing oracle leaks. Negative fixture asserts the aggregate-only shape (per W1 AC-341.12). |
   893	
   108	    /// User says the claim is true but not relevant to this meeting/context.
   109	    /// → context-binding hint recorded against the ability invocation that
   110	    ///   surfaced it (relevance signal, not truth signal); no trust delta.
   111	    NotRelevantHere { invocation_id: InvocationId },
   112	
   113	    /// (V1.1 — 2026-05-21) User nominates a canonical merge target for an
   114	    /// ambiguous entity (Person Detail merge picker; v1.4.4 W2 §5.3 /
   115	    /// DOS-484).
   116	    /// → persists a typed proposal row only; no trust/lifecycle change on
   117	    ///   the source claim. The merge execution flow (DOS-484) consumes
   118	    ///   the typed payload and runs the subject rebind.
   119	    /// → User-only; Agent actor denied at every surface (AC-8.13).
   120	    MergeIntent { merge_target: SubjectRef, supporting_evidence: Option<String> },
   121	}
   122	```
   123	
   124	Ten variants, each mapping to a distinct triple of (claim state, source weight, agent ledger). No 5-point scale. No yes/no.
   125	
   126	> **Implementation note:** the Rust enum is unit-only (`#[derive(Copy)]`); per-variant data (`corrected_to`, `corrected_text`, `merge_target`, etc.) is carried via the `payload_json` field on `ClaimFeedback` and validated against this ADR's schema by `services::claim_receipt::feedback::validate_and_sanitize_metadata`. The variant-with-fields notation above is the canonical conceptual shape.
   127	
   128	### 2. `ClaimFeedback` row shape (closed)
   129	
   130	```rust

codex
1. PATCHED CORRECTLY — Account now `~24` with `FileList` in scope/template/projection: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:154`, `:216`, `:274`. Project now `~15` with `linear-issues-chapter`: `:155`, `:376`, `:419`.

2. PATCHED CORRECTLY — DOS-725 regex is `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$`, `--dailyos-*` only: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:398`, `:834`.

3. PATCHED CORRECTLY — Template arrays present for §5.2 Project, §5.3 Person, §5.4 Meeting: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:363`, `:461`, `:558`.

4. PATCHED CORRECTLY — AC-462.8 says `Filesystem pattern`, theme-registered, not DB-stored synced pattern: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:328`.

5. PATCHED CORRECTLY — Empty sketch emits visible `dailyos-empty-chip` with `data-empty-reason`: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:300`, `:303`.

6. NEW DEFECT — MergeIntent substrate exists, but W2 packet still specifies wrong payload shape: `FeedbackAction::MergeIntent { source_person_id, target_person_id }` at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:485`, `:517`. ADR/code canonical shape is unit enum + `payload_json.merge_target` / optional `supporting_evidence`: `.docs/decisions/0123-typed-claim-feedback-semantics.md:120`, `:126`; `src-tauri/abilities-runtime/src/abilities/feedback.rs:70`, `:80`; `src-tauri/src/services/claim_receipt/feedback.rs:632`, `:704`.
tokens used
129,631
1. PATCHED CORRECTLY — Account now `~24` with `FileList` in scope/template/projection: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:154`, `:216`, `:274`. Project now `~15` with `linear-issues-chapter`: `:155`, `:376`, `:419`.

2. PATCHED CORRECTLY — DOS-725 regex is `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$`, `--dailyos-*` only: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:398`, `:834`.

3. PATCHED CORRECTLY — Template arrays present for §5.2 Project, §5.3 Person, §5.4 Meeting: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:363`, `:461`, `:558`.

4. PATCHED CORRECTLY — AC-462.8 says `Filesystem pattern`, theme-registered, not DB-stored synced pattern: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:328`.

5. PATCHED CORRECTLY — Empty sketch emits visible `dailyos-empty-chip` with `data-empty-reason`: `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:300`, `:303`.

6. NEW DEFECT — MergeIntent substrate exists, but W2 packet still specifies wrong payload shape: `FeedbackAction::MergeIntent { source_person_id, target_person_id }` at `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W2-entity-surfaces.md:485`, `:517`. ADR/code canonical shape is unit enum + `payload_json.merge_target` / optional `supporting_evidence`: `.docs/decisions/0123-typed-claim-feedback-semantics.md:120`, `:126`; `src-tauri/abilities-runtime/src/abilities/feedback.rs:70`, `:80`; `src-tauri/src/services/claim_receipt/feedback.rs:632`, `:704`.
exit: 0
