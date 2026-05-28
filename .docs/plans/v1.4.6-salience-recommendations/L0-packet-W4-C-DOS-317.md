---
ticket: DOS-317
title: "v1.4.6 W4-C — Engagement telemetry + UserFit salience integration"
parent_plan: ../v1.4.6-waves.md
fork_sha: df7706a63647116ff355d162773e84c51723d877
target_branch: codex/v1.4.6-w4c-engagement-telemetry
status: L0 cycle 3 authoritative — un-reset (cycle 2 reset reversed per user direction 2026-05-28); UserFit integration restored as substrate evolution; awaiting cycle 4 review
authors: James Giroux, Claude
related:
  - ADR-0094 (Audit Log and Enterprise Observability — wrong-domain substrate; cited for non-overlap audit)
  - ADR-0098 (Data Governance: Source-Aware Lifecycle and Purge-on-Revocation — opt-out pattern reference)
  - ADR-0102 (Abilities as Runtime Contract)
  - ADR-0104 (Execution Mode and Mode-Aware Services — ServiceContext substrate; mandatory)
  - ADR-0108 (Provenance Rendering and Privacy)
  - ADR-0114 (Scoring Unification — pure factor extractors)
  - ADR-0115 (Signal Granularity Audit — session-level signal granularity)
  - ADR-0123 (Typed Claim Feedback Semantics — separate stream per ADR-0126 inv 9)
  - ADR-0125 (Claim Anatomy / Sensitivity / TypeRegistry)
  - ADR-0126 (Memory Substrate Invariants — invariants 1, 4, 9, 10; inv 9 names canonical table claim_engagement_events)
  - DOS-329 / DOS-330 (W1-A / W1-B — RecommendationClaim + 10-factor salience scoring)
  - DOS-332 (W4-A — feedback substrate; merged PR #410)
  - DOS-316 (W4-B — deviation detection; parallel sibling)
  - DOS-338 (W5-A — eval harness; downstream consumer)
  - DOS-813 (CANCELED 2026-05-28 — Path B FactorRationale split folded back into this ticket)
  - DOS-815 (CANCELED 2026-05-28 — salience integration follow-on folded back into this ticket)
---

# v1.4.6 W4-C — Engagement telemetry + UserFit salience integration (DOS-317)

## §0. Cycle 3 reframing (2026-05-28)

Per user direction 2026-05-28: salience is part of the abilities-runtime substrate. Wiring engagement rollup into UserFit IS the substrate evolution that ships the value; deferring it to a follow-on ticket (the cycle 2 reset path) just punts the work without shipping benefit. The same reframing canceled the W4-B follow-on (DOS-814) and the Path B FactorRationale split (DOS-813).

Cycle 3 un-resets W4-C: UserFit integration is restored as routine substrate-evolution work. DOS-815 + DOS-813 canceled. The cycle 1 codex BLOCKERs (per-user salience plumbing missing, persisted salience opt-out purge missing) are addressed inline as substrate-extension work, not "follow-on prerequisites." Cycle 3 codex findings on ServiceContext patterns, atomic writes, bidirectional CI gate, retention auto-purge, and timestamp validation are also folded inline.

Cycle history below §16 records cycles 0–2 for traceability; their text is superseded by this cycle 3 body.

## §1. Scope statement and UI Surface Deferral compliance

Per UI Surface Deferral Amendment (`.docs/plans/v1.4.6-waves.md:13-31`): no intersection-observer wiring on briefing pages, no soft-dismiss UI on claim chips, no Weekly Activity Log integration. The DOS-317 Linear ticket's frontend pieces are pre-amendment scope.

W4-C **does ship**:

1. **Engagement event capture substrate** — `claim_engagement_events` table + `claim_engagement_opt_out` preference table. Per ADR-0126 invariant 9 canonical name (`.docs/decisions/0126-memory-substrate-invariants.md:85-92`).
2. **Engagement events feed UserFit salience factor** — the substrate evolution. `user_fit_value()` reads engagement rollup; `FactorRationale::UserFit` extended to carry attributable engagement_score.
3. **Per-user salience plumbing** — `LiveSalienceReader` extracts real user_id from `ActorKind` (no longer collapsed to `"user:score_salience"`).
4. **Persisted salience opt-out purge** — opt-out triggers `SCORE_SALIENCE_SCHEMA_VERSION` bump → forces recompute on next read with empty engagement rollup. Single-integer mechanism, no per-user column on `salience_factors`.
5. **Retention auto-purge** — 90-day row deletion via startup hook (the only invocation path with existing substrate).
6. **Bidirectional CI invariant gate** — `services::trust*` cannot import engagement; `engagement.rs` cannot write `claim_feedback`.
7. **Signal pre-declares** for `EngagementOptOutCompleted` with explicit PayloadPrivacy class.
8. **ServiceContext-bound public API** per ADR-0104 — no caller-supplied user_id, all mutations gated by `check_mutation_allowed()`.

## §2. Substrate-name resolution — `claim_engagement_events` canonical

ADR-0126 invariant 9 (verified `.docs/decisions/0126-memory-substrate-invariants.md:85-92`) names the canonical table `claim_engagement_events`. The wave plan body (`.docs/plans/v1.4.6-waves.md:1032`) uses `engagement_telemetry`. ADR-0126 invariant 10 (`:94-107`) fixes substrate vocabulary as canonical — drift = drift in implementation.

W4-C uses `claim_engagement_events`. Wave-plan housekeeping ticket flagged at §15 (non-blocking).

## §3. Substrate-consultation reuse audit (MANDATORY, /cso gate)

| Primitive | Location (verified) | Coverage of W4-C? |
|-----------|---------------------|-------------------|
| `audit_log.rs` append-only JSON-lines | `src-tauri/src/audit_log.rs` + ADR-0094 | Wrong domain — security/compliance events with hash-chain retention. Engagement is product telemetry with 90-day rolling retention + per-user opt-out. Different threat model, different lifecycle. |
| `signal_events` table | `src-tauri/src/signals/bus.rs` + `migrations/018_signal_bus.sql` | Wrong domain — signal_events fire ON substrate writes; engagement events fire on user surface activity. Different write-path discipline. |
| `claim_feedback` table | per ADR-0123 + W4-A's `feedback.rs` | Explicitly separate per ADR-0126 inv 9. Conflating = engagement-attention inflating truth (documented anti-pattern). |
| `entity_engagement_curve` table | `migrations/152_dos_215_temporal_entity_type_keys.sql:7-18` | Different axis — entity-level weekly engagement counts, not per-claim-per-user events. Coexists; W4-B's deviation detection consumes this; W4-C is per-claim. |
| `intelligence_feedback` table | (verify existence at L1 if present) | Explicit feedback only (legacy); W4-C is passive engagement. |
| Existing opt-out / preference substrate | grep returns nothing in `services/preferences*`, `services/privacy*` | None exists. W4-C ships net-new `claim_engagement_opt_out`. |
| ADR-0098 source-revocation purge | `.docs/decisions/0098-data-governance-source-aware-lifecycle.md` | Adjacent — applies to source revocation, not user telemetry opt-out. W4-C's opt-out is a user-preference signal, not a source revocation. Different governance model per ADR-0126 inv 10 vocabulary. |

**Verdict.** `claim_engagement_events` + `claim_engagement_opt_out` are net-new substrate; explicitly authorized by ADR-0126 inv 9. No existing primitive overlaps. /cso gate addressed in §6 threat model.

## §4. Files owned (exclusive)

Substrate work:

1. **`src-tauri/src/services/recommendations/engagement.rs`** — currently 7-line docblock (`src-tauri/src/services/recommendations/engagement.rs:1-7`); W4-C fills with full implementation per §5.
2. **`src-tauri/src/migrations/276_claim_engagement_events.sql`** (new) — `claim_engagement_events` + `claim_engagement_opt_out` tables per §7. (v274 + v275 reserved for W4-B per its packet §6.)
3. **`src-tauri/src/migrations/277_salience_factors_engagement_userfit.sql`** (new) — schema-evolution migration adding the persisted-salience opt-out mechanism (see §8.3).
4. **`src-tauri/src/migrations.rs`** — register v276 + v277.

Salience integration (the substrate evolution):

5. **`src-tauri/src/services/recommendations/contracts.rs`** — extend `FactorRationale::UserFit` from single `feedback_history_score: f64` to `{ feedback_history_score: f64, engagement_score: f64 }` (Path B per §8.4 — cycle 3 reframe ships Path B as substrate evolution, not deferred).
6. **`src-tauri/abilities-runtime/src/abilities/recommendations/contracts.rs`** — runtime mirror.
7. **`src-tauri/src/services/recommendations/salience.rs`** — extend `user_fit_value()` (`salience.rs:660`) to read `rollup_for_user_claim`; thread `user_id` parameter through `extract_factors` + `score_salience` from `ActorKind`; extend `FactorRationale::UserFit` construction at instantiation site (`salience.rs:492-499`).
8. **`src-tauri/src/services/context.rs`** — fix `LiveSalienceReader` actor collapse (`context.rs:363-368` currently maps to literal `"user:score_salience"`); derive real user_id from `ActorKind::User { ... }` variant; pass through to `score_salience`.
9. **`src-tauri/src/services/recommendations/render.rs`** — extend `factor_rationale` rendering for the new UserFit shape (separate prose for feedback contribution vs engagement contribution; W3-A-style redactor names dominant).
10. **`src/services/recommendations/contracts.ts`** — TS mirror of `FactorRationale::UserFit` extension.
11. **`src/services/recommendations/__tests__/contracts.golden.test.ts`** — update golden test for new UserFit rationale shape.

Signal infrastructure (5-site policy_registry touch):

12. **`src-tauri/src/signals/policy_registry.rs`** — add `EngagementOptOutCompleted { user_id }` variant across 5 sites (enum + from_name + canonical_name + known_signal_type_names + policy_for). Same discipline as W4-B §8.

Tests + CI gate:

13. **`src-tauri/tests/recommendations_w4c_engagement.rs`** (new) — integration tests per §12.
14. **`src-tauri/tests/recommendations_w4c_engagement_not_trust.rs`** (new) — bidirectional CI invariant gate per §11.
15. **`src-tauri/tests/recommendations_w4c_opt_out_purge.rs`** (new) — delete-on-opt-out + retention auto-purge integration tests.

**Don't touch.** Trust substrate (CI invariant enforces). claim_feedback (W4-A domain — `engagement.rs` CI-banned from importing it). audit_log (ADR-0094 wrong domain). intelligence_claims columns (ADR-0126 inv 1 immutability). W4-B deviation.rs. W2-A surfacing.rs write paths.

## §5. Public API — `services/recommendations/engagement.rs`

All mutations take `&ServiceContext` per ADR-0104. All reads take `&Connection`. User identity is actor-derived; never caller-supplied. Atomic writes via `BEGIN IMMEDIATE` per codex cycle 3 W4-C R1.

```rust
/// Record a passive engagement event for a single claim render.
/// Atomic: BEGIN IMMEDIATE → check opt_out → INSERT → COMMIT (or ROLLBACK if opted out).
/// User identity derived from ctx.actor via the surfacing.rs:1222 pattern (split ':').
pub fn record_engagement_event(
    ctx: &ServiceContext,
    claim_id: &ClaimId,
    event: EngagementSignal,           // imported from contracts.rs:371-389; carries surface + timestamps
) -> Result<(), EngagementError>;

/// Compute per-user rollup over the rolling 90-day window for a single claim.
/// Reads only; takes ServiceContext for actor-derived user_id (no caller-supplied user_id per codex cycle 3 R1).
/// Returns empty rollup if user opted out (defense-in-depth).
pub fn rollup_for_user_claim(
    ctx: &ServiceContext,
    claim_id: &ClaimId,
    window_days: u16,                  // default 90; capped at MAX_ROLLUP_WINDOW_DAYS
    now: DateTime<Utc>,
) -> Result<EngagementRollup, EngagementError>;

/// Set the opt-out preference for the actor-derived user.
/// Atomic: BEGIN IMMEDIATE → UPSERT preference → DELETE existing events → BUMP salience schema version → COMMIT.
/// SLA: ≤ 1 minute end-to-end (synchronous in practice).
pub fn set_opt_out_for_user(
    ctx: &ServiceContext,
    opted_out: bool,
    now: DateTime<Utc>,
) -> Result<OptOutOutcome, EngagementError>;

/// Read opt-out state for the actor-derived user.
pub fn is_opted_out(
    ctx: &ServiceContext,
) -> Result<bool, EngagementError>;

/// Purge engagement event rows older than retention window.
/// Called by startup hook (the only invocation path with existing substrate; see §9).
pub fn purge_expired_engagement_events(
    ctx: &ServiceContext,
    now: DateTime<Utc>,
) -> Result<u32, EngagementError>;

/// Internal helper: extract user_id from ctx.actor.
/// Pattern: ctx.actor.split(':').next() per surfacing.rs:1222 (verified).
/// Returns Err(ActorNotUser) for non-user actors.
fn actor_user_id(ctx: &ServiceContext) -> Result<&str, EngagementError>;
```

**Types:**

- `RenderSurface` — imported from `abilities-runtime/src/abilities/sensitivity.rs:8` (per cycle 2 verification).
- `EngagementSignal` — imported from `contracts.rs:371-389` (verified existing; W4-C does NOT redeclare per Class D).
- `EngagementRollup` (new): `{ rendered_count: u16, clicked_count: u16, dismissed_count: u16, ignored_count: u16, last_event_at: DateTime<Utc> }`. Counts capped at `u16::MAX` as **overflow guard, not inferential-disclosure bound** (codex cycle 1 F5 reframe).
- `OptOutOutcome` (new): `{ opted_out: bool, rows_purged: u32, salience_version_bumped: bool, purge_completed_at: DateTime<Utc> }`.
- `EngagementError` (new): `enum { Db(rusqlite::Error), ActorNotUser, InvalidTimestamps, MutationBlocked, PurgeFailed(String) }`.

**`actor_user_id()` extraction pattern** (codex cycle 3 W4-C R2 — fixed hallucinated `ctx.actor().user_id()`):

```rust
fn actor_user_id(ctx: &ServiceContext) -> Result<&str, EngagementError> {
    // ServiceContext.actor is &str per abilities-runtime/src/services/context.rs:849.
    // Surfacing.rs:1222 establishes the split-on-':' pattern for user vs system actors.
    // Examples: "user:james@example.com", "user:abc-123", "system:enrichment"
    let prefix = ctx.actor.split(':').next().unwrap_or("");
    let suffix = ctx.actor.split(':').nth(1).unwrap_or("");
    if prefix != "user" || suffix.is_empty() {
        return Err(EngagementError::ActorNotUser);
    }
    Ok(suffix)
}
```

**Timestamp validation** (codex cycle 3 W4-C R2):

```rust
const MAX_DWELL_DAYS: i64 = 7;  // concrete value: session open >1 week is application restart, not genuine dwell

fn validate_event_timestamps(event: &EngagementSignal) -> Result<(), EngagementError> {
    if let EngagementSignal::Ignored { render_at, ignored_at } = event {
        if ignored_at < render_at {
            return Err(EngagementError::InvalidTimestamps);  // negative dwell
        }
        let dwell = *ignored_at - *render_at;
        if dwell > chrono::Duration::days(MAX_DWELL_DAYS) {
            return Err(EngagementError::InvalidTimestamps);  // exceeds bound
        }
    }
    Ok(())
}
```

## §6. /cso gate threat model (mandatory at L0)

### Data captured
`user_id` (actor-derived from `ctx.actor`), `claim_id`, `surface: RenderSurface`, `event: EngagementSignal`, `created_at` (from `ctx.services().clock.now()`, never caller-supplied).

### Data NOT captured
No claim text, no claim payload, no entity attributes (claim_id by reference only). No external network. No cross-claim sequence coupling.

### Data flow
1. **Capture**: actor-derived `record_engagement_event` writes one row atomically (BEGIN IMMEDIATE) if user not opted out.
2. **Aggregation**: `rollup_for_user_claim` returns bounded counts to salience scoring at score time. Bounded rollup is the only thing exiting W4-C's module.
3. **Salience-derived rendering**: salience score with engagement contribution flows into surfacing decisions → ranked claim lists → briefings/account-detail enrichment → user. Per ADR-0126 inv 9, engagement affects RANKING; trust scoring (DOS-5 `fb` factor) is unchanged.

### Opt-out semantics
- `claim_engagement_opt_out` preference table; default-absent = opted-in.
- `set_opt_out_for_user(ctx, true)` atomically: UPSERT preference + DELETE all existing rows for the user + BUMP `SCORE_SALIENCE_SCHEMA_VERSION` + COMMIT.
- The `SCORE_SALIENCE_SCHEMA_VERSION` bump invalidates all persisted salience (cycle 1 codex F2 BLOCKER fix). Next `score_salience` read forces recompute with empty engagement rollup → no engagement-derived ranking signal leaks past opt-out.
- Re-opt-in does NOT restore historical rows (ADR-0126 inv 3).

### Threat model
- **Local malicious process**: SQLite encrypted at rest (existing); engagement rows no more sensitive than the encrypted claim store.
- **Compromised salience scoring path leaking to trust**: §11 bidirectional CI gate (engagement-not-imported-by-trust AND engagement-cannot-write-feedback).
- **Cross-user reconstruction**: actor-derived `user_id` (not caller-supplied) closes the §C7 vulnerability of fabricated user_id; identity comes from ServiceContext binding.
- **Race conditions**: BEGIN IMMEDIATE serialization (codex cycle 3 R1 fix). Concurrent recorder + opt-out caller serialize cleanly via SQLite RESERVED lock.

### Retention
- 90-day rolling window on `claim_engagement_events`.
- `purge_expired_engagement_events` deletes rows older than 90 days from `created_at`.
- Invocation: startup hook (per §9 — only existing-substrate option).

## §7. Tables (v276 migration)

```sql
-- src-tauri/src/migrations/276_claim_engagement_events.sql
CREATE TABLE IF NOT EXISTS claim_engagement_events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         TEXT NOT NULL,
    claim_id        TEXT NOT NULL,
    surface         TEXT NOT NULL,
    event_kind      TEXT NOT NULL CHECK (event_kind IN ('rendered', 'clicked', 'dismissed', 'ignored')),
    render_at       TEXT,                       -- present for Ignored variant; NULL otherwise
    ignored_at      TEXT,                       -- present for Ignored variant; NULL otherwise
    event_at        TEXT NOT NULL,              -- ctx.services().clock.now() at record time
    created_at      TEXT NOT NULL               -- retention window anchor
);

CREATE INDEX IF NOT EXISTS idx_claim_engagement_events_user_claim
    ON claim_engagement_events (user_id, claim_id, created_at);
CREATE INDEX IF NOT EXISTS idx_claim_engagement_events_user_created
    ON claim_engagement_events (user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_claim_engagement_events_created
    ON claim_engagement_events (created_at);    -- retention purge scan

CREATE TABLE IF NOT EXISTS claim_engagement_opt_out (
    user_id    TEXT PRIMARY KEY,
    opted_out  INTEGER NOT NULL DEFAULT 0,
    set_at     TEXT NOT NULL
);
```

**Migration slot.** v276 verified next-free after W4-B's v274 + v275. Slot collision audit per `docs/solutions/conventions/migration-slot-reservation-verification.md` discipline (cycle 1 K-in finding).

**Schema design notes.**
- `event_kind` CHECK constraint enforces the 4 EngagementSignal variants.
- `render_at` + `ignored_at` only populated for the Ignored variant; NULL otherwise. Reconstructs the typed Ignored shape from `EngagementSignal::Ignored { render_at, ignored_at }`.
- Three indexes cover: per-user-per-claim rollup reads (most frequent), per-user purge on opt-out, and global retention purge.

## §8. Salience integration (substrate evolution)

The substrate-evolution work that delivers the user-visible benefit. Without this, engagement events are dead-end rows.

### §8.1. Per-user salience plumbing (codex cycle 1 F1 BLOCKER fix)

**Current state** (verified):
- `ScoreSalienceRequest` (`salience.rs:46`) carries only `claim_id` (cycle 1 codex finding).
- `ScoreSalienceReadRequest` (`abilities-runtime/src/abilities/recommendations/contracts.rs:38`) carries `actor: ActorKind`.
- `LiveSalienceReader` (`services/context.rs:350-388`) collapses actor to `"user:score_salience"` literal at `:363-368` — loses real user_id.

**W4-C extension:**

1. Extend `ScoreSalienceRequest` with `user_id: Option<String>` field. Optional because System actors don't carry user identity.
2. Extend `extract_factors()` and `score_salience()` (`salience.rs:211`, `:437`) to thread `user_id` to factor extractors.
3. Fix `LiveSalienceReader::score_salience` at `context.rs:363-368` to extract real user_id from `ActorKind::User { user_id }` variant (verify variant shape at L1; ActorKind is at `abilities-runtime/src/abilities/registry.rs`):
   ```rust
   let user_id = match &request.actor {
       ActorKind::User { user_id } => Some(user_id.clone()),
       ActorKind::System { .. } => None,
       // ... other variants
   };
   let app_actor = match &request.actor {
       ActorKind::User { user_id } => format!("user:{user_id}"),
       ActorKind::System { name } => format!("system:{name}"),
       _ => "system:score_salience".to_string(),
   };
   let result = app_salience::score_salience(
       /* ctx */,
       ScoreSalienceRequest { claim_id, user_id, .. },
   )?;
   ```

4. `user_fit_value()` (`salience.rs:660`) accepts `user_id: Option<&str>` + `now: DateTime<Utc>`; when present, builds a temporary `ServiceContext` (or reuses the caller's context — depends on call-graph; cycle 4 reviewer locks) to call `rollup_for_user_claim` and combine with existing claim_feedback signal.

### §8.2. `user_fit_value` extension

```rust
// salience.rs:660 (was: takes only conn + claim_id; returns single (Option<f64>, f64))
fn user_fit_value(
    conn: &Connection,
    claim_id: &ClaimId,
    user_id: Option<&str>,                 // new
    now: DateTime<Utc>,                    // new
    ctx: Option<&ServiceContext>,          // new — needed for engagement read; None for System scoring path
) -> Result<UserFitContribution, SalienceError> {
    let feedback_history_score = /* existing claim_feedback read */;

    let engagement_score = match (user_id, ctx) {
        (Some(uid), Some(ctx)) => {
            // Read engagement rollup; defense-in-depth opt-out check inside rollup_for_user_claim.
            let rollup = engagement::rollup_for_user_claim(ctx, claim_id, 90, now)?;
            engagement_signal_score(&rollup)  // local helper: positive events - negative events, normalized
        }
        _ => 0.0,  // no engagement contribution for System actors or missing context
    };

    Ok(UserFitContribution { feedback_history_score, engagement_score })
}
```

Return type `UserFitContribution { feedback_history_score: f64, engagement_score: f64 }` carries both contributions; salience aggregation combines via `value = (feedback_history_score + 0.3 * engagement_score).clamp(0.0, 1.0)` (engagement weighted at 30% relative to feedback; tuned by W5-A eval later).

### §8.3. Persisted salience opt-out purge (codex cycle 1 F2 BLOCKER fix)

**Approach: `SCORE_SALIENCE_SCHEMA_VERSION` bump on opt-out.**

`salience_factors` table persists `factor_value` + `rationale_json` (`migrations/270_salience_factors.sql:22`) with no `user_id` column. Adding a user_id column would require migrating all existing salience rows + extending every read path with user filter. Simpler alternative: bump the cache version, force recompute on next read.

The mechanism:
1. `set_opt_out_for_user(ctx, true)` atomically: UPSERT opt-out row + DELETE existing event rows + INCREMENT `SCORE_SALIENCE_SCHEMA_VERSION` (the persistent counter, not the source constant) + COMMIT.
2. Next call to `score_salience` for any claim sees the version bump; cached-return path at `salience.rs:221` mismatches; full recompute.
3. The recompute calls `user_fit_value` with `user_id = <the opted-out user>`. `rollup_for_user_claim` returns empty (rows purged). `engagement_score = 0.0`. No engagement-derived signal in the rationale.

**Cost.** All users' salience cache is invalidated, not just the opt-out user. In v1.4.6 (single-user local DB), this is moot — one user, all cache, all force-recomputed. If/when multi-user surfaces, this becomes per-user version stamps (filed as a v1.5.x concern post-multi-tenant work; not v1.4.6 scope).

**Migration v277:**

```sql
-- src-tauri/src/migrations/277_salience_factors_engagement_userfit.sql
-- The schema version mechanism is in salience_factors metadata; this migration
-- ensures the version column exists and is correctly indexed.

-- Verify schema_version column exists on salience_factors (per cycle 1 codex F7):
-- If not present, ALTER TABLE ADD COLUMN schema_version INTEGER NOT NULL DEFAULT 1;
-- (Check actual schema at L1; cycle 4 reviewer confirms migration body.)

CREATE INDEX IF NOT EXISTS idx_salience_factors_schema_version
    ON salience_factors (schema_version);
```

Test: opt-out → assert `SCORE_SALIENCE_SCHEMA_VERSION` constant matches a runtime-readable bump counter; `score_salience` recomputes; engagement contribution to UserFit = 0.

### §8.4. FactorRationale::UserFit split (DOS-813 folded in)

**Current shape** (verified `contracts.rs:272-274`):
```rust
UserFit { feedback_history_score: f64 }
```

**New shape:**
```rust
UserFit {
    feedback_history_score: f64,
    engagement_score: f64,
}
```

Path B per former DOS-813. Cycle 3 reframes: this IS substrate evolution. Split lets W3-A-style `why_this_now_surface_text` redactor (`.docs/plans/v1.4.6-w3a-suggested-next-steps/lane-a-l0.md`) name the dominant contribution honestly. Without the split, the rationale text would say "ranked highly because user previously approved similar claims" when engagement is the dominant signal — codex F9 caught this.

Mirror updates across runtime + TS contracts + golden test. Same discipline as W4-B §7.6.

### §8.5. Bidirectional CI invariant gate (codex cycle 3 W4-C — F5 fix)

```rust
// src-tauri/tests/recommendations_w4c_engagement_not_trust.rs
#[test]
fn engagement_module_separation() {
    let trust_scoped = collect_trust_files();
    for path in trust_scoped {
        let body = std::fs::read_to_string(&path)?;
        assert!(!body.contains("use crate::services::recommendations::engagement"),
            "{} imports engagement — violates ADR-0126 inv 9", path.display());
        assert!(!body.contains("use crate::services::recommendations::*"),
            "{} glob-imports recommendations", path.display());
        // Multi-import body check
        for capture in MULTI_IMPORT_RE.captures_iter(&body) {
            assert!(!capture.get(1).unwrap().as_str().contains("engagement"),
                "{} multi-imports engagement", path.display());
        }
        assert!(!body.contains("claim_engagement_events"),
            "{} references engagement table", path.display());
        assert!(!body.contains("claim_engagement_opt_out"),
            "{} references opt-out table", path.display());
    }

    // Reverse direction: engagement module must not write feedback
    let engagement_src = std::fs::read_to_string(
        "src-tauri/src/services/recommendations/engagement.rs"
    )?;
    assert!(!engagement_src.contains("INSERT INTO claim_feedback"));
    assert!(!engagement_src.contains("UPDATE claim_feedback"));
    assert!(!engagement_src.contains("use crate::services::claims::feedback"));
    assert!(!engagement_src.contains("use crate::services::claims::record_claim_feedback"));
    assert!(!engagement_src.contains("submit_recommendation_feedback"));
}

fn collect_trust_files() -> Vec<PathBuf> {
    // Locked enumeration; sha256 in CI artifact.
    vec![
        PathBuf::from("src-tauri/src/services/trust_recompute.rs"),
        PathBuf::from("src-tauri/src/services/trust_extraction.rs"),
        PathBuf::from("src-tauri/src/services/context.rs"),  // trust-adjacent
        // All abilities-runtime/src/abilities/trust/*.rs (24 files; explicit list at L1)
    ]
}
```

## §9. Retention auto-purge — startup hook invocation

Verified options (per cycle 3 ce-security-lens advisory):

- `signals/event_trigger.rs` daily — **NOT VIABLE.** Module doc says scheduling substrate was removed (`event_trigger.rs:8`).
- `MaintenanceTrigger` enum variant — **DOES NOT EXIST.** Grep finds no such type.
- **Startup hook** — viable. Pattern exists (e.g., `purge_expired_db` on db open). W4-C uses this.

```rust
// In the existing db-open / startup wiring (verify exact location at L1; likely src-tauri/src/db/mod.rs or main):
pub fn on_db_open(/* connection, ctx */) {
    // ... existing purges (audit_log retention, etc.) ...
    let _ = engagement::purge_expired_engagement_events(ctx, Utc::now())
        .map_err(|e| warn!("engagement retention purge failed: {e}"));
}
```

90-day boundary: rows where `created_at < now - 90.days` deleted. Best-effort on startup; if app runs >24h without restart, rows live past 90 days within a small window (acceptable for personal-use local DB; not a hard SLA).

## §10. Signal pre-declares — 5-site policy_registry touch

Same discipline as W4-B §8. Single new variant for W4-C:

```rust
// signals/policy_registry.rs SignalType enum (line ~60+):
EngagementOptOutCompleted { user_id: String },

// from_name (line ~145):
"engagement_opt_out_completed" => /* parse */,

// canonical_name (line ~274):
SignalType::EngagementOptOutCompleted { .. } => "engagement_opt_out_completed",

// known_signal_type_names() (line ~426):
"engagement_opt_out_completed",

// policy_for (line ~671):
SignalType::EngagementOptOutCompleted { .. } => SignalPolicy {
    granularity: SignalGranularity::Session,
    coalesce: false,                      // opt-out is single-shot per user
    payload_privacy: PayloadPrivacy::NonPiiMetadata,  // user_id as entity_id, no payload body
},
```

`user_id` placed in `entity_id` (with `entity_type = "user"`) at emission time. PayloadPrivacy::NonPiiMetadata appropriate for local-only single-user threat model per cycle 1 §C5.

## §11. ADR-0126 invariants binding W4-C

- **Inv 1** (immutable assertion core): W4-C does not write `intelligence_claims`. ✓
- **Inv 4** (retrieval additive, not distortive): engagement events affect ranking only. trust_score unchanged. ✓ (CI gate enforces.)
- **Inv 9** (engagement ≠ feedback): two separate streams, two separate tables, never fused. CI gate enforces bidirectional. ✓
- **Inv 10** (vocabulary canonical): W4-C uses `engagement` (canonical noun). `claim_engagement_events` (canonical table name from inv 9). ✓

## §12. Tests required (L1 deliverables, not L4 deferrals)

All ship in W4-C PR:

1. **Capture taxonomy** — each `EngagementSignal` variant round-trips through `record_engagement_event` → DB → `rollup_for_user_claim` correctly.
2. **Rollup correctness** — seed mixed events, assert counts match.
3. **Rollup u16::MAX cap** — seed 70000 events of one kind, assert returns `u16::MAX`, no overflow.
4. **Actor-derived user_id** — pass `ctx.actor = "user:abc"`, assert `user_id = "abc"` in stored row. Pass `ctx.actor = "system:enrichment"`, assert `EngagementError::ActorNotUser`.
5. **Timestamp validation** — `Ignored { render_at: T, ignored_at: T - 1.day }` → `InvalidTimestamps`. `Ignored { render_at: T, ignored_at: T + 100.days }` → `InvalidTimestamps`.
6. **Opt-out blocks writes** — opt-out, call `record_engagement_event`, assert no row inserted, returns Ok.
7. **Opt-out purges existing rows synchronously** — seed N events, call `set_opt_out_for_user(true)`, assert all rows for that user purged in same transaction; `rows_purged == N`.
8. **Opt-out bumps salience schema version** — opt-out, assert `SCORE_SALIENCE_SCHEMA_VERSION` counter incremented; subsequent `score_salience` recomputes.
9. **Persisted salience purge** — seed engagement events + force salience compute (stored with engagement contribution); opt-out; call `score_salience` for same claim; assert recomputed factor has `engagement_score = 0.0`.
10. **Opt-out emits signal** — assert `EngagementOptOutCompleted` row in `signal_events`.
11. **Re-opt-in does not restore** — opt-out, purge, opt back in; new events captured, historical not resurrected.
12. **BEGIN IMMEDIATE atomicity** — spawn 100 concurrent recorders + 1 opt-out caller; assert no row written after opt-out commits; no partial state.
13. **Bidirectional CI gate** (§8.5) — trust-side files don't import engagement; engagement.rs doesn't import/write feedback.
14. **Salience integration smoke** — seed engagement events, call `score_salience`, assert UserFit factor's `engagement_score` differs from baseline.
15. **FactorRationale::UserFit split** — assert serialized rationale has both `feedback_history_score` + `engagement_score` fields.
16. **Retention auto-purge** — seed rows at 95 days + 30 days ago; call `purge_expired_engagement_events`; assert old rows deleted, recent rows preserved.
17. **TS golden test** — `pnpm test` passes; UserFit rationale shape includes both fields.
18. **Type-check** — `pnpm tsc --noEmit` passes.

All pass under: `cargo clippy --lib -- -D warnings && cargo test && pnpm tsc --noEmit && pnpm test` (`cargo test`, not `--lib`, per codex cycle 1 F6 — integration tests in `tests/` only run with full `cargo test`).

## §13. Done-when

- [ ] `claim_engagement_events` + `claim_engagement_opt_out` migrated (v276).
- [ ] `salience_factors` schema-version mechanism verified/extended (v277).
- [ ] `engagement.rs` implements 6 public functions per §5 with ServiceContext, BEGIN IMMEDIATE atomic writes, actor-derived user_id, timestamp validation.
- [ ] `ScoreSalienceRequest` extended with `user_id: Option<String>`.
- [ ] `LiveSalienceReader` extracts real user_id from `ActorKind::User { user_id }`.
- [ ] `user_fit_value()` reads `rollup_for_user_claim` and returns `UserFitContribution { feedback_history_score, engagement_score }`.
- [ ] `FactorRationale::UserFit` extended across **all 3 mirrors** (app + runtime + TS contracts).
- [ ] `render.rs` updated for new UserFit rationale shape; W3-A redactor names dominant contribution.
- [ ] Opt-out triggers `SCORE_SALIENCE_SCHEMA_VERSION` bump; integration test asserts persisted-salience purge (cycle 1 codex F2 fix).
- [ ] Retention auto-purge wired to startup hook; integration test green.
- [ ] Signal `EngagementOptOutCompleted` pre-declared (5-site policy_registry touch).
- [ ] Bidirectional CI invariant gate green; enumeration locked.
- [ ] All §12 tests pass; delete-on-opt-out SLA ≤ 1 minute on 1000-event seed.
- [ ] `cargo clippy --lib -- -D warnings && cargo test && pnpm tsc --noEmit && pnpm test` green.
- [ ] `/cso` L0 approval recorded; `/cso` L2 approval recorded.
- [ ] L2 unanimous APPROVE; L2-status declared.

## §14. Wave plan amendment (W4-C authorized to touch salience scoring)

Same reframing as W4-B §13. Wave plan §1014 was authored under the framing that W4-C's salience integration was UI-consumer wiring. Cycle 3 reframe: salience integration IS substrate evolution. Wave plan amendment authorizes W4-C to extend `FactorRationale::UserFit`, modify `user_fit_value()`, and extend `ScoreSalienceRequest` + `LiveSalienceReader` for per-user plumbing.

Wave plan PR/Linear amendment posted alongside W4-C substrate.

## §15. Verified-against-codebase appendix

| Claim | File:line | Verified |
|-------|-----------|----------|
| `engagement.rs` placeholder | `src-tauri/src/services/recommendations/engagement.rs:1-7` | ✓ |
| `EngagementSignal` enum (Rendered/Clicked/Dismissed/Ignored) | `src-tauri/src/services/recommendations/contracts.rs:371-389` | ✓ |
| `EngagementSignal::Ignored { render_at, ignored_at }` shape | `contracts.rs:381-386` | ✓ |
| `FactorRationale::UserFit { feedback_history_score }` current shape | `contracts.rs:272-274` | ✓ |
| Runtime `FactorRationale` mirror | `abilities-runtime/src/abilities/recommendations/contracts.rs:254` | ✓ |
| TS `FactorRationale` mirror | `src/services/recommendations/contracts.ts` | ✓ |
| `user_fit_value()` signature | `src-tauri/src/services/recommendations/salience.rs:660` | ✓ |
| `UserFit` factor instantiation site | `salience.rs:492-499` | ✓ |
| `extract_factors()` sync entry | `salience.rs:437` | ✓ |
| `score_salience()` sync entry | `salience.rs:211` | ✓ |
| `ScoreSalienceRequest` shape (claim_id only) | `salience.rs:46` | ✓ |
| `ScoreSalienceReadRequest` actor field | `abilities-runtime/src/abilities/recommendations/contracts.rs:38` | ✓ |
| `LiveSalienceReader` actor collapse | `services/context.rs:363-368` | ✓ — needs fix per §8.1 |
| ServiceContext.actor is &str | `abilities-runtime/src/services/context.rs:849` | ✓ |
| actor-split pattern precedent | `services/recommendations/surfacing.rs:1222` | ✓ |
| ADR-0104 ServiceContext mutation gate | `.docs/decisions/0104-execution-mode-and-mode-aware-services.md:373,379` | ✓ |
| ADR-0126 invariant 9 canonical name | `.docs/decisions/0126-memory-substrate-invariants.md:85-92` | ✓ |
| ADR-0126 invariant 4 (retrieval additive) | `:44-50` | ✓ |
| ADR-0126 invariant 10 (vocabulary canonical) | `:94-107` | ✓ |
| salience_factors schema | `src-tauri/src/migrations/270_salience_factors.sql:22` | ✓ |
| signal_events table | `src-tauri/src/migrations/018_signal_bus.sql` | ✓ |
| policy_registry SignalType enum | `src-tauri/src/signals/policy_registry.rs:6+` | ✓ — 5-site touch required (§10) |
| Migration slot v276 next-free | `src-tauri/src/migrations.rs:1101-1102` + W4-B reserves v274+v275 | ✓ |
| audit_log substrate (wrong domain) | `src-tauri/src/audit_log.rs` + ADR-0094 | ✓ |
| signals/event_trigger.rs has no scheduling substrate | `src-tauri/src/signals/event_trigger.rs:8` | ✓ |
| No existing opt-out / preference substrate | grep `services/preferences*`, `services/privacy*` returns nothing | ✓ |
| Wave plan §1014 "Don't touch salience scoring" | `.docs/plans/v1.4.6-waves.md:1014` | ✓ — amendment proposed §14 |

## §16. L0 review routing (cycle 4)

Per wave plan §1054 + §450:

- **Mandatory**: `/codex challenge` adversarial review.
- **Planning reviewer (MANDATORY)**: `ce-security-lens-reviewer` (W4-C is privacy-sensitive). `/cso` skill review fires additionally at L0 per §450.
- **K-in**: `ce-learnings-researcher` re-run for class-pattern check + reset-honesty verification.
- **Wave-plan-amendment review**: §14 wave plan amendment posted to parent wave plan thread.

Cycle 4 goal: unanimous APPROVE on cycle 3 body.

---

## Cycle history (superseded by §0–§16 above)

- **Cycle 0 (2026-05-28)**: Initial packet authored. Substrate audit + 14 sections. ce-security-lens NEEDS_REVISION (6 findings including CI grep gate gap, signal payload class, W3-A path reference error, rollup opt-out guard, user_id validation). K-in BLOCKED on migration slot v273 collision (v273 taken by W2 shape repair).
- **Cycle 1 (2026-05-28)**: Amendments folded ce-security-lens + K-in findings (§C1-§C10). Codex BLOCKER findings (per-user salience plumbing missing, opt-out doesn't purge persisted salience, ServiceContext bypass, write-after-purge race, CI gate gap, `cargo test --lib` skipping integration, retention auto-purge missing, timestamp validation, Path A misrepresentation) were not yet folded.
- **Cycle 2 (2026-05-28)**: Scope reset stripped UserFit integration to DOS-815, Path B split to DOS-813. Substrate-only sections (§D0-§D6). Cycle 3 reviewer surfaced 2 required fixes (rollup_for_user_claim ServiceContext, real ctx.actor extraction + MAX_DWELL_DAYS concrete value) + advisory on retention invocation.
- **Cycle 3 (2026-05-28)**: Per user direction, scope reset reversed — salience integration is substrate evolution, not consumer wiring. DOS-813 + DOS-815 canceled, folded into this packet. Hallucinated APIs corrected with verified file:line citations (§15 appendix). Packet rewritten clean; cycle 0/1/2 amendments superseded by §0–§16.
