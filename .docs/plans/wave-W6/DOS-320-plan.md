# Implementation Plan: DOS-320

## Revision history
- v1 (2026-05-01) — initial L0 draft.

## 1. Contract restated
DOS-320's Linear title says "Render surfaces filter claim input by trust band," but the W6-D slot narrows this to the React render layer for W5 ability outputs. The Linear ticket's backend language is real and must be acknowledged: "`gather_account_context()`, `build_intelligence_context()`, and the briefing prep pipeline default to" partitioning by trust score; "`needs_verification` claims excluded by default from prompt input"; "Briefing surfaces render the partition: main body + collapsed Background + hidden needs-verification"; and "`Show all evidence` toggle implemented; default off; per-session." The motivating production line is also load-bearing: "A 6-month-old resolved-green ticket should not be load-bearing in current-state."

The wave-owned contract is smaller: `.docs/plans/v1.4.0-waves.md:628-633` assigns W6-D only "the surface render layer for `get_entity_context` and `prepare_meeting` outputs that bias by `likely_current` / `use_with_caution` / `needs_verification`" and marks it "deferred-but-eligible." Therefore W6-D does not edit `gather_account_context()`, `build_intelligence_context()`, prompts, Trust Compiler, claims, or Rust services. Backend prompt partitioning remains DOS-287/W5-B territory; W6-D consumes bands already assigned by W4-A/W5 provenance.

The 2026-04-24 amendments that apply are inherited, not DOS-320 comments (Linear has no DOS-320 comments). DOS-287's 2026-04-24 PM recency amendment says older than 90 days is "HISTORICAL CONTEXT, not ... current state" and pairs with DOS-320 as "render-side band filtering." ADR-0125 supplies claim-type freshness/sensitivity primitives; ADR-0126 invariant 5 allows schema compression but forbids anomaly suppression.

## 2. Approach
Future implementation owns only TypeScript/React render files. Add `src/components/ui/TrustBandIndicator.tsx`, `src/components/ui/TrustBandIndicator.module.css`, and `src/lib/trust-band.ts` for a typed `TrustBandWire = "likely_current" | "use_with_caution" | "needs_verification" | "unscored"`, label metadata, field-path extraction, and partition helpers. Use W4-A's frozen enum contract: `TrustBand::{LikelyCurrent, UseWithCaution, NeedsVerification, Unscored}` with stable serde strings (`.docs/plans/wave-W4/DOS-5-plan.md:63-74`).

Add frontend response types in `src/types/index.ts` near the existing `EntityContextEntry`/meeting prep types (`src/types/index.ts:3099-3107`, `:1192-1238`): `AbilityResponseJson<T>`, `RenderedProvenanceSummary`, `RenderedFieldAttribution`, and `TrustAnnotated<T>`. These are render DTOs only; they mirror W4-C's bridge envelope `AbilityResponseJson { invocation_id, ability_name, ability_version, schema_version, data, rendered_provenance, diagnostics }` (`.docs/plans/wave-W4/DOS-217-plan.md:24-30`) and do not invent backend storage.

For `get_entity_context`, update the current hook/render path only as an adapter. `useEntityContextEntries` currently invokes legacy `"get_entity_context_entries"` and returns a naked `EntityContextEntry[]` (`src/hooks/useEntityContextEntries.ts:6-18`, `:71`). W5-A freezes the ability output as `AbilityOutput<Vec<EntityContextEntry>>` and says provenance stays on the wrapper, not entries (`.docs/plans/wave-W5/DOS-218-plan.md:17-23`). The adapter reads `rendered_provenance.field_attributions["/0/content"].trust_band` (and sibling title paths) when the ability bridge is active; legacy naked arrays become `unscored` and must never be hidden. Render changes land in `ContextEntryList` (`src/components/entity/ContextEntryList.tsx:50-96`) and `UnifiedTimeline`, where context entries are currently folded into timeline items (`src/components/entity/UnifiedTimeline.tsx:121-130`, `:163-184`).

For `prepare_meeting`, annotate existing rendered meeting brief items by field path, then apply a shared partition component. Current surfaces are the inline schedule expansion `PrepGrid` (`src/components/dashboard/BriefingMeetingCard.tsx:253-307`, `:333-343`) and full meeting page sections (`src/pages/MeetingDetailPage.tsx:991-1009`, `:1361-1385`, `:1395-1479`, `:1484-1511`). W5-B freezes `AbilityOutput<MeetingBrief>` with field-level trust attribution and an "About this" affordance reading the provenance map (`.docs/plans/wave-W5/DOS-219-plan.md:24-33`, `:43-45`). W6-D does not decide the `MeetingBrief` to legacy-view-model mapping; it wraps whichever mapped field path W5-B supplies.

Default rendering algorithm: extract band per item; compute `likelyCurrent`, `caution`, `needsVerification`, `unscored`; render current + unscored in place, render `use_with_caution` in a "Background" `<details>` block on evidence-list surfaces, hide `needs_verification` until `showAllEvidence` is true, and show the explicit empty state when there are no `likely_current` items but older/low-confidence evidence exists. The most-recent date comes from rendered provenance source summaries when present, then item timestamps such as `createdAt`/`source_asof`; missing date falls back to "No high-confidence current-state evidence."

End-state alignment: W6-D turns W4-A trust bands from invisible metadata into user-visible judgment aids on the two W5 pilot surfaces. It forecloses a backend-only interpretation of DOS-320 and a color-only trust UI.

## 3. Key decisions
Band policy: `likely_current` renders normally with optional compact "Current" label only inside About-this detail; `use_with_caution` remains available but is demoted to collapsed Background with a visible "Use with caution" indicator; `needs_verification` is hidden by default and appears only under "Show all evidence" with "Needs verification"; `unscored` remains visible with "Not scored" so legacy data is not silently suppressed.

"Filter" means render bias, not data deletion. ADR-0126 says retrieval is additive and schema compression must not suppress anomalies (`.docs/decisions/0126-memory-substrate-invariants.md:48-83`); therefore W6-D never mutates claims, trust, provenance, or prompt input.

Accessibility policy: `TrustBandIndicator` renders visible text plus a lucide icon, with `role="img"` and `aria-label="Trust band: Use with caution. Shown in Background evidence."` or the equivalent per band. The show-all control is a real `<button type="button">` with `aria-pressed`, `aria-controls`, and an adjacent `role="status" aria-live="polite"` text node announcing "Showing low-confidence evidence" / "Hiding low-confidence evidence." Collapsed Background uses native `<details><summary>` so keyboard and screen-reader behavior are built in.

Design tokens: no new hex values. Use `--color-garden-sage` / `--color-garden-sage-12` for likely-current, `--color-spice-turmeric` / `--color-spice-turmeric-12` for caution, `--color-spice-terracotta` / `--color-spice-terracotta-12` for verification-needed, and `--color-text-tertiary` / `--color-surface-subtle` for unscored, all already defined in `src/styles/design-tokens.css:36-55`, `:73-90`.

TypeScript shape: do not expect a top-level `trust_band`. W4-C returns `rendered_provenance`; W5-B says field-level trust attribution powers "About this." The frontend parser accepts only structured rendered provenance fields and defaults unknown/missing bands to `unscored`. Exact snake/camel casing stays an open question.

Legacy scope: if W6 lands before the W5 ability cutover, legacy `get_entity_context_entries`/`MeetingDetailPage` paths stay visible as `unscored`; W6-D tests the ability-response adapter separately. It must not block release by requiring backend cutover work outside this slot.

## 4. Security
No new auth/authz surface is introduced: no command, bridge, service, SQL, or MCP change. Rendered provenance is actor-filtered before React; ADR-0108 says Tauri can show full app provenance while MCP/P2 strip sensitive internals (`.docs/decisions/0108-provenance-rendering-and-privacy.md:23-30`, `:54-72`).

The risk is misleading suppression. Therefore low-trust evidence is hidden only behind a visible, reversible per-session toggle, and the empty state states the absence of high-confidence current evidence instead of filling current-state UI with stale claims.

ARIA labels and test fixtures must not include raw claim text, account names, domains, source snippets, prompt hashes, or customer identifiers. Use synthetic labels and rely on `formatProvenanceSource` for source vocabulary (`src/components/ui/ProvenanceLabel.tsx:23-65`).

`FieldAttribution.explanation` is not rendered by this indicator; if an About-this details panel includes it, it must already be sanitized per ADR-0108's explanation sanitizer. W6-D should consume `rendered_provenance`, not raw provenance JSON.

## 5. Performance
Cost is client-side O(number of rendered evidence items + field attributions) partitioning. No DB, provider, network, or Rust hot path changes. Use `useMemo` around partitioning in `ContextEntryList`, `UnifiedTimeline`, `PrepGrid`, and `MeetingDetailPage` so toggling local state does not redo unrelated meeting/detail view-model work.

Rendering budget: trust indicators are compact badges, and the initial provenance-derived trust summary must not exceed ADR-0108's Tauri initial render budget of 2KB (`.docs/decisions/0108-provenance-rendering-and-privacy.md:112-118`). Do not render full source lists inline; About-this/details owns deeper provenance.

Layout stability: indicators have fixed inline-flex dimensions, visible text, and no hover-only content. Collapsed sections use native details and existing spacing tokens to avoid shifting large meeting pages when toggled.

## 6. Coding standards
Services-only mutations are trivially honored: this is UI state only. No `Utc::now()`/`thread_rng()`; JS date formatting may parse existing timestamps but must not generate authoritative trust times. No source code or fixtures with customer-specific data per `CLAUDE.md:16-18`.

Intelligence Loop check (`CLAUDE.md:7-14`): this adds a render affordance, not a new table/column; no new signal type; no health score input; no prompt/prep context mutation; no new briefing callout generator; no feedback weighting path. It simply displays trust metadata already produced by W4/W5.

Product vocabulary: root `CLAUDE.md:72` points to `src/CLAUDE.md`, but that file is absent in the current repo. Use existing UI vocabulary from provenance components ("About this", "you noted", "AI", source display names) and avoid introducing backend terms such as `trust_score` in visible copy.

Frontend standards: TypeScript strict, no `any` parser except at the boundary with narrow type guards, no color-only states, no icon-only unlabeled controls, and no new global CSS except reusing the existing `.sr-only` utility where applicable.

## 7. Integration with parallel wave-mates
W4-A/DOS-5 must stabilize `TrustBand` serde strings before W6-D coding (`.docs/plans/wave-W4/DOS-5-plan.md:74`). W4-C/DOS-217 must either surface `rendered_provenance.field_attributions[*].trust_band` or confirm the exact parseable path in `AbilityResponseJson`.

W5-A/DOS-218 supplies `AbilityOutput<Vec<EntityContextEntry>>`; W6-D must not add trust fields to `EntityContextEntry` because W5-A explicitly keeps provenance on the wrapper (`.docs/plans/wave-W5/DOS-218-plan.md:21`). W5-B/DOS-219 supplies `AbilityOutput<MeetingBrief>` and field paths; W6-D consumes them and does not change `build_brief.rs`, `publish.rs`, or `maintenance.rs`.

W6-A/W6-C provide seeded bundle and release-gate surfaces. W6-D's L4 evidence must run against their seeded workspace and satisfy accessibility-tester plus codex-challenge/codex-consult. No migration numbering coordination.

Do not touch `src-tauri/src/services/context.rs` or `src-tauri/src/intelligence/provider.rs`; this plan reads their contracts only.

## 8. Failure modes + rollback
If rendered provenance is absent, malformed, or missing field-level band data, render the item as `unscored` with a neutral indicator and do not hide it. If the parser encounters an unknown band string, log a non-content warning in development/test only and render `unscored`.

If all likely-current evidence is filtered out, show the explicit empty state and the Background/show-all controls if older evidence exists. If the newest date cannot be determined, omit `{date}` rather than inventing one.

If the toggle state breaks or accessibility tests fail, rollback is removing the trust indicator/partition wrappers; legacy entity and meeting render paths remain unchanged. No migration, data repair, or backend rollback is involved.

W1-B universal write fence is honored by construction: W6-D performs no DB writes, file writes, service calls beyond existing reads, signal emissions, claim updates, or projections.

## 9. Test evidence to be produced
Unit tests: `trustBandIndicator_has_visible_text_and_accessible_name`, `trustBandIndicator_uses_non_color_label_for_each_band`, `extractTrustBand_prefers_field_attribution_band`, `extractTrustBand_defaults_unknown_or_missing_to_unscored`, `partitionTrustEvidence_hides_needs_verification_by_default`, `partitionTrustEvidence_show_all_reveals_low_confidence`.

Entity render tests: `ContextEntryList_renders_caution_entries_in_collapsed_background`, `ContextEntryList_show_all_evidence_announces_low_confidence_entries`, `UnifiedTimeline_does_not_render_needs_verification_until_toggle`, `UnifiedTimeline_unscored_legacy_entries_remain_visible`.

Meeting render tests: extend `src/components/dashboard/BriefingMeetingCard.test.tsx` with `PrepGrid_marks_use_with_caution_without_inline_color_only_state` and `PrepGrid_collapses_needs_verification_until_show_all`; extend `MeetingDetailPage`/view-model coverage with `MeetingDetailPage_no_high_confidence_evidence_empty_state`, `MeetingDetailPage_about_this_trust_summary_has_button_accessible_name`, and `MeetingDetailPage_show_all_evidence_is_per_session_only`.

Manual/accessibility scenarios: keyboard tab to Show all evidence; VoiceOver hears "Show all evidence, button, not pressed"; activating announces via polite live region; Background summary expands/collapses with native details semantics; indicators remain understandable in grayscale/high contrast.

Gate artifact: `pnpm test -- TrustBandIndicator ContextEntryList BriefingMeetingCard MeetingDetailPage`, `pnpm tsc --noEmit`, W6 L4 `/qa` against seeded bundles 1 + 5, accessibility-tester pass, and release-gate proof bundle note. Suite S contribution: no PII/customer data in fixtures/ARIA labels. Suite P contribution: client render partition cost measurement on seeded meeting/entity pages. Suite E contribution: bundle evidence that stale/low-trust items are not current-state by default but remain inspectable.

## 10. Open questions
1. What exact `rendered_provenance` path and casing carries `trust_band` for a field: `trust_band`, `trustBand`, nested `trust.band`, or only a top-level trust summary?
2. Are W5 ability outputs fully routed through `invoke_ability` by W6, or must W6-D support only legacy naked arrays/legacy `FullMeetingPrep` in the shipping app and leave ability-path tests as proof?
3. Does W6-D need to satisfy DOS-320's backend acceptance criteria for prompt partitioning, or are those explicitly owned by DOS-287/W5-B despite the Linear issue text?
4. For W5-A user-authored entity context rows, should valid direct user notes render as `likely_current` or `unscored` when the Trust Compiler did not score a claim row?
5. What date should populate the empty state: newest `source_asof`, newest `observed_at`, newest row timestamp, or a W5-provided rendered provenance summary?
6. Should "Show all evidence" be one per surface (this plan) or one per section/field group for long meeting briefs?

## Revision history

- v2 (2026-05-06) - appended post-W5 reconciliation notes. The v1 plan above is intentionally left intact.

## v2 reconciliation notes (post-W5)

### What already shipped

- W4-C shipped the bridge envelope as `AbilityResponseJson { invocation_id, ability_name, ability_version, schema_version, data, rendered_provenance, diagnostics }` (`src-tauri/src/bridges/types.rs:149-174`). `rendered_provenance` is a wrapper with `{ surface, value }`, where `value` is the full provenance JSON for Tauri/Worker/Eval and MCP-redacted provenance for MCP surfaces (`src-tauri/src/bridges/types.rs:576-581`).
- W5 ability output serialization still has provenance exactly once on the wrapper: `AbilityOutput<T>` serializes `data`, `provenance`, `ability_version`, and `diagnostics` (`src-tauri/src/abilities/provenance/envelope.rs:301-333`), and the bridge transforms `provenance` into `rendered_provenance` (`src-tauri/src/bridges/types.rs:535-565`).
- `get_entity_context` landed as a Read ability returning `Vec<EntityContextEntry>` with provenance on the wrapper, not trust fields in each entry (`src-tauri/src/abilities/get_entity_context.rs:51-99`; `src/types/index.ts:3099-3107`). It now allows User, Agent, and System actors (`src-tauri/src/abilities/get_entity_context.rs:38-50`).
- W5 cycle-6/7 sensitivity work means Agent `get_entity_context` filters claims to Public/Internal before returning data (`src-tauri/src/abilities/get_entity_context.rs:168-181`; `src-tauri/src/services/claims.rs:620-637`). User/Tauri ability calls do not get that Agent-only filter.
- The Tauri UI hook still invokes legacy `"get_entity_context_entries"` and returns a naked `EntityContextEntry[]` (`src/hooks/useEntityContextEntries.ts:10-18`, `:71`). The command still reads the legacy `entity_context_entries` table (`src-tauri/src/commands/workspace.rs:1235-1243`; `src-tauri/src/services/context.rs:1093-1110`, `:1227-1259`). W5 cycle 3 intentionally rolled back the Tauri read cutover; DOS-411 now owns the write/read migration.
- `prepare_meeting` landed as a Transform ability returning `MeetingBrief` sections (`src-tauri/src/abilities/prepare_meeting/mod.rs:14-31`; `src-tauri/src/abilities/prepare_meeting/synthesis.rs:115-185`). It carries evidence `confidence` and `sensitivity` internally (`src-tauri/src/abilities/prepare_meeting/synthesis.rs:88-105`, `:468-486`), filters prompt-input claims by sensitivity (`:437-465`, `:1494-1496`), and omits ambiguous/blocked source subjects before accepting items (`:900-967`).
- W4-A shipped `TrustBand` serde strings and claim-level `TrustComputation.band` (`src-tauri/src/abilities/trust/types.rs:22-36`), with labels `likely_current`, `use_with_caution`, `needs_verification`, and `unscored` (`src-tauri/src/abilities/trust/mod.rs:447-454`). This exists at the Trust Compiler/claim recompute layer, not in W5 field attribution.

### Path/API/shape changes v2 must absorb

- The v1 assumption `rendered_provenance.field_attributions["/0/content"].trust_band` is wrong for the shipped bridge shape. The frontend path, if using the ability bridge, is `response.rendered_provenance.value.field_attributions["/0/content"]`; `FieldAttribution` has `subject`, `derivation`, `source_refs`, `confidence`, and `explanation`, but no `trust_band` (`src-tauri/src/abilities/provenance/field.rs:151-158`).
- Top-level provenance has `trust: TrustAssessment`, but that is an `effective` trusted/untrusted classification with contributions, not the W4-A `TrustBand` enum (`src-tauri/src/abilities/provenance/envelope.rs:341-362`; `src-tauri/src/abilities/provenance/trust.rs:12-17`, `:104-120`).
- `MeetingBrief` output items do not include trust bands (`src-tauri/src/abilities/prepare_meeting/synthesis.rs:115-185`), and source/candidate `confidence` is not equivalent to `TrustBand`.
- Any frontend-only implementation can safely parse `AbilityResponseJson` and render provenance warnings/source freshness, but it cannot truthfully partition items by W4-A trust band until backend/bridge output exposes a per-field or per-item band.

### Reduced or remaining scope

- Reduce W6-D to an adapter/proof design unless the backend adds a per-field trust-band shape first. A frontend-only patch should default all legacy Tauri entity-context entries to `unscored` and must not hide them.
- Do not cut over `useEntityContextEntries` to `invoke_ability("get_entity_context", ...)` as part of W6-D unless DOS-411 is also in scope. The current UI create/update/delete flow depends on legacy `entity_context_entries`, and W5 already proved a read-only cutover causes divergence.
- For `prepare_meeting`, W6-D can test against bridge envelopes and provenance warnings, but "likely/current/caution/needs verification" partitioning needs a new source of truth: either `TrustComputation` attached to each claim-derived field, or a rendered-provenance summary added by Rust.
- The backend prompt-input filtering part of DOS-320 is already substantially handled by W5 for `prepare_meeting` and Agent `get_entity_context`; output-side sensitivity rendering is tracked by DOS-412 and should not be silently absorbed into W6-D.

### New dependencies and follow-ups

- DOS-411 (`Tauri entity-context write cutover to claim-backed path`) blocks any W6-D strategy that replaces the shipping UI's legacy note read path with claim-backed ability reads.
- DOS-412 (`ADR-0108 sensitivity-rendering audit across UI surfaces and MCP responses`) owns broad output-side sensitivity policy. W6-D may depend on its policy helper or explicitly stay limited to trust-band affordances for W5 ability proof surfaces.

### Open questions before implementation

1. Should Rust add per-field trust-band data to `rendered_provenance.value.field_attributions` before W6-D, or should W6-D defer trust partitioning and render only top-level provenance/warnings?
2. Is W6-D still intended to ship in W6 if the only current Tauri entity-context UI path is legacy and unscored?
3. For `prepare_meeting`, should W6-D compute bands from existing `claim.trust_score`/`EvidenceSource.confidence`, or is that prohibited because it would duplicate Trust Compiler semantics in TypeScript?
4. Should the "Show all evidence" UI wait for DOS-412's output-sensitivity policy so Confidential/UserOnly behavior is consistent across non-W5 surfaces?
