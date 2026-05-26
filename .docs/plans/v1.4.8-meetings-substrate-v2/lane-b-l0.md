# Lane B — L0 Packet: Meetings Writers Hardening (DOS-258 Shape)

**Date:** 2026-05-26 (revised after L0 cycle-2)
**Status:** Cycle-3 draft (cycle-2 verdicts: K-in APPROVE, Feasibility APPROVE on grep/DST/cites, Codex challenge BLOCK on visibility-doesn't-compile + atomicity + AC contradictions)
**Branch:** TBD (`fix/v1.4.8-lane-b-meetings-writers` proposed)
**Ticket:** [DOS-775](https://linear.app/a8c/issue/DOS-775)
**Wave:** v1.4.8 Meetings Substrate v2 ([DOS-773](https://linear.app/a8c/issue/DOS-773) parent)
**Depends on:** Lane A ([DOS-774](https://linear.app/a8c/issue/DOS-774)) — see §6 for concrete handoff predicate.

**Cycle-1 → cycle-2 changes:**
- §3.5 enumeration corrected — added 2 missed writer classes (`reclassify_meeting_types_from_attendees` UPDATEs, `data_lifecycle.rs` privacy purges + ID canonicalization). Pinned the gate as **INSERT-only** + explicit UPDATE-exemption list (cycle-1 reviewers caught the conflation).
- §3.4 `intelligence_state` initialization corrected — **Backfill is `detected`, not `archived`** (cycle-1 codex caught: `archived` in meeting code specifically means *cancelled* per `google.rs:911`; downstream consumers exclude it).
- §3.4 paired-transcripts justification rewritten — cycle-1 feasibility found all 5 existing paths already write paired rows; the "4 of 5 paths write it" claim was inverted.
- §3.6 backfill task #3 (orphan transcripts) downgraded to "defensive cleanup, fires only if pre-migration audit finds orphans" — no evidence orphans exist today.
- §3.6 TZ fallback **pinned** at L0 — `America/New_York` default + dry-run report (matching read-side `services/context.rs:803-807` pattern). Cycle-1 feasibility correctly flagged that deferring to L1 was an architectural abdication.
- §3.7 grep reframed as defense-in-depth (per K-in: class-pattern recurrence from `capability-boundary-needs-crate-split-not-grep-2026-05-18.md`); WARN-only after cycle-2 feasibility caught false-positive risk on inline `#[cfg(test)]` modules. Structural gate is the **sealed `MeetingWriteToken` capability** (cycle-3 — see §3.8 step 6). Cycle-2 cited `pub(in crate::services::meetings_writer)` visibility narrowing as the gate; cycle-2 codex caught that this **does not compile** because `db.upsert_meeting` lives in `src-tauri/src/db/meetings.rs` and `services/` is a sibling module to `db/`, not an ancestor — Rust `pub(in ...)` only restricts to ancestors.
- §3.3 dispatcher now wraps the 3-table write in `db.with_transaction(...)` — cycle-2 codex caught that `db.upsert_meeting` runs 3 standalone statements without atomicity (atomicity only exists when callers wrap, e.g., `services/meetings.rs:22-35 upsert_meeting_for_reconcile`). Cycle-3 fixes the dispatcher contract.
- §4 AC #4 + #6 rewritten for INSERT-only scope (per §3.5/§3.7 decision); AC #6 changed from "enforces" to "WARNs" to match §3.7 reframing.
- §3.1 trimmed — dropped `repository.rs` raw_* layer (cargo-culted from DOS-258; no forcing case here, backfill migration lives in `migrations/` conventionally).
- §2 K-in cites expanded — ADR-0101, ADR-0126, ADR-0061, migration filename-offset convention.
- §6 Lane A handoff predicate pinned — concrete trigger, not "stable in production."

---

## §0 — Origination, scope, threat topology

- **Origination class:** Extension. Lane B extends the meetings substrate; it is **not** debug-driven (no §0 trace required). The forcing observation is the same recurring class that produced DOS-771 — substrate inconsistency across writers — but Lane A closes the read-side gap; Lane B closes the write-side gap structurally so a *future* DOS-771-shaped bug can't reintroduce ghost rows.
- **Scope tier:** Standard (multi-file, single domain — meetings write path. Net new module + invariant enforcement + targeted backfill migration. Touches 5 production write paths. No cross-domain reshape; no ADR-named contract change). May tier up to Wave if invariant matrix surfaces ADR-level contract questions during L0 review.
- **Threat topology:** local-to-local single-user. Tauri runtime writing workspace-private meeting data; no remote surface, no multi-actor gates.

---

## §1 — Mission

Apply the DOS-258 deterministic-engine adapter pattern to meeting-row writers. Today 5 production code paths INSERT/UPSERT into the `meetings` table with different column sets, different paired-table behavior, and different `start_time` formats. There's no enforced contract that says "a meeting row is created via exactly one of these adapters with these required fields." Lane B introduces typed adapters per source, an invariant matrix every row must satisfy, a backfill sweep for existing violators, and a pre-commit grep guard against bypass.

---

## §2 — K-in sources (substrate-grep)

### Substrate-grep targets (cycle-1 K-in results folded in)

Cycle-1 K-in found 5 relevant prior learnings + 4 ADRs. Folded here.

**ADRs to cite explicitly:**

- **[ADR-0101](https://github.com/jamesgiroux/daily-operating-system/blob/dev/.docs/decisions/0101-service-boundary-enforcement.md) — Service boundary enforcement.** Lane B is the canonical realization for the meetings table. Phase 3 (service-boundary enforcement) is realized via the sealed `MeetingWriteToken` capability pattern in §3.8 step 6 (cycle-3 — see history for why `pub(in ...)` doesn't work here). Rule 2 (signal emission per write) — the dispatcher MUST emit a signal; §3.3 implements.
- **[ADR-0126](https://github.com/jamesgiroux/daily-operating-system/blob/dev/.docs/decisions/0126-memory-substrate-invariants.md) — Memory substrate invariants.** Closer match than ADR-0125 (which is claim-specific). Codifies the "feature ticket either honors invariants or amends the ADR" pattern; Lane B's §3.4 invariant matrix is the meetings-table parallel structure. §3 of ADR-0126 ("Dormancy is recoverable, deletion is not") informs the `intelligence_state` initialization decision.
- **[ADR-0061](https://github.com/jamesgiroux/daily-operating-system/blob/dev/.docs/decisions/0061-calendar-event-id-as-meeting-key.md) — Calendar event ID as meeting key.** Locks the decision that `calendar_event_id IS NOT NULL` is the authoritative calendar-source identifier. Lane B's source-scoped invariant for `CalendarWriter` ("calendar_event_id required") inherits from this ADR. "What does NOT change" point 5 ("`make_meeting_id` retained for non-calendar meetings") justifies `ManualWriter`/`BackfillWriter` `calendar_event_id: None` semantics.
- **ADR-0129** (composable surfaces) — Lane B's invariants must not silently shift contract for WP block consumers reading the meetings table.

**Prior solutions to cite explicitly:**

- **[capability-boundary-needs-crate-split-not-grep-2026-05-18.md](../../docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md)** — DOS-304/DOS-349 L6 finding that proc-macros and grep regexes cannot enforce capability boundaries inside a single crate. Module-scope `use` aliases, re-exports, function-local aliases, and trait-method calls all bypass grep. **Direct hit on §3.7.** Lane B reframes the grep as defense-in-depth (not enforcement) and uses ADR-0101 Phase 3 visibility narrowing as the structural gate.
- **[migration-filename-version-offset-2026-05-18.md](../../docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md)** — File as `<registered_version - 1>_<slug>.sql`, register as `registered_version`. Lane B's §3.6 backfill migration honors this; slot reserved at L1 commit time. Parallel-wave migration-slot reservation memory also applies if Lane A is landing migrations.

**Substrate-existed check:** no prior `services/meetings_writer/` attempt. No prior "meeting writer adapter" pattern in `docs/solutions/`. **No `docs/solutions/` entry exists for DOS-258 itself** — Lane B treats DOS-258 as the gold reference without retro context. K-in flagged this; reviewers should validate fidelity against current DOS-258 code, not a written retro.

If a future grep surfaces a prior attempt at this hardening: BLOCK and cite the path.

### Prior-work background

- [DOS-258](https://linear.app/a8c/issue/DOS-258) — entity linking deterministic-engine rewrite. **Lane B explicitly mirrors this structure.** §3 below maps DOS-258's actual file layout to the meetings-writer equivalent.
- [DOS-774](https://linear.app/a8c/issue/DOS-774) (Lane A) — read-side counterpart. Lane B's invariant matrix is informed by what Lane A's `read_surface_meetings` projection assumes about its input rows.

---

## §3 — Implementation / design direction

### 3.1 Module layout — mirror DOS-258's `services/entity_linking/`

New module: **`src-tauri/src/services/meetings_writer/`**

| File | Purpose | DOS-258 equivalent |
|---|---|---|
| `mod.rs` | Public API: `pub async fn write()` dispatcher + per-source manual wrappers | `entity_linking/mod.rs` (`evaluate()`) |
| `types.rs` | `WriteRequest`, `MeetingSource`, `WriteOutcome`, `InvariantViolation` | `entity_linking/types.rs` (`LinkingContext`, `OwnerType`, `RuleOutcome`) |
| `invariants.rs` | The invariant matrix: required fields + paired-table contract enforced before any INSERT | (no direct DOS-258 equivalent — write-side specific) |
| `calendar_adapter.rs` | Normalizes calendar-poll input → `WriteRequest` (calendar_event_id required, RFC3339 UTC start_time) | `entity_linking/calendar_adapter.rs` |
| `reconcile_adapter.rs` | Normalizes workspace-reconcile input → `WriteRequest` (start_time normalized to RFC3339 UTC from `start_iso` or local-format fallback) | `entity_linking/email_adapter.rs` |
| `manual_adapter.rs` | Normalizes user-action input → `WriteRequest` (calendar_event_id None semantics explicit) | (no direct DOS-258 equivalent — manual entity-link mutations) |
| `backfill_adapter.rs` | Normalizes historical-backfill input → `WriteRequest` (noon-UTC RFC3339 fallback, partial fields acknowledged) | (no direct DOS-258 equivalent) |

**`repository.rs` raw_* layer dropped (cycle-1 feasibility):** DOS-258 needs it because entity_linking has migration/repair operations bypassing the multi-phase engine; Lane B's equivalent operations live in `migrations/<n>_meeting_invariants_backfill.sql` (conventionally placed, not a runtime concern). If L1 surfaces a forcing case (e.g., the backfill genuinely needs to write through Rust rather than SQL), introduce `repository.rs` at that point. Trim-don't-strip: keep the architectural option, decline the speculative implementation.

### 3.2 Common normalized type — `WriteRequest`

```rust
// src-tauri/src/services/meetings_writer/types.rs
#[derive(Debug, Clone)]
pub struct WriteRequest {
    pub id: String,                          // required
    pub title: String,                       // required
    pub meeting_type: String,                // required (MeetingType serialized)
    pub start_time: String,                  // required, RFC3339 UTC (enforced by adapter)
    pub end_time: Option<String>,            // RFC3339 UTC if present
    pub calendar_event_id: Option<String>,   // None semantics depend on source (see invariant matrix)
    pub attendees: Option<String>,           // JSON array if calendar-sourced
    pub description: Option<String>,
    pub notes_path: Option<String>,
    pub source: MeetingSource,               // enum: Calendar | Reconcile | Manual | Backfill
}

pub enum MeetingSource {
    Calendar { event_id_required: bool },
    Reconcile { allow_local_format_fallback: bool },
    Manual,
    Backfill,
}
```

Mirrors DOS-258's `LinkingContext` — adapters normalize their source type into this common shape, then the dispatcher validates against the invariant matrix and writes through `db.upsert_meeting`.

### 3.3 Per-adapter signatures

Each adapter exposes a single `build_request` function — same shape as DOS-258's `build_context`:

```rust
// calendar_adapter.rs (mirrors entity_linking/calendar_adapter.rs:27)
pub fn build_request(event: &CalendarEvent, db: &ActionDb) -> Result<WriteRequest, String>

// reconcile_adapter.rs
pub fn build_request(meeting_summary: &MeetingSummary, date: &str) -> Result<WriteRequest, String>

// manual_adapter.rs
pub fn build_request(input: &ManualMeetingInput) -> Result<WriteRequest, String>

// backfill_adapter.rs
pub fn build_request(historical: &HistoricalMeetingInput) -> Result<WriteRequest, String>
```

The dispatcher `meetings_writer::write(ctx, db, request) -> Result<WriteOutcome, String>` then:
1. Validates `request` against the invariant matrix.
2. Mints a `MeetingWriteToken` (sealed — only this module can construct one; see §3.8 step 6).
3. Wraps the write + signal emission in **`db.with_transaction(...)`** for atomicity. Closure binding (per cycle-3 feasibility advisory + the cited precedent at `services/meetings.rs:28-42`):

   ```rust
   db.with_transaction(|tx| {
       tx.upsert_meeting(meeting, &token)?;            // 3-table write inside upsert_meeting
       services::signals::emit(ctx, tx, "meeting", &meeting.id, "meeting_upserted", "<source>", None, 0.9)?;
       Ok(WriteOutcome::from(...))
   })
   ```

   The closure binds the `&ActionDb` inner handle (`tx`) and reuses it for both the 3-statement write inside `upsert_meeting` (`db/meetings.rs:703, 733, 759` — currently NOT atomic without wrap; cycle-2 codex's finding) and the subsequent `signals::emit` call. Nested-tx-safe per `db/core.rs:165-167`.

4. Returns `WriteOutcome::{New, Updated, Unchanged}` (mirroring `db::MeetingSyncOutcome`).

### 3.4 Invariant matrix (per codex audit of all 5 production paths)

**Global invariants** (all sources):

| Field | Required | Notes |
|---|---|---|
| `id` | Yes | Non-empty, sanitized (no `@` per `sanitize_calendar_event_id` precedent at `workflow/reconcile.rs:298`) |
| `title` | Yes | Non-empty |
| `meeting_type` | Yes | Must parse as `MeetingType` enum |
| `start_time` | Yes | **RFC3339 UTC**, lexically comparable against `services/context.rs:856-866` readiness predicate |
| `created_at` | Yes | Set by writer (not adapter) — `Utc::now().to_rfc3339()` |
| Paired `meeting_prep(meeting_id)` row | Yes | One transaction with the meetings INSERT |
| Paired `meeting_transcripts(meeting_id)` row | Yes | One transaction; `intelligence_state` initializes per source (see source-scoped below) |

**Source-scoped invariants (cycle-1 corrections):**

| Source | `calendar_event_id` | `end_time` | `attendees` | `intelligence_state` init |
|---|---|---|---|---|
| Calendar | Required (NOT NULL) | Required | Required | `detected` (default per `migrations/055_schema_decomposition.sql:40-50`) |
| Reconcile | Optional | Optional | None | `detected` |
| Manual | Always None | Optional | None | `detected` |
| Backfill | Optional | None | None | **`detected`** (not `archived` — see correction below) |

**Cycle-1 correction — `intelligence_state` initialization.** My original packet specified `archived` for Backfill. **Wrong.** Cycle-1 codex challenge caught: in shipped meeting code, `intelligence_state = 'archived'` specifically means *cancelled calendar meeting* (`src-tauri/src/google.rs:911 detect_cancelled_meetings`). Downstream consumers EXCLUDE archived rows (`src-tauri/src/services/dashboard.rs:183`, `src-tauri/src/services/meetings.rs:1962`). Initializing backfilled historical rows as `archived` would have silently marked every backfilled meeting as cancelled. **Real correctness bug had this shipped.** Cycle-2: Backfill uses `detected` like all other sources. The "historical" nature is captured by the row's `start_time` being in the past, not by a state-table sentinel. ADR-0125 was mis-cited in cycle-1; the meeting `intelligence_state` lifecycle is a separate vocabulary from claim-substrate state.

**Justifications:**
- `start_time` RFC3339 UTC: Lane A's `read_surface_meetings` does TZ-aware UTC range compare. Local-format strings (current reconcile-path output at `workflow/reconcile.rs:246-253`) silently fail lex compare and drop from the briefing window. Pinning RFC3339 closes that class. **Note:** the DB schema (`migrations/055_schema_decomposition.sql:16`) has `start_time TEXT NOT NULL` with no format constraint; the invariant is enforced at the writer adapter layer, not the schema. Backfill migration brings existing rows into compliance.
- Paired transcript row required: cycle-1 feasibility confirmed **all 5 existing production paths already write paired transcripts** (`db/meetings.rs:838-844` via `ensure_meeting_in_history`; `db/meetings.rs:733-773` via `upsert_meeting`; `backfill_meetings.rs:377-387` via explicit `INSERT OR IGNORE`). The invariant codifies existing behavior, not new behavior. **Orphan-transcript backfill task downgraded** in §3.6 — fires only if pre-migration audit finds any (no evidence they exist today).
- Calendar source: `calendar_event_id IS NOT NULL` required because `record_cancelled_calendar_meetings` filters on it for cancellation candidacy (per ADR-0061). A calendar-origin row without `calendar_event_id` is unreachable by the cancellation sweep — orphan-by-construction. Lane B rejects.
- Manual source: `calendar_event_id` is **always** None per ADR-0061's `make_meeting_id` carve-out. If a manual mutation has a calendar event association, it's a misclassified source — route through CalendarWriter instead.

### 3.5 Production writer enumeration — corrected after cycle-1

Cycle-1 codex + feasibility both caught that my original 5-path enumeration was incomplete. The full picture is 5 **INSERT** paths (the original list, now confirmed correct) plus additional **UPDATE** paths that mutate existing meeting rows without creating them. The gate's scope decision below distinguishes the two.

**Group 1 — INSERT paths (Lane B owns; routes through adapters):**

| Existing call site | File:line | Lane B adapter | Notes |
|---|---|---|---|
| Calendar attendance batch | `services/people.rs:86-110` → `db/meetings.rs:822-844` | `CalendarWriter` | Currently calls `ensure_meeting_in_history`. Re-route. |
| Manual meeting-entity override/add | `services/meetings.rs:1119-1128, :1221-1230` | `ManualWriter` | Currently calls `ensure_meeting_in_history` with `calendar_event_id: None`. Re-route. |
| Workspace reconcile | `workflow/reconcile.rs:209-287` → `services/meetings.rs:22-29` → `db.upsert_meeting` | `ReconcileWriter` | The only writer that emits local-format start_time. Adapter MUST normalize to RFC3339 UTC. |
| Prepare/timeline calendar fallback | `prepare/orchestrate.rs:781, :1229`, `commands/integrations.rs:2518-2555` → `services/mutations.rs:448-456` → `db.upsert_meeting` | `CalendarWriter` | Cycle-1 feasibility verified: ALL these calls are calendar-origin (`prepare/orchestrate.rs:705-784, :1185-1247` + `commands/integrations.rs:2505-2565`). Original "mixed source" worry was overstated. |
| Historical filesystem backfill | `commands/planning_reports.rs:893-903` → `backfill_meetings.rs:37-55, :361-364` | `BackfillWriter` | Tauri command path (not just admin migration); creates full row stack. Separate adapter justified. |

**Group 2 — UPDATE paths (Lane B *does not own*; gate decision below):**

Cycle-1 reviewers caught that the meetings table also has production UPDATE writers I had missed. The adapter pattern fits creates, not arbitrary field mutations. These paths stay out of the adapter module but are explicitly exempt from the §3.7 gate.

| Existing call site | File:line | What it does | Why outside the adapter pattern |
|---|---|---|---|
| `reclassify_meeting_types_from_attendees()` | `src-tauri/src/db/accounts.rs:2176, :2253, :2293, :2339` | `UPDATE meetings SET meeting_type = 'internal' | 'customer'` based on attendee classification. Called from `lib.rs:621` (boot) and `services/settings.rs:624` (domain-settings save). | Mutates a single field on existing rows; doesn't create. Adapter pattern would force inventing a `ReclassifyWriter` for one column. Explicit gate exemption. |
| Privacy data purge | `src-tauri/src/db/data_lifecycle.rs:1031` | `UPDATE meetings SET description = NULL` for redaction. | Field-level data hygiene; orthogonal to the creation contract. Explicit gate exemption. |
| Meeting-ID canonicalization | `src-tauri/src/db/data_lifecycle.rs:1964, :1999` | `UPDATE meetings SET id = ...` for merge resolution after canonicalization. | Same row, identity migration. Orthogonal to creation. Explicit gate exemption. |

**Gate decision (cycle-1 forcing question):** Lane B's §3.7 gate enforces **INSERT-only**. UPDATE paths above are exempt by explicit allowlist. Rationale: the bug class Lane B prevents (writer drift creating malformed rows that read-side filters defend against) is a *creation* problem. Single-field UPDATEs on existing rows can't introduce orphan-construction issues; they have their own narrower invariants that Lane B doesn't try to enforce.

If a future class of bug surfaces from these UPDATE paths, the equivalent adapter pattern can be applied to *that* class without conflating with this one.

### 3.6 Backfill migration (cycle-1 corrections folded in)

Filename per the project convention (`docs/solutions/conventions/migration-filename-version-offset-2026-05-18.md`): file as `<registered_version - 1>_meeting_invariants_backfill.sql`, register as `registered_version`. Slot reserved at L1 commit time; if Lane A is concurrently landing migrations, claim non-overlapping slot blocks per the memory.

**Targets (cycle-2 corrected):**

1. **Local-format `start_time` → RFC3339 UTC.** Existing rows from old reconcile passes. **TZ fallback pinned at L0** (cycle-1 feasibility correctly flagged that deferring to L1 was abdication): use `America/New_York` as the fallback TZ when the workspace config is missing or unparseable, **matching the read-side pattern at `services/context.rs:803-807`** (`unwrap_or(chrono_tz::America::New_York)`). This bounds the corruption risk: every row gets *some* TZ assignment; users not in America/New_York receive a known 3-5 hour offset. The dry-run report enumerates which rows received the fallback so the user can correct config before re-running.
2. **Calendar-origin rows missing `end_time`/`attendees`.** Populate from `calendar_events` table if available; otherwise leave NULL and accept as a known pre-Lane-B remnant. Cycle-1 codex correctly noted these rows would still fail the new CalendarWriter invariants; backfill repairs what it can, the residual remains as documented legacy. New writes are blocked from creating more such rows.
3. ~~Rows missing paired `meeting_transcripts`.~~ **Downgraded** (cycle-1 feasibility): all 5 existing production INSERT paths already write paired rows. No evidence orphans exist. Pre-migration audit script greps for orphans; if zero results, the backfill task is a no-op. If non-zero, surfaces for review before fix.
4. ~~Rows missing paired `meeting_prep`.~~ Same downgrade.

**Dry-run protocol:** ships in `scripts/backfill_meetings_dry_run.sh`. Reports counts per repair class + sample rows. The report **distinguishes** (per cycle-2 feasibility advisory):

- (a) rows where workspace TZ config is missing/unparseable and the `America/New_York` fallback was used,
- (b) rows where workspace TZ was set BUT the row's local timestamp falls in a DST spring-forward gap (`from_local_datetime.earliest()` would absorb the gap silently on the read side at `services/context.rs:843`; the backfill MUST report these explicitly because once written the row is immutable).

(a) is recoverable by user config correction + re-running the dry-run. (b) is documented residual; the user accepts the small offset for affected rows. Migration runs only after dry-run review against James's workspace.

### 3.7 Defense-in-depth: visibility narrowing (gate) + pre-commit WARN (heuristic)

**Cycle-2 reframing (after cycle-2 feasibility).** Both cycle-1 K-in and cycle-2 feasibility independently caught that grep enforcement is broken in this codebase. K-in cited `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md`: `use` re-exports, function-local aliases, trait-method dispatch bypass any grep. Cycle-2 feasibility added a concrete failure mode: Rust's idiomatic `#[cfg(test)] mod tests { ... }` puts test fixtures INSIDE production-named `.rs` files (~20 such files have `INSERT INTO meetings` in their test modules today), so a path-glob exemption like `*_test.rs|*/tests/*` cannot distinguish in-file test code from production code without per-hunk Rust-AST awareness the hook doesn't have.

Cycle-2 separates enforcement from noticing:

**Structural gate (primary, the only blocking layer): sealed `MeetingWriteToken` capability.** Cycle-2 cited `pub(in crate::services::meetings_writer)` visibility narrowing; cycle-2 codex caught that Rust `pub(in ...)` only restricts visibility to **ancestor** modules, and `services/` is a sibling of `db/` not an ancestor — the visibility narrowing would not compile.

Cycle-3 replaces it with the sealed-token pattern. In `services/meetings_writer/types.rs`:

```rust
/// Capability required to call `db.upsert_meeting` / `db.ensure_meeting_in_history`.
/// Constructed only inside this module — outside callers cannot mint one,
/// so the only path that writes meetings is through this adapter module.
pub struct MeetingWriteToken(());

impl MeetingWriteToken {
    pub(super) fn mint() -> Self { Self(()) }
}
```

And in `src-tauri/src/db/meetings.rs`:

```rust
pub fn upsert_meeting(&self, meeting: &DbMeeting, _token: &crate::services::meetings_writer::MeetingWriteToken) -> Result<(), DbError> { /* ... */ }
pub fn ensure_meeting_in_history(&self, input: EnsureMeetingHistoryInput<'_>, _token: &crate::services::meetings_writer::MeetingWriteToken) -> Result<MeetingSyncOutcome, DbError> { /* ... */ }
```

Only code inside `services/meetings_writer/` can call `MeetingWriteToken::mint()`, so it's the only code that can call `db.upsert_meeting` / `db.ensure_meeting_in_history`. The compiler refuses bypass attempts statically. Inside `db/meetings.rs`, the raw `INSERT INTO meetings` is the only one in the codebase — every other production write path now requires a token and is gated.

This matches DOS-258's "single writer, two adapters" pattern (ADR-0101 Phase 3) — same intent, different mechanism (sealed capability vs visibility) because Rust's module hierarchy in this codebase makes sibling visibility unavailable.

**Heuristic notice (secondary, WARN-only, never blocks commit): pre-commit grep.** Surfaces matches to stderr so the author can sanity-check them; does NOT fail the commit. Catches the narrow class the structural gate can't catch — raw `INSERT INTO meetings` SQL strings constructed dynamically (where Rust visibility doesn't apply because the string never becomes a typed `db.*_meeting` call). Even there, the WARN-only framing is honest about what grep can do.

```sh
# Lane B (v1.4.8): WARN-only heuristic on raw INSERT INTO meetings.
# Structural gate is the sealed MeetingWriteToken capability per §3.8 step 6
# (ADR-0101 Phase 3 spirit; sibling-module visibility doesn't exist in Rust).
# This grep does NOT block; it surfaces matches for
# author review. The author decides whether the match is legitimate (test fixture,
# documentation comment, dynamic SQL builder) or a real bypass attempt.
meetings_insert_matches=""
for file in $(git diff --cached --name-only --diff-filter=ACM); do
  # Skip the canonical writer module and its target.
  case "$file" in
    src-tauri/src/services/meetings_writer/*) continue;;
    src-tauri/src/db/meetings.rs) continue;;
    src-tauri/src/migrations.rs|src-tauri/src/migrations/*) continue;;
    src-tauri/src/devtools/*|src-tauri/src/demo.rs) continue;;
  esac
  hit=$(staged_added_lines "$file" 2>/dev/null \
    | grep -Ei 'INSERT[[:space:]]+(OR[[:space:]]+(IGNORE|REPLACE)[[:space:]]+)?INTO[[:space:]]+meetings\b' \
    || true)
  if [ -n "$hit" ]; then
    meetings_insert_matches="${meetings_insert_matches}${file}:\n${hit}\n"
  fi
done
if [ -n "$meetings_insert_matches" ]; then
  echo "WARN (Lane B / v1.4.8): raw INSERT INTO meetings outside services/meetings_writer/ + db/meetings.rs." >&2
  echo "If this is a test fixture inside #[cfg(test)], a doc comment, or a one-off devtools/migration, it's fine." >&2
  echo "If this is a new production write path, it should route through services/meetings_writer/ adapters." >&2
  printf '%b' "$meetings_insert_matches" >&2
  # Intentionally non-blocking. Visibility narrowing is the structural gate.
fi
```

Key cycle-2 design decisions:

- **No `exit 1`.** Grep is WARN-only. Visibility narrowing is the gate. Cycle-2 feasibility correctly noted that the cycle-1 blocking grep would have false-positive-blocked test-fixture edits in ~20 production-named files; cycle-2 doesn't fight that, it accepts grep can't distinguish and lets the visibility narrowing carry the load.
- **`db/meetings.rs` added to exempt list.** Post §3.8 step 6, `db/meetings.rs` is the only file that legitimately contains the canonical INSERT (inside `upsert_meeting`). Cycle-2 feasibility caught this as an advisory; folded.
- **No exempt list for Group 2 UPDATE files** (`db/accounts.rs`, `db/data_lifecycle.rs`). The grep regex matches `INSERT INTO meetings` only, not `UPDATE meetings`; Group 2 UPDATEs don't trigger. Simpler than the cycle-1 attempt to enumerate.
- **`meetings\b` boundary check:** does not match `meetings_view`, `meetings_history` (cycle-1 codex verified).

**What this acknowledges:** the grep won't catch `use crate::db as foo; foo.upsert_meeting(...)` because the call site lacks the SQL substring AND because the sealed-token requirement at the type level makes this call impossible without the token. The grep is best-effort dynamic-SQL-string reporting, not a security gate. The **sealed token** IS the security gate, and it carries the full load — and unlike Rust visibility, it compiles.

### 3.8 Sequencing within the lane

1. Add `services/meetings_writer/` module with types + invariants + dispatcher (no callers migrated yet).
2. Add adapters one at a time, with per-adapter unit tests covering invariant enforcement.
3. Migrate production call sites one at a time, each in its own commit, each with a regression test verifying the migration doesn't change observable behavior.
4. Add pre-commit grep.
5. Ship backfill migration (dry-run first).
6. Lock the structural gate: add the `_token: &MeetingWriteToken` parameter to `db.upsert_meeting` and `db.ensure_meeting_in_history` signatures. Mint the token only inside `services/meetings_writer/`. Outside callers can no longer call these functions because they can't construct a token. (Replaces the cycle-2 plan to use `pub(in ...)` visibility, which doesn't compile across sibling modules in Rust.)

---

## §4 — Acceptance criteria

1. `src-tauri/src/services/meetings_writer/` module exists with the file layout in §3.1.
2. `WriteRequest` + `MeetingSource` types per §3.2.
3. 4 adapters (Calendar, Reconcile, Manual, Backfill) each with `build_request()` and per-adapter unit tests covering invariant enforcement.
4. Every production **INSERT** path on the `meetings` table (5 paths enumerated in §3.5 Group 1) routes through the appropriate adapter. **Group 2 UPDATE paths** (`reclassify_meeting_types_from_attendees`, `data_lifecycle.rs` purge + canonicalization) explicitly stay outside the adapter module per the INSERT-only gate decision; they're enumerated in §3.5 Group 2 and exempt by design.
5. Backfill migration ships and runs cleanly against a representative DB (James's workspace as the validation target).
6. Pre-commit grep **WARNs** on raw `INSERT INTO meetings` outside the exempt paths (see §3.7 — WARN-only by design after cycle-2 feasibility caught false-positive risk on inline `#[cfg(test)]` modules). Structural enforcement is the sealed `MeetingWriteToken` capability per §3.8 step 6.
7. **Sealed `MeetingWriteToken` ships**: `db.upsert_meeting` / `db.ensure_meeting_in_history` signatures gain the `_token: &MeetingWriteToken` parameter; token is mintable only inside `services/meetings_writer/`. Production code outside the adapter module CANNOT call these functions (compile error).
8. Dispatcher wraps the 3-table write in `db.with_transaction(...)` for atomicity (cycle-3 correction).
9. `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` clean.
10. Lane A's `read_surface_meetings` regression test from DOS-774 still passes (no read-side regression from write-side hardening).
11. Manual L4 sanity: today page renders correctly after migration; account/person detail pages render correctly.
12. Regression test on the sealed-token gate: a test file attempting to `db.upsert_meeting` without minting a token fails to compile (trybuild-style negative test).
13. Regression test on transaction atomicity: a test induces a failure on the second of the 3-table writes and verifies the meeting row is rolled back (no orphan meeting without paired prep/transcripts rows).

---

## §5 — Risks + open issues for reviewers

### Risks (cycle-2)

- **Backfill TZ fallback bounded but not eliminated.** §3.6 pins `America/New_York` as the fallback when workspace TZ config is missing. Bounded corruption: any user not in that TZ receives a 3-5 hour offset on backfilled rows. The dry-run report surfaces affected rows so the user can correct config before running. Cycle-1 feasibility's flag drove the L0 pin.
- **Pre-commit grep is best-effort, not enforcement.** §3.7 reframed per K-in finding (class-pattern recurrence). Structural enforcement is the **sealed `MeetingWriteToken` capability** (§3.8 step 6 — cycle-3 replacement for the `pub(in ...)` visibility narrowing that doesn't compile across sibling modules). Grep catches what the token doesn't see (raw dynamic SQL strings); it's WARN-only per cycle-2 feasibility.
- **Token leak risk.** If `MeetingWriteToken::mint()` is accidentally made `pub` instead of `pub(super)`, the gate is wide open. Mitigation: clippy lint or audit pattern on the token type's visibility. L1 author adds a `compile-fail`-style trybuild test that proves a non-module caller can't mint (AC #12).
- **Group 2 UPDATE paths uncovered by Lane B.** §3.5 enumerates 3 production UPDATE paths (`reclassify_meeting_types_from_attendees`, privacy purge, ID canonicalization) that intentionally stay outside the adapter pattern. If a future bug surfaces from one of those paths, it requires its own targeted hardening — Lane B doesn't reach. Documented as scope-bound, not gap.
- **Backfill orphan-row claim downgraded.** Cycle-1 feasibility caught that all 5 existing INSERT paths already write paired transcripts/prep rows. §3.6 backfill tasks #3-4 fire only if the pre-migration audit finds orphans. If audit returns zero, tasks are no-ops.
- **No DOS-258 retrospective entry exists.** K-in flagged Lane B treats DOS-258 as gold-reference without retro context. Cycle-2 packet trims the cargo-culted `repository.rs` layer (§3.1) as the most likely over-fit. Reviewers should validate adapter fidelity against current DOS-258 code, not a written retro.

### Open questions for reviewers (cycle-2)

1. **Migration sequencing aggressive vs cautious.** §3.8 step 3 says "one call site per commit." That's 5+ commits before adapter is the only path. Reviewer may prefer batch migration (one commit moves all 5 INSERT paths) for atomicity. Recommendation: batch.
2. **Manual writer entry surface.** Cycle-1 didn't directly address: should the manual adapter be invoked from `commands/people_entities.rs` directly, or always go through `services/meetings.rs:1119-1128, :1221-1230` first? Affects test fixture scope.
3. **`reclassify_meeting_types_from_attendees` should it move to a writer adapter at some future point?** Today exempt (§3.5 Group 2). If product surface ever wants to surface "I reclassified your meeting from internal to customer" as a user-visible signal, the UPDATE path is the right place to emit a signal — at which point routing through an adapter makes sense.

---

## §6 — Sequencing + migration

**Depends on Lane A — concrete handoff predicate (cycle-1 feasibility correction).**

Original cycle-1 packet said "Lane A merged + stable in production," which the reviewer correctly flagged as open-ended. Pinned at cycle-2:

Lane B can start when **all three** of these conditions hold:

1. Lane A's PR is merged to `dev`.
2. Lane A's `read_surface_meetings` regression test (`cargo test --lib`) is present in the merged code and passes.
3. James has run the app for at least one boot cycle without surfacing a Lane-A-related regression on the today page (L4 sanity smoke test).

This is concrete enough that an L1 implementer reading this packet knows when they're cleared to start. It is **not** "wait N days" — it's three checkboxable conditions.

**Single PR or split?**

Cycle-1 codex + feasibility audit gives ~7 commits minimum: 1 module skeleton + 4 adapters + 1 backfill + 1 grep-hook. Single PR for atomicity; if PR grows beyond ~1500 LOC, split at adapter boundaries (one adapter + its migration sites = one PR). The visibility-narrowing step (§3.8 step 6) ships LAST in either case — it's the structural gate that closes the door.

**L2 router-selected reviewers** (preview, finalized at L2): `pr-review-toolkit:type-design-analyzer` (new types + new module surface), `ce-data-migrations-reviewer` (backfill migration safety), `ce-data-integrity-guardian` (data integrity + migration safety together).

**No L4 cycle** — Lane B is write-path only; user-facing surfaces don't change. L4 sanity check (§4 AC #9) is a smoke test, not a full L4 cycle.

---

## §7 — L0 review composition

Per the engineering ladder, Standard tier L0:

- **Required (default 2):**
  - `/codex challenge` — adversarial. Specifically target: "does the adapter pattern actually catch every writer, or is there a hidden 6th path?"
  - One planning reviewer: **`ce-feasibility-reviewer`** — will the adapter pattern survive contact with all 5 production call sites? Will the invariant matrix reject too many legitimate edge cases?
- **K-in (mandatory, parallel-grep):** `ce-learnings-researcher` — cite hits from §2 substrate-grep targets.

Conditional second planning reviewer at Standard tier is NOT default. Promote to Wave + add `ce-scope-guardian-reviewer` if L0 surfaces scope-creep risk (e.g., "while we're in the meetings writer, let's also refactor X" — push back).

No design lens needed: write-path only, no user-facing surface change.
No security lens needed: local-to-local single-user, no new trust boundary, no new privileged action. Backfill migration is read-then-write within the existing trust scope.

---

## §8 — What "done" looks like

- A reviewer reading any production code path that creates a meeting row can trace it through exactly one adapter in `services/meetings_writer/`, with the invariants enforced inline.
- A new contributor trying to write `INSERT INTO meetings ...` outside that module fails the pre-commit gate with a clear error pointing at the adapter.
- The backfill migration repairs known violators; subsequent invariant checks pass on the live DB.
- Lane A's read-side projection has a clean substrate to read from — no local-format start_time rows, no orphan transcript-less rows, no calendar-source rows missing `calendar_event_id`.
- A future DOS-771-shape bug (a new consumer forgetting a filter) is still possible on the read side (Lane A guards via `MeetingsViewIntent`), but the write side is closed: drift can no longer reintroduce malformed rows that read-side filters then have to defend against.
