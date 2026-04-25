# Storage Shape Review — 2026-04-24

**Purpose:** audit the existing storage landscape DailyOS reads and writes from today, against the v1.4.x substrate transformation. Identify what the substrate replaces, what it composes with, what gets dropped or denormalized, and what knock-on effects propagate. Surface the storage shape before code starts, the same way the Claim Anatomy Review surfaced the claim shape.

**Scope:** SQLite schema (122 migrations, 131 CREATE TABLE statements as of 2026-04-24), the off-database `intelligence.json` file substrate, and the Rust types that mediate them.

**Method:** grep + read. Not a runtime data audit (no actual row-counting in the dev DB) — that would be a complementary v1.4.0 spine task ("Phase 2 reality check") if the user wants it.

---

## Headline findings

These are the things that change the substrate calculus and warrant action before code starts:

### Finding 1 — DOS-7's "consolidate 3 tombstone mechanisms" is actually "consolidate 9+"

DOS-7 + ADR-0113 R1.3 documented `suppression_tombstones`, `account_stakeholder_roles.dismissed_at`, and `DismissedItem` in `intelligence.json` blobs. **Reality check: there are at least nine distinct dismissal/suppression mechanisms in production today.**

Full inventory (each with its own schema, its own write paths, its own read paths):

| Mechanism | Migration | Shape | Domain |
|---|---|---|---|
| `suppression_tombstones` | 084 | dedicated table; `entity_id`, `field_key`, `dismissed_at`, `expires_at`, `superseded_by_evidence_after` | generic field-level |
| `account_stakeholder_roles.dismissed_at` | 107 | column on stakeholder roles | stakeholder roles |
| `email_dismissals` | 030 | dedicated table; `item_type`, `dismissed_at` | emails |
| `meeting_entity_dismissals` | 099, migrated to substrate by 115 | per-meeting entity link dismissals | entity links |
| `linking_dismissals` | 111 | entity-linker dismissals | entity links (separate path) |
| `briefing_callouts.dismissed_at` | 020 | column on callouts | briefing surface |
| `work_tab_actions.dismissed_at` | 108 | column on work-tab actions | actions surface |
| `triage_snoozes` | 109 | snooze (tombstone with expiry) | triage surface |
| `DismissedItem` in `intelligence.json` | in-blob, off-DB | `field`, `content`, `dismissed_at` | intelligence file substrate |

DOS-7's consolidation migration as scoped is 33% complete. The other six mechanisms either need to be folded into the consolidation, kept separate with documented rationale, or migrated independently. **This isn't a 1-day task; this is a 3-5 day migration with backfill considerations across nine surfaces.**

### Finding 2 — AI output is stored in three places, not one

The mental model of "v1.4.x replaces `entity_intelligence`" is incomplete. AI output today lives in:

1. **`entity_intelligence` SQLite table** (baseline migration 001) — canonical persistence, columns: `executive_assessment`, `risks_json`, `recent_wins_json`, `current_state_json`, `stakeholder_insights_json`, `next_meeting_readiness_json`, `company_context_json`. **This is the cross-account bleed surface.**
2. **`success_plans` SQLite table** (migration 068) — mirrors the same shape (`executive_assessment`, `risks_json`, `stakeholder_insights_json`, etc.). Separate read path. Separate consumers.
3. **`intelligence.json` file** in each entity directory — authoritative `IntelligenceJson` Rust type (`intelligence/io.rs`), atomic-written. Holds `ItemSource` attribution per item, `DismissedItem` tombstones, `UserEdit` records, `ConsistencyFinding` records — much richer than the SQLite columns.

Plus: **`accounts.company_overview`, `accounts.strategic_programs`, `accounts.notes`** are AI-output columns directly on the accounts table (per `db/accounts.rs:1205-1287`). Not in `entity_intelligence`. Same bleed-bug exposure.

The substrate transformation (DOS-218/DOS-219 pilots producing claim-shaped output) needs to decide for each surface: **replace? compose? denormalize from? leave alone for v1.4.1 cleanup?** The current spine scope answers this for none of them.

### Finding 3 — `ItemSource` already exists as a typed Rust struct with `sourced_at`

`intelligence/io.rs:29-41`:

```rust
pub struct ItemSource {
    pub source: String,         // "user_correction", "glean_crm", "transcript", "pty_synthesis", ...
    pub confidence: f64,        // 0.0–1.0
    pub sourced_at: String,     // ISO 8601
    pub reference: Option<String>,  // "Salesforce", "you edited this", ...
}
```

This is the pre-substrate version of what the substrate calls `SourceAttribution + source_asof`. It's already shipped, already populated by the LLM-prompt contract (`dimension_prompts.rs:608`), already consumed by `is_suppressed()` for tombstone-vs-evidence resurrection logic.

**Implication for DOS-299:** the `source_asof` work isn't "build new substrate"; it's "promote ItemSource.sourced_at into a first-class column on the new claims table, and normalize the ~10 places that stamp `Utc::now()` to consult upstream timestamps instead." The semantics already exist in working code; the substrate is consolidating them.

### Finding 4 — `is_suppressed()` already implements the substrate's intended tombstone resurrection logic

`intel_queue.rs:2014`:

> "Items with newer evidence (sourced_at > dismissed_at) pass through — the is_suppressed function handles that logic."

The substrate's planned behavior — "tombstone PRE-GATE check, but allow override on stronger fresh corroboration" (ADR-0113 §5) — is already implemented for risks/wins via `is_suppressed()`. The substrate version generalizes this to all claim_types.

**Implication for DOS-7:** the propose/commit + tombstone-pre-gate substrate should **lean on `is_suppressed()`'s contract**, not reinvent it. Keep the existing function as the primitive; the substrate adds claim_state state machine + ledger around it.

### Finding 5 — `account_stakeholder_roles` (migration 080) is the typed-row pattern in miniature

This table already does what the substrate generalizes:
- Typed row per assertion (one row per `(account_id, person_id, role)`)
- `data_source` column ('user', 'glean_crm', 'pty_synthesis', etc.)
- `dismissed_at` for tombstones
- `sourced_at` semantics (per the `intel_queue.rs` path)

**Implication for DOS-7:** the migration story isn't "build claims table from scratch"; it's "extend the `account_stakeholder_roles` pattern across more claim types, then merge it into the unified `intelligence_claims` substrate." The pattern is proven. Use it as the precedent rather than treating claims as alien.

### Finding 6 — Time-series tables already exist for user sentiment

`user_sentiment_history` (migration 094) is an append-only time-series of user health sentiment per account. `user_health_sentiment` is the corresponding column on `accounts` (migration 091).

**Implication for ADR-0124 longitudinal threading:** the append-only-time-series pattern is already shipping. Consider whether `user_sentiment_history` becomes a thread (Theme entity Option A) or a typed claim sequence (Option B) when v1.4.2 makes the threading decision. **It's a real, populated, in-production candidate dataset for the v1.4.0 pilot.**

---

## Storage inventory (categorized)

### Core entity & relationship tables (stable; substrate composes with)

`accounts`, `projects`, `people`, `entities`, `entity_people`, `meeting_entities`, `meeting_attendees`, `person_relationships`, `person_emails`, `attendee_display_names`. **No transformation in v1.4.x.** Subject attribution (ADR-0105 amendment) references entity IDs in these tables.

### AI-output surfaces (transformation targets)

`entity_intelligence`, `success_plans`, `accounts.company_overview/strategic_programs/notes`, `intelligence.json` file. **Substrate replaces over multiple cycles.** v1.4.0 spine writes claims; v1.4.1+ reads start consuming claims; v1.4.2+ removes legacy columns/files when readers complete migration.

### Action / work surfaces (DOS-276 transforms into typed CommitmentClaim)

`actions`, `work_tab_actions`, `captures`. **DOS-276 in v1.4.1 transforms `actions`. The other two stay until v1.4.2/3 surface work consumes typed commitments.**

### Signal / propagation infrastructure (DOS-235/236/237 in v1.4.1)

`signal_events`, `signal_weights`, `signal_derivations`, `email_signals`, `signal_propagation` artifacts (`post_meeting_emails`, `briefing_callouts`, `proactive_insights`, `proactive_scan_state`). **No transformation in v1.4.0 spine; all the policy registry / coalescing / invalidation work in v1.4.1.**

### Tombstones / dismissals (Finding 1 — 9 mechanisms)

See finding above. **DOS-7 consolidation needs scope expansion or explicit out-of-scope per-mechanism.**

### Feedback & corrections

`entity_resolution_feedback` (migration 019), `attendee_group_patterns` (019), `intelligence_feedback` (db module), `feedback_events` (likely from migration 084). **DOS-294 typed feedback enum integrates here; DOS-294 spec should reference these as the existing surfaces being typed.**

### Quality / consistency

`entity_quality` (migration 040), `intelligence_consistency_metadata` (migration 054), `consistency_findings` in `intelligence.json` (off-DB). **Substrate's TrustAssessment + warnings absorbs much of this; v1.4.1 work should explicitly retire the legacy quality columns.**

### Email / source ingestion

`emails`, `email_threads`, `email_signals`, `email_dismissals`, `email_pending_retry_state`, `email_thread_dismissals`, `gravatar_cache`. **No transformation in v1.4.x.**

### Sync state

`quill_sync_state`, `clay_sync_state`, `linear_sync` artifacts, `drive_watched_sources`, `granola` paths. **Substrate doesn't touch.**

### Calendar / meetings

`meetings_history`, `meeting_prep_state`, `meeting_entities`, `meeting_attendees`. **Subject attribution references meeting IDs; otherwise no transformation.**

### User profile / context

`user_entity` (044), `user_context_entries` (044), `user_sentiment_history` (094), `user_health_sentiment` column. **See Finding 6.**

### Linear integration

`linear_issues`, `linear_projects`, `linear_entity_links`. **No transformation in v1.4.x.**

### Operational / debug

`processing_log`, `enrichment_log` (016), `hygiene_actions_log` (029), `risk_briefing_jobs` (098), `entity_graph_version` (113, 117), `entity_graph_sweep_state` (121). **v1.4.0 ServiceContext / ExecutionMode infrastructure replaces some operational logging; substrate audit log per ADR-0103 §8 is the destination.**

### Search / embeddings

`content_index`, `content_embeddings` (006). **No transformation; substrate consumes for retrieval.**

### Reports & publication

`reports` (050), report-related artifacts. **No transformation in v1.4.0; DOS-298 reserves "Suggested Next Steps" sections; v1.4.3 briefing surfaces.**

---

## Knock-on effects of the v1.4.x transformation

### Read-path duplication risk

When DOS-218 `get_entity_context` writes claim-shaped output, the **legacy `entity_intelligence` SQLite columns and `intelligence.json` file** continue to be read by ~20+ existing code paths (export.rs, intel_queue.rs, prompts.rs, json_loader.rs, devtools, plus surface readers). If the substrate only writes claims and the legacy path keeps writing AI blobs, the user will see **two versions of the same claim** rendered from different sources.

**Mitigation:** v1.4.0 pilot abilities must dual-write — claims for the new substrate AND the legacy `entity_intelligence` columns + `intelligence.json` blob — until the readers are migrated. Spec this explicitly in DOS-218 / DOS-219 acceptance criteria.

### Cache invalidation complexity

The `entity_graph_version` / `entity_graph_sweep_state` triggers (migrations 113/117/121) are intelligent cache-busting infrastructure. Substrate writes to `intelligence_claims` need to participate in this cache invalidation — otherwise downstream caches miss claim updates.

**Mitigation:** DOS-7 schema migration includes adding `intelligence_claims` to the entity_graph_version trigger set. Currently not in DOS-7 scope.

### Migration order matters more than v1.4.0 plan acknowledges

The `account_stakeholder_roles → intelligence_claims` consolidation (DOS-7 §R1.3) writes to a table that's currently the source of truth for stakeholder reads. If the migration runs before reader migration, stakeholder pages break. If reader migration runs before write migration, stakeholder pages double-render.

**Mitigation:** explicit phase order in DOS-7: (1) write substrate + dual-write old + new; (2) reader migration; (3) old-write removal; (4) old-table drop. Each phase is a separate PR with a soak window.

### `success_plans` is unowned

The `success_plans` table (migration 068) mirrors the `entity_intelligence` shape but isn't named in any v1.4.x scope. Either it shares the substrate fate (also gets claim-ified) or it's explicitly out of scope and stays as-is. **Currently unaddressed.**

### `accounts.company_overview` and siblings are unowned

`db/accounts.rs:1205-1287` mutates `company_overview`, `strategic_programs`, `notes` directly on the accounts row from AI output. Same bleed-bug surface as `entity_intelligence`. Not in any v1.4.x issue scope. **Currently unaddressed.**

---

## Recommendations (per finding)

| # | Finding | Recommendation | Where |
|---|---|---|---|
| 1 | 9 tombstone mechanisms | DOS-7 scope expansion: explicit migration per mechanism. Estimate: +2-3 days. OR split into "consolidation v1" (suppression_tombstones + DismissedItem + account_stakeholder_roles.dismissed_at) and "consolidation v2" (the other 6, in v1.4.1). | Update DOS-7 spec or split |
| 2 | 3 AI-output surfaces (4 if you count accounts columns) | DOS-218 + DOS-219 acceptance criteria: dual-write to `entity_intelligence` SQLite + `intelligence.json` file during spine; reader migration is v1.4.1; legacy column drop is v1.4.2. Add `success_plans` to the migration plan. Add `accounts.company_overview/strategic_programs/notes` to v1.4.1 cleanup scope. | Update DOS-218 + DOS-219; new v1.4.1 issue for accounts AI-columns cleanup |
| 3 | `ItemSource` exists already | DOS-299 should explicitly call out that this is consolidation/promotion, not invention. Promote `ItemSource.sourced_at` into the first-class `source_asof` column; refactor the ~10 `Utc::now()` write sites; preserve `ItemSource` as the wire format for backward compatibility. | Update DOS-299 description |
| 4 | `is_suppressed()` already implements substrate logic | DOS-7 spec: explicitly reference `is_suppressed()` as the pre-existing primitive that the propose/commit + tombstone-pre-gate consumes. Don't reinvent. | Update DOS-7 description |
| 5 | `account_stakeholder_roles` is typed-row precedent | Add to ADR-0125 / DOS-300 references: cite this as the prior art. Use the same `data_source` column shape on `intelligence_claims` for consistency. | Update ADR-0125 references |
| 6 | `user_sentiment_history` time-series | Add to ADR-0124 / DOS-296 + DOS-297 references: real candidate dataset for v1.4.0 longitudinal pilot. Worth running through the substrate to validate the threading pattern against existing data. | Update ADR-0124 / DOS-297 |
| Knock-on A | Read-path duplication | DOS-218 + DOS-219 dual-write acceptance criteria. | Spine spec adjustment |
| Knock-on B | Cache invalidation | DOS-7: add `intelligence_claims` to entity_graph_version triggers. | Spine spec adjustment |
| Knock-on C | Migration phase order | DOS-7: explicit phase ordering with soak windows. | Spine spec adjustment |
| Knock-on D | `success_plans` unowned | New v1.4.1 issue: scope `success_plans` in the substrate transformation. | New v1.4.1 issue |
| Knock-on E | `accounts.company_overview` unowned | New v1.4.1 issue: scope these AI-output columns into substrate transformation. | New v1.4.1 issue |

---

## What's not addressed and why

- **Runtime data quality audit.** This review is grep-based. Actual `SELECT COUNT(*) WHERE source_asof IS NULL` on the dev DB would change findings 3 and 6 from inferences to measurements. Optional Phase 2 if the user wants.
- **Performance impact of dual-write windows.** Spine adds claim writes alongside legacy writes. SQLite write throughput is fine for 6-user scale but should be benchmarked before v1.4.2 readers come online and remove legacy writes.
- **Storage growth.** `intelligence_claims` + `claim_corroborations` + `claim_contradictions` + `agent_trust_ledger` + `claim_feedback` + `claim_repair_job` + invalidation_jobs is a lot of new tables. Bounded but worth measuring after the spine pilots run.
- **Off-database file substrate beyond `intelligence.json`.** Entity directories also hold `dashboard.json`, `dashboard.md`, prep snapshots, transcripts. These aren't substrate-relevant but are part of the "what does an entity actually own on disk" picture.

---

## Validation

Like the Claim Anatomy Review, this is a pre-kickoff design pass. The Tuesday gate validates user-visible failure modes; this review validates **substrate completeness against the existing storage landscape we're transforming**.

If a storage-shape surprise surfaces post-kickoff, the bar is high: stop, reassess, decide explicitly whether to amend substrate or accept the surprise as a known limitation. The substrate is being designed against an assumed-clean baseline; the storage shape review reduces that assumption to ground truth.

---

## Summary of pre-kickoff substrate adjustments

If accepted, these are the non-negotiable updates to spine before code starts on Tuesday:

1. **DOS-7 spec update:** scope tombstone consolidation across all 9 mechanisms (or explicitly split into v1.4.0 / v1.4.1 phases per-mechanism). Reference `is_suppressed()` as the pre-existing primitive. Add explicit phase ordering with soak windows. Add `intelligence_claims` to entity_graph_version triggers.
2. **DOS-218 + DOS-219 spec update:** dual-write acceptance criteria (claims + `entity_intelligence` SQLite + `intelligence.json`) during spine. Reader migration = v1.4.1. Legacy drop = v1.4.2.
3. **DOS-299 spec update:** frame as "promote `ItemSource.sourced_at` into `source_asof`," not "build new field." Reference the ~10 existing `Utc::now()` write sites that need normalization.
4. **ADR-0125 + DOS-300 references update:** cite `account_stakeholder_roles` as the typed-row pattern precedent.
5. **ADR-0124 + DOS-296/297 references update:** cite `user_sentiment_history` as a v1.4.0 pilot candidate dataset for longitudinal threading validation.

If accepted, these are new v1.4.1 issues (no spine impact):

6. **`success_plans` substrate transformation** — new v1.4.1 issue.
7. **`accounts.company_overview/strategic_programs/notes` substrate transformation** — new v1.4.1 issue (this is also where the cross-account bleed bug surfaces; closes the loop on DOS-287).

Spine count if all spec updates land: still 17 (no new spine issues). v1.4.1 count: +2 issues.
