# W4-B L0 packet — concurrency contract: server-assigned versions + stale-write rejection

Date: 2026-05-13 (V9)
Project: v1.4.2 — Personal Intelligence Engine: WordPress Foundation
Parent: DOS-546
Wave: 4 stage-1 (gates W4-A0 / W4-C / W4-D / W4-E and the W4-A renderer)
Issue: DOS-567 (W4-B: three-view consistency concurrency contract)
Sibling: DOS-589 (signal subscriber + scope-filter dispatch — split per Cycle 2 partial-convergence rule)

This packet captures the W4-B contract decisions resolved at L0. The Linear issue description remains the canonical execution contract; this packet supersedes it only where it makes explicit a decision the issue leaves open.

## Changelog

- **V9 (2026-05-13):** §6.5 precedence table amendment per W4-C Cycle 2 eng P1-N1 coordination gap. W4-C requires two new `BridgeSurfaceError` variants — `ProjectionTampered` and `ProjectionVersionRollback` — at higher precedence than the stale-watermark family. Both fire on signature/ledger verification failures before the bridge evaluates expected-version logic, so tampered or replay-rolled-back projections never receive a `correction.claim` payload (per §6 signature-check-before-409 ordering). New ac §45 fixtures pin pairwise precedence (tamper-and-stale → tamper wins; rollback-and-stale → rollback wins). No structural changes to W4-B's own substrate contract; amendment is contract-coordination with W4-C only.
- **V8 (2026-05-13):** Class-pattern lifts from stage-2 packet Cycle 1 reviews — three recurring shapes promoted out of per-packet patching into upstream W4-B contract: (a) **NEW §17 wp_user_id session binding** — class-level rule lifted as inherited acceptance to DOS-568/569/570/571/572/573 + DOS-589. Every endpoint receiving a wp_user_id in the request body MUST verify it against the SurfaceClient session's bound wp_user_id (established at W2-C pairing). Body-asserted mismatch → 403 wrong_user before any further validation. Closes /cso Cycle 1 W4-E CRITICAL C1; preempts same-shape findings on W5-A, W4-A. (b) **§37 promoted from path-α to canonical** — pin `src-tauri/src/bridges/surface_client.rs` as the SurfaceClient route module owner. All `/v1/surface/*` endpoints (W4-C keyring + event-log fetch, W4-E nonce issue/verify, W5-A feedback router, DOS-589 dispatcher routes) land here. Closes devex Cycle 1 W4-A0/C/E packet-owner ambiguity in one pin. (c) **§Interlocks W4-D row updated** — substrate publishes `project_composition_for_surface(composition, ctx) → ProjectedComposition` as the canonical public API. Block-level helpers are internal. Composition-level cap (unknown-block count per composition) enforced inside this API; W4-A renderer consumes only the composition-level surface. Closes codex Cycle 1 W4-D HIGH-1. No structural changes to W4-B's own contract; eng + cso + devex + codex APPROVE from V6 carries forward.
- **V7 (2026-05-13):** Migration slot correction per user — v1.4.1 (in-flight, finishing up) is using v169. W4-B's slot shifts from v169 → **v170**. Recommended block reservation updates: W4-C = v171–v174, W4-E = v175 (if persisted). Single-field rename across operative §§ (Pre-work, "What W4-B authors net-new" table, §7 schema, §8, §11, §13 backfill audit `migration_version`, Open Q1, L0 closure conditions). Historical V3/V4 changelog entries describing the v168→v169 correction are left as-is for audit trail. No structural changes; no re-review needed.
- **V6 (2026-05-13):** Cycle 5 codex CONDITIONAL closure — codex flagged 2 residual stale `signal_events` references at lines 56 (Pre-work bullet) and 207 (§5 opening paragraph) that V5 missed. Both replaced with `version_events`. No other changes. Codex Cycle 5 explicitly: "Replace those two with `version_events`, and I would approve." V6 is text-only; no Cycle-6 re-review needed beyond codex-only confirmation of the two-line patch.
- **V5 (2026-05-13):** Cycle 4 codex CONDITIONAL fold — 0 CRITICAL, 0 HIGH, 2 MEDIUM, 2 LOW. eng + cso + devex APPROVE confirmed at Cycle 4 (V4). All four codex Cycle 4 findings are bounded local schema/wording tweaks — material changes: (a) **§15 schema rewrite** — `version_events` gains `event_seq INTEGER PRIMARY KEY AUTOINCREMENT` for replay ordering; `cursor TEXT NOT NULL UNIQUE` is UUIDv4 (unguessable opaque ID); DOS-589 replay clause: `WHERE event_seq > ? ORDER BY event_seq` (codex MEDIUM C4-1). (b) **§13 Insert dedup concurrency rule pinned** — `ClaimMutationTarget::Insert` MUST flow through canonical commit/dedup-lock; duplicate detection inside same `BEGIN IMMEDIATE` Tx; dedup-key collisions redirect to `Reinforced` per existing `claims.rs:5286-5314` (verified by /cso Cycle 4 probe 3c). New fixture for two concurrent Inserts with same dedup_key (codex MEDIUM C4-2). (c) **§15 XOR CHECK** — `CHECK ((claim_id IS NOT NULL) != (composition_id IS NOT NULL))` replaces OR (codex LOW C4-3). (d) **§3 + Interlocks stale `signal_events` references substituted with `version_events`** (codex LOW C4-4). No structural changes; eng + cso + devex don't need Cycle-5 re-confirmation per W3 L0 precedent. Cycle-5 codex-only confirmation is the L0-closure gate.
- **V4 (2026-05-13):** Cycle 3 reviewer fold (eng + cso + devex APPROVE; codex BLOCK with 2 HIGH + 1 MEDIUM + 1 LOW). All four codex findings are valid substrate-side bugs the other reviewers missed; V4 closes them. Material changes: (a) **§7 + §15 cursor durability rewrite** — V3 claimed cursor allocation "inside mutation transaction" makes it durable, but transaction rollback erases the row. V4 introduces a `mutation_attempts` reservation table: cursor is committed BEFORE the mutation transaction in a separate pre-mutation Tx; the mutation Tx attaches its event to the reserved cursor; on rollback/panic, a post-rollback Tx emits a terminal `mutation_aborted` event at the same cursor. 423 responses now reference a cursor that always resolves (codex HIGH C3-1). (b) **§13 `ClaimMutationTarget` enum rewrite** — V3 trait `MutatingProposal` required `expected_claim_version()` + `claim_id()`, but `ClaimProposal` (claims.rs:68) has `id: Option<String>` and fresh-insert call sites construct claims with `id: None`. V4 introduces `ClaimMutationTarget::{ Insert | Mutate }` enum; `MutatingProposal::target()` returns the enum variant; `commit_claim` dispatches per variant — fresh inserts skip version CAS by design, existing-claim mutations require both fields populated (codex HIGH C3-2). (c) **NEW `version_events` side table (v169)** — V3 §5/§15 relied on the generic `signal_events` table for outbox rows without DB-level constraints; V4 ships a dedicated `version_events` table with CHECK constraints on substrate-owned discriminants (`event_kind` enum, `cursor` format, `scope_redacted` bool) and TEXT NULL columns for caller-controlled fields, closing codex MEDIUM C3-3 + folding cso Cycle 3 path-α guard. (d) **syn dev-dependency** — added to `src-tauri/Cargo.toml` `[dev-dependencies]` per ac §34 (codex LOW C3-4). (e) §13 NOTE — `commit_claim` documented as the only path to claim_version assignment; `MutatingProposal::target()` is the contract, not the version itself (per cso Cycle 3 LOW observation). Acceptance criteria expanded from 38 to 42. ac numbering normalized.

- **V3 (2026-05-13):** Cycle 2 reviewer fold (eng + cso + devex CONDITIONAL APPROVE; codex BLOCK with 1 CRITICAL + 2 HIGH still open). Material changes: (a) **CRITICAL outbox pattern** — §3 + new §15 require signal event + cursor inserted in the SAME DB transaction as the version mutation (closes codex CRITICAL C2-1, cso N4, MidFlightMutation cursor durability codex C2-5); (b) §13 rewrite — `MutatingProposal` trait at `commit_claim` boundary (compile-time hard fail) replaces `#[ability]` macro fiction (closes codex HIGH C2-3 + devex D-N1); (c) §29 rewrite — `tests/version_assignment_gate.rs` integration test using already-transitive `syn` dep, NOT a "v1.4.0 precedent" claim (closes devex D-N3); (d) NEW acceptance criterion §33 — CorrectionRef event-log fetch is scope-filtered identically to inline 409 (closes codex HIGH C2-2 + cso N2); (e) NEW §15 — SQLite domain `1..=i64::MAX` for persisted version columns; overflow at `i64::MAX` not `u64::MAX` (closes codex MEDIUM C2-4); (f) NEW §6.5 — `BridgeSurfaceError` precedence table (closes devex D-N2); (g) ac §30 enum extended with `inflated_version_rejected` (closes eng P2-NEW-2); (h) §7 rewrite — `MutationLock` is defense-in-depth, CAS is correctness; multi-process is phase-deferred to v1.4.3+, NOT W4-C tamper territory (closes eng P2-NEW-1 + cso N3); (i) §8 ac BEGIN IMMEDIATE transaction isolation (closes cso N5); (j) §1 pins `Block.field_bindings: Vec::new()` default for back-compat (closes devex side-note); (k) §5 rewrite — references DOS-589 for delivery; W4-B owns substrate-side emission + event-log schema only; (l) **Migration slot corrected: v168 was wrong (already merged on dev DOS-546 W0); W4-B = v169** (closes eng P1-NEW-1 factual). (m) New §16 — class-level scope-filter rule lifted as comment on DOS-569 + DOS-573 (closes cso scope-leak class recurrence). Acceptance criteria expanded from 30 to 36. **All Cycle 2 codex findings substrate-side fixed.**
- **V2 (2026-05-13):** Cycle 1 fold — `commit_composition` chokepoint (closed C2), scope-filtered correction (closed C1), `field_bindings` (closed H5), signal payload schema (closed C1 partial), out-of-band ordering (closed H6), MidFlightMutation promoted, naming collision resolved.
- **V1 (2026-05-13):** Initial draft.

## Status snapshot

- W2 closed clean 2026-05-12.
- W3 L0 packet closed Cycle 3 (eng + devex + cso APPROVE); V4 Cycle-4 codex approval is the W3-A/B/C unblock gate. W3-0 (DOS-563) running in parallel session.
- W1-A (`Actor::SurfaceClient`) + W1-E (`Composition`, `Block`, `ClaimRef`, `ProvenanceRef`) merged. Both named DOS-567 blockers — both clear.
- W4-B is the first stage-1 substrate work and gates four parallel stage-2 lanes.
- **Cycle 1 panel (2026-05-13):** eng CONDITIONAL, /cso CONDITIONAL, /plan-devex-review CONDITIONAL APPROVE, /codex challenge BLOCK. V2 folded all findings bounded by DOS-567 acceptance.
- **Cycle 2 panel (2026-05-13):** eng CONDITIONAL (9/10 ADDRESSED; factual slot error), /cso CONDITIONAL APPROVE (all ADDRESSED, H2 PARTIALLY), /plan-devex-review CONDITIONAL APPROVE (all ADDRESSED), /codex challenge **BLOCK** (1 CRITICAL + 2 HIGH + 2 MEDIUM + 1 LOW). V3 folded all Cycle 2 substrate-side findings. DOS-589 sibling ticket filed for signal-bus dispatcher rewrite (partial-convergence). Class-level scope-filter rule lifted to DOS-569 + DOS-573 via Linear comments.
- **Cycle 3 panel (2026-05-13):** eng **APPROVE**, /cso **APPROVE** (1 path-α maintenance), /plan-devex-review **APPROVE**, /codex challenge **BLOCK** (2 HIGH + 1 MEDIUM + 1 LOW). 3-of-4 APPROVE. Codex's 2 HIGH findings are real bugs the other three missed — cursor durability inside mutation transaction and `MutatingProposal` non-trivial for `ClaimProposal`. V4 closes all four codex findings. Convergence trajectory: codex BLOCK severity Cycle 1 (3C+3H) → Cycle 2 (1C+2H) → Cycle 3 (0C+2H). No class-pattern recurrence (eng/cso/devex all verified). Cycle 4 panel queued.

## Pre-work confirmed (substrate reuse audit)

**Headline finding:** most of the watermark surface DOS-567 asks for already exists in code. W4-B is enforcement + one migration + one bridge chokepoint + one outbox pattern + additive type changes + trait-based input enforcement — not a new contract.

### Already in `src-tauri/abilities-runtime/src/abilities/composition.rs` (W1-E)

- **`CompositionMetadata.composition_version: CompositionVersion`** (u64 wrapper) — line 374. Baked into wire shape.
- **`ClaimRef { claim_id: String, claim_version: Option<u64> }`** — line 135. V3 makes population mandatory at the substrate boundary (bridge rejects `None` per §10).
- **`ClaimRef::with_version(claim_id, version)`** — line 149. V3 adds `with_field(claim_id, version, field_path)` constructor.
- **`ProvenanceRef { invocation_id: InvocationId, field_path: FieldPath }`** — line 263. Pre-existing per-Block. Anchors block-origin pointer per ADR-0105.
- **`CompositionVersion::bump()`** — line 122. Currently uses `saturating_add(1)`. V3 replaces with `checked_add → Option<Self>`.
- **`Composition::new` is `pub(crate)`** per ADR-0130 §1 substrate-owned authorship — `commit_composition` lives inside `abilities-runtime` crate.
- **`Block::new` at line 458** — V3 adds `field_bindings: Vec<FieldBinding>` field; existing call sites get `Vec::new()` default for back-compat per §1.

### Already in `src-tauri/src/services/claims.rs`

- **`commit_claim(ctx, db, proposal)`** — line 4974. Single-writer for every authoritative claim mutation; gated on `ctx.check_mutation_allowed()`. V3 makes `proposal: impl MutatingProposal` (trait-bound) per §13. **Claim_version assignment site.**
- **`CLAIM_UPDATE_ALLOWED_COLUMNS`** — line 234. `claim_version` is assigned exclusively inside `commit_claim`.
- **No `claim_version` column on `intelligence_claims` yet.** Migration adds `claim_version INTEGER NOT NULL DEFAULT 0 CHECK (claim_version BETWEEN 0 AND 9223372036854775807)`. **Migration slot v170** (v168 merged on dev as `migrate_v168_reconcile_missing_claim_shadow_trust_columns` at 75eda588; v169 reserved by v1.4.1 in-flight work).
- **Existing `current_claim_version_for_subject` (6 call sites at claims.rs:6272/6676/7523, db/invalidation_jobs.rs:352/730, services/invalidation_jobs.rs:88)** — per-subject version. V3 renames to `current_subject_claim_version` per §9.
- **`db.with_transaction` pattern (claims.rs:3396)** — V3 `commit_composition` opens with `BEGIN IMMEDIATE` per §8 to prevent TOCTOU on CAS read.

### Already in `src-tauri/src/signals/bus.rs` (ADR-0080)

- **`emit_signal_event(...)` → `SignalEmitOutcome { id, coalesced }`** — line 73. **Confirmed today: a SQLite event-log emitter, NOT a pub/sub bus.** No subscriber registry, no dispatch loop, no `scope_permits_claim_read` predicate.
- §5: W4-B emits structured `ClaimVersionEvent` / `CompositionVersionEvent` rows into the dedicated `version_events` table (per §15) **inside the same transaction as the version mutation** (outbox pattern). Delivery + scope-filter dispatch is **DOS-589's responsibility**, not W4-B's. W4-B owns the row schema + outbox guarantee; DOS-589 owns delivery.

### Already in `src-tauri/src/audit_log.rs` (W1-A0)

- **`emit_surface_audit(logger, event_kind, actor, fields) → Result<(), AuditError>`** — line 203. Enforces `wp_user_id` presence for `Actor::SurfaceClient`. V3 pins the `detail` shape per acceptance §30.

### Already in `src-tauri/src/bridges/types.rs` (W2)

- **`BridgeSurfaceError { AbilityUnavailable, Validation(String), Ownership(OwnershipError) }`** — line 249. V3 adds 5 new variants with precedence table per §6.5: `StaleVersion`, `StaleComposition`, `MidFlightMutation`, `ClaimVersionOverflow`, `MissingExpectedClaimVersion`.
- **`InvocationContext`** — line 133. V3 does NOT add version fields here; `expected_claim_version` carrier lands in the ability's typed input via `MutatingProposal` trait per §13.

### What W4-B authors net-new (V3)

| Surface | Status | Authoring scope |
|---|---|---|
| `claim_version INTEGER NOT NULL DEFAULT 0 CHECK (claim_version BETWEEN 0 AND i64::MAX)` column on `intelligence_claims` (v170) | Missing | Migration; backfill per §11 |
| `claim_version` assignment in `commit_claim` | Missing | Per accepted mutation: `max(prior) + 1` |
| `composition_versions` table (v170) with same CHECK constraint | Missing | Per §8 |
| `commit_composition(ctx, db, proposal: CompositionProposal) → Result<CommittedComposition, CompositionError>` | Missing | Bridge chokepoint; CAS + BEGIN IMMEDIATE |
| `MutatingProposal` trait (per §13) | Missing | New trait at `commit_claim` boundary; compile-time enforcement |
| `ClaimRef.field_path: Option<FieldPath>` field | Missing | New optional field |
| `Block.field_bindings: Vec<FieldBinding>` field with `Vec::new()` default constructor | Missing | New field; back-compat via default |
| `FieldBinding { field_path, role, claim_refs }` + `BindingRole` enum | Missing | Per §1 |
| 5 new `BridgeSurfaceError` variants with HTTP mapping + precedence per §6.5 | Missing | Per §2, §3, §4, §7, §10 |
| Scope-filtered `CorrectionPayload` projection | Missing | Per §2 |
| Scope-filtered `CorrectionRef` event-log lookup endpoint | Missing | Per §16 + ac §33 |
| Outbox-pattern signal emission in same DB transaction as version mutation | Missing | Per §15 |
| In-memory mutation lock as defense-in-depth (NOT correctness boundary) | Missing | Per §7 |
| `mutation_attempts` table (v170) | Missing | Per §7 three-Tx cursor protocol |
| `version_events` dedicated table (v170) with typed CHECKs | Missing | Per §15 outbox shape |
| `ClaimMutationTarget` enum + `MutatingProposal::target() -> &ClaimMutationTarget` | Missing | Per §13 V4 rewrite |
| `MutationGuard` Rust type with `Drop` impl for finalize-aborted | Missing | Per §7 |
| Startup recovery scan for stuck `mutation_attempts.status = 'in_flight'` > 30s old | Missing | Per §7 |
| `tests/version_assignment_gate.rs` integration test using `syn` crate | Missing | Per §29 (no fictional precedent claim) |
| `syn = { version = "2", features = ["full", "visit"] }` in `src-tauri/Cargo.toml` `[dev-dependencies]` | Missing | Per codex C3-4 |
| `dailyos doctor` claim/composition watermark consistency check | Missing | Per ac §32 (new CLI subcommand) |

## Directional decisions resolved at L0 (V3)

### §1. `field_path` on `ClaimRef` + `field_bindings: Vec<FieldBinding>` on `Block`

V3 pins both surfaces. They answer different questions:

- **`ClaimRef.field_path: Option<FieldPath>`** — "which field of which claim does this specific ClaimRef contribute to this block?" Required for 1:1 claim→field blocks.
- **`Block.field_bindings: Vec<FieldBinding>`** — "what is this block's full field topology, including derived/computed fields with multiple or no source claims?" Required for synthesized fields.

```rust
pub struct FieldBinding {
    pub field_path: FieldPath,
    pub role: BindingRole,
    pub claim_refs: Vec<ClaimRefIndex>,
}

pub enum BindingRole {
    Source,
    ComputedFrom,
    DisplayOnly,
    FeedbackTarget,
}
```

**Back-compat rule (devex side-note):** `Block::new()` adds `field_bindings: Vec::new()` as default value; existing W1-E call sites compile clean. The compile-time check that feedback-eligible blocks populate `field_bindings` lives in the W4-B-authored lint test (acceptance §15), not in the constructor.

`ClaimRef::with_field(claim_id, version, field_path)` is the new ergonomic constructor.

### §2. `409 stale_watermark` envelope — scope-filtered correction

```jsonc
// HTTP 409 Conflict (scope permits)
{
  "ok": false,
  "error": "stale_watermark",
  "claim_id": "<string>",
  "expected": { "claim_version": <u64> },
  "current":  { "claim_version": <u64> },
  "correction": {
    "claim": { /* IntelligenceClaim re-projected through Actor::SurfaceClient { scopes } */ },
    "scope_redacted": false,
    "retry_after_ms": <u32 | null>,
    "cursor": "<event-cursor-string>"
  }
}

// HTTP 409 Conflict (scope does not permit)
{
  "ok": false,
  "error": "stale_watermark",
  "claim_id": "<string>",
  "expected": { "claim_version": <u64> },
  "current":  { "claim_version": <u64> },
  "correction": {
    "claim": null,
    "scope_redacted": true,
    "reason": "out_of_scope",
    "retry_after_ms": <u32 | null>,
    "cursor": "<event-cursor-string>"
  }
}
```

Composition-level stale: `error: "stale_composition_watermark"`, no `correction.claim`. Inflated-version case (`expected > current`): identical envelope to stale, no correction payload, `event_kind: "inflated_version_rejected"` audit (per §6).

### §3. `composition_version` is bridge-assigned via `commit_composition()` chokepoint

DOS-567 acceptance: "Versions are never generated or trusted ... agent-side." V3 enforces:

```rust
pub fn commit_composition(
    ctx: &ServiceContext,
    db: &ActionDb,
    proposal: CompositionProposal,
) -> Result<CommittedComposition, CompositionError>;

pub struct CompositionProposal {
    pub composition_id: CompositionDocId,
    pub expected_composition_version: u64,    // CAS guard; 0 for first-ever under this ID
    pub composition: Composition,             // metadata.composition_version IGNORED on input
}

pub enum CompositionError {
    StaleVersion { expected: u64, current: u64 },
    Overflow { composition_id: CompositionDocId },
    AbilityFault(String),
}

pub struct CommittedComposition {
    pub composition_id: CompositionDocId,
    pub composition_version: u64,             // assigned by commit_composition; overwrites ability-supplied
    pub composition: Composition,
}
```

**Authority rule:** `composition_version` assigned by `commit_composition` using `checked_add(1)` against the `composition_versions` row under transactional CAS. Ability's supplied value is overwritten on output; non-zero supplied values logged at warning level.

**Idempotency rule:** `composition_version` advances on every accepted `commit_composition` call regardless of payload equality.

**Outbox rule:** `commit_composition` inserts the structured `CompositionVersionEvent` row into the dedicated `version_events` table (per §15) within the **same DB transaction** as the `composition_versions` UPDATE/INSERT. Either both persist or neither. Delivery to subscribers is DOS-589's responsibility. This closes codex Cycle 2 CRITICAL C2-1 (post-commit signal window).

**Concurrent producer rule:** two abilities concurrently calling `commit_composition` for same `composition_id` race through CAS under `BEGIN IMMEDIATE` (per §8); exactly one succeeds, other receives `StaleVersion`.

### §4. Overflow defense — typed variant + checked_add + i64::MAX domain + rollback

V3 fixes:
- `CompositionVersion::bump()` at composition.rs:122 changes from `saturating_add` to `checked_add` returning `Option<Self>`.
- `commit_claim` and `commit_composition` use `checked_add`; `None` returns typed `BridgeSurfaceError::ClaimVersionOverflow { claim_id }` (HTTP 500) or `CompositionError::Overflow { composition_id }`.
- **Persisted domain: `1..=i64::MAX` (V3 NEW per codex C2-4).** SQLite INTEGER is signed 64-bit; persisting `u64` values above `i64::MAX` fails at storage codec. V3 pins the domain at `i64::MAX = 9223372036854775807`. Overflow fixture (`dos567_fixture_overflow.rs`) tests `claim_version = i64::MAX` not `u64::MAX`. CHECK constraint on both `intelligence_claims.claim_version` and `composition_versions.composition_version`: `CHECK (claim_version BETWEEN 0 AND 9223372036854775807)`.
- **Transaction rollback:** overflow detection happens before DB write. No in-memory state mutates; rejection propagates without poisoning. Audit `event_kind: "claim_version_overflow"` / `composition_version_overflow`.
- Practical reachability: `i64::MAX` mutations per claim is ~9.2 × 10^18 — unreachable. Defense is correctness, not capacity.

### §5. Signal-event payload schema (W4-B substrate-side; DOS-589 delivery)

W4-B's scope: substrate emits structured `ClaimVersionEvent` / `CompositionVersionEvent` rows into the dedicated `version_events` table (per §15) inside the version-mutation transaction (outbox pattern). **Delivery, subscriber registry, scope-filter dispatch, `subscription.backpressure`, replay-from-cursor are all DOS-589's responsibility.**

```rust
pub struct ClaimVersionEvent {
    pub event_kind: ClaimEventKind,
    pub claim_id: String,
    pub previous_version: Option<u64>,
    pub current_version: u64,
    pub cursor: SignalCursor,
    pub reason: Option<String>,
    pub correction_ref: Option<CorrectionRef>,
}

pub enum ClaimEventKind {
    Updated, Corrected, Superseded, Tombstoned, WriteRejected, ConflictDetected,
}

pub struct CompositionVersionEvent {
    pub event_kind: CompositionEventKind,
    pub composition_id: CompositionDocId,
    pub previous_version: Option<u64>,
    pub current_version: u64,
    pub cursor: SignalCursor,
    pub reason: Option<String>,
}

pub struct CorrectionRef {
    pub event_log_id: String,
    pub claim_id: String,
    // Body fetch via GET /v1/surface/event-log/{event_log_id} — scope-filtered identically to §2 (per §16 + ac §33)
}
```

**409 envelope vs signal-bus channel disambiguation (devex D-N4):**
- 409 envelope (synchronous, 5xx-recovery path): consumed by the SurfaceClient that issued the stale write. Carries inline `correction.claim` body (scope-filtered).
- `claim.write_rejected` signal (asynchronous, push-via-DOS-589): consumed by other subscribers observing the rejection event. Carries `correction_ref` only; full body fetch is separate.
- **The two channels serve different consumers.** A single SurfaceClient does NOT consume both for the same rejection; the 409 is its synchronous response, and signal-bus subscribers are observers, not the writer.

### §6. Out-of-band edits — ordering with W4-C + `expected == substrate_current` rule

1. **W4-C signature check runs before W4-B 409 path** in the loopback endpoint. Tamper error does NOT emit `correction.claim`.
2. **Mutation input MUST present `expected == substrate_current`.** Greater-than rejected as `StaleVersion`, audit `event_kind: "inflated_version_rejected"`.

**Linear interlock edge:** DOS-569 (W4-C) inherits the signature-check-before-409 ordering as acceptance criterion.

### §6.5. `BridgeSurfaceError` variant precedence table (V3 NEW per devex D-N2)

When multiple rejection conditions could fire on a single request, the bridge evaluates in this order and returns the first matching variant:

| Precedence | Variant | HTTP | Trigger | Owned by |
|---|---|---|---|---|
| 0 | `ProjectionTampered` | 422 | W4-C signature verification fails on input projection envelope (Ed25519 mismatch, malformed sentinel, missing signature). Fires BEFORE expected-version logic; tamper error does NOT emit `correction.claim` payload. | W4-C (DOS-569) |
| 1 | `ProjectionVersionRollback` | 422 | W4-C currentness check: signature verifies but `signed_payload.composition_version < ledger.composition_version` (replayed older projection) OR `signed_payload.claim_version < ledger.claim_version`. Quarantine; no correction payload. | W4-C (DOS-569) |
| 2 | `MissingExpectedClaimVersion` | 400 | Input lacks `expected_claim_version` (via `MutatingProposal::expected_claim_version()` returning `None` or call site bypassing trait) | W4-B (this packet) |
| 3 | `MidFlightMutation` | 423 | `MutationLock` held on `claim_id`; rejection has lock-holder's mutation_id + cursor | W4-B |
| 4 | `ClaimVersionOverflow` / composition overflow | 500 | `checked_add(1)` returns `None`; or persistence would exceed `i64::MAX` | W4-B |
| 5 | `StaleVersion` (claim) | 409 | `expected != substrate_current_claim_version` (includes inflated case) | W4-B |
| 6 | `StaleComposition` | 409 | `expected != substrate_current_composition_version` | W4-B |

Read top-down; first match returns. AST CI gate (per §29) asserts the precedence is enforced at the bridge entry point via exhaustive match arms.

**V9 NEW (precedence 0 + 1 added):** `ProjectionTampered` and `ProjectionVersionRollback` are W4-C-owned variants but their precedence belongs in this table because they fire at the same bridge entry-point as the W4-B family. The runtime evaluates them FIRST so that a tampered or replay-rolled-back projection never reaches stale-watermark CAS evaluation (which would leak `correction.claim` body). W4-C's L0 packet implements the variant + ledger schema; this table coordinates the ordering.

### §7. `MidFlightMutation` is W4-B contract surface — lock is defense-in-depth, CAS is correctness, cursor reservation is committed BEFORE mutation transaction

V4 rewrites the cursor-durability mechanism per codex Cycle 3 HIGH C3-1. V3's "cursor inside mutation transaction" was internally contradictory — a transaction rollback erases the cursor row, leaving the 423 loser polling for an event that never appears.

- **Variant:** `BridgeSurfaceError::MidFlightMutation { claim_id, mutation_id, retry_after_event: SignalCursor }` → HTTP 423 Locked.
- **Locking primitive:** in-memory `HashMap<ClaimId, MutationLock>` keyed on `claim_id`, guarded by `tokio::sync::Mutex`.
- **Correctness vs defense-in-depth:** `claim_version` CAS in `commit_claim` is the correctness boundary; the in-memory lock is defense-in-depth. **On crash + retry, the worst-case observable is a `StaleVersion` rejection** (CAS catches the duplicate), not "duplicate accepted mutation."
- **No durable lock state.** Startup does NOT restore lock state. Correctness is in the CAS.
- **Multi-process safety: phase-deferred to v1.4.3+** when multi-process runtime lands. v1.4.2 single-process is canonical. Multi-process is NOT W4-C tamper territory; it's a concurrent-process correctness concern with documented 423→409 fallback (a second process bypasses the in-memory lock, hits the row CAS, receives 409 — same correctness, different code).

#### Cursor durability — three-transaction protocol (V4 NEW per codex C3-1)

The `retry_after_event` cursor is durable across all mutation outcomes (commit, rollback, panic) via a three-Tx protocol:

1. **Pre-mutation Tx (`reserve_mutation_attempt`)** — commits FIRST, before the lock is exposed via 423:
   - INSERT into `mutation_attempts (mutation_id, claim_id, cursor, started_at, status)` with `status = 'in_flight'`.
   - Cursor allocated here is **durable from the moment the Tx commits.**
   - 423 response carries `retry_after_event = cursor` from this committed row.

2. **Mutation Tx (`commit_claim` / `commit_composition`)** — does the actual version mutation:
   - Reads current version under `BEGIN IMMEDIATE`.
   - CAS-checks `expected_version`.
   - Writes `intelligence_claims.claim_version = N+1` (or `composition_versions.composition_version = N+1`).
   - INSERTs into `version_events` (per §15) referencing the **same cursor** from `mutation_attempts`.
   - UPDATEs `mutation_attempts.status = 'committed'`.
   - COMMITs.

3. **Post-rollback Tx (`finalize_mutation_attempt_aborted`)** — runs IF Tx 2 rolls back or panics:
   - Triggered by `Drop` impl on a `MutationGuard` value OR by startup recovery scan of `mutation_attempts WHERE status = 'in_flight' AND started_at < now() - 30s`.
   - UPDATEs `mutation_attempts.status = 'aborted'`.
   - INSERTs a terminal `mutation_aborted` event into `version_events` at the reserved cursor.
   - COMMITs.

**Loser polling guarantee:** the 423 loser sees the cursor in `mutation_attempts` immediately (Tx 1 already committed). When the winner finishes (commits OR aborts), Tx 2 or Tx 3 writes the terminal event at that cursor. Loser receives `claim.updated` (winner committed) OR `mutation_aborted` (winner failed) — never a missing event.

**`mutation_attempts` schema (v170):**

```sql
CREATE TABLE mutation_attempts (
    mutation_id TEXT PRIMARY KEY,
    claim_id TEXT NOT NULL,
    cursor TEXT NOT NULL UNIQUE,
    started_at TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('in_flight', 'committed', 'aborted')),
    finalized_at TEXT,
    CHECK ((status = 'in_flight' AND finalized_at IS NULL) OR (status != 'in_flight' AND finalized_at IS NOT NULL))
);
CREATE INDEX idx_mutation_attempts_in_flight ON mutation_attempts (started_at) WHERE status = 'in_flight';
```

**`MutationGuard` Rust type:** wraps the lock + reservation; `Drop` impl runs `finalize_mutation_attempt_aborted` on panic-unwind paths. Startup recovery scans for stuck `in_flight` attempts > 30 seconds old and marks them `aborted` (covers panic-abort and process-kill paths where Drop didn't run).

- **Audit:** `event_kind: "mid_flight_rejected"` (loser-side) + `event_kind: "mutation_aborted"` (winner-side post-rollback) with `AuditFields.detail` shape per ac §34.
- **W5-A consumer:** retry-after via cursor event (push from DOS-589); NOT a tight loop.

### §8. `composition_versions` table + BEGIN IMMEDIATE

Required by §3's bridge-assigned watermark contract.

```sql
CREATE TABLE composition_versions (
    composition_id TEXT PRIMARY KEY,
    composition_version INTEGER NOT NULL,
    generated_at TEXT NOT NULL,
    generated_by_invocation_id TEXT NOT NULL,
    generated_by_actor_kind TEXT NOT NULL,
    CHECK (composition_version BETWEEN 1 AND 9223372036854775807)
);
```

- **Durable**, slot v170.
- **Bootstrap on first encounter:** `commit_composition` INSERTs with `composition_version = 1` on first call for a new `composition_id`; subsequent calls UPDATE under CAS. Audit `event_kind: "composition_version_bootstrap"`.
- **Transaction isolation (V3 NEW per cso N5):** `commit_composition` opens transaction with **`BEGIN IMMEDIATE`** (writer lock at BEGIN), not deferred. Prevents TOCTOU on read-modify-write CAS sequence.

### §9. Naming collision — rename `current_claim_version_for_subject` → `current_subject_claim_version`

Existing 6 call sites (claims.rs:6272, 6676, 7523; db/invalidation_jobs.rs:352, 730; services/invalidation_jobs.rs:88) renamed. Mechanical sweep; W4-B's first commit.

### §10. `ClaimRef.claim_version: None` is bridge-rejected

- Mutation input with `claim_version: None` returns `BridgeSurfaceError::MissingExpectedClaimVersion { claim_id }` (HTTP 400) before reaching `commit_claim`.
- Producer-output serializer guard rejects `None` on `Source`/`FeedbackTarget` bindings.
- Fixture `dos567_fixture_missing_version.rs` exercises both.

### §11. Migration slot + backfill semantics

- **Slot reservation:** W4-B claims **v170** (v168 merged on dev as `migrate_v168_reconcile_missing_claim_shadow_trust_columns`, commit 75eda588; v169 reserved by v1.4.1 in-flight work). Recommended block: W4-B = v170; W4-C = v171–v174 (Ed25519 keys, projection ledger, quarantine); W4-E = v175 (nonce persistence) if persisted, otherwise no slot.
- **`intelligence_claims.claim_version` backfill:** existing rows initialize to `claim_version = 1`. Next `commit_claim` advances to 2.
- **Migration-origin audit event:** v170 emits one-shot audit `event_kind: "claim_version_backfill"`, `detail: { row_count, migration_version: 170 }`, `actor_kind: "system"`. Anchors audit hash chain.
- **Off-by-one fixture:** `dos567_fixture_backfill_off_by_one.rs` asserts fresh insert via `commit_claim` post-migration produces `claim_version = 1`; re-mutation → 2.

### §12. Signal event types — emit both `claim.write_rejected` AND `claim.conflict_detected`

V1/V2 unchanged in V3. Loser gets `claim.write_rejected`; concurrent-write winner gets BOTH `claim.updated` AND `claim.conflict_detected`. DOS-589 routes both with scope-filtered dispatch.

### §13. `MutatingProposal` trait with `ClaimMutationTarget` enum (V4 REWRITE per codex C3-2)

V3's trait shape required `expected_claim_version()` + `claim_id()`, which doesn't work for fresh inserts (current `ClaimProposal` has `id: Option<String>`; many call sites construct claims with `id: None`, e.g. `services/intelligence.rs:160`). V4 splits the cases via an enum:

```rust
pub enum ClaimMutationTarget {
    /// Fresh insert; no prior version to compare. claim_version assigned at 1 on commit.
    Insert {
        subject_ref: SubjectRef,
        claim_type: ClaimType,
        dedup_key: Option<String>,
    },
    /// Existing-claim mutation; CAS guard required.
    Mutate {
        claim_id: String,
        expected_claim_version: u64,
    },
}

pub trait MutatingProposal {
    fn target(&self) -> &ClaimMutationTarget;
}

// commit_claim signature changes:
pub fn commit_claim<P: MutatingProposal>(
    ctx: &ServiceContext,
    db: &ActionDb,
    proposal: P,
) -> Result<CommittedClaim, ClaimError>;
```

**Dispatch in `commit_claim`:**
- `Insert` variant: skip version CAS by design; INSERT new row with `claim_version = 1`. No `MissingExpectedClaimVersion` possible because there is no version to compare. **Concurrent Insert defense (V5 NEW per codex C4-2):** `Insert` MUST flow through the existing canonical commit/dedup-lock path at `services/claims.rs:5286-5314`. Duplicate detection (by `dedup_key` + subject + claim_type) runs inside the same `BEGIN IMMEDIATE` transaction as the row insert; collisions redirect to `CommittedClaim::Reinforced` (corroboration) rather than producing a duplicate `Inserted`. Two concurrent fresh-insert proposals with the same dedup identity produce exactly one `Inserted` + one `Reinforced` — never two duplicate rows. The existing substrate already enforces this; V4's `Insert` variant inherits the rule (does NOT introduce a parallel insert path that bypasses dedup).
- `Mutate` variant: run version CAS against `claim_id`'s current row. If `expected_claim_version != current.claim_version`, return `StaleVersion`. If row missing, return `Validation("claim_not_found")`.
- Bridge entry: if the caller's payload deserialization produces `ClaimMutationTarget::Mutate { expected_claim_version: 0, .. }`, that's a wire-shape violation (0 is reserved for pre-migration backfill rows, never a legitimate CAS value) — return `MissingExpectedClaimVersion`.

**Compile-time enforcement:** trait bound on `commit_claim<P: MutatingProposal>` is the gate. Mutating ability whose proposal type doesn't `impl MutatingProposal` fails the type check. `commit_claim` cannot be called without going through the trait.

**Migration of existing call sites (devex Cycle 3 verified ~20 sites):** `ClaimProposal` (claims.rs:68) gets `expected_claim_version: Option<u64>` field. `impl MutatingProposal for ClaimProposal` returns `ClaimMutationTarget::Mutate` when `id: Some(_)` AND `expected_claim_version: Some(_)`, else `ClaimMutationTarget::Insert`. All ~20 call sites (intel_queue.rs:4611/4784/4813/4863, bridges/mcp.rs:890, services/intelligence.rs:157, plus test sites) are updated as part of W4-B's first commit. Compile errors surface every site that needs `expected_claim_version` threaded through.

**Per cso Cycle 3 LOW (commit_claim visibility):** `MutatingProposal::target()` MUST return a value sourced from a substrate read-after-write boundary, not synthesized client-side. The bridge layer captures `substrate_current_claim_version` from the most recent successful read on that `claim_id` and re-emits it on the next mutation; client-side construction with `expected_claim_version: 0` is a contract violation per the rule above. Documented as a comment-doc rule on `MutatingProposal::target()`.

**Replaces V2's `#[ability]` macro fiction and V3's optional-version foot-gun.** The trait + enum IS the boundary.

### §14. Rate-limit retry policy in 409 envelope

`correction.retry_after_ms` populated when W2-D rate-limit budget below threshold (e.g. < 20% remaining). W5-A waits on cursor event from DOS-589 dispatcher; `retry_after_ms` is fallback upper bound. Prevents tight conflict-loop hitting W2-D 429s.

### §15. Outbox pattern + dedicated `version_events` table (V4 REWRITE per codex C3-3)

V3's outbox used the generic `signal_events` table (created in migration 018) directly. Codex Cycle 3 MEDIUM C3-3 + cso Cycle 3 path-α: the generic schema is TEXT-typed with no CHECK constraints on cursor format, event_kind enum, or scope_redacted bool. Substrate-correctness assumption needs DB-level enforcement on substrate-owned discriminants WITHOUT introducing caller-controlled-CHECK availability surfaces.

V4 introduces a **dedicated `version_events` table** for W4-B outbox rows; V5 adds `event_seq` AUTOINCREMENT (for deterministic replay ordering) and pins `cursor` as UUIDv4 (unguessable opaque ID):

```sql
CREATE TABLE version_events (
    event_seq INTEGER PRIMARY KEY AUTOINCREMENT,
    cursor TEXT NOT NULL UNIQUE CHECK (length(cursor) = 36 AND cursor GLOB '*-*-*-*-*'),
    event_kind TEXT NOT NULL CHECK (event_kind IN (
        'claim.updated', 'claim.corrected', 'claim.superseded', 'claim.tombstoned',
        'claim.write_rejected', 'claim.conflict_detected',
        'composition.updated', 'composition.write_rejected',
        'mutation_aborted'
    )),
    claim_id TEXT,                   -- NULL for composition events; TEXT NULL, no CHECK (caller-controlled)
    composition_id TEXT,             -- NULL for claim events; TEXT NULL, no CHECK (caller-controlled)
    previous_version INTEGER,        -- NULL for inserts and rejections
    current_version INTEGER NOT NULL,
    reason TEXT,                     -- caller-controlled; TEXT NULL, no CHECK (per cso path-α)
    scope_redacted INTEGER NOT NULL CHECK (scope_redacted IN (0, 1)),
    correction_event_log_id TEXT,    -- nullable pointer to durable correction body
    mutation_id TEXT,                -- nullable; populated for events tied to mutation_attempts row
    created_at TEXT NOT NULL,
    actor_kind TEXT NOT NULL CHECK (actor_kind IN ('user', 'agent', 'admin', 'system', 'surface_client')),
    CHECK ((claim_id IS NOT NULL) != (composition_id IS NOT NULL))   -- XOR: exactly one of the two
);
CREATE INDEX idx_version_events_claim ON version_events (claim_id, current_version);
CREATE INDEX idx_version_events_composition ON version_events (composition_id, current_version);
```

**Cursor generation rule:** every `cursor` is a fresh UUIDv4 (16 bytes random, 122 bits entropy) generated at `mutation_attempts` row reservation time (per §7 pre-mutation Tx). The format CHECK on `version_events.cursor` (length 36 + GLOB pattern matching `8-4-4-4-12` hex blocks) catches mistyped insertions; unguessability is enforced by construction, not by CHECK.

**Replay ordering (DOS-589):** consumers replay with `SELECT * FROM version_events WHERE event_seq > ? ORDER BY event_seq LIMIT N`. `event_seq` is monotonic per SQLite AUTOINCREMENT semantics; subscribers store last-seen `event_seq` (not last-seen `cursor`). Cursors are still useful for direct addressing (e.g., the 423 envelope's `retry_after_event` carries the cursor; the subscriber resolves cursor → event_seq via `SELECT event_seq FROM version_events WHERE cursor = ?`). This separation gives DOS-589 a stable replay order independent of cursor predictability.

**Outbox rule:** every `commit_claim` and `commit_composition` mutation runs as one transaction that writes both the version row AND the `version_events` row. Combined with the §7 three-Tx cursor protocol:

1. Pre-mutation Tx (committed first): INSERTs `mutation_attempts` row, allocates `cursor`.
2. Mutation Tx: reads current version (`BEGIN IMMEDIATE`), CAS, writes version row, INSERTs `version_events` row referencing the same `cursor`, UPDATEs `mutation_attempts.status = 'committed'`. COMMITs.
3. Post-rollback Tx (only if Tx 2 failed): UPDATEs `mutation_attempts.status = 'aborted'`, INSERTs `mutation_aborted` row into `version_events` at the same `cursor`.

**Schema-level constraint scope (per cso Cycle 3 path-α):**
- **Substrate-owned discriminants get CHECK:** `event_kind`, `scope_redacted`, `actor_kind`, claim/composition mutual-exclusion.
- **Caller-controlled fields stay TEXT NULL without CHECK:** `reason` (free-form), `claim_id`/`composition_id` (opaque identifiers). Caller payload cannot trigger a CHECK violation that aborts the mutation transaction.
- Substrate-owned values (event_kind, actor_kind) use Rust enum → canonical string mapping via Serde; the strings inserted into the DB are always one of the enumerated values by construction.

**Delivery (DOS-589):** dispatcher polls (or trigger-subscribes to) `version_events` after each Tx commit; W4-B's responsibility ends at the row insert.

**Why a dedicated table, not extending `signal_events`:**
- `signal_events` (018) is a generic event log used by many subsystems (ADR-0080 propagation). Adding CHECK constraints there could regress unrelated callers.
- `version_events` has watermark-specific shape (`previous_version`/`current_version`/`mutation_id`) and is read by DOS-589's W4-B-specific dispatcher path. Cleaner separation.
- The DOS-589 sibling-ticket dispatcher can union over both tables if cross-subsystem replay is later needed; for v1.4.2 the dispatcher only reads `version_events`.

**No "post-commit emit" race possible.** Replaces V2's "emit AFTER row persisted" and V3's "cursor inside mutation transaction" with the durable-cursor protocol from §7 + this dedicated table.

### §17. Class-level `wp_user_id` session binding — lifted to all SurfaceClient consumers (V8 NEW)

Per stage-2 Cycle 1 /cso review of W4-E (CRITICAL C1): every endpoint that receives a `wp_user_id` in the request body trusts that field directly, but a compromised WP admin role or buggy WP plugin can craft a signed request asserting any wp_user_id. HMAC + pairing scope check pass; the audit row then attests to a falsified human. Class shape — applies to all current and future SurfaceClient write paths.

**Class-level rule (inherited by DOS-568/569/570/571/572/573 + DOS-589):**

> Every endpoint that accepts a `wp_user_id` value (either in the request body, query string, header, or any other request channel) MUST verify it equals the wp_user_id bound to the SurfaceClient session at W2-C pairing time. Body-asserted values are NEVER trusted on their own. If a request's body wp_user_id does not match the session-bound value, the endpoint rejects with HTTP 403 `wrong_user` BEFORE running any further validation (signature check, scope check, nonce verify, claim read, etc.). The reject path emits an audit event with `event_kind: "wrong_user_rejected"` carrying the session-bound wp_user_id (the truthful value), the asserted wp_user_id, and the SurfaceClient instance.

**Implementation in W4-B (single point of enforcement):**

The `SurfaceClientBridge` (W2-D's bridge surface) gains a precondition check at request entry: when an authenticated SurfaceClient request payload contains a `wp_user_id` field at any depth, the bridge runs `validate_session_bound_wp_user_id(actor, payload)` BEFORE dispatching to the ability or service layer. The helper traverses the payload (serde-driven), collects every `wp_user_id` occurrence, and compares each to `actor.session.wp_user_id`. Any mismatch → 403 + audit, no further dispatch.

This makes the rule structurally enforced rather than per-endpoint asserted. Consumers (W4-E, W5-A, etc.) do not need to re-implement the check; the bridge fail-closes for them.

**Linear comments posted to DOS-568/569/570/571/572/573/589** lift this rule as inherited acceptance.

### §16. Class-level scope-filter rule — lifted to DOS-569 and DOS-573 (V3 NEW)

Per Cycle 2 /cso class-pattern observation (scope-leak fired V1 C1 + V2 N1 + V2 N2 = three times), V3 lifts a class-level acceptance into DOS-569 (W4-C) and DOS-573 (W5-A) via separate Linear comments:

> **Class-level acceptance criterion (W4-C, W5-A, and any future claim-bound consumer):** every read endpoint that returns claim-bound content must route through the same `Actor::SurfaceClient { scopes }` projection pipeline. Direct-key fetches by `event_log_id`, `mutation_id`, `correction_id`, `projection_id`, `composition_id`, `claim_id`, or any other bearer pointer are equally scope-gated. Out-of-scope requesters receive a redacted envelope or 404; never a leaked claim body.

This is structural rather than a per-finding patch. Future direct-key fetch endpoints introduced in v1.4.3+ inherit the rule.

## Acceptance criteria lifted into DOS-567 (V4)

### Implementation (substrate-side)

1. **Migration v170** is a single migration that creates three new schema surfaces:
   (a) adds `claim_version INTEGER NOT NULL DEFAULT 0 CHECK (claim_version BETWEEN 0 AND 9223372036854775807)` to `intelligence_claims`; existing rows initialized to `claim_version = 1`. One-shot audit `claim_version_backfill` per §11.
   (b) creates `composition_versions` table per §8 with matching CHECK constraint.
   (c) creates `mutation_attempts` table per §7 (`mutation_id PK, claim_id, cursor UNIQUE, started_at, status CHECK IN ('in_flight','committed','aborted'), finalized_at` + status/finalized_at consistency CHECK + partial index on in-flight attempts).
   (d) creates `version_events` table per §15 with typed columns + CHECK constraints on substrate-owned discriminants (`event_kind`, `scope_redacted`, `actor_kind`); TEXT NULL for caller-controlled fields (`reason`, `claim_id`, `composition_id`).
2. **`claim_version` assigned exclusively inside `commit_claim`.** `tests/version_assignment_gate.rs` integration test (using `syn` per ac §42) enforces.
3. **`composition_version` assigned exclusively inside `commit_composition`.** Ability-supplied value overwritten on output.
4. **`commit_composition` operates under `BEGIN IMMEDIATE` transactional CAS** against `composition_versions` row. Two concurrent abilities for same `composition_id` → one success, one `StaleVersion`. Bootstrap inserts `composition_version = 1`.
5. **`ClaimRef.field_path: Option<FieldPath>` field added; `Block.field_bindings: Vec<FieldBinding>` field added with `Vec::new()` default; `FieldBinding`/`BindingRole` types added.** W1-E serialization round-trip preserved.
6. **`BridgeSurfaceError` extended** with 5 new variants per §6.5 precedence table. HTTP mappings: 409, 409, 423, 500, 400.
7. **`CorrectionPayload` is scope-filtered** through `Actor::SurfaceClient { scopes }`. Out-of-scope returns `scope_redacted: true` with no claim body.
8. **W4-C signature check runs before W4-B 409 path** in loopback endpoint. Tamper error does NOT emit `correction.claim`.
9. **Mutation input requires `expected_claim_version == substrate_current`.** Greater-than rejected as `StaleVersion` with no correction payload (inflated-version defense).
10. **`MutatingProposal` trait + `ClaimMutationTarget` enum** per §13. `commit_claim<P: MutatingProposal>` accepts both `Insert` (fresh; skip version CAS; assign `claim_version = 1`) and `Mutate { claim_id, expected_claim_version }` (CAS required) variants. Compile-time enforcement via trait bound; no mutating ability can call `commit_claim` without going through the trait.
11. **`ClaimRef.claim_version: None` in mutation proposal** triggers `MissingExpectedClaimVersion` (only when the proposal's `target()` is `Mutate { .. }` per §13 — `Insert` variant skips this check by design). Producer-output serializer guard rejects on `Source`/`FeedbackTarget` bindings regardless of variant.
12. **In-memory mutation lock** (`HashMap<ClaimId, tokio::sync::Mutex>`) as defense-in-depth, NOT correctness boundary (§7). CAS is correctness. No durable lock state.
13. **`commit_claim` + `commit_composition` follow three-Tx outbox protocol (§7 + §15):** (1) pre-mutation Tx commits `mutation_attempts` row with cursor; (2) mutation Tx writes version row + `version_events` row + UPDATEs `mutation_attempts.status='committed'`, all atomic; (3) on rollback/panic, post-rollback Tx writes `mutation_aborted` event + UPDATEs `status='aborted'`. Cursor is durable from end of Tx 1; 423 responses always reference a cursor that resolves.
14. **Bridge emits `claim.write_rejected` + audit on rejected stale write.** Detail shape per ac §34.
15. **Bridge emits `claim.conflict_detected`** to winner's audit channel on concurrent-write race.
16. **Rename `current_claim_version_for_subject` → `current_subject_claim_version`** across all 6 call sites.
17. **`ClaimRef::with_field(claim_id, version, field_path)` constructor added** alongside `with_version`. Lint test (per ac §33 part b) asserts every Block with feedback-eligible BlockType has every `Source`/`FeedbackTarget` binding's `ClaimRef.field_path = Some(_)`.
18. **`CompositionVersion::bump()` rewritten** from `saturating_add` to `checked_add → Option<Self>`. Persisted domain `1..=i64::MAX` per §4.
19. **`MutationGuard` Rust type with `Drop` impl** (per §7): wraps the in-memory `MutationLock` + `mutation_attempts` reservation; on panic-unwind, `Drop` runs `finalize_mutation_attempt_aborted` Tx 3. Startup recovery scans `mutation_attempts WHERE status='in_flight' AND started_at < now() - 30s` and marks them aborted (covers process-kill paths where `Drop` didn't run).

### Negative fixtures (`src-tauri/tests/dos567_fixture_<N>_<name>.rs`)

20. **`dos567_fixture_1_concurrent_writes.rs`** — two SurfaceClients writing same claim at v=5; one succeeds with v=6; loser receives 409 + scope-filtered correction; audit emits `claim.write_rejected` for loser + `claim.conflict_detected` for winner.
21. **`dos567_fixture_2_reconnect_replay.rs`** — simulated reconnect via DOS-589 dispatcher (mock or fake); cursor replay `a→b→a` in order.
22. **`dos567_fixture_3_stale_mcp_write.rs`** — MCP at v=8 against current v=10; rejected; scope-filtered correction.
23. **`dos567_fixture_4_runtime_correction_during_editor_draft.rs`** — runtime v=6 arrives while editor holds v=5 draft; save rejected; draft preservation is W5-A scope.
24. **`dos567_fixture_5_mid_flight_mutation.rs`** — concurrent invoke while lock held; HTTP 423 `MidFlightMutation`; `retry_after_event` cursor durably committed to `mutation_attempts` BEFORE 423 is emitted; loser observes either `claim.updated` (winner committed) or `mutation_aborted` (winner failed) at the cursor — never a missing event.
25. **`dos567_fixture_overflow.rs`** — simulate `claim_version = i64::MAX`; attempt commit; `ClaimVersionOverflow { claim_id }`; audit `claim_version_overflow`; no state poisoning.
26. **`dos567_fixture_missing_version.rs`** — `ClaimRef { claim_version: None }` on `Source` binding; mutation input rejected with `MissingExpectedClaimVersion`; producer output rejected at serialization.
27. **`dos567_fixture_scope_leak.rs`** — SurfaceClient with `read.account_overview` submits stale write against tighter-scope claim; 409 `scope_redacted: true`, no claim body leaks.
28. **`dos567_fixture_inflated_version.rs`** — SurfaceClient presents `expected > current`; rejected `StaleVersion`, no correction; audit `inflated_version_rejected`.
29. **`dos567_fixture_backfill_off_by_one.rs`** — fresh post-migration insert produces `claim_version = 1`; re-mutation advances to 2.
30. **`dos567_fixture_lock_state.rs`** — `commit_claim` panics mid-flight via injected fault; lock released on rollback; no permanent deadlock.
31. **`dos567_fixture_outbox_atomicity.rs`** — version row write succeeds but `version_events` insert fails via injected fault → mutation Tx rolls back; no version advancement observed; cursor in `mutation_attempts` is then resolved by post-rollback Tx 3 emitting `mutation_aborted` event.
32. **`dos567_fixture_correction_ref_scope_leak.rs`** (V3 NEW per §16 + ac §37) — out-of-scope fetch by `event_log_id` returns redacted envelope; no claim body leaks. Asserts the substrate-side endpoint, not the DOS-589 dispatcher (which DOS-589 owns).
32a. **`dos567_fixture_cursor_durability_panic.rs`** (V4 NEW per codex C3-1) — winner acquires lock, commits pre-mutation Tx 1 (cursor reserved), then mutation Tx 2 panics via injected fault. Assertions: (i) `mutation_attempts.status = 'aborted'` after Drop/recovery; (ii) `version_events` row at the reserved cursor has `event_kind = 'mutation_aborted'`; (iii) a loser that received 423 with that cursor can deterministically observe the terminal event. Covers Drop path AND startup-recovery path (process-kill simulation).
32b. **`dos567_fixture_fresh_insert_via_target.rs`** (V4 NEW per codex C3-2) — proposal with `ClaimMutationTarget::Insert { .. }` flows through `commit_claim` without version CAS; produces row with `claim_version = 1`; emits `claim.updated` with `previous_version: None`. Negative companion: proposal with `Mutate { expected_claim_version: 0, .. }` is rejected as `MissingExpectedClaimVersion` (0 is reserved for backfill, never legitimate CAS).
32c. **`dos567_fixture_insert_dedup_race.rs`** (V5 NEW per codex C4-2) — two `tokio::test` spawned tasks submit `ClaimMutationTarget::Insert` proposals with the same `(subject_ref, claim_type, dedup_key)` triple concurrently. Assertions: (i) exactly one produces `CommittedClaim::Inserted`; (ii) exactly one produces `CommittedClaim::Reinforced` (corroboration on dedup-key collision); (iii) zero duplicate rows in `intelligence_claims`; (iv) both emit `version_events` rows with distinct `event_seq` and `cursor` values. Exercises the canonical commit/dedup-lock path at `claims.rs:5286-5314` under V5 §13's Insert variant.

### CI invariants

33. **`tests/version_assignment_gate.rs`** (V3 — replaces `src-tauri/scripts/check_version_assignments.rs` fiction): integration test using `syn` crate (already a transitive dep via `serde_derive`). Walks every `.rs` file under `src-tauri/src/` and `src-tauri/abilities-runtime/src/`; fails the build if any assignment expression to a field named `claim_version` or `composition_version` exists outside the allowlist (`services/claims.rs` for `claim_version`; `commit_composition` module for `composition_version`). Lives in `tests/` not `scripts/`. **No fictional "v1.4.0 precedent" claim — this is the first AST-based CI gate in the substrate.**
34. **`AuditFields.detail` schema pin** — every W4-B audit record carries:

```jsonc
{
  "claim_id": "<string | null>",
  "composition_id": "<string | null>",
  "expected_version": <u64 | null>,
  "current_version": <u64 | null>,
  "rejection_reason": "stale_watermark" | "stale_composition_watermark" | "mid_flight_mutation" | "missing_expected_claim_version" | "inflated_version_rejected" | "claim_version_overflow" | "composition_version_overflow" | "claim_version_backfill" | "composition_version_bootstrap",
  "ability_ref": "<string | null>",
  "invocation_id": "<uuid | null>",
  "scope_redacted": <bool>
}
```
`wp_user_id` + `actor_scopes` via `emit_surface_audit`. No PII in `detail`.

35. `cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green.

36. **`dailyos doctor` claim/composition watermark consistency** — new diagnostic subcommand:
    - Walks `intelligence_claims`, asserts every `claim_version >= 1`.
    - Walks `composition_versions`, asserts every `composition_version >= 1`.
    - Walks `mutation_attempts WHERE status = 'in_flight'`, flags entries older than 60s as zombie attempts (startup recovery should have aborted them).
    - Verifies outbox integrity: every `intelligence_claims.claim_version` change has a corresponding `version_events` row with matching `cursor` and an entry in `mutation_attempts` with `status IN ('committed', 'aborted')`.

### V3 NEW acceptance items

37. **`CorrectionRef` event-log fetch endpoint** at `GET /v1/surface/event-log/{event_log_id}` is scope-filtered identically to §2's inline 409 projection. Substrate-side endpoint owned by W4-B; DOS-589's dispatcher routes via this endpoint. Out-of-scope requesters get redacted envelope or 404. Fixture §32 asserts. **Endpoint owner module (V8 promoted from path-α):** `src-tauri/src/bridges/surface_client.rs` is the canonical SurfaceClient route module. All `/v1/surface/*` endpoints (this one, W4-C keyring at `/v1/surface/keyring`, W4-E nonce at `/v1/surface/nonce/{issue,verify}`, W5-A feedback at `/v1/surface/feedback`, DOS-589 dispatcher routes) land in this module. W4-B's first commit creates the module skeleton + the `validate_session_bound_wp_user_id` precondition helper (per §17); downstream waves register their routes inside it.

38. **`BridgeSurfaceError` variant precedence table (§6.5)** enforced at bridge entry point via top-down evaluation (NOT a single `match` on a discriminant — the variants represent independent conditions that may all be true; evaluation walks them in precedence order and returns the first match). Integration test (`tests/dos567_fixture_variant_precedence.rs`) exercises each pairwise precedence case (e.g., `expected: None` + mid-flight → 400, not 423).

### V4 NEW acceptance items

39. **`mutation_attempts` table + three-Tx cursor protocol per §7.** Pre-mutation Tx commits cursor reservation BEFORE lock exposure. Mutation Tx attaches event to reserved cursor. Post-rollback Tx 3 (driven by `Drop` impl OR startup recovery scan for stuck `in_flight` > 30s) emits `mutation_aborted` at the reserved cursor. Cursor durability is independent of mutation outcome.

40. **`ClaimMutationTarget::{Insert, Mutate}` enum** is the contract surface of `MutatingProposal::target()`. `Insert` variant: skip version CAS, INSERT new row with `claim_version = 1`. `Mutate` variant: run CAS against `claim_id` row; `expected_claim_version = 0` is reserved for backfill and rejected as `MissingExpectedClaimVersion` if presented by a caller.

41. **`version_events` dedicated outbox table per §15** with typed CHECK constraints ONLY on substrate-owned discriminants (`event_kind`, `scope_redacted`, `actor_kind`) and TEXT NULL for caller-controlled fields (`reason`, `claim_id`, `composition_id`). Substrate-owned values are inserted via Rust enum → canonical string mapping (Serde); caller-controlled values cannot trigger a CHECK violation that aborts the mutation transaction (closes cso Cycle 3 path-α guard).

42. **`syn` dev-dependency** added to `src-tauri/Cargo.toml` `[dev-dependencies]`:
```toml
syn = { version = "2", features = ["full", "visit"] }
```
Required for ac §33's `tests/version_assignment_gate.rs` integration test to import `syn` directly (transitive dep via `serde_derive` is not callable from `tests/`).

### V8 NEW acceptance items

43. **`bridges/surface_client.rs` module + `validate_session_bound_wp_user_id` precondition (§17)** — W4-B's first commit creates `src-tauri/src/bridges/surface_client.rs` with the wp_user_id session-binding precondition helper. Helper traverses any request payload (serde-driven), collects all `wp_user_id` occurrences at any depth, compares each to `Actor::SurfaceClient.session.wp_user_id`. Mismatch → 403 `wrong_user` rejection + audit emission with `event_kind: "wrong_user_rejected"` before any further dispatch. CI invariant: grep gate fails any `/v1/surface/*` route handler that does not run through `validate_session_bound_wp_user_id` at entry.

44. **Fixture `dos567_fixture_wrong_user_rejected.rs`** — paired SurfaceClient session bound to `wp_user_id = 100`. Submit signed request with body asserting `wp_user_id = 200`. Assertions: (i) endpoint returns HTTP 403 `wrong_user`; (ii) no claim read, no nonce issue, no scope check runs; (iii) audit row carries session-bound `wp_user_id = 100`, asserted `wp_user_id = 200`, surface_client_id; (iv) no audit row carries `wp_user_id = 200` (the forged value never enters the audit chain).

### V9 NEW acceptance items

45. **Fixture `dos567_fixture_precedence_tamper_over_stale.rs`** (V9 per W4-C coordination) — request carries tampered projection envelope (Ed25519 mismatch) AND stale `expected_claim_version`. Assertion: bridge returns `ProjectionTampered` (422), NOT `StaleVersion` (409). Audit emits `projection.tamper_detected`; no `claim.write_rejected`; no `correction.claim` in response body.

46. **Fixture `dos567_fixture_precedence_rollback_over_stale.rs`** (V9 per W4-C coordination) — request carries valid signature but `signed_payload.composition_version = 5` while `ledger.composition_version = 12` (replayed older projection). Assertion: bridge returns `ProjectionVersionRollback` (422), NOT `StaleComposition` (409). Audit emits `projection.version_rollback_detected`; no `correction.claim`.

## Interlocks with W4 stage-2 + downstream

| Consumer | What it needs from W4-B (V3 contract) | Status after W4-B merge |
|---|---|---|
| **DOS-589 (signal dispatcher sibling)** | `version_events` rows (with `event_seq` AUTOINCREMENT + UUIDv4 `cursor`) written via outbox pattern; `ClaimVersionEvent` / `CompositionVersionEvent` payload schemas defined; replay clause `WHERE event_seq > ? ORDER BY event_seq`; cursor→event_seq lookup; `CorrectionRef` event-log endpoint live | All surfaces ready; DOS-589 implements pub/sub on top |
| **W4-A0 (DOS-568, producer)** | `ClaimRef::with_field()`; `commit_composition()` API; ability proposal carries `expected_composition_version`; `MutatingProposal` trait for mutating producers | All surfaces stable |
| **W4-C (DOS-569, tamper)** | `(claim_id, claim_version, composition_id, composition_version)` quadruple; outbox events as cache-bust triggers; **signature-check-before-409 ordering acceptance**; **class-level scope-filter rule per §16** | W4-C signs against watermark; ordering + class-rule lifted into DOS-569 acceptance |
| **W4-D (DOS-570, fallback)** | `Block.field_bindings` for derived/computed; `ClaimRef.field_path`; `BindingRole` drives admitted-field rules; **V8: canonical public API is `project_composition_for_surface(composition, ctx) → ProjectedComposition`** (composition-level, enforces unknown-block cap). Block-level helpers are internal. W4-A consumes only the composition-level surface. | Surfaces landed |
| **W4-E (DOS-571, nonce)** | `composition_version` in nonce tuple; composition refresh invalidates nonces | Tuple ready; rule explicit |
| **W4-A (DOS-572, renderer)** | Trust-band rendering on stale projection; 409 handling with scope-redacted fallback; `field_bindings` for layout | Renderer reads W4-C ledger; envelope locked |
| **W5-A (DOS-573, feedback)** | `(claim_id, claim_version, field_path)` triple; refuse-to-route on `ComputedFrom`/`DisplayOnly`; 409 sync + DOS-589 push channel disambiguation; presence-nonce from W4-E; **class-level scope-filter rule per §16** | All surfaces ready |

## What W4-B explicitly does NOT own (V3)

- **Signal subscriber + scope-filter dispatcher + replay-from-cursor + backpressure** — DOS-589 sibling.
- **Projection signing / Ed25519 / key lifecycle** — W4-C (DOS-569).
- **Fallback projection rules for unknown blocks** — W4-D (DOS-570).
- **User-presence nonce issue/verify** — W4-E (DOS-571).
- **Gutenberg block rendering or save handler** — W4-A (DOS-572).
- **WP-side feedback router** — W5-A (DOS-573).
- **Out-of-band edit detection or quarantine** — W4-C.
- **Multi-process runtime safety** — phase-deferred to v1.4.3+ when multi-process runtime lands (§7). Not W4-C tamper territory.
- **Tight retry loops on 409/423** — W5-A consumes cursor + retry-after surfaces from DOS-589.
- **WebSocket vs SSE transport for event push** — DOS-589 + deferred per artifact 02.
- **Composition snapshot materialization in post meta** — deferred per artifact 02.
- **Conflict panel rendering** — deferred per artifact 02.

## Open questions

All closed in V2/V3:

| ID | Resolution |
|---|---|
| Q1 (migration slot) | v170 (V7 corrected from V6 v169 — v1.4.1 in-flight is using v169; v168 already merged on dev) |
| Q2 (mid_flight_mutation scope) | W4-B owns variant + 423 + lock + cursor pre-allocation per §7 |
| Q3 (conflict_detected vs write_rejected) | Emit both per §12 |
| Q4 (backfill claim_version=1 vs 0) | 1 with one-shot audit per §11 |
| Q5 (composition_versions table) | Required per §3+§8; durable; BEGIN IMMEDIATE |

## Linear dependency edges

- DOS-567 (W4-B) blocked by DOS-551 (W1-A) ✓ done.
- DOS-567 (W4-B) blocked by DOS-556 (W1-E) ✓ done.
- DOS-567 (W4-B) blocks DOS-568 (W4-A0).
- DOS-567 (W4-B) blocks DOS-569 (W4-C). **Signature-check-before-409 ordering + class-level scope-filter rule per §16 inherited as acceptance.**
- DOS-567 (W4-B) blocks DOS-570 (W4-D).
- DOS-567 (W4-B) blocks DOS-571 (W4-E).
- DOS-567 (W4-B) blocks DOS-572 (W4-A) — transitively.
- DOS-567 (W4-B) blocks DOS-573 (W5-A). **Class-level scope-filter rule per §16 inherited.**
- **DOS-567 (W4-B) blocks DOS-589** (signal dispatcher sibling — substrate row schema + outbox pattern must land first).
- **DOS-589 blocks DOS-569 + DOS-572 + DOS-573** (delivery for signal consumers).

## Path-α maintenance filings

- **Multi-process framing (§7).** Filed to maintenance project (id `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`) per eng P2-NEW-1: rewrite §7 framing of multi-process safety from "W4-C tamper" to "phase-deferred concurrency to v1.4.3+ multi-process runtime work."

## L0 reviewer panel — required runners

- `/plan-eng-review`, `/cso`, `/codex challenge`, `/plan-devex-review`. Unanimous APPROVE for L0 closure.

## Acceptance for L0 closure

This packet is L0-approved when:

1. eng + cso + devex + codex unanimous APPROVE on V4 (or successor cycle). Cycle 3 panel: eng + cso + devex APPROVE; codex BLOCK with 4 findings all V4-folded.
2. DOS-567 issue description updated to reference this packet by URL for lifted criteria.
3. Linear dependency edges added (especially DOS-569 + DOS-573 inheriting §16; DOS-589 substrate dependency).
4. Class-level scope-filter rule comments posted on DOS-569 + DOS-573.
5. Migration slot v170 confirmed.
6. Reviewer findings folded into successive packet versions and dated.

W4-B implementation may start as soon as L0 closes. Stage-2 fan-out (W4-A0 / W4-C / W4-D / W4-E) starts the moment W4-B merges. DOS-589 implementation may start as soon as W4-B substrate schema lands (row format + outbox guarantee).
