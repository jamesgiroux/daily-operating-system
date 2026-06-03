# DOS-832 L0 Packet — First-Class Rebuild From Canonical Workspace Inputs

**Version:** v1.4.9 · W1b storage hardening  
**Issue:** [DOS-832](https://linear.app/a8c/issue/DOS-832)  
**Author date:** 2026-06-03  
**Tier:** Tier 3 (markdown-only)  
**Scope tier:** Wave-coupled substrate. L0 requires `/codex challenge` or a project-approved equivalent, `ce-feasibility-reviewer`, `ce-security-lens-reviewer`, and mandatory K-in. Local adversarial document review may harden the draft, but it does not make the packet L0-approved on its own.
**Status:** Draft for L0 review; not approved until the adversarial, feasibility, security-lens, and K-in verdicts are recorded.

---

## §0 Origination + Boundary

**Origination class:** Debug-driven. DOS-832 comes from the 2026-05-28/29 storage-loss recovery path and the v1.4.9 storage reset program. The existing manual recovery shape was "delete/recreate/re-enrich until the app works." This packet turns that into a first-class, repeatable, auditable rebuild path.

**Critical dependency:** DOS-832 is not a generic import script and not a W1-only feature. The v1.4.9 wave plan routes correction-preserving rebuild through W3/DOS-628's structured corrections sidecar. DOS-832 can design and implement source inventory, fresh-schema orchestration, source registration, ingestion replay, and Replica proof now, but **the correction-preserving release gate cannot pass until DOS-628 defines and ships the sidecar projection contract**.

**Branch-note:** this branch is based on `public/dev`, whose `.docs/plans/v1.4.9-waves.md` still contains stale W1b text: ADR-0136, split-build decrypt/sentinel migration, v273/v274 migration slots, and "writer-priority lane." DOS-831 PR #434 corrects those assumptions. DOS-832 consumes the corrected assumptions:

- SQLCipher retirement is an ADR-0092 amendment path, not ADR-0136.
- There is no in-place decrypt/sentinel migration in DOS-831.
- Existing encrypted-looking active DBs fail loud into storage-health/rebuild guidance.
- Current schema head is v276; next free slot is v277.
- Writer-priority lane language is stale; ADR-0133 owns FIFO writer/gate responsibility.

**Authority precondition:** DOS-832 implementation does not begin from the stale local wave-plan text. Before L1, either DOS-831 PR #434 (or an equivalent wave-plan/ADR correction) is merged into the branch, or this packet remains a draft and the first L1 task is to rebase/apply those authority corrections. A packet note pointing at an unavailable PR is not sufficient for shipping implementation.

### Trust Topology

- Local-to-local, single-user machine trust boundary.
- Rebuild operates on local workspace files and the local app DB only.
- Rebuild output can contain claims, provenance, trust bands, signal state, and feedback history; therefore DOS-832 runs the full Intelligence Loop integration check.
- Real-workspace validation may run locally, but committed fixtures, docs, logs, PR text, and test output must be PII-free.

---

## §1 What Exists

### §1.1 Existing Rebuild Is Insufficient

`src-tauri/src/db_backup.rs::rebuild_from_filesystem` scans `Accounts/*/dashboard.json`, `Projects/*/dashboard.json`, and `People/*/person.json`, then syncs accounts, projects, and people. Its own doc comment names the known gaps: email enrichment state, meeting history, and action source references can be lost. It does not rebuild the claim substrate, provenance, trust inputs, workspace source lifecycle, embeddings, signals, invalidation jobs, claim feedback, or contradictions.

ADR-0048 accurately described that older safety net in February 2026, but it now conflicts with v1.4.9's correction-preserving rebuild goal. DOS-832 must update ADR-0048 Principle 4 to distinguish:

- legacy partial rebuild from entity JSON; and
- first-class intelligence rebuild from canonical workspace inputs plus re-enrichment plus correction replay.

### §1.2 Workspace Source Substrate Exists

The v1.4.5 workspace ingestion substrate already provides the pieces DOS-832 should consume:

- `WorkspaceSourceRegistry::open_validated` is the workspace file trust boundary and guards traversal, symlinks, races, workspace escape, size, and unsupported formats.
- `workspace_file_lifecycle` stores source provenance (`source_type`, `data_source`, `source_asof`, lifecycle state, entity link, content hash, category).
- `workspace_source_registry` and `workspace_category_registry` define typed `DataSource::WorkspaceFile { kind }` categories.
- `document_ingestion_runs` provides per-file/content/mode ingestion idempotency.
- `document_entity_links` records source-to-entity attribution and tombstones.
- `workspace_backfill` already scans the workspace, computes PII-safe handles, detects duplicate content, records resumable state, and registers sources. It intentionally does **not** create claims.

### §1.3 Claim + Correction Substrate Exists

Rebuild must reuse the shipped claim services:

- `services::claims::commit_claim` is the only intelligence-claim writer. It computes canonical subject identity, item hash, semantic `compute_dedup_key`, trust initialization, same-meaning merge, tombstone PRE-GATE, contradiction forks, claim edges, version events, and invalidation bumps.
- `services::claims::record_claim_feedback` is the typed correction writer. It records append-only `claim_feedback`, updates verification/lifecycle state, emits version events, tombstones edges where appropriate, bumps invalidation, and queues targeted repair.
- `claim_receipt::feedback` is already a receipt-shaped, sensitivity-gated, idempotency-aware caller for feedback.

DOS-832 must not raw-copy `intelligence_claims`, raw-insert `claim_feedback`, preserve old claim IDs as a correctness assumption, or update claim lifecycle columns directly.

### §1.4 DB Mode + Writer Discipline Exists

DB-mode isolation exists in `ActionDb`: non-release defaults to Replica, `DbMode::Replica` resolves to `dailyos-replica.db`, and structural prod-open denial forbids production DB opens outside Live. Maintenance binaries already use explicit Live opt-in patterns.

ADR-0133 owns writer queue responsibility: no fresh mutating DB connections to "avoid starving foreground," no priority in the gate, no DB guards across `.await`, and mutations route through the service/writer path. DOS-832 is write-heavy and must consume this substrate.

### §1.5 Current-Code Blockers

Current `public/dev` is not yet a plain-SQLite rebuild target. `ActionDb` still detects plaintext DB files and migrates them to SQLCipher through `migrate_to_encrypted`. Therefore:

- DOS-832's plain-SQLite fresh-install proof is gated on DOS-831 L1 landing the storage-mode change.
- Until then, DOS-832 L1 may prove orchestration in Replica/current-encrypted mode, but it cannot claim the final plain-SQLite recovery AC.
- The packet remains release-gated with DOS-831: encrypted-looking active stores fail loud into storage-health/rebuild guidance, but the plain rebuild target must not be marked complete while auto-encrypt paths still exist.

Meeting history, email enrichment cache state, and action source-reference repair are the historical gaps that motivated DOS-832. This packet does **not** claim a lossless clone of every legacy SQLite table. L1 must either name a canonical input/producer and verification metric for each of those categories or report it as out-of-scope/degraded in the rebuild result. User-facing proof must say "reconstructed intelligence substrate" unless those categories are covered explicitly.

---

## §2 Target Shape

### §2.1 Rebuild Is Fresh-Schema Replay, Not Migration

The rebuild path creates a fresh database at schema head, then replays canonical inputs into it. It does not:

- mutate a damaged/encrypted source DB in place;
- convert cipher to plain;
- repair arbitrary corruption inside the source DB;
- copy claim rows by ID from an old DB;
- treat generated markdown as canonical truth.

The intended phases:

1. **Plan:** resolve workspace root, DB mode, target DB path, existing DB health, and source inventory. Produce a PII-safe summary using handles/counts/reason codes.
2. **Fresh schema:** create or open the target rebuild DB and run migrations to head (`v276` on current `dev`; reserve `v277+` only if L1 adds rebuild-run schema).
3. **Canonical entity seed:** run the existing entity JSON sync for `Accounts`, `Projects`, and `People`, while preserving the fact that this is only the entity seed layer.
4. **Source registration:** reuse/extend `workspace_backfill` to register workspace files and entity links. It remains privacy-aware and resumable.
5. **Source ingestion:** run the workspace ingestion pipeline over registered eligible files. Claims must enter through `commit_claim` with `DataSource::WorkspaceFile`, `source_ref`, `source_asof`, `observed_at`, temporal scope, sensitivity, and provenance intact.
6. **Re-enrichment:** run existing claim/enrichment producers needed to reproduce entities, claims, provenance, trust inputs, salience, embeddings, and derived context. Any producer newly invoked by rebuild must pass the runtime-wide trust audit in `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`.
7. **Correction replay:** after DOS-628/W3 supplies the corrections sidecar, match regenerated claims by semantic content identity and replay typed corrections through `record_claim_feedback`.
8. **Verification:** compare source inventory, entity counts, claim counts by type/state/trust band, correction replay outcomes, orphaned sidecar entries, and surface smoke checks. No PII in committed proof.

Implementation shape:

- Add a `services::rebuild` owner. Commands and maintenance binaries only parse/validate options and invoke the service.
- Pause/drain background intelligence queues before Live replacement or long writer-exclusive phases, then resume or requeue pending work. Rebuild cannot race normal startup/background writers.
- Add durable rebuild/replay run state if the existing `workspace_backfill_runs` tables are insufficient. If new schema is required, reserve from v277 upward and update the wave plan after DOS-831's corrections merge.

### §2.2 Canonical Inputs

Canonical inputs are:

- workspace JSON where ADRs define it as durable structured state (`dashboard.json`, `person.json`, project/account equivalents);
- governed workspace files opened through `WorkspaceSourceRegistry::open_validated`;
- source registry/lifecycle metadata derived from those files and source kinds;
- DOS-628 corrections sidecar once shipped.

Derived replay outputs are not canonical inputs. L1 must inventory each producer before implementation:

| Producer family | Canonical inputs | Derived outputs | Verification metric |
| --- | --- | --- | --- |
| Entity JSON sync | account/project/person JSON | entity rows, tracker paths, basic relationships | counts by entity type, archived/internal exclusions, tracker-path parity |
| Workspace registration/ingestion | validated workspace files + source metadata | lifecycle rows, entity links, workspace-backed claims | file counts by source kind/category, claim counts by type/source, source-time confidence counts |
| Re-enrichment/trust/salience | canonical entities, workspace sources, service evidence, AI runtime output | claims, trust inputs/bands, salience, embeddings, derived contexts | counts by producer/claim type/trust band, provenance completeness, invalidation/recompute markers |
| Correction replay | DOS-628 sidecar events | feedback rows, claim lifecycle/verification changes, contradiction/supersession state | applied/skipped/orphaned counts by stable event id and reason |

Non-canonical inputs:

- generated markdown as a source of database truth;
- old DB row IDs;
- old runtime `dedup_key` values copied verbatim;
- re-enrichment outputs treated as authority instead of derived results;
- logs, proof output, or human-only summaries.

`source_asof` resolution must be deterministic. Existing public intake/backfill paths derive `source_asof` from file mtime; that is not enough for replay proof. DOS-832 L1 must define a source-time resolver with explicit precedence:

1. persisted lifecycle/source metadata or canonical source sidecar timestamp when available;
2. source-native timestamp inside canonical JSON when the ADR for that file type names it;
3. filesystem mtime only as `filesystem_unverified`, with that confidence recorded in the report.

Rebuild time is never a substitute for source time.

### §2.3 Correction Replay Contract

DOS-832 consumes the v1.4.9 rebuild decision:

- W3/DOS-628 projects a structured `corrections` sidecar keyed by content-derived semantic identity in the `compute_dedup_key(item_hash, subject_ref_compact, claim_type, field_path)` shape.
- Rebuild re-enriches first, producing fresh claim UUIDs.
- Replay resolves sidecar entries to regenerated claims by semantic identity.
- Non-unique matches are disambiguated by `source_ref` and `observed_at`.
- Ambiguous or missing matches become **orphaned replay entries** in the report; they are never guessed, silently dropped, or wired to the wrong claim.
- Tombstones, dormant/withdrawn state, contradictions, supersession, and typed `FeedbackAction` semantics are preserved by replaying through `services::claims`, not by raw SQL.
- Replay must never resurrect a tombstoned claim or write around PRE-GATE.
- Feedback replay uses the same user-correction actor semantics accepted by `record_claim_feedback`; non-user replay actors require an explicit writer extension in the sidecar/replay L0 before implementation.
- DOS-628 sidecar events must expose a stable `correction_event_id` or equivalent replay key. DOS-832's replay journal claims that key before calling any feedback/claim mutation service.
- DOS-832 uses a named `services::claims` replay helper for sidecar events that are not expressible as a plain `record_claim_feedback` call. The helper owns contradiction/supersession reconciliation, endpoint resolution, idempotency, orphaning, and internal calls to `record_claim_feedback` / `commit_claim` / existing reconciliation routines. It is the only place where sidecar edge semantics enter the claim substrate.
- Replay has its own durable journal: restarted rebuilds must not duplicate feedback rows, repair jobs, contradiction edges, or supersession effects.

The exact sidecar schema and key derivation are owned jointly by DOS-628 and DOS-832 L0/L1. DOS-832 cannot pass its full AC without that contract.

### §2.4 Operator Boundary

Default posture:

- `--dry-run` and Replica rebuild proof are first-class.
- Live destructive replacement requires explicit Live mode plus an explicit operator flag.
- Existing active DB replacement must create a restore point and validate storage health before and after.
- Live replacement uses a DB-service cutover protocol, not a blind file copy: block new readers/writers, pause/drain background queues, close/drop the active DB pool, checkpoint/handle WAL, stage the rebuilt DB, atomically swap, remove stale WAL/SHM, harden permissions, validate, then reopen.
- If an encrypted-looking active DB is present after DOS-831, the app fails loud and routes to storage-health rebuild/restore guidance; it does not run an in-place decrypt repair.
- Exported DB copies are plaintext egress after DOS-831 and must use destination-boundary warnings and restrictive permissions where possible.

### §2.5 Sensitive Artifacts

The DOS-628 corrections sidecar and DOS-832 replay journal are sensitive local artifacts. They can contain claim feedback actions, wrong-source/wrong-subject metadata, nuance text, tombstone/supersession state, and contradiction references.

Security contract:

- Sidecars live only under the configured workspace or DailyOS app-support recovery directory; arbitrary output paths are rejected unless the operator explicitly chooses an export destination.
- Sidecar and replay-journal files are classified at least `confidential`; user-authored free text inherits higher sensitivity when the source claim requires it.
- Files are written with owner-only permissions where supported.
- Reports/logs use handles, counts, hashes, reason codes, and sensitivity labels; they do not include raw correction text, entity names, absolute local paths, or source payloads.
- Retention is explicit: after successful replay, the sidecar is retained only if it is part of the durable DOS-628 projection contract; transient replay journals are pruned or marked completed according to the operator policy. Failed/orphaned entries retain only the minimum data needed for safe retry.
- Exporting a sidecar follows the same destination-boundary warning and evidence-governance rules as exported DB copies.

---

## §3 Acceptance Criteria

**AC1 — Fresh intelligence-substrate proof.** Starting from an empty target DB at schema head, rebuild reproduces the reconstructable intelligence substrate from canonical workspace inputs plus service re-enrichment: accounts, projects, people, workspace sources, entity links, claims, provenance, trust-band inputs, salience/derived context needed by covered shipped surfaces, and source lifecycle state. Meeting history, email enrichment cache state, and action source references are either covered by named canonical producers and verification counts or explicitly reported as degraded/out-of-scope.

**AC2 — Correction preservation.** With a DOS-628 corrections sidecar containing representative feedback, demotions/tombstones, contradictions, supersession, stable replay IDs, and a deliberate ambiguous match, rebuild replays corrections through the named `services::claims` replay helper. Regenerated claims reflect the correction state; ambiguous entries are reported as orphans.

**AC3 — No raw claim copy.** Tests and static review show rebuild does not copy old `intelligence_claims` rows, old claim IDs, raw `claim_feedback`, or old runtime `dedup_key` values as authoritative state. All claim production flows through `commit_claim`; all correction replay flows through `record_claim_feedback` or its receipt-layer wrapper.

**AC4 — Source/provenance fidelity.** Rebuilt claims preserve `DataSource`, `source_ref`, knowable `source_asof`, `observed_at`, provenance JSON, temporal scope, sensitivity, and trust inputs. Rebuild time is not used as source time unless the source truly has no earlier timestamp and the report marks the confidence accordingly.

**AC5 — DB-mode safety.** Dry-run and Replica proof cannot open or mutate the production DB. Live replacement refuses unless DB mode is Live and an explicit operator flag is present. Existing DB replacement creates a restore point and validates storage health.

**AC6 — Exported DB egress.** Exported DB copies after DOS-831 are treated as plaintext egress: the UI/operator flow shows an explicit warning, destination handling avoids leaking absolute paths or source details into logs, and owner-only permissions are applied where supported.

**AC7 — Correction sidecar security.** DOS-628 sidecars and DOS-832 replay journals have a classified storage/retention contract: bounded path, owner-only permissions where supported, no raw correction/source payloads in logs or reports, explicit retention/prune behavior, and safe orphan retry metadata.

**AC8 — Live cutover exclusivity.** Live destructive replacement blocks new readers/writers, pauses/drains background queues, closes/drops the active DB service/pool, stages and atomically swaps the rebuilt DB, cleans WAL/SHM, validates the replacement, and reopens service access. Failure restores the prior DB or leaves a clear restore point.

**AC9 — Single-writer/service boundary.** Rebuild writes route through service-owned mutation paths and the writer discipline. No command handler or maintenance bin directly mutates tables outside services. No fresh mutating DB connection is introduced to bypass the writer path.

**AC10 — Resumability and observability.** Long rebuilds are resumable by durable run state. The run report uses PII-safe handles/counts/reason codes and records failures for source registration, ingestion, enrichment, correction replay, and verification. Replay resume is idempotent across process restarts by claiming stable sidecar event IDs before mutation.

**AC11 — ADR + docs.** ADR-0048 Principle 4 is amended to describe first-class intelligence rebuild and its dependency on correction sidecars. Operator recovery docs explain dry-run, Replica proof, Live cutover, encrypted-looking DB failure, sidecar retention, plaintext export warnings, and restoration.

**AC12 — Gates.** Focused tests plus full gates pass:

```bash
cargo clippy -- -D warnings
cargo test
pnpm tsc --noEmit
```

---

## §4 Intelligence Loop Integration Check

1. **Claim model:** Rebuild produces claims, not display-only rows. Existing claim types and `ClaimProposal` metadata remain authoritative. New rebuild-run metadata is operational, not a claim, unless L1 introduces a user-visible assertion about rebuild health.
2. **Provenance + trust:** Producers must pass full provenance and trust inputs into `commit_claim`. Trust recomputation/audit covers source lifecycle, freshness, corroboration, contradiction, correction state, sensitivity, and verification state. Rebuild must not seed arbitrary trust scores.
3. **Signals + invalidation:** Rebuild must emit or reconstitute the signals needed for source ingestion, claim commits, feedback replay, targeted repair, trust recompute, and surface invalidation. A rebuilt DB with stale rendered surfaces is not accepted.
4. **Runtime + surfaces:** Tauri and MCP read the rebuilt DB through existing services and sensitivity gates. `build_intelligence_context()`, account/project/person contexts, workspace graph reads, and claim receipt routes must behave against the rebuilt store. Meeting prep/readiness surfaces are included only where their canonical source inputs/producers are covered; otherwise rebuild proof must show an explicit source-gap/degraded state rather than silent stale output.
5. **Feedback loop:** User corrections survive by structured sidecar replay into `claim_feedback` and claim lifecycle state. New feedback after rebuild continues through the same services and source-reliability/trust inputs. Sidecar storage and replay logs preserve the correction loop without exposing raw sensitive correction text.

---

## §5 Scope Boundaries

In scope:

- L0 design for full correction-preserving rebuild.
- Reuse/extension of workspace backfill, workspace ingestion, claim producers, correction replay, and run reporting.
- ADR-0048 amendment requirement.
- Replica-first and Live-cutover operator guard design.

Out of scope:

- In-place cipher/plain migration or corruption repair.
- Re-authoring DOS-628 sidecar schema inside DOS-832.
- New claim mutation API parallel to `commit_claim` / `record_claim_feedback`.
- New source-of-truth model where generated markdown becomes canonical.
- Customer-specific fixtures or committed real-workspace proof.
- MCP auth/transport changes from W2.

---

## §6 Test Plan

Focused L1 tests:

- Unit tests for rebuild plan/source inventory skip reasons and PII-safe report shape.
- Replica-mode test proving production DB path denial.
- Workspace source registration integration test reusing existing generic fixtures.
- Deterministic `source_asof` precedence test where file mtime changes but canonical source metadata keeps the same provenance timestamp.
- Ingestion replay test proving claims enter through `commit_claim` and carry `DataSource::WorkspaceFile`, `source_ref`, `source_asof`, provenance, and sensitivity.
- Producer-inventory test or fixture proof that each claimed derived output has named canonical inputs, produced tables/claims, trust/provenance behavior, and verification counts.
- Correction replay test using generic sidecar fixtures with stable replay IDs for confirm/current, false/outdated, wrong subject/source, nuance, surface inappropriate, not relevant, contradiction, supersession, tombstone, unknown claim ID, and ambiguous/non-unique endpoints.
- Idempotency/resume test that claims sidecar event IDs before mutation and restarts after source registration, ingestion, enrichment, and correction replay without duplicate claims, feedback, repair jobs, or replay effects.
- Queue pause/drain/resume and DB-service close/drop/reopen test for Live cutover phases.
- Exported DB copy test for warning state, destination-boundary handling, owner-only permissions where supported, and no raw destination/path/source details in logs.
- Sidecar/replay-journal security test for bounded path, owner-only permissions where supported, report redaction, orphan metadata shape, and retention/prune behavior.
- Storage-health test for encrypted-looking / unreadable active DB guidance.
- Operator CLI/command tests for dry-run, apply, resume, and Live refusal.

Real-data proof:

- Run locally against the real workspace in Replica mode.
- Commit only aggregate PII-free counts and pass/fail summaries.
- Any committed proof record follows `.docs/evals/evaluation-evidence-contract.md` and `.docs/evals/fixture-governance.md`: repo-relative paths, input hashes, privacy metadata, and no customer data, identity maps, private payload paths, or absolute local paths.

Full gates:

```bash
cargo clippy -- -D warnings
cargo test
pnpm tsc --noEmit
```

---

## §7 L0 Reviewer Dispatch

- **K-in:** `ce-learnings-researcher` completed; findings folded into §1-§6.
- **Feasibility:** `ce-feasibility-reviewer` required to verify this can be built from current source registration, ingestion, claim, feedback, and DB-mode services.
- **Security:** `ce-security-lens-reviewer` required because DOS-832 touches filesystem trust boundaries, Live destructive operation, recovery docs, exported DB copies, and sensitive correction data.
- **Adversarial review:** external `/codex challenge` or a project-approved equivalent is required for formal L0 approval. If the external run is unavailable, record the failure and keep the packet in draft/not-approved status; local adversarial review is only an input to hardening edits.

Approval requires unanimous pass or explicit L6 decision on any residual release gate.

---

## §8 K-In Findings Folded

- `docs/solutions/architecture-patterns/claim-producers-require-runtime-wide-trust-audit-2026-05-22.md`: any rebuild/backfill that produces claims must audit producer path, provenance, trust inputs, recompute trigger, and surface behavior.
- `docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`: rebuild touches write-heavy paths; consume ADR-0133 writer discipline, no fresh writer connections, no locks across `.await`.
- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`: search by substrate primitives. DOS-832 extends existing workspace ingestion/claim feedback substrate rather than inventing a new import/correction system.
- `.docs/evals/evaluation-evidence-contract.md` and `.docs/evals/fixture-governance.md`: real-data proof must be PII-free, repo-relative, hash-bound, and lintable.
- Security L0 cycle: exported DB copies, correction sidecars/replay journals, and Live file-swap exclusivity are explicit AC/test surfaces, not posture-only bullets.
- Adversarial L0 cycle: DOS-831 authority correction is a precondition, derived producer outputs are not canonical inputs, legacy rebuild gaps are either covered or reported as degraded, and correction replay requires stable event IDs plus a named `services::claims` replay helper.
- ADR-0048: current rebuild principle is partial and must be amended.
- ADR-0107: `DataSource::WorkspaceFile { kind }` exists and sets file-derived facts as reference posture.
- ADR-0123/0126: corrections are typed feedback against immutable claim core; no direct claim mutation.
- ADR-0131: canonicalization/dedup must respect tombstone shadowing and ambiguity.
- ADR-0133: writer queue owns serialization and telemetry, not priority lanes or extra connections.

---

## §9 Definition of Done

- L0 packet passes K-in, feasibility, security-lens, and external/project-approved adversarial review.
- L1 implementation either waits for DOS-628 or remains explicitly non-releaseable until DOS-628 sidecar semantics exist.
- Fresh-schema rebuild succeeds in Replica mode with generic fixtures and real-workspace PII-free proof.
- Correction replay proves typed feedback, tombstone, contradiction, and ambiguity behavior.
- ADR-0048 and operator docs are updated.
- Full gates pass.
