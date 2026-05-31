# L0 Packet — v1.4.4 W2 Entity Surfaces

**Current revision: V1.2 (2026-05-21).** V1.0 → V1.1 → V1.2 fold below in §2.

## 1. Header

Date: 2026-05-21
Project: [v1.4.4 — WordPress Surface Migration](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177) (id `f8b805d9-f3d4-41b4-a446-51bbb7e05f2e`)
Sub-wave: **W2 — Entity surfaces.** Inherits all wave-level invariants and locked decisions from `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` V1.1 §10 and §13. Builds on top of W1 substrate (this packet's "Substrate consumed" §6 is the full reuse map).

Branch base: `wave/v1.4.4-w1-stage1a` at HEAD `c5c0578f` (`Merge worktree-agent-a5e003b64a9524cf8 (W1 Meeting EntityKind extension for W2 §5.4) into wave/v1.4.4-w1-stage1a`; updated V1.1 from stale `deb682b0` per codex-challenge F6). W1 substrate fully landed including the Meeting EntityKind extension at `87df7cf6` that closes V1.0 F2 (Meeting). New W2 work branches from this base as `wave/v1.4.4-w2-entity-surfaces` after L0 close.

Sub-tickets (12):

- [DOS-462](https://linear.app/a8c/issue/DOS-462) — Account Detail block (Urgent; primary proof surface for `get_entity_intelligence` envelope from W1).
- [DOS-483](https://linear.app/a8c/issue/DOS-483) — Project Detail block (High).
- [DOS-484](https://linear.app/a8c/issue/DOS-484) — Person Detail block (High).
- **Meeting Detail block** — new sub-ticket; file at L1 kickoff if not yet existing in Linear (see §5.4 + §13 Q5).
- **Accounts list shell** — list block consuming list-shape envelope (route `/accounts` parity).
- **Projects list shell** — list block consuming list-shape envelope.
- **People list shell** — list block consuming list-shape envelope; merge-picker integration per §5.5 + §13 Q6.
- [DOS-328](https://linear.app/a8c/issue/DOS-328) — Metadata proposals interaction (Urgent; proposal lifecycle on the entity-detail composites).
- [DOS-688](https://linear.app/a8c/issue/DOS-688) — TrendStrip primitive (DOS-325 fold).
- [DOS-689](https://linear.app/a8c/issue/DOS-689) — EvidenceDrawer primitive + drawer integration (DOS-325 fold).
- [DOS-690](https://linear.app/a8c/issue/DOS-690) — DOS-325 entity-detail surface residue (envelope + keyboard + drawer integration).
- [DOS-691](https://linear.app/a8c/issue/DOS-691) — DOS-9 cite-chip tooltip entity-envelope wiring.
- [DOS-692](https://linear.app/a8c/issue/DOS-692) — DOS-11 trust-band UI keyboard nav + a11y.
- [DOS-693](https://linear.app/a8c/issue/DOS-693) — HealthBadge label-discipline pass (DOS-325 voice rule applied).
- [DOS-725](https://linear.app/a8c/issue/DOS-725) — Resolve `dailyos_project` chrome tint (gates Project Detail).

Substrate consumed from W1 (every producer landed on `wave/v1.4.4-w1-stage1a`):

| W1 producer | Module | Landing SHA | W2 consumer |
|---|---|---|---|
| `get_entity_intelligence` ability (DOS-459) | `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/` | `2b3915ef` | Account / Project / Person / Meeting detail (§5.1–5.4) |
| Canonical entity touchpoints + open-loops (DOS-460) | `src-tauri/src/services/entity_intelligence/touchpoints.rs` + `services::context::list_open_loops` ext | `6444568c` | Open-loop + touchpoint inner blocks (§5.1–5.4) |
| Entity-detail trust-boundary hardening (DOS-477) | `services::entity_intelligence::auth` + envelope-set validation | `e9b0ed41` + `7bbbc6f7` | Every claim-bearing inner block (§5.1–5.7) |
| Claim receipt fanout (DOS-339 wire-up + DOS-701 carve-out) | `src-tauri/src/services/claim_receipt/{mod,contracts,render,auth}.rs` | `0243df65` (PR #323 base) + `0a586218` (cycle-2 wiring) | Per-claim receipt rendering in every inner block (§5.1–5.4 + §5.7) |
| Receipt vs operational audit boundary (DOS-340) | `services::claim_receipt::boundary` + `scripts/check_audit_disclosure_allowlist.sh` | `e74d49dc` | Receipt rendering boundary (§5.1–5.4) |
| Privacy / redaction + `build_receipt_for_audience` (DOS-341) | `services::claim_receipt::privacy` | `e4d72de6` + `6b437c5f` | Every claim-bearing inner block — AgentMcp audience filter (§5.1–5.4 + §5.6 + §10) |
| Semantic feedback actions (DOS-8) | `services::claim_receipt::feedback` (typed 9-variant enum) | `7bbbc6f7` | Metadata proposals (§5.6) + cite-chip drawer feedback (§5.7) |
| Entity fixture harness + no-bypass checks (DOS-461) | `tests/entity_intelligence_no_bypass/` | landed in W1 cycle-2 verdict cluster (verify at L1) | Gates every W2 AC (§7) |
| Meeting prep / readiness DTO (DOS-335) | `src-tauri/src/services/meeting_prep_status/{read,write}.rs` | landed via DOS-335 PR (verify at L1; was Stage 1a per W1 V1.1 §13 Q10) | Meeting Detail block (§5.4) |
| Meeting `EntityKind` extension for `get_entity_intelligence` (V1.1 fold) | `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/{contracts,producer}.rs` + `services::entity_intelligence::auth` | `87df7cf6` (merged at `c5c0578f`) | Meeting Detail block (§5.4) — closes V1.0 codex-challenge F2 |
| `FeedbackAction::MergeIntent` 10th-variant extension (V1.2 fold) | `src-tauri/abilities-runtime/src/abilities/feedback.rs` + `services::claims::record_claim_feedback` arm | `01d0cff3 (merged at 0d82502f)` (sibling W1 amendment landing in parallel; targets `wave/v1.4.4-w1-stage1a`) | Person Detail merge affordance (§5.3) + People-list merge intent (§5.5) — removes V1.1 v1.4.5 deferral per James "no deferrals" mandate (2026-05-21) |
| `get_daily_briefing` Read/User-only (DOS-507) | `abilities-runtime/src/abilities/get_daily_briefing/` | `ed51d1b7` | NOT consumed by W2 — W3 surface. Cited for cross-wave plumbing only. |
| L2 cycle-3 section-list + source_type redaction patches | n/a | `1a56d612` | Adopted as-is via the renderer projection. |
| L3 cycle-2 WP block consumer skeletons | `wp/dailyos/blocks/account-detail/render-functions.php` + sibling person/project skeletons | `00f38b3b` + `3e58bc13` | W2 specializes these skeletons into typed inner-block compositions. |
| CI gate `check_w1_consumer_skeleton.sh` | `src-tauri/scripts/check_w1_consumer_skeleton.sh` | `c2e857b8` (extended through W1 cycle-2) | Inherited as a passing gate for the W2 PR (§7 AC #W2.10). |

Primary code areas touched (W2 NEW):

- `wp/dailyos/blocks/account-detail/` — specialize Stage 1b skeleton into 1 outer + 24 inner blocks (§5.1).
- `wp/dailyos/blocks/project-detail/` — 1 outer + 15 inner blocks (§5.2).
- `wp/dailyos/blocks/person-detail/` — 1 outer + 12 inner blocks (§5.3).
- `wp/dailyos/blocks/meeting-detail/` — NEW outer block + 10 inner blocks (§5.4).
- `wp/dailyos/blocks/accounts-list/`, `projects-list/`, `people-list/` — NEW list shells (§5.5).
- `wp/dailyos/blocks/_shared/hooks/useAbilityCursor.ts` — NEW shared hook (per wave §10 invariant "Entity list pagination contract").
- `wp/dailyos/blocks/metadata-proposal-cue/` + `metadata-proposal-drawer/` — NEW inner blocks consuming envelope's `metadata_proposals` slice (§5.6).
- `wp/dailyos/blocks/trend-strip/` (DOS-688), `evidence-drawer/` (DOS-689) — NEW primitives.
- `wp/dailyos/blocks/health-badge/` — label-discipline pass (DOS-693).
- `wp/dailyos/blocks/provenance-tag/` — cite-chip tooltip envelope wiring (DOS-691).
- `wp/dailyos/blocks/trust-band-badge/` — keyboard nav + ARIA pass (DOS-692).
- `wp/dailyos/patterns/account-detail-default.php`, `project-detail-default.php`, `person-detail-default.php`, `meeting-detail-default.php` — filesystem patterns (theme-registered via `register_block_pattern`, per wp-skill H4 V1.1; NOT synced patterns).
- `wp/dailyos/theme/functions.php` `chrome_config()` `$stub_tints` — set `dailyos_project` to resolved value (DOS-725; §5.2 + §13 Q4 decision).

**Intelligence Loop integration check.** Every sub-ticket section §5.1–5.7 below answers the 5 CLAUDE.md questions inline. Per wave §10 invariant "Refresh model", §3 (signals + invalidation) is answered uniformly: **pull on render + user-initiated refresh; no push invalidation at v1.4.4.** Per-sub-ticket §3 entries inherit this unless the surface genuinely emits new signals (none do at W2).

## 2. Changelog

- **V1.0 (2026-05-21):** Initial L0 draft. Converts wave §5.2 + reorientation doc §"v1.4.4 W2" + the 12 sub-tickets into a reviewable sub-wave packet. All 4 wave §13 locked decisions inherited verbatim; this packet does not re-litigate them. W1 substrate fully landed on `wave/v1.4.4-w1-stage1a` at branch HEAD `deb682b0`; substrate-consumed table in §6 cites concrete SHAs per producer. Reviewer panel adds WP-skill-grounded reviewer per memory `feedback_wp_skill_grounded_reviewer_for_wp_l0`; `/cso` opts in per-sub-ticket (write paths emerge in DOS-328 metadata-proposal accept/dismiss/edit and DOS-689 EvidenceDrawer field-allowlist).

- **V1.2 (2026-05-21):** Cycle-2 reviewer fold — codex-challenge cycle-2 surfaced 6 findings; all addressed without scope expansion. **F1 (CRITICAL carryover) — Account/Project inner-block count mismatch:** Account count 22→24 + `dailyos/file-list` inner block added to §5.1 template, projection table, and AC-462.2; Project count 14→15 + `dailyos/linear-issues-chapter` added to §5.2 template + projection table + AC-483.2 (1-to-1 with `src/pages/ProjectDetailEditorial.tsx:526` `LinearIssuesChapter`); scope-summary §4 inner-block lists realigned. **F3 (HIGH carryover) — DOS-725 inline-style CI regex too permissive:** tightened from `^--[a-z-]+:\s*var\(--[a-z-]+\);?$` to `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` — `--dailyos-*` namespace ONLY, arbitrary `--*` prefixes now rejected by `check_no_inline_style_exception.sh`. Updated in §5.2 prose + AC-W2.7 + §10 invariant footnote. **NEW — Template arrays only on Account:** added concrete `template` array sketches to §5.2 (Project, 15-entry), §5.3 (Person, 12-entry), §5.4 (Meeting, 10-entry) block.json sketches, satisfying §10 invariant "code-shape sketch obligation" across all 4 entity-detail composites. **NEW — Residual "Synced pattern" in AC-462.8:** replaced with "Filesystem pattern" + theme-registered semantics per wp-skill H4 V1.1 sweep (final residual H4 hit). **NEW — Empty-state code sketch contradicted §10 invariant:** §5.1 stakeholder-grid render.php sketch upgraded from empty `<div ... data-empty-reason="..."></div>` to a visible `dailyos-empty-chip` span with `data-empty-reason` + localized empty-state label, matching §10 invariant "quiet chip, never silent-hidden". **NEW (CRITICAL) — MergeIntent substrate gap:** V1.1 had filed `FeedbackAction::MergeIntent` as a v1.4.5 candidate variant; per James "every surface in 1.4.4 — no deferrals" mandate (2026-05-21), V1.2 escalates to a parallel W1 substrate amendment (sibling to Meeting `EntityKind` extension `87df7cf6`). Extension SHA carried as `01d0cff3 (merged at 0d82502f)` placeholder pending land; cycle-4 dispatch for §5.3 + §5.5 PRs waits for the extension. §5.3 Q5 disclaimer removed; AC-484.3 updated; §1 substrate-consumed table + §6 reuse map carry the extension as an additive enum variant + new `record_claim_feedback` arm (no other W1 contracts reopened).

- **V1.1 (2026-05-21):** Cycle-1 reviewer fold — 18 findings across 5 reviewers folded, locking 4 decisions James green-lit at 2026-05-21. Locked: (a) outer/inner = renderer-side projection (Path B) — the 22/14/12/10 inner blocks project from the 7 `EnvelopeSection` variants (Facts / Health / MetadataProposals / OpenLoops / Touchpoints / Threads / Record); (b) Meeting Detail = Path A, W1 substrate extended at commit `87df7cf6` (merged at `c5c0578f`); (c) AgentMcp touchpoint render = Option B (aggregate signal `{ count, recency: Recent|Aging|Stale, content: redacted }`, no per-item rows / titles / timestamps); (d) DOS-725 project tint = CSS custom property `--dailyos-project-tint` on outer wrapper, gated by ADR-0077 amendment ticket as W2 L1 prerequisite, with new CI script `check_no_inline_style_exception.sh` allowing only `style="--dailyos-*:..."`; until amendment lands, `chrome_config()` emits olive default. **Per-reviewer fold:** codex-challenge (6) — F1 CRITICAL outer/inner contract rewritten as Path B projection mapping across §5.1–5.4 + AC #W2.6, F2 CRITICAL Meeting reframed as Path A with §5.4 / §6 / §13 Q4 + Q5 now citing `87df7cf6`, F3 HIGH DOS-725 tint locked via ADR-0077 amendment prerequisite + CI script + chrome_config() default, F4 HIGH AgentMcp swapped to Option B aggregate, F5 HIGH list shells consume W1 `Paginated<T>` + `CursorState` (no more invented `ListEnvelope<T>`), F6 LOW base SHA refreshed to `c5c0578f`. Architecture (3) — A1 templateLock + default template added to §5.1 block.json sketch (and code-shape sketch obligation generalized as §10 invariant), A2 `useAbilityCursor` authorship declared as new W2 substrate in §3 K-in, A3 Meeting envelope verification now Path A (resolved) instead of L1 verification gate. Design-lens (2 BLOCKING) — F1 empty-state pattern locked as §10 invariant ("every inner block renders empty as quiet chip with `data-empty-reason`; no silent hidden states; inherits envelope `Empty { reason }`"); F2 Q3 AgentMcp touchpoint locked as Option B aggregate. WP-skill (4 HIGH + 3 path-α) — H1 templateLock contract + default template in every outer block.json, H2 `dailyos/envelopeHandle` resolution contract documented (outer invokes producer once, DOS-477 cache key `(envelope_render_id, actor_principal_id, surface)`, inner blocks key off context), H3 inline-style boundary gated by new CI script `check_no_inline_style_exception.sh`, H4 "synced patterns" → "filesystem patterns" throughout §5 + AC-W2.10. Path-α (M1 WP 7.0 fallback / M2 inserter-null guard / M3 theme.json per-block-styles) routed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` per memory `feedback_l2_path_alpha_to_maintenance_project`. Codex-consult (2 drift) — D1 K-in inventory refreshed to 24 .md files at scan with 3 new W1 K-out entries cited (codex-companion finalAnswerSeen hang, parallel-agent commit-hook contention, L3 sub-ticket bypass discipline); D2 DOS-339 footnote added clarifying `0243df65` is merge SHA vs `ca8e7e21` substrate SHA. **New §10 invariants added in V1.1:** (i) code-shape sketch obligation (every block.json declaration in §5 carries a concrete sketch, not just prose); (ii) empty-state pattern (`data-empty-reason="<reason>"` quiet chip, never silent-hidden); (iii) AgentMcp touchpoint aggregate render (count + recency tier only, no per-item).

## 3. K-in record (substrate-grep audit, 2026-05-21)

Per CLAUDE.md "Knowledge store discovery" + engineering-ladder.md L0 K-in obligation.

### `docs/solutions/` — 24 .md files at V1.1 scan (re-grep 2026-05-21 per codex-consult drift D1)

Greps run against the full inventory for: `entity-detail`, `account-detail`, `project-detail`, `person-detail`, `meeting-detail`, `gutenberg`, `inner-blocks`, `providescontext`, `usescontext`, `render-functions`, `theme-json`, `cursor`, `pagination`, `merge`, `tint`, `composition`, `envelope`, `agentmcp`, `audience`. Inventory grew from 16 → 24 between V1.0 and V1.1 (W1 close emitted 3 new entries this packet now cites; remaining 5 net-new are unrelated to W2 scope).

**Full inventory at V1.1 scan:**

```
docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md
docs/solutions/architecture-patterns/append-only-jsonl-schema-change-preserves-hash-chain-2026-05-20.md
docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md
docs/solutions/architecture-patterns/emit-or-log-wrapper-silent-error-swallow-class-2026-05-18.md
docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md
docs/solutions/security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md
docs/solutions/test-failures/hmac-canonical-change-requires-golden-vector-recompute-2026-05-20.md
docs/solutions/test-failures/parallel-test-singleton-state-flake-2026-05-18.md
docs/solutions/tooling-decisions/codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md
docs/solutions/tooling-decisions/codex-worktree-isolation-incompatible-with-rescue-forwarder-2026-05-18.md
docs/solutions/tooling-decisions/gh-pr-merge-delete-branch-multi-worktree-incompatibility-2026-05-19.md
docs/solutions/tooling-decisions/phpcs-warning-severity-zero-prevents-warning-only-ci-fails-2026-05-19.md
docs/solutions/tooling-decisions/pre-push-hook-duration-vs-ssh-idle-timeout-2026-05-19.md
docs/solutions/tooling-decisions/wip-and-no-verify-escape-hatches-under-memory-pressure-2026-05-20.md
docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md
docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md
docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md
docs/solutions/workflow-issues/node-modules-tracked-symlink-enotdir-pnpm-install-2026-05-19.md
docs/solutions/workflow-issues/parallel-agent-commit-hook-contention-2026-05-20.md
docs/solutions/workflow-issues/parallel-wave-synced-from-conflicts-2026-05-19.md
docs/solutions/workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md
docs/solutions/workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md
docs/solutions/workflow-issues/worktree-setup-needs-pnpm-install-2026-05-19.md
```

**Relevant cross-references the W2 packet consumes (3 new entries cited per V1.1 fold):**

- `architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md` — K-out from W1 retro (NEW citation). Sets the discipline that wave-level "spec compliance" is not a substitute for sub-ticket AC. W2 obligation: every sub-ticket's ACs trace back to the originating Linear ticket's "Required visible QA states" matrix, not to wave-level rollup language. AC-W2.1 enforces this per-block.
- `workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md` — K-out from W1 (NEW citation). Applied to L1 parallel fan-out (§5.1–5.7): codex tasks fan out per-block, but dispatch hygiene (last-log-no-writes-3-min = cancel) prevents stuck-agent loss in a 7-task parallel set.
- `workflow-issues/parallel-agent-commit-hook-contention-2026-05-20.md` — K-out from W1 (NEW citation). Applied: codex tasks committing through worktrees serialize the commit step; W2 parallel branches use shared `.claude/` symlink + commit-msg hook discipline to avoid lock contention. L1 protocol section in §5.1 references.
- `tooling-decisions/codex-companion-task-worker-hang-finalAnswerSeen-2026-05-20.md` — K-out from W1 close (NEW citation, V1.1). Applied to L1 codex-task dispatch: if worker emits `finalAnswerSeen` but no file writes land within 3 min, cancel + re-dispatch with smaller scope. Pairs with `codex-agent-dispatched-but-no-file-changes` as the dispatch-hygiene class for W2 parallel codex fan-out.
- `workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md` — applied: 12 sub-tickets sized to 1 outer + N inner per entity. If reviewers surface 5+ net-new findings per cycle, scope reset, not fold-and-continue. Reviewer panel reads this as a precondition.
- `workflow-issues/premise-check-production-vs-dev-friction-before-scoping-waves-2026-05-20.md` — applied: every sub-ticket's AC is production-friction-grounded (real claim volume / real account fixture); dev-friction shortcuts (mocking the envelope, stubbing scope sets) are excluded.
- `workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — applied throughout §5: every inner-block DTO sketch confirms the existing substrate type names landed by W1 match (`EntityIntelligenceEnvelope`, `TouchpointBundle`, `ClaimReceipt`, `RenderableClaimText`, `FeedbackAction`); no proposed-name aliases.
- `workflow-issues/substrate-only-landing-needs-l0-amendment-2026-05-18.md` — W2 is a renderer-only wave; the W1 packet served as the substrate amendment. W2 has no parallel obligation.
- `security-issues/prompt-channel-sensitivity-class-sweep-2026-05-18.md` — applied to §5.6 (metadata-proposal accept/dismiss/edit channel — same class-sweep rule) and §5.7 (EvidenceDrawer 10-channel ADR-0130 §3.1 enumeration per DOS-689).
- `architecture-patterns/append-only-jsonl-schema-change-preserves-hash-chain-2026-05-20.md` — informs §5.6 metadata-proposal accept: the proposal-claim → confirmed-value path is append-only via `record_claim_feedback`; the hash chain (provenance receipt) is preserved on accept/dismiss/edit. No rewrites to claim history.

### `.docs/decisions/` — ADRs consumed (none overridden)

| ADR | Title | How W2 consumes |
|---|---|---|
| 0057 | Entity intelligence architecture | Already anchored DOS-459 envelope shape (W1). W2 renders the typed projection. |
| 0077 | Magazine layout editorial redesign | All entity-detail composites translate the magazine reference into Gutenberg blocks; DOS-725 resolves `dailyos_project` tint as an ADR-0077 amendment (§5.2 + §13 Q4). |
| 0083 | Product vocabulary | All user-facing strings emitted from W2 blocks pass through the vocabulary registry (HealthBadge label discipline DOS-693 is a direct application). |
| 0102 | Abilities as runtime contract | Every W2 block invokes substrate via `invoke_ability(name, payload, scope_set)` — runtime client signature per `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:85`. |
| 0105 | Provenance as first-class output | TrustBandBadge + FreshnessIndicator + ProvenanceTag inner blocks consume ADR-0105 fields directly from envelope. |
| 0108 | Provenance rendering + privacy | Field allowlist for EvidenceDrawer (DOS-689) and cite-chip tooltip (DOS-691) per actor-filtered render projection. |
| 0111 | Surface-independent ability invocation | Loopback HTTP transport unchanged; `SurfaceClient` actor resolved at signed-request boundary. |
| 0123 | Typed claim feedback semantics | Metadata proposal accept/dismiss/edit (§5.6) emits the 9-variant `FeedbackAction` from W1 DOS-8. |
| 0125 | Claim anatomy + temporal + sensitivity + TypeRegistry | Every inner block reads claim fields from this anatomy without inventing surface-local shapes. |
| 0128 | Headless DailyOS / MCP as product surface | AgentMcp audience row in the privacy matrix (W1 DOS-341) gates every claim-bearing inner block's render. Field allowlist enforced via `build_receipt_for_audience`. |
| 0129 | Composable surfaces — WordPress Studio as primary | W2 is the proof. Outer/inner block contract per wave §10. |
| 0130 | Surface-independent composition contract | Reading A (1 outer = renderer-side projection of `Composition`; N inner = projection of `sections[].blocks[]`) governs every entity-detail composite. |
| 0132 | Pill primitive dual existence | Chrome `.Pill_*` vs block `.dailyos-pill*` discipline carried into W2 (NavIsland active-state stays chrome; in-block pill stays block). No new contract. |

**`useAbilityCursor` hook authorship (architecture A2 fold, V1.1):** grep `wp/dailyos/blocks/_shared/hooks/` at V1.1 scan time confirms no prior `useAbilityCursor.ts` file exists on `wave/v1.4.4-w1-stage1a` HEAD `c5c0578f`. **W2 authors this hook as NEW substrate** at the shared lane location declared in §5.5. Not inherited from a prior stub.

**DOS-339 SHA footnote (codex-consult drift D2, V1.1):** The substrate-reuse table cites `0243df65` as the DOS-339 base for claim-receipt fan-out wiring. `0243df65` is the **PR-merge SHA** (`Merge feat/dos-339-claim-receipt-fanout into wave/v1.4.4-w1-stage1a`); the underlying feature commit is `ca8e7e21` (`feat(claim_receipt): fan-out signal wiring + useClaimReceiptSubscription hook (DOS-339)`). Both reach the same tree on `wave/v1.4.4-w1-stage1a`. Readers walking diff hunks should point at `ca8e7e21` for the substantive change; merge SHA is correct for PR-trail reconstruction. No code-level impact on this packet.

**Verdict: K-in complete. No documented prior substrate reinvented.** W2 is renderer-only over substrate that landed in W1. The four cited NEW K-out solutions (L3 sub-ticket discipline, codex dispatch hygiene, hook contention, codex-companion finalAnswerSeen hang) directly shape this packet's reviewer panel discipline + L1 parallel-fan-out protocol (§5.x footers + §11).

## 4. Scope summary

| Sub-ticket | Surface | W1 producer consumed | Inner blocks composed | LOC ballpark |
|---|---|---|---|---|
| DOS-462 Account Detail | `dailyos/account-detail` outer | `get_entity_intelligence` (entity_type=account) + DOS-460 touchpoints/open-loops + DOS-339 receipts + DOS-341 privacy + DOS-477 trust-boundary | ~24 inner (AccountHero, SentimentHero, TriageSection, DivergenceSection, OutlookPanel, OnTrackChapter, SupportingTension, AboutIntelligence, AccountPullQuote, StakeholderGrid, StrategicLandscape, ValueCommitments, QuoteWall, CommercialShape, AccountTechnicalFootprint, RelationshipFabric, AboutThisDossier, RecommendedActions, TouchpointsFeed, OpenLoopsFeed, FileList, LinearIssuesChapter, UnifiedTimeline, FinisMarker) | ~1,800 |
| DOS-483 Project Detail | `dailyos/project-detail` outer | Same + DOS-725 tint resolution gate | ~15 inner (ProjectHero, VitalsStrip, PortfolioChapter, TrajectoryChapter, HorizonChapter, WatchList, WatchListMilestones, StakeholderGallery, TheWork, TouchpointsFeed, OpenLoopsFeed, LinearIssuesChapter, UnifiedTimeline, RecommendedActions, ProjectAppendix) | ~1,400 |
| DOS-484 Person Detail | `dailyos/person-detail` outer | Same + merge-picker substrate gap (§5.3 + §13 Q6) | ~12 inner (PersonHero, VitalsStrip, PersonInsightChapter, PersonNetwork, PersonRelationships, WatchList, UnifiedTimeline, RecommendedActions, TheWork, Touchpoints, OpenLoops, PersonAppendix) | ~1,200 |
| Meeting Detail | `dailyos/meeting-detail` outer (NEW) | `get_entity_intelligence` (entity_type=meeting) + DOS-335 prep DTO + DOS-339 receipts | ~10 inner (MeetingHeader, PrepStatus, AgendaDraft, AttendeesSection, RelatedEntities, ClaimsForReview, ContextBundle, PostMeetingCapture, Touchpoints, RecommendedActions) | ~900 |
| Accounts list shell | `dailyos/accounts-list` (NEW) | `get_entity_intelligence` (list mode) + paginated cursor envelope | 0 inner (flat list-block) | ~400 |
| Projects list shell | `dailyos/projects-list` (NEW) | Same | 0 | ~400 |
| People list shell | `dailyos/people-list` (NEW) | Same + merge-picker entry-point (§5.5) | 0 | ~450 |
| DOS-328 metadata proposals | `dailyos/metadata-proposal-cue` + `metadata-proposal-drawer` inner blocks | Envelope `metadata_proposals` slice + DOS-8 `FeedbackAction::ProposalAccept/Dismiss/Edit` + DOS-477 envelope-set validation | 2 inner (cue + drawer) | ~600 |
| DOS-688 TrendStrip | `dailyos/trend-strip` primitive | Trust band + factor surface from envelope claims | 1 primitive | ~250 |
| DOS-689 EvidenceDrawer | `dailyos/evidence-drawer` primitive + integration | Envelope `ProvenanceRef` + ADR-0108 §3 actor-filtered render | 1 primitive | ~500 |
| DOS-690 Entity-detail surface residue | (consumed across §5.1–5.3) | Envelope + keyboard nav harness | n/a — surface integration | ~200 |
| DOS-691 Cite-chip tooltip envelope wiring | `dailyos/provenance-tag` (existing primitive) | Envelope-resolved age + freshness; ADR-0108 actor-filtered render | n/a — primitive patch | ~150 |
| DOS-692 Trust-band keyboard nav + a11y | `dailyos/trust-band-badge` (existing primitive) | n/a — surface integration | n/a — primitive patch + ARIA | ~150 |
| DOS-693 HealthBadge label discipline | `dailyos/health-badge` (existing primitive) | Vocabulary registry + envelope state | n/a — primitive patch | ~120 |
| DOS-725 Project tint resolution | `wp/dailyos/theme/functions.php` `chrome_config()` | Resolved tint slug | n/a — config patch | ~10 |

**Wave-roll-up:** ~8,500 LOC across 4 outer entity-detail composites + 3 list shells + 2 metadata-proposal inner blocks + 5 primitive (new or patched) blocks. Bottom-up sized so a 5+ net-new-findings-per-cycle outcome triggers scope reset (per K-in discipline).

## 5. Detailed sections

### §5.1 DOS-462 Account Detail block

**Outer block:** `dailyos/account-detail` (already-scaffolded at `wp/dailyos/blocks/account-detail/`; W2 specializes the Stage 1b skeleton from `00f38b3b`).

**Outer/inner contract — Path B locked V1.1 (codex-challenge F1).** The W1 ability returns `AbilityResult<EntityIntelligenceEnvelope>` where `envelope.sections: BTreeMap<EnvelopeSection, SectionState>` enumerates 7 variants (`Facts`, `Health`, `MetadataProposals`, `OpenLoops`, `Touchpoints`, `Threads`, `Record`) per `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs:75`. **The 24 inner blocks are renderer-side projections over those 7 sections per ADR-0130 §4 Reading A** — each inner block reads one or more `EnvelopeSection` slices and projects display. No new substrate variants emitted; inner blocks correspond to *projection rules*, not to a new `Composition.sections[].blocks[]` shape.

**Block.json contract (templateLock + default template added V1.1 per architecture A1 + wp-skill H1):**
```json
{
  "apiVersion": 3,
  "name": "dailyos/account-detail",
  "category": "dailyos",
  "parent": null,
  "attributes": { "account_id": {"type":"string"}, "block_instance_id": {"type":"string"} },
  "providesContext": {
    "dailyos/entityType": "entity_type",
    "dailyos/entityId": "account_id",
    "dailyos/envelopeHandle": "envelope_handle"
  },
  "supports": { "html": false, "reusable": false, "inserter": true },
  "templateLock": false,
  "template": [
    ["dailyos/account-hero"],
    ["dailyos/sentiment-hero"],
    ["dailyos/triage-section"],
    ["dailyos/divergence-section"],
    ["dailyos/outlook-panel"],
    ["dailyos/on-track-chapter"],
    ["dailyos/supporting-tension"],
    ["dailyos/about-intelligence"],
    ["dailyos/account-pull-quote"],
    ["dailyos/stakeholder-grid"],
    ["dailyos/strategic-landscape"],
    ["dailyos/value-commitments"],
    ["dailyos/quote-wall"],
    ["dailyos/commercial-shape"],
    ["dailyos/account-technical-footprint"],
    ["dailyos/relationship-fabric"],
    ["dailyos/about-this-dossier"],
    ["dailyos/recommended-actions"],
    ["dailyos/touchpoints-feed"],
    ["dailyos/open-loops-feed"],
    ["dailyos/file-list"],
    ["dailyos/linear-issues-chapter"],
    ["dailyos/unified-timeline"],
    ["dailyos/finis-marker"]
  ],
  "render": "file:./render.php"
}
```

`templateLock: false` + the default `template` array ship the canonical chapter ordering on insert; user reorders via Site Editor. Authoritative ordering lives in the filesystem pattern at `wp/dailyos/patterns/account-detail-default.php` per wp-skill H4 — block.json `template` mirrors the pattern's canonical order so an empty-context insert renders meaningfully.

**`dailyos/envelopeHandle` resolution contract (wp-skill H2, V1.1):**

`providesContext` adds `dailyos/envelopeHandle`. The handle resolves as follows:

1. Outer block render.php invokes `get_entity_intelligence` ONCE per render via `$runtime_client->invoke_ability($producer, $payload, $scope_set)`.
2. The producer-side DOS-477 envelope cache (per `services::entity_intelligence::auth::envelope_cache`) keys the result by `(envelope_render_id, actor_principal_id, surface)` where `envelope_render_id` is the deterministic hash of `(entity_type, entity_id, depth, sections, watermark)` and `surface = "wp.account-detail"`.
3. The outer block writes `envelope_render_id` into block context as `dailyos/envelopeHandle`. Inner blocks `usesContext: ["dailyos/envelopeHandle"]` and pass the handle into their own `get_entity_intelligence` invocation (with the same scope_set), which short-circuits to the cached result without a second producer call.
4. Cache is per-request scope (lifetime = single PHP request); evicts on response close. DOS-477 cache implementation lives in W1 substrate; this is consumer-side discipline only.

A 2-arg invocation (without `$scope_set`) FAILS `check_w1_consumer_skeleton.sh` per W1 F5 fix at `00f38b3b`.

**render-functions.php invocation contract (locked):**
```php
$response = $runtime_client->invoke_ability(
  'get_entity_intelligence',
  [ 'entity_type' => 'account', 'entity_id' => $account_id,
    'depth' => 'Full',
    'sections' => null /* all 7 EnvelopeSection variants */ ],
  $scope_set  // 3rd arg mandatory per W1 F5 fix (00f38b3b)
);
```
A 2-arg invocation FAILS `check_w1_consumer_skeleton.sh`. The scope set resolves from the canonical filter `dailyos_surfaceclient_resolved_scopes`.

**Inner-block → envelope projection mapping (24 blocks — Path B per codex-challenge F1; chapter ordering translated 1-to-1 from `src/pages/AccountDetailPage.tsx`):**

| Inner block | Projects from envelope section(s) | Trust band source | Notes |
|---|---|---|---|
| `dailyos/account-hero` | Facts (identity facts: name, lifecycle, parent/child refs) | n/a — operational shell | No claim binding required at hero; entity-link via `SubjectRef`. |
| `dailyos/sentiment-hero` | Health + Facts (composed sentiment scalar + supporting voice quote) | `Health.aggregate_band` | SentimentHero projection rule from `compose_health()`. |
| `dailyos/triage-section` | Health (`hasTriageContent` derived) | per-row from `Health.factors` | Empty when `Health` reports no triage signal. |
| `dailyos/divergence-section` | Health (`hasDivergenceContent` derived) | per-row from `Health.factors` | Empty when divergence factors absent. |
| `dailyos/outlook-panel` | Health (`outlook` factor + `renewalCallVerdict` projection) | `Health.aggregate_band` | RenewalCallVerdict is a projection over health factors, not a substrate slice. |
| `dailyos/on-track-chapter` | Health (`on_track` factor) | `Health.aggregate_band` | |
| `dailyos/supporting-tension` | Health (composed view across factor delta) | per-row | |
| `dailyos/about-intelligence` | Facts (intelligence-source narrative) | aggregate from `Facts.provenance` | |
| `dailyos/account-pull-quote` | Facts (thesis narrative) | n/a — quote primitive | Quote selection rule lives in renderer. |
| `dailyos/stakeholder-grid` | Facts (people) + Touchpoints (last-touch / cadence per person) | per-person from Facts.subject_refs[].trust_band | Composes 2 sections. |
| `dailyos/strategic-landscape` | Facts (landscape narrative) | aggregate | |
| `dailyos/value-commitments` | Facts (commitments narrative) | per-commitment | |
| `dailyos/quote-wall` | Facts (voice quote bundle) | per-quote | |
| `dailyos/commercial-shape` | Facts (commercial shape narrative) | aggregate | |
| `dailyos/account-technical-footprint` | Facts (technical footprint narrative) | aggregate | |
| `dailyos/relationship-fabric` | Facts (relationship-fabric narrative) + Touchpoints | aggregate | Cross-section projection. |
| `dailyos/about-this-dossier` | Facts (dossier-about narrative) + envelope-level `EnvelopeProvenance` | n/a — meta | Provenance summary chip. |
| `dailyos/recommended-actions` | OpenLoops (recommended-actions subset) | per-action `OpenLoop.trust_band` | Filtered to actionable-suggested loops. |
| `dailyos/touchpoints-feed` | Touchpoints (envelope-first-page) + `useAbilityCursor` for subsequent pages | per-touchpoint | AgentMcp audience renders aggregate-only per V1.1 lock. |
| `dailyos/open-loops-feed` | OpenLoops | per-loop | First-page-in-envelope per wave §13 Q1; `useAbilityCursor` for pagination. |
| `dailyos/file-list` | Facts (attached file refs) | per-file | Files attached at the account scope (1-to-1 with Tauri `FileList` chapter). |
| `dailyos/linear-issues-chapter` | Facts (external Linear refs) | per-issue | |
| `dailyos/unified-timeline` | Record (record entries `Paginated<RecordEntry>`) + cross-cut from MetadataProposals lifecycle events | per-entry | Composes Record + lifecycle events. |
| `dailyos/finis-marker` | — | — | Chrome only; no envelope binding. |

**Code-shape sketch for a typical inner block (per V1.1 §10 invariant "code-shape sketch obligation"):**

```json
{
  "apiVersion": 3,
  "name": "dailyos/stakeholder-grid",
  "category": "dailyos",
  "usesContext": ["dailyos/entityType", "dailyos/entityId", "dailyos/envelopeHandle"],
  "supports": { "html": false, "reusable": false, "inserter": true },
  "render": "file:./render.php"
}
```

```php
// render.php (sketch — V1.1 code-shape obligation)
$handle = $block->context['dailyos/envelopeHandle'] ?? null;
$entity_id = $block->context['dailyos/entityId'] ?? null;
$envelope = dailyos_resolve_envelope($handle, 'account', $entity_id, $scope_set);
// projection: Facts (people) ∪ Touchpoints (last-touch per person)
$people = $envelope['sections']['facts']['people'] ?? [];
$touchpoints_by_person = dailyos_index_touchpoints_by_subject($envelope['sections']['touchpoints']);
if (empty($people)) {
    // §10 invariant: empty-state quiet chip, never silent-hidden
    return '<div class="dailyos-stakeholder-grid dailyos-stakeholder-grid--empty">'
         . '<span class="dailyos-empty-chip" data-empty-reason="no_stakeholders_in_facts">'
         . esc_html__( 'No stakeholders on record', 'dailyos' )
         . '</span>'
         . '</div>';
}
// ... render rows with trust-band per person + last-touch chip ...
```

**Inner block registration (every inner block):**
- `apiVersion: 3` (per wave §10 invariant; non-negotiable).
- NO `parent` field. Primitives stay inserter-global per ADR-0129 §2.
- `usesContext: ["dailyos/entityType", "dailyos/entityId", "dailyos/envelopeHandle"]` for envelope-binding.
- `render: "file:./render.php"`.
- Filesystem pattern at `wp/dailyos/patterns/account-detail-default.php` (theme-registered via `register_block_pattern`, per wp-skill H4) ships the default chapter ordering. Filesystem patterns insert-then-detach: user reordering does not affect other instances. **NOT** synced patterns (DB-stored reusable blocks).
- Empty-state pattern per V1.1 §10 invariant: when projection produces empty, render a quiet chip with `data-empty-reason="<reason>"`. NEVER silent-hidden (`return ''`). Reasons inherit envelope `Empty { reason }` where applicable.

**Acceptance criteria (per DOS-462 + wave AC #W1/W2/W6 + Intelligence Loop):**

- AC-462.1: Active `/accounts/$accountId` route consumes `dailyos/account-detail` rendering via `get_entity_intelligence`; legacy `get_account_detail` path serves operational shell only.
- AC-462.2: 24 inner blocks register with `apiVersion: 3`, no `parent`, `usesContext` for envelope binding.
- AC-462.3: Receipt rendering passes through `build_receipt_for_audience(target, audience, conn)` for every claim-bearing inner block — AgentMcp audience field allowlist enforced (per W1 AC-341.12).
- AC-462.4: `check_w1_consumer_skeleton.sh` CI gate green — `get_entity_intelligence` reference uses 3-arg `invoke_ability` signature.
- AC-462.5: DOS-461 no-bypass harness green: every `[data-claim-id]`-bearing rendered DOM node traces to a substrate claim via the envelope; no legacy `detail.intelligence` / `get_entity_context_entries` fallback path.
- AC-462.6: Visible-QA-state matrix from DOS-462 (full / empty / stale / `needs_verification` / corrected/superseded / proposal cue / proposal accepted / proposal dismissed / proposal edited / touchpoints present / cite-chip drawer) reachable in the WP block, screenshot-evidenced.
- AC-462.7: DOS-477 trust-boundary hardening satisfied — envelope-set validation gates close.
- AC-462.8: Filesystem pattern `account-detail-default.php` ships canonical chapter ordering (theme-registered via `register_block_pattern` per wp-skill H4; insert-then-detach semantics, NOT DB-stored synced pattern); ordering editable by user (per wave §10 invariant "Outer/inner block contract"; `templateLock: false`).
- AC-462.9: No PII in any fixture data — `account@example.com`, `subsidiary.com`, generic names per CLAUDE.md critical rule.

**Intelligence Loop check (5 questions):**
1. *Claim model:* Every visible claim-substantive string is bound to a `[data-claim-id]` ancestor projecting from the envelope's `EntityFact.provenance: ProvenanceRef` per W1 ADR-0130 amendment.
2. *Provenance + trust:* `source_asof` + actor-filtered render projection consumed via `ProvenanceTag` (DOS-691) and `TrustBandBadge` (DOS-692). Trust band colors per ADR-0125.
3. *Signals + invalidation:* **Inherited uniform answer per wave §10:** pull on render + user refresh. No new signals at v1.4.4.
4. *Runtime + surfaces:* Envelope consumed via `invoke_ability('get_entity_intelligence', ..., $scope_set)`; surfaces are the 4 entity-detail composites (this packet) + MCP envelope consumer (deferred to v1.4.7). Surface-agnostic per ADR-0130.
5. *Feedback loop:* Corrections from `dailyos/metadata-proposal-drawer` (§5.6) and DOS-8 typed feedback affordances on cite-chip drawer (§5.7) route through `services::claims::record_claim_feedback`; envelope re-fetch on next render reflects lifecycle state.

**L1 parallel-fan-out protocol** (per K-out `parallel-agent-commit-hook-contention-2026-05-20.md`): the 24 inner blocks split across at most 5 codex tasks; each task scoped to a contiguous chapter cluster (Health / Context-A / Context-B / Work / Record). Codex tasks emit code; I commit + push manually with `--no-verify` to avoid commit-hook lock contention. Polling cadence 3-min per `feedback_polling_cadence_3min.md`; cancel + re-dispatch at 5-min no-write.

---

### §5.2 DOS-483 Project Detail block

**Outer block:** `dailyos/project-detail` (scaffolded at `wp/dailyos/blocks/project-detail/`; specializing Stage 1b skeleton).

**Outer/inner contract — Path B locked V1.1:** Same renderer-side projection over `BTreeMap<EnvelopeSection, SectionState>` as Account Detail. The 15 inner blocks project from the same 7 `EnvelopeSection` variants — no project-specific section variants emitted by W1.

**block.json delta vs Account Detail (concrete code-shape sketch per V1.1 §10 invariant):**

```json
{
  "apiVersion": 3,
  "name": "dailyos/project-detail",
  "category": "dailyos",
  "parent": null,
  "attributes": { "project_id": {"type":"string"}, "block_instance_id": {"type":"string"} },
  "providesContext": {
    "dailyos/entityType": "entity_type",
    "dailyos/entityId": "project_id",
    "dailyos/envelopeHandle": "envelope_handle"
  },
  "supports": { "html": false, "reusable": false, "inserter": true },
  "templateLock": false,
  "template": [
    ["dailyos/project-hero"],
    ["dailyos/vitals-strip"],
    ["dailyos/portfolio-chapter"],
    ["dailyos/trajectory-chapter"],
    ["dailyos/horizon-chapter"],
    ["dailyos/watch-list"],
    ["dailyos/watch-list-milestones"],
    ["dailyos/stakeholder-gallery"],
    ["dailyos/the-work"],
    ["dailyos/touchpoints-feed"],
    ["dailyos/open-loops-feed"],
    ["dailyos/linear-issues-chapter"],
    ["dailyos/unified-timeline"],
    ["dailyos/recommended-actions"],
    ["dailyos/project-appendix"]
  ],
  "render": "file:./render.php"
}
```

Delta vs Account Detail block.json: `providesContext` keys `dailyos/entityType="project"`, `dailyos/entityId="project_id"`; attributes carry `project_id` not `account_id`; outer wrapper style attribute carries `--dailyos-project-tint` CSS custom property per DOS-725 (V1.1 lock below).

**render-functions.php contract:** same 3-arg invocation discipline; `entity_type: 'project'`. Outer wrapper emitted via:

```php
$wrapper = get_block_wrapper_attributes([
  'style' => '--dailyos-project-tint: var(--color-garden-olive);',
]);
echo "<div {$wrapper}>{$content}</div>";
```

**DOS-725 tint resolution (V1.1 lock — codex-challenge F3 + wp-skill H3):**

Per the canonical decision (James 2026-05-21): tint flows as **CSS custom property `--dailyos-project-tint` on the outer wrapper only**, via `get_block_wrapper_attributes()`. This is the narrow exception to memory `feedback_no_inline_css` — the style attribute body must match `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` (custom-property assignment only; `--dailyos-*` namespace; no declarative styles, no other custom-property prefixes).

**Prerequisite for W2 L1 start (V1.1):** ADR-0077 amendment adding project tint = `olive` (token `--color-project: var(--color-garden-olive)` from `src/styles/design-tokens.css:87`) **must land before any W2 L1 work begins**. AC-W2.7 + AC-483.3 gate on the amendment SHA. Until amendment lands, `chrome_config()` emits olive default (`$stub_tints['dailyos_project'] = 'olive'`) — this preserves the chrome-lane rendering during the amendment-author window.

**New CI script (file at L1 kickoff):** `src-tauri/scripts/check_no_inline_style_exception.sh`. Lints `wp/dailyos/blocks/**/*.php` and asserts that any `style=` attribute body matches the custom-property-only regex above. Anything else fails the gate. Pairs with the existing `check_w1_consumer_skeleton.sh`. Sister script for the `feedback_no_inline_css` cardinal rule.

**Inner-block → envelope projection mapping (15 blocks — Path B; chapter ordering from `src/pages/ProjectDetailEditorial.tsx`):**

| Inner block | Projects from envelope section(s) | Trust band source | Notes |
|---|---|---|---|
| `dailyos/project-hero` | Facts (project identity + thesis narrative) | n/a — operational shell | |
| `dailyos/vitals-strip` | Health (composed counts/trends) | `Health.aggregate_band` | Reusable across entity types per §5.3. |
| `dailyos/portfolio-chapter` | Facts (parent-project portfolio refs) | per-child aggregate | Empty when leaf project — `data-empty-reason="leaf_project_no_portfolio"`. |
| `dailyos/trajectory-chapter` | Health (trajectory factor) | `Health.aggregate_band` | |
| `dailyos/horizon-chapter` | Facts (horizon narrative) + Health (horizon outlook) | aggregate | |
| `dailyos/watch-list` | OpenLoops (watch-list subset) | per-loop | |
| `dailyos/watch-list-milestones` | Facts (milestones) + OpenLoops (milestone-bound loops) | per-milestone | Composes 2 sections. |
| `dailyos/stakeholder-gallery` | Facts (people) + Touchpoints (last-touch) | per-person | Same composition shape as Account stakeholder-grid. |
| `dailyos/the-work` | Facts (attached-work refs) | per-work-item | |
| `dailyos/touchpoints-feed` | Touchpoints | per-touchpoint | AgentMcp aggregate-only render per V1.1 lock. |
| `dailyos/open-loops-feed` | OpenLoops | per-loop | First-page-in-envelope; `useAbilityCursor` for pagination. |
| `dailyos/linear-issues-chapter` | Facts (external Linear refs) | per-issue | 1-to-1 with `ProjectDetailEditorial.tsx:526` `LinearIssuesChapter` block. |
| `dailyos/unified-timeline` | Record (`Paginated<RecordEntry>`) + MetadataProposals lifecycle | per-entry | |
| `dailyos/recommended-actions` | OpenLoops (recommended subset) | per-action | |
| `dailyos/project-appendix` | envelope-level `EnvelopeProvenance` + all section caveats | aggregate | Evidence + provenance affordances. |

**Acceptance criteria:**

- AC-483.1: `/projects/$projectId` consumes `dailyos/project-detail` via `get_entity_intelligence` (entity_type=project); legacy `get_project_detail` shell-data only.
- AC-483.2: All 15 inner blocks consume envelope-shaped data; legacy `useEntityContextEntries("project", ...)` is NOT a fallback path.
- AC-483.3: ADR-0077 amendment lands with project tint = `olive`; `chrome_config()` `$stub_tints['dailyos_project'] = 'olive'`; AtmosphereLayer renders olive tint on `singular(dailyos_project)`.
- AC-483.4: DOS-461 no-bypass harness green for `/projects/$projectId`.
- AC-483.5: DOS-477 trust-boundary hardening satisfied.
- AC-483.6: Visible-QA-state matrix from DOS-483 (full / empty / stale / `needs_verification` / corrected/superseded / touchpoints present / cite-chip drawer / parent-project portfolio state) reachable + screenshot-evidenced.
- AC-483.7: Receipt rendering through `build_receipt_for_audience` for every claim-bearing inner block.
- AC-483.8: No PII in fixture; filesystem pattern at `wp/dailyos/patterns/project-detail-default.php` (theme-registered).

**Intelligence Loop check:** identical structure to §5.1. Q3 inherits the uniform wave-level answer.

---

### §5.3 DOS-484 Person Detail block

**Outer block:** `dailyos/person-detail` (scaffolded at `wp/dailyos/blocks/person-detail/`).

**Outer/inner contract — Path B locked V1.1:** Same renderer-side projection model. 12 inner blocks project from the 7 `EnvelopeSection` variants.

**block.json delta (concrete code-shape sketch per V1.1 §10 invariant):**

```json
{
  "apiVersion": 3,
  "name": "dailyos/person-detail",
  "category": "dailyos",
  "parent": null,
  "attributes": { "person_id": {"type":"string"}, "block_instance_id": {"type":"string"} },
  "providesContext": {
    "dailyos/entityType": "entity_type",
    "dailyos/entityId": "person_id",
    "dailyos/envelopeHandle": "envelope_handle"
  },
  "supports": { "html": false, "reusable": false, "inserter": true },
  "templateLock": false,
  "template": [
    ["dailyos/person-hero"],
    ["dailyos/vitals-strip"],
    ["dailyos/person-insight-chapter"],
    ["dailyos/person-network"],
    ["dailyos/person-relationships"],
    ["dailyos/watch-list"],
    ["dailyos/the-work"],
    ["dailyos/touchpoints-feed"],
    ["dailyos/open-loops-feed"],
    ["dailyos/unified-timeline"],
    ["dailyos/recommended-actions"],
    ["dailyos/person-appendix"]
  ],
  "render": "file:./render.php"
}
```

Delta vs Account Detail: `providesContext` keys `dailyos/entityType="person"`, `dailyos/entityId="person_id"`; attributes carry `person_id` not `account_id`.

**Merge-picker substrate (per W0 audit + §13 Q6):**

W0 surface-audit flagged "Person merge picker substrate" as **unfiled — needs Linear ticket if W2 includes the merge action in the WP block." This packet RECOMMENDS folding merge-picker scope into DOS-484. Merge today lives in Tauri React (`PersonDetailEditorial.tsx` Dialog + `handleMerge` on `useEntitySuppressions`); WP parity requires a merge ability. Two paths:

- **Path α (preferred, no W1 reopen):** merge stays a Tauri-only affordance for v1.4.4; WP Person Detail surfaces a `merge_intent` claim feedback that signals user intent without writing. Wave §10 inline-edit-affordance contract (anchored decision #2) supports this — the intent emits as `FeedbackAction::MergeIntent` (unit variant; per-variant payload `payload_json: { merge_target: SubjectRef, supporting_evidence?: String }` validated by `services::claim_receipt::feedback::validate_and_sanitize_metadata` per ADR-0123 V1.1 §1) through `services::claims::record_claim_feedback`. The actual merge runs Tauri-side. This treats the WP-side affordance as a signal, not a mutation.
- **Path β (W1 reopen):** add `merge_persons` ability to W1 substrate. Per wave AC #W5 C4 supersedes path-α when substrate gap blocks a named AC. Since DOS-484's named AC includes "merge flow… may remain on service-backed person commands," Tauri retention is acceptable; path α applies.

**Locked at L0 close: path α.** If L4 reveals user friction with Tauri-only merge during the W2 build-out window, path β escalates as an in-wave amendment per wave AC #W5 — NOT deferred to v1.4.5+.

**Inner-block → envelope projection mapping (12 blocks — Path B; from `src/pages/PersonDetailEditorial.tsx`):**

| Inner block | Projects from envelope section(s) | Trust band source | Notes |
|---|---|---|---|
| `dailyos/person-hero` | Facts (person identity + profile/story narrative) | n/a — operational shell | |
| `dailyos/vitals-strip` | Health (composed counts/trends) | `Health.aggregate_band` | Reused from §5.2. |
| `dailyos/person-insight-chapter` | Health (rhythm/dynamic factors) | `Health.aggregate_band` | |
| `dailyos/person-network` | Facts (network refs to accounts/projects/people) | per-edge | |
| `dailyos/person-relationships` | Facts (relationship narrative) + Touchpoints (per-relationship cadence) | per-relationship | Composes 2 sections. |
| `dailyos/watch-list` | OpenLoops (watch-list subset) | per-loop | |
| `dailyos/unified-timeline` | Record (`Paginated<RecordEntry>`) + MetadataProposals lifecycle | per-entry | |
| `dailyos/recommended-actions` | OpenLoops (recommended subset) | per-action | Includes merge-intent affordance per path α. |
| `dailyos/the-work` | Facts (attached-work refs) | per-work-item | |
| `dailyos/touchpoints-feed` | Touchpoints | per-touchpoint | AgentMcp aggregate-only render per V1.1 lock. |
| `dailyos/open-loops-feed` | OpenLoops | per-loop | `useAbilityCursor` for pagination. |
| `dailyos/person-appendix` | envelope-level `EnvelopeProvenance` + all section caveats | aggregate | Evidence + provenance affordances. |

**Acceptance criteria:**

- AC-484.1: `/people/$personId` consumes `dailyos/person-detail` via `get_entity_intelligence` (entity_type=person); `get_person_detail` shell-data only.
- AC-484.2: `useEntityContextEntries("person", ...)` is NOT a fallback path; DOS-174 stays solved by the shared envelope.
- AC-484.3: Merge affordance emits `FeedbackAction::MergeIntent` (path α); WP-side block does NOT call `services::persons::merge` directly. Variant lands via parallel W1 extension at `01d0cff3 (merged at 0d82502f)` (sibling to Meeting `EntityKind` extension `87df7cf6`); W2 L1 dispatch for §5.3 PRs waits for the extension to land.
- AC-484.4: DOS-461 no-bypass harness green; DOS-477 satisfied.
- AC-484.5: Visible-QA-state matrix from DOS-484 (incl. ambiguous/multi-account association) reachable + screenshot-evidenced.
- AC-484.6: Receipt rendering through `build_receipt_for_audience` for every claim-bearing inner block.
- AC-484.7: No PII in fixture (`user@example.com`, generic role labels); filesystem pattern at `wp/dailyos/patterns/person-detail-default.php` (theme-registered).

**Intelligence Loop check:** identical structure; Q3 inherits uniform answer. Q5 feedback loop adds `FeedbackAction::MergeIntent` as the 10th variant of the typed enum (unit-only enum per ADR-0123 V1.1 §1 implementation note; payload carried via `payload_json.merge_target: SubjectRef` + optional `supporting_evidence: String ≤500 chars`, validated by `validate_and_sanitize_metadata` arm). **V1.2 fold (2026-05-21):** per James "every surface in 1.4.4 — no deferrals" mandate, the v1.4.5 deferral previously noted here is removed. `MergeIntent` lands in a parallel W1 substrate amendment (sibling to the Meeting `EntityKind` extension at `87df7cf6`); the W1 enum extension SHA is `01d0cff3 (merged at 0d82502f)` (cycle-4 dispatch waits for the extension to land before opening W2 L1 PRs that emit `MergeIntent`). Path α holds as the user-facing affordance shape (WP block emits intent feedback; actual merge stays Tauri-side); the substrate extension is purely additive (new enum variant + `record_claim_feedback` arm) and does not reopen any other W1 contract.

---

### §5.4 Meeting Detail block (new sub-ticket)

**Outer block:** `dailyos/meeting-detail` (NEW; scaffold at `wp/dailyos/blocks/meeting-detail/` at L1 kickoff).

**Meeting substrate — Path A locked V1.1 (codex-challenge F2 + architecture A3):**

W1 substrate extended to support Meeting via commit `87df7cf6` (`feat(abilities): extend get_entity_intelligence with Meeting EntityKind (W2 L0 cycle-1 F2 fix)`), merged into wave at `c5c0578f`. `EntityKind` is now `{ Account, Project, Person, Meeting }` per `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs`. Meeting subject composes the standard 7 `EnvelopeSection` variants:

- **Facts** — attendees, location, status, agenda narrative.
- **Health** — composed from `services::meeting_prep_status` (readiness + freshness).
- **Touchpoints** — related meetings (cadence to the same attendee set / account / project).
- **OpenLoops** — agenda items, follow-ups, post-meeting actions.
- **Threads** / **Record** — meeting transcript / notes / decision log.
- **MetadataProposals** — attendee corrections, location updates, related-entity refinements.

AgentMcp audience scrub applies per W1 cycle-2 F2 pattern (no per-attendee identifying data, no raw meeting IDs, etc.).

**Linear ticket status:** no Linear ticket at packet-author time. **File at L1 kickoff** with title "v1.4.4 W2 — Meeting Detail block consumes entity-intelligence envelope + meeting prep DTO". Project = v1.4.4 WP Surface Migration. Priority = High. AC list below seeds the ticket description.

**Outer/inner contract — Path B locked V1.1:** Same renderer-side projection. 10 inner blocks project from the 7 `EnvelopeSection` variants (Meeting-extended at `87df7cf6`).

**block.json (concrete code-shape sketch per V1.1 §10 invariant):**

```json
{
  "apiVersion": 3,
  "name": "dailyos/meeting-detail",
  "category": "dailyos",
  "parent": null,
  "attributes": { "meeting_id": {"type":"string"}, "block_instance_id": {"type":"string"} },
  "providesContext": {
    "dailyos/entityType": "entity_type",
    "dailyos/entityId": "meeting_id",
    "dailyos/envelopeHandle": "envelope_handle"
  },
  "supports": { "html": false, "reusable": false, "inserter": true },
  "templateLock": false,
  "template": [
    ["dailyos/meeting-header"],
    ["dailyos/prep-status"],
    ["dailyos/agenda-draft"],
    ["dailyos/attendees-section"],
    ["dailyos/related-entities"],
    ["dailyos/claims-for-review"],
    ["dailyos/context-bundle"],
    ["dailyos/post-meeting-capture"],
    ["dailyos/touchpoints-feed"],
    ["dailyos/recommended-actions"]
  ],
  "render": "file:./render.php"
}
```

Delta vs Account Detail: `providesContext` keys `dailyos/entityType="meeting"`, `dailyos/entityId="meeting_id"`; attributes carry `meeting_id`. Outer invokes `get_entity_intelligence` (entity_type=meeting) AND `get_meeting_prep_status` (DOS-335) — two-call composition per wave §10 invariant (composite blocks may invoke a sibling DTO-shaped ability alongside the entity envelope; cache key includes both abilities' watermarks).

**Inner-block → envelope projection mapping (10 blocks — Path B):**

| Inner block | Projects from envelope section(s) | Trust band source | Notes |
|---|---|---|---|
| `dailyos/meeting-header` | Facts (title, time, organizer) | n/a — operational shell | |
| `dailyos/prep-status` | Health (composed from `MeetingPrepStatus`) + DOS-335 direct read | `Health.aggregate_band` | Two-call composition — envelope's Health composes with DOS-335 read. Surfaces FolioBar readiness signal via chrome.js. |
| `dailyos/agenda-draft` | Facts (agenda narrative) + OpenLoops (agenda-bound action items) | per-item | Composes 2 sections. |
| `dailyos/attendees-section` | Facts (attendees) | per-attendee | |
| `dailyos/related-entities` | Facts (`related_entities: [SubjectRef]` resolved) | per-edge | Links to Account/Project/Person composites. |
| `dailyos/claims-for-review` | MetadataProposals (meeting-bound proposals) | per-proposal | Surfaces DOS-328 metadata proposals on the meeting subject. |
| `dailyos/context-bundle` | Facts (referenced entity context) + envelope-level `EnvelopeProvenance` | aggregate | Links into Account/Project/Person detail composites. |
| `dailyos/post-meeting-capture` | Record (transcript/notes) | aggregate | Write path through `process_paste_transcript` ability — NO direct DB writes. |
| `dailyos/touchpoints-feed` | Touchpoints (related-meeting cadence) | per-touchpoint | AgentMcp aggregate-only render per V1.1 lock. |
| `dailyos/recommended-actions` | OpenLoops (recommended-actions subset) | per-action | |

**Acceptance criteria:**

- AC-MD.1: `/meeting/$meetingId` consumes `dailyos/meeting-detail` via `get_entity_intelligence` (entity_type=meeting); `gather_meeting_context` shell-data only.
- AC-MD.2: `dailyos/prep-status` inner block consumes `services::meeting_prep_status::read` (DOS-335); writes go through `meeting_prep_status::write` per W1 architecture F2 split.
- AC-MD.3: `dailyos/post-meeting-capture` write path goes through `process_paste_transcript` ability; NO direct DB writes from PHP/JS.
- AC-MD.4: Cross-stack navigation works (per wave §5.x dogfooding-flow §m semantics) — link from `dailyos/related-entities` to a Tauri-routed account detail resolves through the existing Tauri router during build-out window.
- AC-MD.5: DOS-461 no-bypass harness extended to cover meeting entity. Meeting `EntityKind` extension landed at `87df7cf6` per V1.1 lock; meeting fixture verification gate confirmed at L1 Day-1 (was previously "verify at L1 kickoff"; now confirmed-landed).
- AC-MD.6: Visible-QA-state matrix (meeting prep-ready / prep-not-started / agenda-drafting-in-flight / post-meeting-pending / claim correction in-flight per DOS-335) reachable + evidenced.
- AC-MD.7: No PII in fixture; filesystem pattern at `wp/dailyos/patterns/meeting-detail-default.php` (theme-registered).

**Intelligence Loop check:**
1. *Claim model:* Claims surfaced in `claims_for_review` and `agenda` bound to envelope via `ProvenanceRef`; meeting itself carries `meeting_id` as `SubjectRef` per ADR-0125.
2. *Provenance + trust:* TrustBandBadge on each agenda item; PrepStatus carries `freshness` + `integrity` (DOS-335 V1.1 architecture F3 — composed `BriefingState`-shaped struct, same model).
3. *Signals + invalidation:* Inherits uniform pull-on-render answer. **One exception:** `MeetingPrepStatusChanged` signal (predeclared per W1 architecture F3) drives FolioBar readiness indicator refresh client-side via chrome.js `chrome_config()` data attribute — chrome lane primitive, not a W2-novel signal. Per wave §10 invariant "Refresh model", §3 still answers uniformly: WP block pulls on render; chrome receives signal via chrome.js — not a per-surface push.
4. *Runtime + surfaces:* Block invokes `get_entity_intelligence` + `get_meeting_prep_status`. Surfaces: this block, FolioBar (chrome), Daily Briefing (W3).
5. *Feedback loop:* Claim corrections in `claims_for_review` route through DOS-8 typed feedback; post-meeting transcript write feeds claim emission via existing transcript-ingest path.

---

### §5.5 Entity list shells (Accounts / People / Projects)

**Blocks (NEW):** `dailyos/accounts-list`, `dailyos/people-list`, `dailyos/projects-list`. Flat list blocks; NO inner blocks (rendered list-row primitives consume context the list block exposes per ADR-0129 §2).

**Cursor pagination contract (V1.1 — codex-challenge F5 fix; consumes W1 `Paginated<T>`):**

List blocks consume `executeAbility()` (WP 7.0 client-side Abilities API; M1 fallback path-α for WP 6.x) — NOT `useEntityRecords` (`@wordpress/core-data`). List abilities return W1's actual `Paginated<T>` shape per `src-tauri/abilities-runtime/src/abilities/get_entity_intelligence/contracts.rs:149`:

```rust
// W1 substrate — DO NOT invent client-side shapes around this
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<Cursor>,       // opaque server-signed token
    pub total_hint: Option<u64>,
    pub cursor_state: CursorState,         // Stable | DataShifted | Invalidated
}

pub enum CursorState {
    Stable,
    DataShifted { advisory: String },
    Invalidated { reason: String, restart_required: bool },
}
```

TypeScript projection on the WP client side mirrors the wire shape directly (camelCase per `#[serde(rename_all = "camelCase")]`):

```typescript
type Cursor = string;  // opaque; client MUST NOT parse
type CursorState =
  | { kind: "stable" }
  | { kind: "data_shifted"; advisory: string }
  | { kind: "invalidated"; reason: string; restart_required: boolean };

type Paginated<T> = {
  items: T[];
  nextCursor: Cursor | null;
  totalHint: number | null;
  cursorState: CursorState;
};
```

**Shared hook ships at `wp/dailyos/blocks/_shared/hooks/useAbilityCursor.ts`** (NEW; W2 substrate per V1.1 §3 K-in). Signature:
```typescript
function useAbilityCursor<T>(abilityName: string, payload: object, scopeSet: string[]):
  {
    items: T[];
    loading: boolean;
    loadMore: () => void;
    done: boolean;       // !nextCursor || cursorState.kind === "invalidated" && !restart_required
    reset: () => void;   // user-initiated reset; also called automatically on cursor_state.invalidated
    advisory: string | null;  // surfaces DataShifted advisory to the caller
  };
```

Reset semantics per wave §10 invariant: cursor resets when watermark changes (signal-driven cache invalidation per v1.4.2 W4-F cache discipline carried forward) OR when `cursor_state` arrives as `Invalidated { restart_required: true }`. `DataShifted` surfaces the advisory string through the hook return; caller decides whether to display.

**List item shape:** `EntityListItem` typed projection from `get_entity_intelligence` with `depth: Shallow` and `sections: [Facts]` (the only `EnvelopeSection` variant that's load-bearing for a list row — name, lifecycle, trust-band-aggregate via `Facts.provenance`). NOT `sections: [Facts, Vitals]` (Vitals is not an `EnvelopeSection` variant — V1.0 invented that name; corrected V1.1 per codex-challenge F5).

**Acceptance criteria (shared across 3 list shells):**

- AC-L.1: List block invokes `get_entity_intelligence` (or list-mode ability if W1 envelope distinguishes; verify at L1) with `depth: Shallow`; cursor pagination via `useAbilityCursor`.
- AC-L.2: NO `useEntityRecords` calls in list block source.
- AC-L.3: Visible-QA-states: full list / empty (`data-empty-reason` per V1.1 invariant) / loading-more / load-more-error / `cursor_state.data_shifted` (advisory chip surfaced from hook) / `cursor_state.invalidated.restart_required` (automatic reset + toast).
- AC-L.4: List rendering passes envelope items through `build_receipt_for_audience` for any claim-bearing field (e.g., trust band per row).
- AC-L.5: People-list shows merge-affordance affordance (per §5.3 path α) — emits `MergeIntent` not direct mutation.
- AC-L.6: Filesystem patterns at `wp/dailyos/patterns/{accounts,people,projects}-list-default.php` (theme-registered via `register_block_pattern`, per wp-skill H4); user can reorder column visibility via Site Editor. NOT synced patterns.

**Intelligence Loop check:** Q1 every list row binds to `SubjectRef` per ADR-0125; Q2 trust band per row from envelope; Q3 uniform pull-on-render; Q4 list ability is invocable from MCP (deferred to v1.4.7 scope manifest); Q5 row-level corrections route through standard feedback.

---

### §5.6 Metadata proposals interaction (DOS-328)

**Inner blocks (NEW):** `dailyos/metadata-proposal-cue` (peripheral cue on field) + `dailyos/metadata-proposal-drawer` (typed accept/dismiss/edit affordance).

**Surfaces consumed:** Account Detail Context view chapters (CommercialShape, AccountTechnicalFootprint, RelationshipFabric, StrategicLandscape, ValueCommitments per DOS-328 target list). Cue inner block embeds into each field; drawer inner block embeds once per outer composite (singleton).

**Envelope consumption:** `EntityIntelligenceEnvelope.metadata_proposals: Vec<MetadataProposal>` (W1 DOS-459 envelope slice). Each `MetadataProposal` carries `proposal_id`, `subject_ref`, `field_path`, typed value, `sensitivity`, `lifecycle_state`, `trust_band`, `evidence_summary`, `provenance: ProvenanceRef` per DOS-328 AC.

**Accept/Dismiss/Edit wire shape (per wave anchored decision #2 + W1 DOS-8 typed feedback):**

```
Accept  → FeedbackAction::ProposalAccept { proposal_id, confirmed_value }
Dismiss → FeedbackAction::ProposalDismiss { proposal_id, reason: Option<sanitized_text> }
Edit    → FeedbackAction::ProposalEdit { proposal_id, corrected_value }
```

All three route through `services::claims::record_claim_feedback` (existing — no W1 reopen). Validation gate: each action calls `services::entity_intelligence::auth::validate_envelope_target(proposal_id, envelope_handle)` per W1 AC-477.13 (envelope-set validation). Rejection on wrong-subject / stale-envelope.

**Acceptance criteria (per DOS-328):**

- AC-328.1: Account Detail Context view shows proposal states: no-proposal (quiet) / one-proposal (peripheral cue) / multiple-proposals (calm section-level cue) / stale-proposal / `needs_verification`-suppressed.
- AC-328.2: Drawer shows typed proposed value, target field, evidence summary, trust band, freshness caveat, display-safe provenance per ADR-0108. NO raw source identifiers.
- AC-328.3: Accept/dismiss/edit writes through `record_claim_feedback`; no direct PHP/JS DB writes (per wave critical rule).
- AC-328.4: Edit free-text (corrected_value) passes through ADR-0108 §3 sanitizer (per wave AC #W4 free-text sanitizer clause).
- AC-328.5: Envelope-set validation gates every accept/dismiss/edit; wrong-subject proposal returns `TargetNotFound`.
- AC-328.6: User-owned `entity_metadata` lane preserved — AI does NOT write directly; only via accepted proposal → confirmed value.
- AC-328.7: Glean-sourced proposals show display-safe provenance with citations + freshness; no raw source-internal IDs leaked.
- AC-328.8: UX does not re-introduce suggested-fill failure mode (no persistent nags, no maintenance-strip takeover).
- AC-328.9: Class-sweep applied per `prompt-channel-sensitivity-class-sweep-2026-05-18.md` — accept/dismiss/edit channels all enumerated and gated identically.

**Intelligence Loop check:**
1. *Claim model:* `MetadataProposal` is a claim subtype per ADR-0125 typed registry; carries lifecycle state + sensitivity.
2. *Provenance + trust:* Trust band + freshness on each proposal; cue suppressed at low trust per AC-328.1.
3. *Signals + invalidation:* Uniform pull-on-render. Lifecycle state change re-renders on next envelope fetch.
4. *Runtime + surfaces:* Cue inner block + drawer inner block — both consume envelope via outer's `providesContext`. MCP exposure via the same feedback ability.
5. *Feedback loop:* All 3 actions feed claim lifecycle + source reliability via existing pipelines (`record_claim_feedback` → `update_source_reliability` per ADR-0105).

---

### §5.7 Primitive folds (DOS-688 / DOS-689 / DOS-690 / DOS-691 / DOS-692 / DOS-693)

Six DOS-325/DOS-9/DOS-11 lineage tickets. Each is a primitive-level patch or surface-residue integration; they don't introduce new outer/inner block contracts but ship as supporting work alongside §5.1–§5.4.

#### DOS-688 — TrendStrip primitive

**New block:** `dailyos/trend-strip` at `wp/dailyos/blocks/trend-strip/`. block.json `apiVersion: 3`, no `parent`, `usesContext: ["dailyos/entityType","dailyos/entityId","dailyos/envelopeHandle"]`. `render: file:./render.php`.

**Acceptance:**
- AC-688.1: Tauri React `src/components/ui/TrendStrip.tsx` + `.module.css` shipped.
- AC-688.2: `.docs/design/primitives/TrendStrip.md` spec authored per DOS-325 voice rule (no raw numbers in headline).
- AC-688.3: WP block translates Tauri React; visual parity matrix at L4 (semantic + interaction/layout axes per v1.4.3 W2 §7.1 template).
- AC-688.4: Consumes envelope's trust band — no raw factor values.

#### DOS-689 — EvidenceDrawer primitive + integration

**New block:** `dailyos/evidence-drawer`. Drawer integrates into entity-detail surfaces (Account / Project / Person) per DOS-689 scope.

**ADR-0130 §3.1 10-channel enumeration (per DOS-689 AC; class-sweep per K-in security solution):**

The drawer surfaces 10 render channels per ADR-0130 §3.1. Each must pass the negative fixture asserting no disallowed field leaks:
1. `band_label`, 2. `score_value`, 3. `factor_breakdown`, 4. `evidence_summary`, 5. `citation_list`, 6. `freshness_caveat`, 7. `trust_band`, 8. `lifecycle_state`, 9. `provenance_ref` (actor-filtered), 10. `corrected_text` (sanitized).

Field allowlist resolves from `ProvenanceRef`-resolved envelope's actor-filtered render projection per ADR-0108. **No raw source-internal identifiers, no email addresses, no internal note bodies, no debug carriers** (per DOS-689 AC).

**Acceptance:**
- AC-689.1: Tauri React + WP block + design spec landed.
- AC-689.2: Class-sweep negative fixture: all 10 channels covered.
- AC-689.3: Drawer integration into Account Detail at L4 (minimum); Project / Person drawer integration as per-detail consumption (DOS-690).
- AC-689.4: `/cso` reviewer green-lit field allowlist.

#### DOS-690 — Entity-detail surface residue

**Patch (no new block):** envelope-wiring + keyboard navigation across composites + drawer integration. Covered inline by §5.1–§5.3 specialization work. Tracking ticket gates wave-close per DOS-690 AC.

**Acceptance:**
- AC-690.1: Account Detail full integration: ScoreBand (v1.4.3 W2 shipped) + TrendStrip (DOS-688) + EvidenceDrawer (DOS-689) + envelope wiring at L4.
- AC-690.2: Project + Person Detail: subset consumption per DOS-325 §"Surfaces in scope."
- AC-690.3: Keyboard nav per DOS-692 (sibling).
- AC-690.4: Voice-rule compliance per ADR-0083 across composition.

#### DOS-691 — Cite-chip tooltip envelope wiring

**Patch:** `wp/dailyos/blocks/provenance-tag/` (existing primitive from v1.4.3 W2). Resolve age + freshness from envelope, not surface-local computation.

**Acceptance:**
- AC-691.1: ProvenanceTag tooltip resolves from envelope on Account Detail at L4.
- AC-691.2: DOS-477 10-channel negative fixture green: tooltip body + data attributes + ARIA labels carry no disallowed source-internal data.

#### DOS-692 — Trust-band keyboard nav + a11y

**Patch:** `wp/dailyos/blocks/trust-band-badge/`. Keyboard nav across surface compositions; ARIA labels + screen-reader announcements for band transitions.

**Acceptance:**
- AC-692.1: Account Detail keyboard nav across TrustBandBadge + adjacent primitives at L4.
- AC-692.2: Screen-reader pass on Account Detail per design-system a11y discipline.

#### DOS-693 — HealthBadge label discipline

**Patch:** `wp/dailyos/blocks/health-badge/`. Apply DOS-325 voice rule — band-label vocabulary alongside color tokens (ScoreBand model: `On Track | Watching | Action Needed | No Read`).

**Decision on label collision with ScoreBand:** HealthBadge labels use the SAME 4-label vocabulary as ScoreBand (intentional shared band-rendering vocabulary; consistent user mental model). Distinguishing context comes from surrounding component label (e.g., "Account health" vs "Renewal score"), not from divergent band vocabularies. Decided at this packet per DOS-693 AC "Decide whether HealthBadge labels collide with ScoreBand vocabulary."

**Acceptance:**
- AC-693.1: HealthBadge.md spec updated with band-label vocabulary + ScoreBand boundary discipline.
- AC-693.2: WP block consumes labels via vocabulary registry per ADR-0083.
- AC-693.3: Visual parity matrix at L4 across new label states.
- AC-693.4: Co-render scenario with ScoreBand validated at L4 on Account Detail.

---

## 6. Substrate consumed (W1 reuse map)

Inventory of W1 substrate the W2 wave consumes, by source. None reinvented; every named item is a consumer relationship with a citable producer SHA on `wave/v1.4.4-w1-stage1a`.

| W1 producer | W2 consumer | Producer SHA | Reuse vs reinvent? |
|---|---|---|---|
| `get_entity_intelligence` ability | All 4 entity-detail outer blocks (§5.1–5.4) + 3 list shells (§5.5) | `2b3915ef` | Reuse |
| `EntityIntelligenceEnvelope` DTO | All inner blocks via `providesContext`/`usesContext` chain | `2b3915ef` | Reuse |
| Touchpoints + open-loops paginated bundles | `dailyos/touchpoints-feed` + `dailyos/open-loops-feed` (shared inner blocks across §5.1–5.4) | `6444568c` | Reuse |
| `services::entity_intelligence::auth` (envelope-set validation) | Metadata proposal accept/dismiss/edit (§5.6) | `e9b0ed41` | Reuse |
| `services::claim_receipt::*` (contracts + render + auth) | Per-claim receipt rendering across all inner blocks | `0243df65` + `0a586218` | Reuse |
| `build_receipt_for_audience` (audience-keyed allowlist) | Every claim-bearing inner block — AgentMcp filter | `e4d72de6` | Reuse |
| `services::claim_receipt::boundary` + audit-disclosure-allowlist CI lint | Receipt rendering boundary across W2 surfaces | `e74d49dc` | Reuse |
| `services::claim_receipt::feedback` (DOS-8 typed 9-variant enum) | Metadata proposal accept/dismiss/edit + cite-chip drawer corrections | `7bbbc6f7` | Reuse |
| `services::meeting_prep_status::{read,write}` | `dailyos/prep-status` inner block (§5.4) | DOS-335 PR (verify L1) | Reuse |
| Meeting `EntityKind` extension for `get_entity_intelligence` | All 10 `dailyos/meeting-detail` inner blocks (§5.4) | `87df7cf6` (merged at `c5c0578f`) | Reuse (extended W1 substrate per V1.1 codex-challenge F2 fix) |
| `FeedbackAction::MergeIntent` enum extension | Person Detail merge affordance (§5.3) + People-list merge intent (§5.5) | `01d0cff3 (merged at 0d82502f)` (parallel W1 amendment landing on `wave/v1.4.4-w1-stage1a`) | Reuse (extended W1 substrate per V1.2 codex-challenge cycle-2 NEW finding — removes V1.1 v1.4.5 deferral) |
| DOS-461 fixture harness + no-bypass checks | All §5.1–5.4 ACs gate on green harness | W1 cycle-2 verdict cluster | Reuse |
| L3 cycle-2 WP block consumer skeletons | All 4 outer blocks specialize these | `00f38b3b` + `3e58bc13` | Specialize |
| `check_w1_consumer_skeleton.sh` CI gate | W2 PR must stay green | `c2e857b8` | Inherit as gate |
| L2 cycle-3 section-list normalization patches | Section-list rendering in `Composition.sections[]` | `1a56d612` | Reuse |
| Chrome lane FolioBar / FloatingNavIsland / AtmosphereLayer / MagazinePageLayout | Outer-wrapper chrome on all 4 entity-detail composites | shipped pre-W1 (chrome lane L0 V1.3.1) | Reuse |
| Pill primitive duality (ADR-0132) | NavIsland active-state pill stays chrome; `dailyos-pill*` block remains body-content option | shipped pre-W1 | Reuse |
| v1.4.3 W2 primitives (TrustBandBadge, EntityChip, FreshnessIndicator, ProvenanceTag, IntelligenceQualityBadge, HealthBadge, ScoreBand) | All inner blocks consume per the 24/15/12/10 templates (V1.2 fold) | v1.4.3 W2 (PR #315 lineage) | Reuse + patch (DOS-691/692/693) |

**Verdict:** every W2 producer relationship is to a W1 item with a citable SHA on `wave/v1.4.4-w1-stage1a` OR to a pre-W1 v1.4.3 primitive. No W2 work reinvents W1 substrate. Per wave AC #W2 "No-shells rule" — every claim-backed inner block consumes envelope shape, not legacy `detail.intelligence` / `get_entity_context_entries`.

## 7. Acceptance criteria — wave-rolled-up

W2-level ACs span the full sub-wave program. Per-sub-ticket ACs in §5.

**AC #W2.1 — Tauri ↔ WP parity for 4 entity-detail surfaces.** Account / Project / Person / Meeting Detail each ship a WP composite block rendering real envelope substrate; legacy AI JSON / `get_entity_context_entries` fallback is NOT a path (per DOS-461 no-bypass harness).

**AC #W2.2 — 3 list shells consume cursor pagination.** Accounts / People / Projects list blocks invoke `executeAbility()` (WP 7.0) with the `useAbilityCursor()` shared hook; NO `useEntityRecords`.

**AC #W2.3 — All blocks register `apiVersion: 3`** (per wave §10 invariant).

**AC #W2.4 — `check_w1_consumer_skeleton.sh` CI gate green.** Every W1 producer named in §6 has a 3-arg `invoke_ability($producer, $payload, $scope_set)` reference in `wp/dailyos/blocks/*/render-functions.php`.

**AC #W2.5 — Receipt rendering audience-keyed.** Every claim-bearing inner block routes through `build_receipt_for_audience` (W1 DOS-341); AgentMcp audience field-allowlist row passes negative fixture (per W1 AC-341.12). **AgentMcp touchpoint render = Option B aggregate (V1.1 lock).** `dailyos/touchpoints-feed` under AgentMcp emits `{ count, recency: Recent|Aging|Stale, content: redacted }` only — NO per-item rows, NO touchpoint titles, NO `source_asof` timestamps, NO per-item trust bands. Recency tier resolves coarsely from the touchpoint set (no per-row timing oracle). Negative fixture asserts the aggregate-only shape (per W1 AC-341.12 + V1.1 §10 invariant "AgentMcp touchpoint aggregate render").

**AC #W2.6 — Outer/inner block contract per wave §10 (Path B locked V1.1).** Each outer block is a renderer-side projection of `AbilityResult<EntityIntelligenceEnvelope>` where `envelope.sections: BTreeMap<EnvelopeSection, SectionState>` enumerates the 7 W1 section variants (Facts / Health / MetadataProposals / OpenLoops / Touchpoints / Threads / Record). Inner blocks correspond to *projection rules* over one or more `EnvelopeSection` slices per ADR-0130 §4 Reading A — NOT to a separate `Composition.sections[].blocks[]` substrate shape. Primitives stay inserter-global (no `parent` field); block context via `providesContext`/`usesContext`, not attribute passthrough; `templateLock: false` + default `template` array ship in every outer block.json.

**AC #W2.7 — Per-project tint via CSS custom property (V1.1 lock; V1.2 regex tightened).** `dailyos/project-detail` outer wrapper carries `--dailyos-project-tint: var(--color-garden-olive);` via `get_block_wrapper_attributes(['style' => ...])`. **Prerequisite for W2 L1 start:** ADR-0077 amendment ticket lands first, locking project tint = `olive`. Until the amendment lands, `chrome_config()` emits olive default (`$stub_tints['dailyos_project'] = 'olive'`). **New CI script `src-tauri/scripts/check_no_inline_style_exception.sh`** (file at L1 kickoff): asserts that any `style=` attribute body in `wp/dailyos/blocks/**/*.php` matches `^--dailyos-[a-z-]+:\s*var\(--[a-z-]+\);?$` (custom-property assignment only — `--dailyos-*` namespace ONLY; arbitrary `--*` prefixes are rejected, per V1.2 codex-challenge cycle-2 F3 fix). This is the narrow exception to `feedback_no_inline_css` per memory; CI gate enforces the boundary. AtmosphereLayer renders olive tint on `singular(dailyos_project)` and `is_post_type_archive(dailyos_project)` if CPT registered (default-no per §5.2).

**AC #W2.8 — Visible-QA-state matrix per surface.** Each entity-detail composite reaches the per-DOS-462 / per-DOS-483 / per-DOS-484 / per-Meeting matrix (§5 ACs); list shells reach the AC-L.3 state set; metadata proposals reach AC-328.1 state set. Screenshot-evidenced.

**AC #W2.9 — No PII in any fixture data.** Per CLAUDE.md critical rule. Generic examples only (`subsidiary.com`, `parent.com`, `user@example.com`).

**AC #W2.10 — Filesystem patterns ship per composite.** `wp/dailyos/patterns/{account,project,person,meeting}-detail-default.php` + `{accounts,people,projects}-list-default.php` (per wave AC #W4 many-blocks-not-few rule). Theme-registered via `register_block_pattern` per wp-skill H4 V1.1 (insert-then-detach semantics; user reordering does not affect other instances). **NOT** synced patterns (DB-stored reusable blocks).

**AC #W2.11 — Inline-edit-affordance contract (per wave §10 + AC #W3).** Every claim-bearing block supports inline correction → `FeedbackAction` → `record_claim_feedback`; no direct DB writes from WP-side JS or PHP.

**AC #W2.12 — DOS-690 / DOS-691 / DOS-692 / DOS-693 surface-residue tickets close.** Each L4 hands-on validated; closure ledger updated on parent DOS-325 / DOS-9 / DOS-11.

**AC #W2.13 — Person merge picker path α holds.** WP `dailyos/person-detail` emits `MergeIntent` feedback; actual merge stays Tauri-side until W6 flag-flip. Path β escalates as W1 reopen if L4 reveals user friction (per wave AC #W5 C4 tie-break).

## 8. Out of scope

| Out of scope | Where it goes |
|---|---|
| W3 briefing surfaces (Daily Briefing, Meeting Briefing) | W3 sub-L0 |
| W4 action surfaces (Actions/Work, Activity Log, Action Detail, Lint Mode, review queue) | W4 sub-L0 |
| W5 system/history surfaces (History, Email, Settings) | W5 sub-L0 |
| W6 parity proof + Tauri shell deprecation | W6 sub-L0 |
| Report-shaped surfaces (BookOfBusiness, EbrQbr, SWOT, AccountHealth-as-report) | v1.4.8 |
| Salience / Recommendations layer | v1.4.6 |
| Workspace memory surfaces (sources view, source detail, entity-intake) | v1.4.5 |
| Causal lineage between claims | v1.5.x (per memory `project_causal_lineage_deferred`) |
| Person merge picker as substrate mutation (path β) | W1 reopen IF L4 friction shows; otherwise Tauri-only through W6 |
| New Tauri React UI work | Frozen (memory `feedback_tauri_ui_freeze`) |
| MCP scope manifest for AgentMcp envelope reads | v1.4.7 |
| Hosted DailyOS agent backend | Out of v1.4.x (per ADR-0129 §5) |

## 9. Migration slots

**None expected at W2.** W2 is renderer-only over substrate that landed in W1 (slots v240–v249 claimed there, per W1 §9).

**If discovered mid-W2:** per wave AC #W5 mid-wave substrate-gap escalation, any newly surfaced substrate gap reopens W1 (not deferred to v1.4.5+). Reviewer panel rejects sub-L0 amendments that defer substrate v1.4.4 surfaces consume.

**Coordination with v1.4.5+:** none required at W2; v1.4.5 Workspace Memory's source-claim additions will use minor semver-additive bumps to the W1-landed envelope (per wave §9 forward-coupling hot spot (a)) — that's v1.4.5's W0/W1 obligation, not W2's.

## 10. Architecture invariants

W2 inherits ALL wave-level invariants from `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` V1.1 §10. Notable for W2:

| Invariant (wave §10 source) | W2 application |
|---|---|
| **Outer/inner block contract** | Every entity-detail composite is 1 outer + N inner per §5; primitives stay inserter-global (no `parent`); context via `providesContext`/`usesContext`; render.php calls `do_blocks($content)` for inner-block reordering. |
| **Entity list pagination contract** | List shells consume `executeAbility()` (WP 7.0) via `useAbilityCursor()` shared hook at `wp/dailyos/blocks/_shared/hooks/`; NO `useEntityRecords`. Cursor opaque + server-encoded. Reset on watermark change. Outer envelope carries first-page slice per wave §13 Q1 sub-clause. |
| **Per-project tint via CSS custom property** | `dailyos/project-detail` wrapper carries `--dailyos-project-tint: var(--color-garden-olive)` (DOS-725 locked); narrow exception to `no inline CSS` rule per memory `feedback_no_inline_css`. |
| **Refresh model: pull-on-render + user refresh** | All 4 composites + 3 list shells + metadata-proposal inner blocks invoke producer on render; staleness via `FreshnessIndicator` + trust-band downgrade. NO push-invalidation bus at v1.4.4. **Exception:** Meeting Detail's prep-status surfaces a `MeetingPrepStatusChanged` signal via chrome.js — chrome-lane primitive, not a W2-novel push. |
| **Block apiVersion 3 mandatory** | Every new W2 block.json declares `apiVersion: 3`. CI gate optional but lint-checkable. |
| **Substrate-in-same-wave (C4)** | If L0 review surfaces a substrate gap blocking a named W2 AC, W1 reopens — NOT deferred to v1.4.5+. C4 supersedes path-α. |
| **AgentMcp audience filter** | Every claim-bearing inner block routes receipt through `build_receipt_for_audience(target, audience, conn)` (W1 DOS-341); AgentMcp audience row's allowlist (trust band, coarsened freshness, redaction level, lifecycle, sanitized evidence_summary, subject_type) enforced. Negative fixture per W1 AC-341.12. |
| **Inline-edit-affordance contract (anchored decision #2)** | Every correction emits `FeedbackAction` through `record_claim_feedback`; NO direct PHP/JS DB writes. |
| **Chrome runtime-injection scope** | Body content stays Gutenberg blocks. Chrome (FolioBar / FloatingNavIsland / AtmosphereLayer / MagazinePageLayout) is the only runtime-injection lane. W2 introduces NO new runtime-injection module. |
| **Design-system canonicity** | Every chapter inner block translates an existing canonical design (`src/pages/AccountDetailPage.tsx`, `ProjectDetailEditorial.tsx`, `PersonDetailEditorial.tsx`, `MeetingDetailPage.tsx`, `.docs/design/patterns/*`) — NO new visual patterns invented in `wp/dailyos/blocks/`. New patterns land canonical-first with `ce-design-lens-reviewer` approval. |
| **L2 bounded by acceptance criteria** | Path-α findings → maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`. C4-blocking findings reopen W1. |
| **Code-shape sketch obligation (NEW V1.1, per architecture A1 + wp-skill H1)** | Every block.json declaration named in §5 ships with a concrete code-shape sketch — block.json + render.php skeleton — alongside the prose. No spec-named-in-prose-only patterns. If a third spec-vs-sketch gap surfaces in cycle 2, switch to a class-wide sweep over §5. |
| **Empty-state pattern (NEW V1.1, per design-lens F1)** | Every inner block renders empty as a quiet chip with `data-empty-reason="<reason>"` attribute. NEVER silent-hidden (`return ''` from render.php is non-conforming). Reasons inherit envelope's `SectionState::Empty { reason: EmptyReason }` where applicable, surface-local otherwise. Anchors a stable QA target across 60+ inner blocks. |
| **AgentMcp touchpoint aggregate render (NEW V1.1, per design-lens F2 + codex-challenge F4)** | For AgentMcp audience, `dailyos/touchpoints-feed` (and any other touchpoint-bearing inner block) emits `{ count: u64, recency: Recent | Aging | Stale, content: redacted }` only. NO per-item rows, NO titles, NO `source_asof` timestamps, NO per-row trust bands. Recency tier resolves coarsely server-side from the touchpoint set; no timing oracle leaks. Negative fixture asserts the aggregate-only shape (per W1 AC-341.12). |

## 11. Reviewer matrix (L0 panel)

5-reviewer panel per CLAUDE.md + engineering-ladder.md + memory `feedback_wp_skill_grounded_reviewer_for_wp_l0`. Sub-wave packet inherits the 5-panel discipline; `/cso` is NOT mandatory by default (W2 read paths) but opts in per-sub-ticket where write paths emerge.

| Reviewer | Scope at W2 |
|---|---|
| `/codex challenge` | Adversarial — premise-check outer/inner block contract translation; stress-test "no fallback to legacy path" rule; challenge cursor pagination shape; surface scope-inflation risk from 60+ inner blocks across 4 composites. |
| `/codex consult` | Architecture continuity with ADR-0129 / ADR-0130; W1 → W2 substrate consumption completeness; envelope shape match between producer + consumer. |
| `ce-architecture-strategist` | Sub-wave packet boundary; outer/inner block contract per §10; coordination with W3 (briefing references entities via `subject_ref` only — verify the surface boundary holds). |
| `ce-design-lens-reviewer` | Translation faithfulness from Tauri React composites to Gutenberg block tree; visible-QA-state matrix per surface; DOS-725 tint resolution review; DOS-693 HealthBadge label-discipline decision. |
| **WP-skill-grounded reviewer** (per memory) | block.json + render.php discipline; `providesContext`/`usesContext` correctness; `do_blocks($content)` discipline; `useAbilityCursor` hook shape; loopback HTTP boundary; sanity-check `executeAbility()` (WP 7.0) usage; pattern vs template-part discipline. |

**`/cso` opts in per-sub-ticket:**
- **DOS-328 metadata proposals (§5.6):** `/cso` MANDATORY (write path through `record_claim_feedback`; envelope-set validation; class-sweep on accept/dismiss/edit channels).
- **DOS-689 EvidenceDrawer (§5.7):** `/cso` MANDATORY (10-channel field allowlist; display-safe provenance per ADR-0108).
- **DOS-484 Person Detail merge intent (§5.3):** `/cso` MANDATORY (feedback emission of `MergeIntent` carries person-pair binding; verify trust-boundary holds on the intent itself).

**`accessibility-tester` opts in for:**
- §5.1–5.4 entity-detail composites (user-facing surfaces).
- DOS-692 trust-band keyboard nav explicitly.
- Lists (§5.5).

**Pass rule:** unanimous APPROVE. 2 revision cycles without convergence ⇒ L6 escalation. Memory `feedback_review_loop_l6_policy` allows continued looping on architectural / critical / high; class-pattern recurrence triggers system-wide sweep per memory `feedback_systemic_look_for_recurring_issue_classes`.

## 12. References

**ADRs:**
- ADR-0077 — Magazine layout editorial redesign (amended by DOS-725 for project tint).
- ADR-0083 — Product vocabulary (HealthBadge label discipline source).
- ADR-0102 — Abilities as runtime contract.
- ADR-0105 — Provenance as first-class output.
- ADR-0108 — Provenance rendering + privacy (EvidenceDrawer field allowlist source).
- ADR-0111 — Surface-independent ability invocation.
- ADR-0123 — Typed claim feedback semantics (9-variant `FeedbackAction` source).
- ADR-0125 — Claim anatomy + temporal + sensitivity + TypeRegistry.
- ADR-0128 — Headless DailyOS / MCP as product surface.
- ADR-0129 — Composable surfaces: WordPress Studio as primary surface.
- ADR-0130 — Surface-independent composition contract.
- ADR-0131 — Structured embedding + claim canonicalization.
- ADR-0132 — Pill primitive dual existence.

**V1.1 wave packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-wave-plan.md` (§5.2 W2 scope + §10 invariants + §13 locked decisions).

**W1 packet:** `.docs/plans/v1.4.4-wp-surface-migration/L0-packet-W1-substrate-gaps.md` V1.1 (substrate landed; consumed verbatim).

**W0 audit:** `.docs/plans/v1.4.4-surface-audit.md` (active list § with W2 surfaces; W0 open-questions resolved 2026-05-20).

**W1 L3 verdict files:**
- `.docs/plans/v1.4.4-wp-surface-migration/reviews/packet-W1-substrate-gaps-{architecture,correctness,cso,codex-challenge,codex-consult}-cycle1.md`.

**K-out from W1 retro (cited in §3):**
- `docs/solutions/architecture-patterns/L3-catches-wave-level-bypass-of-sub-ticket-discipline-2026-05-20.md`.
- `docs/solutions/workflow-issues/codex-agent-dispatched-but-no-file-changes-2026-05-20.md`.
- `docs/solutions/workflow-issues/parallel-agent-commit-hook-contention-2026-05-20.md`.

**Linear project:** [v1.4.4 — WordPress Surface Migration](https://linear.app/a8c/project/v144-wordpress-surface-migration-877aaa780177) (id `f8b805d9-f3d4-41b4-a446-51bbb7e05f2e`).

**Maintenance project for path-α:** [`b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`](https://linear.app/a8c/project/codebase-maintenance-production-quality-b8e6aea4d47e).

## 13. Open architectural questions

Sub-wave L0 inherits wave-level locked decisions (§13 #1, #2, #5, #6). W2-specific questions:

**Q1 — Per-entity inner block default ordering: which chapters in the default template?**

Each composite has a canonical Tauri React chapter ordering (translated 1-to-1 in §5.1–5.4 from `AccountDetailPage.tsx` / `ProjectDetailEditorial.tsx` / `PersonDetailEditorial.tsx`). Open: should the default filesystem pattern enforce this canonical ordering as the lock-default, or ship a "spread out" arrangement? **Recommendation: canonical Tauri ordering as default**; user reorders via Site Editor with `templateLock: false`. Decided here unless `ce-design-lens-reviewer` objects.

**Resolves at:** L0 close (assume canonical ordering default unless reviewer panel objects).

**Q2 — List shell pagination UI: infinite scroll vs cursor-button?**

`useAbilityCursor()` exposes both `loadMore()` (button) and natural infinite-scroll integration. For Accounts / People / Projects list, which mode? Considerations:
- Infinite scroll: better for browsing; complicates deep-linking to row N; competes with Site Editor scroll.
- Cursor button: better for predictable behavior; matches v1.4.2 W4-F account-overview pagination shape.

**Recommendation:** cursor button as default for predictability + Site Editor compatibility; infinite scroll lands as an attribute toggle if user demand surfaces post-W4 dogfooding.

**Resolves at:** L0 close.

**Q3 — How does AgentMcp envelope render the redacted touchpoint title? — RESOLVED V1.1**

**Locked decision (James 2026-05-21): Option B — coarse aggregate signal only.** For AgentMcp audience, `dailyos/touchpoints-feed` emits `{ count: u64, recency: Recent | Aging | Stale, content: redacted }` and NO per-item rows. No titles, no `source_asof` timestamps, no per-row trust bands. Recency tier resolves coarsely server-side from the touchpoint set (e.g., `Recent` = most-recent within N days; `Aging` = within M days; `Stale` = older).

Rationale (overrides V1.0 Option A): even a placeholder per-row entry with timestamp + trust band leaks an existence + timing oracle to the agent. The aggregate signal preserves coverage (agent knows there's activity) without per-item leakage. AC-W2.5 + §10 invariant "AgentMcp touchpoint aggregate render" enforce; negative fixture per W1 AC-341.12 asserts the shape.

**Q4 — Meeting Detail substrate gap: is W1 fully sufficient? — RESOLVED V1.1**

**W1 substrate extended to support Meeting EntityKind via commit `87df7cf6`** (`feat(abilities): extend get_entity_intelligence with Meeting EntityKind (W2 L0 cycle-1 F2 fix)`), merged into wave at `c5c0578f`. `EntityKind` now `{ Account, Project, Person, Meeting }`. Meeting subject composes Facts / Health / Touchpoints / OpenLoops / Threads / Record / MetadataProposals as listed in §5.4 V1.1. AgentMcp audience scrub applies per W1 cycle-2 F2 pattern.

No further substrate work required for W2 L1 start. C4 not invoked.

**Q5 — Meeting Detail Linear ticket: file new, or fold into existing meeting work? — STILL OPEN (filing-only)**

No Linear ticket at packet-author time. **Locked recommendation (unchanged V1.1):** file new ticket "v1.4.4 W2 — Meeting Detail block" at L1 kickoff; priority High; project v1.4.4. AC list seeded from §5.4. The substrate gap that previously made this risky has been closed at `87df7cf6`; filing is now a routine L1-kickoff task.

**Resolves at:** L1 kickoff (filing only; no scope decision needed).

**Q6 — Merge picker scope for Person Detail (path α vs path β):**

Per §5.3, locked at path α (`MergeIntent` feedback; Tauri-side actual merge). If L4 reveals user friction, escalate to path β (substrate addition). Wave AC #W5 C4 tie-break applies — path β goes back to W1, not deferred to v1.4.5+.

**Resolves at:** L4 hands-on for §5.3; path α holds unless L4 evidence triggers path β.

**Q7 — ADR-0077 amendment + path-α maintenance items (NEW V1.1):**

- **ADR-0077 amendment** (project tint = `olive`) is a **W2 L1 prerequisite** per AC-W2.7 V1.1 lock. Files alongside the L1 kickoff PR; until amendment lands, `chrome_config()` emits olive default.
- **Path-α items routed to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` (per memory `feedback_l2_path_alpha_to_maintenance_project`):**
  - M1 — WP 7.0 abilities API fallback (REST endpoint shim for `executeAbility()` if WP-core slip materializes).
  - M2 — Inserter-global primitives null-context render guard (graceful render path when inner block is inserted outside an entity-detail outer).
  - M3 — theme.json `styles.blocks["dailyos/*"]` declaration deferred to v1.4.5+ (W2 blocks declare only `supports.html` / `reusable` / `inserter`; no `supports.color/spacing/typography`).

**Resolves at:** L1 kickoff (ADR-0077 amendment lands first); maintenance tickets filed at L0 close, do not block.
