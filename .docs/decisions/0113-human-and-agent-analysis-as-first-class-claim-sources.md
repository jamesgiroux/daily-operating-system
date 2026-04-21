# ADR-0113: Human and Agent Analysis as First-Class Claim Sources

**Status:** Proposed
**Date:** 2026-04-19
**Target:** v1.4.0 substrate (actor taxonomy, propose/commit data model) / v1.5.0 implementation (Analysis Inbox, agent trust ledger, active surfaces)
**Extends:** [ADR-0102](0102-abilities-as-runtime-contract.md), [ADR-0103](0103-maintenance-ability-safety-constraints.md), [ADR-0104](0104-execution-mode-and-mode-aware-services.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0107](0107-source-taxonomy-alignment.md)
**Related:** [ADR-0114](0114-scoring-unification.md), [ADR-0115](0115-signal-granularity-audit.md), [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) (Gap D)
**Consumed by:** [DOS-7](https://linear.app/a8c/issue/DOS-7) (`intelligence_claims.actor` column, tombstones), [DOS-241](https://linear.app/a8c/issue/DOS-241) (enrichment refactor design)

## Context

DailyOS's synthesis layer produces claims about entities — "Alice is the economic buyer at Acme," "renewal risk is elevated," "champion has been silent for 14 days." Today those claims are written by enrichment pipelines and a handful of abilities. Three things are about to happen that reshape this:

1. [ADR-0102](0102-abilities-as-runtime-contract.md) makes every capability a typed, versioned ability. Many of those are Transform abilities that produce claims. The product will ship with multiple ability authors — `prepare_meeting`, `detect_risk_shift`, `generate_weekly_narrative`, future MCP agents — each asserting claims into the same substrate.
2. [DOS-7](https://linear.app/a8c/issue/DOS-7) elevates `intelligence_claims` to a first-class table with per-claim identity and append-only assertion history.
3. The v1.4.1 enrichment refactor ([DOS-241](https://linear.app/a8c/issue/DOS-241)) explicitly asks: "what should enrichment look like when claims have provenance, trust scores, and a propose/commit boundary?"

Underneath these three is a harder question the substrate as drafted does not answer: **who is allowed to commit a claim, and when does a claim become load-bearing versus merely under consideration?** Today the answer is implicit: whoever writes it. That works when DailyOS is one engineer's enrichment pipeline. It stops working the moment an LLM-backed agent tries to assert "the champion is departing" based on a faint signal, and the briefing tomorrow morning treats that as ground truth.

Separately, humans produce claims too. When the user marks a champion as at-risk, or confirms a renewal date, or removes a stakeholder from a list — those are assertions about the world that the system needs to treat with at least as much weight as an AI's. Today the data model barely distinguishes human edits from AI outputs. The **ghost-resurrection bug** documented in [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap D — user removes a role type, enrichment silently repopulates it — is one specific manifestation of this missing distinction.

This ADR specifies the actor taxonomy, the propose/commit boundary, the contradiction-handling rules, the agent trust ledger, and the deduplication policy that collectively make humans and agents first-class, equal-structured claim sources. The substrate pieces needed in v1.4.0 are limited: an `actor` column and a `claim_state` column on `intelligence_claims`, plus the propose/commit state machine. The user-facing surfaces (Analysis Inbox, agent trust dashboard, contradiction resolution UI) ship in v1.5.0+ and are out of scope for v1.4.0.

## Decision

### 1. Actor taxonomy

Every claim in `intelligence_claims` carries an `actor` string with one of the following canonical shapes. The `actor` column is non-nullable. Provenance's `Actor` type ([ADR-0105](0105-provenance-as-first-class-output.md) §1) uses the same shape.

| Actor prefix | Shape | Examples | Authority |
|---|---|---|---|
| `user` | `user` | `user` | Human user of DailyOS taking direct UI action |
| `user_removal` | `user_removal` | `user_removal` | User removed a value (tombstone) |
| `human:<role>:<id>` | `human:cs:james`, `human:analyst:ana` | `human:cs:james@a8c.com` | Human analyst acting through a review surface, may not be the primary user |
| `agent:<name>:<version>` | `agent:prepare_meeting:1.3` | `agent:detect_risk_shift:2.1` | LLM-backed or mixed-mode DailyOS ability producing claims |
| `system:<component>` | `system:scheduler`, `system:signal_propagator` | | Deterministic system writes (no LLM, no user input) |
| `external:<source>` | `external:salesforce`, `external:glean` | | Mirrored from an external system of record; the external system is the authoritative asserter |

- The `:<version>` suffix on `agent:` is mandatory and matches `ability_version` from provenance ([ADR-0102](0102-abilities-as-runtime-contract.md) §8).
- The `:<id>` suffix on `human:` is a stable user identifier (email in single-tenant, user UUID in multi-tenant per [ADR-0116](0116-tenant-control-plane-boundary.md)).
- An actor value is append-only to the catalog; once shipped, its shape does not change.
- Parsing is strict: any claim with an unparseable actor is rejected at write time.

### 2. Claim state machine

Every claim carries a `claim_state` column with one of four values:

| State | Meaning | Read semantics |
|---|---|---|
| `proposed` | Claim exists but is not authoritative. | Excluded from default reads; visible in the Analysis Inbox surface. |
| `committed` | Claim is authoritative. Live in briefings, MCP, chat. | Included in default reads. |
| `superseded` | Claim was committed but has been replaced by a later claim at the same `field_path`. | Included in history reads; excluded from default reads. |
| `retracted` | Claim was withdrawn (tombstone, source revocation, contradiction resolution). | Included in history; tombstones consulted by writers; otherwise excluded from default reads. |

Default reads use `claim_state = 'committed'` plus the `superseded_at IS NULL` filter from [DOS-7](https://linear.app/a8c/issue/DOS-7). History reads drop both filters.

### 3. Commit policy

Commit policy is per-actor-class, declared in config, and applied at claim-write time:

```toml
[commit_policy.user]
strategy = "immediate"                 # user direct actions commit immediately

[commit_policy.user_removal]
strategy = "immediate"                 # tombstones commit immediately

[commit_policy.human]
strategy = "immediate"                 # human analysts are trusted

[commit_policy.system]
strategy = "immediate"                 # deterministic writes commit immediately

[commit_policy.external]
strategy = "immediate"                 # external source of record is authoritative

[commit_policy.agent]
strategy = "gated"                     # agents propose; gate decides
gate = "trust_and_corroboration"
auto_commit_trust_threshold = 0.80
corroboration_threshold = 2            # 2 independent sources agreeing
fallback = "analysis_inbox"            # falls to review if gate fails
```

The `trust_and_corroboration` gate commits an agent-authored claim if **either** condition holds:

- The claim's computed trust score ([ADR-0114](0114-scoring-unification.md)) ≥ `auto_commit_trust_threshold`, **and** the authoring agent's trust ledger (§6) is above its own configured floor.
- The claim has `corroboration_threshold` or more independent sources producing identical or compatible assertions at the same `field_path` within the corroboration window (default 7 days).

Claims that do not pass the gate transition to `proposed` and become visible in the Analysis Inbox (v1.5.0 surface). They do not appear in briefings, MCP responses, or any default-read surface.

### 4. Propose/commit mechanics

Claim insertion is split across two ability categories:

- **Transform abilities** produce `AbilityOutput<ClaimProposal>` — synthesized assertions with full provenance, but no side effect on `intelligence_claims`.
- **Maintenance abilities** consume `ClaimProposal` + `AbilityContext` and apply the commit policy. The write itself happens inside a service function (`services/claims.rs::commit_claim` or `::propose_claim`) that honors [ADR-0104](0104-execution-mode-and-mode-aware-services.md) mode gates and [ADR-0103](0103-maintenance-ability-safety-constraints.md) safety constraints.

This matches [ADR-0102](0102-abilities-as-runtime-contract.md) §3's call-graph distinction — Transform abilities must not mutate. The propose/commit split is the structural enforcement of that boundary for claim production.

Legacy path: where current enrichment writes claims directly (pre-v1.4.1), that write counts as a Maintenance ability call with a synthesized `ClaimProposal` and is subject to the same commit policy. The v1.4.1 enrichment refactor ([DOS-241](https://linear.app/a8c/issue/DOS-241)) is where this distinction becomes fully structural.

### 5. Tombstones

A tombstone is a claim with:

- `actor = 'user'` (or `user_removal` for the specific tombstone variant when the user explicitly cleared a field)
- `claim_text = NULL`
- `retraction_reason = 'user_removal'`
- `claim_state = 'committed'` (tombstones commit immediately per §3)

Tombstones participate in supersede semantics: when a user removes a field value, a tombstone claim is written and supersedes any prior committed claim at the same `(entity_id, claim_type, field_path)`.

**Writers must consult tombstones before proposing — PRE-GATE CHECK (clarified 2026-04-20 per outside voice finding #3).** The tombstone check is **not** part of the commit policy gate. It is a **hard pre-gate rejection** that runs inside `services::claims::propose_claim` before any commit policy evaluation. Order of operations:

1. Tombstone check (this §5) — reject any actor targeting a tombstoned `field_path` within the tombstone window, regardless of commit policy config.
2. Dedup check (§8) — match existing claim; increment corroboration if yes.
3. Commit policy gate (§3) — if actor is `agent` and policy is `gated`, run trust + corroboration logic; otherwise `immediate`.

This order matters because it means **the ghost-resurrection fix is live in v1.4.0 even when the commit policy config is `immediate` for all actor classes** (which R1.9 specifies for v1.4.0). The tombstone protection does not depend on the gate activating; it's a structural pre-check. When v1.4.1 flips agent policy to `gated`, the tombstone check still runs first.

An agent wishing to override a tombstone must produce a higher-threshold corroboration (e.g., three independent sources within 7 days) and surface the override as a contradiction (§7) rather than a silent commit.

This closes the ghost-resurrection bug documented in [ADR-0118](0118-dailyos-as-ai-harness-principles-and-residual-gaps.md) Gap D **in v1.4.0**, not deferred to v1.4.1.

### 6. Agent trust ledger

Each `agent:<name>:<version>` actor has a durable accuracy record:

```sql
CREATE TABLE agent_trust_ledger (
  agent_name        TEXT NOT NULL,
  agent_version     TEXT NOT NULL,
  claim_type        TEXT NOT NULL,
  alpha             REAL NOT NULL DEFAULT 1.0,   -- corroborated / correct
  beta              REAL NOT NULL DEFAULT 1.0,   -- contradicted / incorrect
  posterior_score   REAL NOT NULL DEFAULT 0.5,   -- Beta posterior mean, cached
  last_updated      TIMESTAMP NOT NULL,
  PRIMARY KEY (agent_name, agent_version, claim_type)
);
```

Updates:

- User accepts an agent claim via corroboration or explicit confirmation → `alpha += 1`.
- User contradicts an agent claim (retraction, correction, "wrong") → `beta += 1`.
- Corroboration by an independent source within the window → `alpha += 0.5`.
- Contradiction by an independent source → `beta += 0.5`.

`posterior_score` is the Beta(α, β) mean, recomputed on update. Agents with `posterior_score` below a configurable floor (default 0.45) have all claims auto-routed to the Analysis Inbox regardless of trust-and-corroboration gate outcome, until their score recovers.

Version bumps reset the ledger: `agent:prepare_meeting:1.3` and `agent:prepare_meeting:1.4` are different actors. Rationale: a prompt or logic change can materially change behavior; we don't inherit credit or blame across versions.

**Clarification — 2026-04-20 (outside voice finding #4):** `agent_version` in the ledger key must capture behavioral identity including prompt changes. The [VERSIONING.md](../architecture/VERSIONING.md) guide says prompt edits bump `prompt_template_version`, not `ability_version` — which would mean a prompt-only change inherits trust from the prior version under a naive reading of this section.

The ledger's `agent_version` column is therefore a **composite identity**, not a direct reference to `ability_version` alone. Its value is:

```
agent_version = f"{ability_version}-p{prompt_template_version}"
```

e.g., `agent:prepare_meeting:1.3-p2.1`. A prompt edit bumps `prompt_template_version`, which changes the composite ledger key, which triggers trust warming on the new key per R1.4. This preserves the intent that prompt changes reset trust without abusing `ability_version` semantics.

For abilities whose behavior depends on multiple prompt templates (rare; future extensibility), the composite extends: `agent_version = f"{ability_version}-p{hash_of_all_template_versions}"`. Behavior-affecting model changes (via `IntelligenceProvider` model swap) also bump the composite — because the `PromptFingerprint` includes model name per [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md) §1. In practice, `agent_version` in the ledger is best understood as "the fingerprint-equivalent identity of the agent."

### 7. Contradiction handling

Two committed claims at the same `(entity_id, claim_type, field_path)` with different `claim_text` are a contradiction. Contradictions are **never auto-resolved**. Both claims remain in `claim_state = 'committed'` with a contradiction marker:

```sql
CREATE TABLE claim_contradictions (
  id              TEXT PRIMARY KEY,
  field_path_ref  TEXT NOT NULL,   -- "<entity_id>:<claim_type>:<field_path>"
  claim_a_id      TEXT NOT NULL REFERENCES intelligence_claims(id),
  claim_b_id      TEXT NOT NULL REFERENCES intelligence_claims(id),
  detected_at     TIMESTAMP NOT NULL,
  resolved_at     TIMESTAMP,
  resolution      TEXT,            -- 'accept_a', 'accept_b', 'both_wrong', 'both_right_merge'
  resolved_by     TEXT             -- actor string of resolver
);
```

When a contradiction exists, default reads at that `field_path` return **both** claims with a contradiction flag. Consuming abilities and surfaces render the conflict explicitly rather than choosing silently. The user resolves contradictions through the Analysis Inbox surface (v1.5.0); resolution writes a new claim superseding the resolved one(s).

Feedback from contradictions updates the trust ledger of the agent(s) involved: the agent whose claim was rejected takes a `beta += 1`; the agent whose claim was accepted takes `alpha += 1`.

### 8. Deduplication

A claim is content-addressed by `dedup_key = hash(entity_id, claim_type, field_path, normalized_claim_text)`. Normalization includes trimming, Unicode normalization, and type-specific canonicalization (e.g., dates to ISO, names to case-folded).

When a new claim is proposed whose `dedup_key` matches an already-committed or proposed claim in the active window (default 7 days, configurable per `claim_type`):

- **Match and within window** → do not create a new row; increment `corroboration_count` on the existing row; append the new `source_ref` to its corroboration list; update `trust_computed_at`.
- **Match but outside window** → create a new row with a fresh claim (the fact that the same assertion is being made again after a gap is information).
- **No match** → normal propose path.

Corroboration count feeds the corroboration factor in the trust compiler ([DOS-5](https://linear.app/a8c/issue/DOS-5)) and the trust-and-corroboration gate in §3.

### 9. Provenance integration

Provenance ([ADR-0105](0105-provenance-as-first-class-output.md)) is extended:

- The `Actor` enum gains the shapes in §1 as its canonical values.
- A new field on `Provenance`: `proposal_context: Option<ProposalContext>` carries the trust-gate outcome (`auto_commit | corroboration_commit | proposed | rejected_by_tombstone`) and the gate inputs at commit time. Populated only for Maintenance ability calls that write claims.
- When a claim is committed via corroboration, its provenance `children[]` includes the corroborating claims' provenance, making the corroboration chain auditable.

### 10. Source taxonomy integration

[ADR-0107](0107-source-taxonomy-alignment.md)'s `DataSource` enum is **not** extended with new variants for actors. `DataSource` describes the *origin of information* (Glean downstream, Clay, Gong transcript, user input). `actor` describes *who asserted the claim*. A single agent claim has `actor = 'agent:prepare_meeting:1.3'` and sources `[Glean { downstream: Salesforce }, Glean { downstream: Gong }]`. The two axes are orthogonal and remain so.

### 11. Non-goals for v1.4.0

The substrate shipping in v1.4.0 is intentionally minimal:

- `actor` column on `intelligence_claims` with canonical values from §1.
- `claim_state` column with the four-state machine.
- `dedup_key` and `corroboration_count` columns.
- `claim_contradictions` table.
- `agent_trust_ledger` table.
- `propose_claim` and `commit_claim` service functions honoring the gate.
- Default commit policy in config; agents configured with `strategy = "immediate"` until trust ledger gating is validated.

Out of scope for v1.4.0:

- Analysis Inbox UI — v1.5.0.
- Agent trust dashboard — v1.5.0.
- Contradiction resolution UI — v1.5.0.
- Multi-human-analyst workflows — v2.x (single-user today).
- Cross-entity claim reasoning (claim about A implies claim about B) — future.

## Consequences

### Positive

- **Ghost resurrection fixed structurally.** Tombstones are consulted before any agent commit; the role-type-removal bug cannot recur.
- **Agent claims are auditable by actor.** Per-agent-per-claim-type trust posterior is durable and visible. A misbehaving agent version is detectable without log archaeology.
- **Propose/commit makes trust operational.** Below-threshold agent output is isolated in the Analysis Inbox rather than leaking into briefings.
- **Humans and agents share one substrate.** A human analyst's claim and an agent's claim share schema, provenance shape, and trust semantics. Any future multi-agent or multi-analyst scenario composes on this foundation.
- **Dedup via content addressing makes corroboration cheap.** Two agents independently concluding the same thing become evidence, not duplicate rows.
- **Contradictions never silently resolve.** The user is the final arbiter on disagreement; the system's job is to surface, not hide.

### Negative / risks

- **Two new tables plus two new columns.** Storage and migration surface is real. Mitigated by a minimal v1.4.0 scope and incremental rollout behind commit-policy config.
- **Commit policy tuning requires real data.** The auto-commit trust threshold, corroboration threshold, tombstone window, and agent trust floor are all empirical. Expect a 60-day shadow-run post-substrate before the config is stable. During that period, agents run `strategy = "immediate"` to preserve current behavior; propose/commit is observational.
- **Agent version bumps lose trust history.** A conservative choice; the alternative (carry trust across versions) is worse because it masks regressions. Mitigated by warming new-version trust from a prior-version prior when acceptable.
- **Contradictions surfacing twice in the UI is annoying.** By design — silent resolution is the greater harm. Surfaces must render well; v1.5.0 UI owns that.
- **Agent trust ledger is per-claim-type.** Some agents produce many claim types; the ledger can fan out. Acceptable for v1.4.0 scope (≤10 agents × ≤20 claim types = 200 rows); will revisit if fan-out becomes a real problem.

### Neutral

- No changes to existing enrichment behavior until v1.4.1 refactor lands. v1.4.0 ships the data model; v1.4.1 activates propose/commit for enrichment pipelines ([DOS-241](https://linear.app/a8c/issue/DOS-241)).
- `DataSource` taxonomy unchanged. Actor and source are separate axes.
- Briefings render identically during v1.4.0 — the commit policy for agents starts at `immediate` and tightens only after shadow data validates thresholds.

---

## Revision R1 — 2026-04-19 — Reality Check

Post-draft adversarial review (codex) and five parallel codebase reference passes surfaced structural problems and ground-truth discrepancies. This revision amends the original decision. The original sections above are preserved for history; the clarifications below supersede any conflict.

### R1.1 State model fix — tombstones are a distinct state, not a committed-with-NULL

The original §2 and §5 read as if a tombstone is a committed claim with `claim_text = NULL` — but §7 also says retraction transitions a claim to `retracted` with the same semantics. Codex flagged this correctly: readers cannot tell whether a user removal is an asserted fact, a marker, or a hidden historical event.

**Revised state machine:**

| State | Meaning | `claim_text` | Read semantics |
|---|---|---|---|
| `proposed` | Not authoritative. Visible in Analysis Inbox only. | Any | Excluded from default reads. |
| `committed` | Authoritative positive assertion. | Non-null. | Included in default reads. |
| `tombstoned` | Authoritative negative assertion — "this field is absent by intent." | NULL; `retraction_reason` required. | Included in default reads as a negative; blocks writers. |
| `superseded` | Replaced by a later claim at the same `(entity_id, claim_type, field_path)`. | As-written. | History only. |
| `withdrawn` | Explicitly retracted without supersede (source revocation, contradiction resolution, draft expiry). | As-written. | History only. |

`tombstoned` is its own state, not a variant of `committed`. It is a positive assertion of absence. Supersede/withdraw are for already-committed claims that leave the default read. This removes the internal contradiction.

### R1.2 Supersede vs contradiction — clarified by source identity

Codex flagged that the ADR simultaneously says "same field with different value → contradiction, keep both" and "same field replaced by later claim → supersede." Both are true but for different cases:

- **Supersede** happens when the **same `actor`** (or same agent at the same version) re-asserts a different value at the same `field_path`. The later assertion wins; earlier goes to `superseded`.
- **Contradiction** happens when **different actors** assert different values at the same `field_path` within the active window. Both stay `committed`; a `claim_contradictions` row is written; reads surface both with a contradiction flag per §7.

The rule is: self-correction is supersede, cross-source disagreement is contradiction. Codify in the `commit_claim` service function: look up the latest committed claim at `field_path`; compare actors; route accordingly.

### R1.3 Consolidate existing tombstone infrastructure, don't duplicate it

Reference pass found three pre-existing tombstone-like mechanisms that the original ADR ignored:

- `suppression_tombstones` table (migration `084_feedback_events.sql`) — generic item-level dismissals with `dismissed_at`, `expires_at`, `superseded_by_evidence_after`.
- `DismissedItem` struct in `src-tauri/src/intelligence/io.rs:65` — stored inside `intelligence.json` for per-item dismissal.
- `account_stakeholder_roles.dismissed_at` column (migration `107_stakeholder_role_dismissals.sql`) — soft-delete with `data_source='user'` used by `intel_queue` to refuse re-insertion.

**Consolidation timing — 2026-04-20 update:** Per plan-eng-review AskUserQuestion #1 and the brownfield-as-greenfield founder commitment (D1), consolidation ships in v1.4.0 as a destructive migration alongside the `intelligence_claims` table creation. Original R1.3 deferred consolidation to v1.4.1 out of migration caution; that deferral is retracted. Rationale: at AI velocity + minimal user base, keeping three parallel tombstone mechanisms alive through v1.4.0 creates interop cost that exceeds consolidation cost. The interop is what's expensive; one-table-to-rule-them-all is cleaner.

**v1.4.0 consolidation migration:**

- Write a `tombstoned` claim for every live row in `suppression_tombstones` (preserve `dismissed_at` as `created_at`; preserve `field_path` as `field_path`).
- Write a `tombstoned` claim for every `DismissedItem` found in `intelligence.json` blobs during the one-time backfill.
- Write a `tombstoned` claim for every `account_stakeholder_roles` row with `dismissed_at IS NOT NULL` and `data_source='user'`.
- Reads switch to `intelligence_claims` for all tombstone queries.
- Old tables / columns kept read-only for a short window (~1 release) for fallback parity testing, then dropped in v1.4.1 cleanup migration.

The ghost-resurrection fix becomes structural, not row-by-row. Consistency across stakeholder roles, dismissed intelligence items, and generic suppression tombstones is one code path, not three.

### R1.4 Trust ratchet — shadow sampling prevents permanent quarantine

Codex flagged: a quarantined agent can only recover trust if humans manually review its rejected claims. That's a ratchet — once down, stays down. The fix:

Add a **shadow-sampling policy**. For agents with `posterior_score < floor`, a configurable fraction (default 10%) of their below-threshold claims are surfaced to the Analysis Inbox for human review anyway, bypassing the gate. If the human accepts, the agent's ledger recovers normally. This turns "quarantine" into "probation with structured opportunity to recover" and prevents false-positive quarantines from being permanent.

Also: trust ledger **warms from prior version** when an agent bumps. Original §6 reset on version bump; that's too aggressive. Instead, start the new version at `α = prior.α × 0.5`, `β = prior.β × 0.5` — half the evidence carries forward, halving the sensitivity to prior behavior while not starting cold. Codex's concern about "prompt-only changes inheriting old trust" is addressed by the v1.4.0 project convention that non-breaking prompt edits bump a minor version (per [ADR-0106](0106-prompt-fingerprinting-and-provider-interface.md)), so the warming applies across both cases.

### R1.5 Actor representation — align with ADR-0102 `Actor` enum

The original §1 declared actor as a string with canonical shapes. [ADR-0102](0102-abilities-as-runtime-contract.md) §1 defines `Actor` as an enum (`User | Agent | System`). These conflict.

**Revised:** `actor` column on `intelligence_claims` stores a **typed representation serialized to string** for schema stability:

```rust
pub enum ClaimActor {
    User,
    UserRemoval,
    Human { role: HumanRole, id: UserId },
    Agent { name: AgentName, version: AgentVersion },
    System { component: SystemComponent },
    External { source: DataSource },
}
```

Serialization produces the same canonical strings the original §1 proposed. Rust code uses the enum; the column stores the rendered string. This preserves ADR-0102's typing while accepting that a columnar DB stores serialized forms.

The `Actor` enum in ADR-0102 §1 is extended by this ADR (not replaced). The `Agent` variant gains `name` + `version` fields; `Human` is new; `External` is new. This extension requires an amendment to ADR-0102; tracked.

### R1.6 Dedup semantics — preserve per-asserter history

Original §8 said: identical re-assertion within the window updates `corroboration_count` in place. Codex flagged: this mutates the claim row and loses per-asserter provenance.

**Revised:** dedup does **not** mutate the claim row. Instead, a `claim_corroborations` child table records each corroborating assertion by a distinct asserter:

```sql
CREATE TABLE claim_corroborations (
  id                TEXT PRIMARY KEY,
  claim_id          TEXT NOT NULL REFERENCES intelligence_claims(id),
  corroborating_actor TEXT NOT NULL,
  corroborating_source_ref TEXT,
  asserted_at       TIMESTAMP NOT NULL,
  UNIQUE(claim_id, corroborating_actor)
);
```

`corroboration_count` on the claim row becomes a cached denormalization (updated via trigger or on write) for query speed. Provenance of each corroborator is preserved. The trust compiler's corroboration factor reads from this table.

### R1.7 Commit policy stance — match non-goals to §3

Original §3 described a gated commit policy; §11 (Non-goals) said agents run `strategy = "immediate"` until thresholds are validated. Codex flagged the mismatch.

**Revised:** §3's gated policy is the **data model** that ships in v1.4.0. The **active config** in v1.4.0 sets all actor classes to `strategy = "immediate"` as before. The gate logic is implemented and exercised by tests, but is not in the hot path for any actor class until a subsequent release flips the config. This keeps the shape shipped while preserving current behavior. The v1.4.1 enrichment refactor ([DOS-241](https://linear.app/a8c/issue/DOS-241)) is where `strategy = "gated"` first lands for agents.

### R1.8 Ground-truth corrections

- The "three tombstone mechanisms already exist" finding means the migration story is larger than v1.4.0 alone. Flag for [DOS-7](https://linear.app/a8c/issue/DOS-7) design review: decide whether v1.4.0 ships the `intelligence_claims` table + schema + service functions but keeps existing tombstone tables authoritative, with full migration to v1.4.1.
- `intelligence_claims` does not exist yet — confirmed. [DOS-7](https://linear.app/a8c/issue/DOS-7) creates it.
- No abilities runtime code exists yet. ADR-0102 is also still doc-only. v1.4.0 project work creates both; this ADR's implementation cannot precede ADR-0102's skeleton landing.
- Agent trust ledger as a separate table is fine; the per-claim-type fan-out (max ~10 agents × ~20 claim types = 200 rows) is trivial storage.

### R1.9 Scope for v1.4.0 — revised (updated 2026-04-20)

Ships in v1.4.0:

- `intelligence_claims` table with schema from [DOS-7](https://linear.app/a8c/issue/DOS-7) + `actor`, `claim_state`, `dedup_key`, `corroboration_count` columns.
- `claim_corroborations` child table (R1.6).
- `claim_contradictions` table (§7).
- `agent_trust_ledger` table.
- `propose_claim` and `commit_claim` service functions with gate logic implemented but config set to `immediate` for all classes.
- **Consolidation migrations active** (updated 2026-04-20 per plan-eng-review AskUserQuestion #1): `suppression_tombstones`, `DismissedItem` in-blob entries, and `account_stakeholder_roles.dismissed_at` values all backfilled into `intelligence_claims` as `tombstoned` rows. Reads switch to `intelligence_claims` for tombstone queries. Old tables kept read-only for parity testing.
- Ghost-resurrection regression test on stakeholder role removal under the consolidated model.

Ships in v1.4.1 (enrichment refactor):

- Agent commit policy flips to `gated`.
- Cleanup migration drops the old `suppression_tombstones` table, old `DismissedItem` JSON-blob entries, and the `account_stakeholder_roles.dismissed_at` column.
- Analysis Inbox surface ships.
- Shadow sampling (R1.4) activated.
- Trust warming on version bump (R1.4) activated.

---

## Revision R2 — 2026-04-20 — Pessimistic row-lock on `commit_claim`

Addresses persona-review finding S6 (concurrency primitives).

R1.2 established that supersede vs contradiction is routed by actor identity at read time in `commit_claim`. Persona review flagged that actor-based routing decided at read time + a concurrent second writer produces a race: both writers read "no existing claim at this field_path," both route to supersede, both insert, both hit the unique constraint on `(entity_id, claim_type, field_path, claim_sequence)`, both retry. SQLite's serializable isolation prevents corruption but produces churn and potentially wrong routing decisions.

**Revised semantics:** `commit_claim` acquires a **pessimistic row-lock** on the target `(entity_id, claim_type, field_path)` key for the duration of the routing decision + insert. Conceptually:

```rust
async fn commit_claim(
    ctx: &ServiceContext,
    proposal: ClaimProposal,
) -> Result<CommittedClaim, ClaimError> {
    ctx.check_mutation_allowed()?;

    let lock_key = (proposal.entity_id, proposal.claim_type.clone(), proposal.field_path.clone());

    // Acquire exclusive lock on this field_path for the duration of this function.
    // SQLite: either via a dedicated `claim_commit_locks` table with INSERT-RETURNING semantics,
    // or via an in-memory Mutex<HashMap<LockKey, ()>> if we decide app-level locking is enough.
    let _lock = ctx.services.claim_locks.acquire(&lock_key).await?;

    // Now-uncontested read + routing decision + insert.
    let latest = ctx.services.db.read_latest_claim(&lock_key)?;
    let route = route_for(&proposal, &latest);                // supersede | contradiction | fresh
    apply_route(ctx, route, proposal).await?;
    // _lock drops at function end, releasing the row for other writers.
}
```

**Lock implementation choices (to decide at DOS-7 design review):**

- **App-level `Mutex<HashMap<LockKey, ()>>`.** Fast, simple, lives in `services::claims` module. Sufficient for single-process single-user DailyOS. Does not survive a crash — but no lock held across a crash is meaningful anyway. **Recommended default.**
- **Dedicated `claim_commit_locks` table with uniqueness constraint.** DB-backed; survives crashes; higher overhead per commit. Overkill for single-user; potentially useful if multi-process access ever becomes relevant.
- **SQLite `BEGIN EXCLUSIVE` transaction.** Locks the entire DB, not just this field_path. Too coarse.

The app-level mutex map is the default. If the multi-process scenario emerges (daemon process + Tauri main process both committing claims), revisit.

**Fairness and starvation:** an active hot field (user rapidly correcting a value) + concurrent agent attempting commits could theoretically queue agent commits indefinitely. Mitigate with a commit-attempt timeout (default 500 ms) returning `Err(ClaimError::LockTimeout)` — caller retries or gives up. User-initiated commits always proceed (locked for short duration); agent commits may time out and be rescheduled.

**Test case:** property test — N=100 concurrent commit_claim calls against the same field_path from a mix of agent + user actors. Assert: exactly one claim exists at `claim_state = committed`; all others resolved as supersede or contradiction per R1.2 rules.
