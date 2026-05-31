# v1.4.5 W1 — Contract & Registry retro

**Wave version:** v1.4.5 W1 — Contract & Registry
**Closed:** 2026-05-21 (PR [#345](https://github.com/jamesgiroux/daily-operating-system/pull/345) merged)
**Author:** orchestrator (Claude Code)
**Linear project:** [v1.4.5 — Workspace Memory Refactor](https://linear.app/a8c/project/v145-workspace-memory-refactor-cdb9d2c17102)
**Sourced from:** `HANDOFF-2026-05-21.md` + W1 reviews under `.docs/plans/v1.4.5-workspace-memory/reviews/`

## What shipped

| Lane | Spec | L0 | L1 | L2 | Tests |
|---|---|---|---|---|---|
| **W1-A** Lifecycle + contracts | [DOS-463](https://linear.app/a8c/issue/DOS-463) | UNANIMOUS APPROVE (cycle 6) | PASS (commit `34a4cf37`) | UNANIMOUS APPROVE + `/cso` APPROVE (cycle 1) | 14 integration |
| **W1-B** Registry migration | [DOS-464](https://linear.app/a8c/issue/DOS-464) | PARTIAL CONVERGENCE (cycle 5; architect + `/cso` APPROVE) | PASS | UNANIMOUS APPROVE after 3 cycles | 27 unit + 12 security fixtures |
| **W1-C** Links + ingestion runs | [DOS-465](https://linear.app/a8c/issue/DOS-465) | PARTIAL CONVERGENCE (cycle 5; architect APPROVE) | PASS | UNANIMOUS APPROVE after 3 cycles | 25 unit |

**76 W1 tests green.** `cargo clippy --lib -- -D warnings` green. Substrate-reinvention CI gate active. Ephemeral-refs lint green.

### Migrations shipped (v250–v254)

| Slot | Lane | Table / change |
|---|---|---|
| v250 | W1-A | `workspace_file_lifecycle` (16 cols + 3 indexes) |
| v251 | W1-A | `ALTER TABLE workspace_file_lifecycle ADD COLUMN category TEXT` |
| v252 | W1-B | `workspace_source_registry` (7 source seeds) + `workspace_category_registry` (18 category seeds) |
| v253 | W1-C | `document_ingestion_runs` (UNIQUE partial idempotency index on success rows) |
| v254 | W1-C | `document_entity_links` (UNIQUE partial tombstone index on active rows + rejected-lookup index) |

Cycle-11 renumbered from stale v200–v204 to v250–v254 (live-dev ceiling at v240). Cycle-12 substrate sweep dropped `SourceType` mirror + local `SourceAttribution` reinvention; consumes canonical primitives instead.

## Path-α filed

10 path-α nits surfaced at L2 — none blocking. Tracked at [DOS-751](https://linear.app/a8c/issue/DOS-751) (medium priority). Summary:
- `WorkspaceCategory::Other(known_slug)` round-trip asymmetry.
- u64 → i64 inode/device conversion at W1-C persistence boundary (`lifecycle.rs:69-70`).
- `migrations_slice_max_version_is_at_least_251` tautology test.
- Missing trybuild compile-fail tests.
- Substrate-reinvention grep gate scope (catches `pub struct/enum/trait` only; not `pub type` / `pub(crate)` / renamed clones).
- 5 additional `start_run` / `add_link` cleanup items.

## What broke + how we recovered

### Cycle-11 migration slot renumber

**Original framing:** v200–v204 reserved for v1.4.5 per cycle-2 amendment.

**Reality:** W0 reuse-audit's "v178 ceiling" claim missed live-dev ceiling at v240 because the audit walked the migrations directory listing without grepping the `MIGRATIONS` slice version literals. Cycle-11 caught it and renumbered to v250–v254. Cross-version ceiling (v1.4.4 v245) was missed because the audit only looked at `dev`, not sibling wave branches. This is what triggered the PR #345 BLOCKED-ON note (resolved after v1.4.4 W1 substrate gaps merged).

### Cycle-12 substrate-reinvention sweep

**Original framing:** W1-A would ship a local `SourceType` enum + local `SourceAttribution` struct for workspace-file source provenance.

**Reality:** Both already exist as canonical primitives in `abilities_runtime`. Cycle-6 originally promoted `SourceType` into W1-A's `contracts.rs`, but cycle-12 grep-check found the canonical definitions and rewrote the contract to consume them. Local mirror dropped. Substrate-reinvention CI gate now ships as a structural prevention.

### Cycles 3–5: stale propagation

Cycles 3, 4, and 5 all surfaced the same class of finding: amendments touched specific lane sections but didn't propagate to every active reference. Cycle 3 added `&dyn SignalEmitter` to `link::override_link` but left the trait in W2-A's `pipeline.rs`; cycle 4 relocated the trait to W1-A's `contracts.rs` but missed downstream lane bodies; cycle 5 enumerated the trait method surface and swept all references in lockstep.

**Lesson learned:** When amending an API signature, contract location, or scope language, the same commit must grep every active reference and update them in lockstep.

## K-out class patterns

Six patterns captured for compound-engineering:

1. **W0 reuse-audit ceiling check must grep `MIGRATIONS` slice version literals** (not just the migrations subdirectory listing). The audit's "v178 ceiling" claim missed v240; forced cycle-11 renumber.
2. **Substrate-reinvention is a recurring L0 class.** Fired 3× during W1-A L0 (`TrustFactorInput`, `SourceAttribution`, `SourceType`). Structural prevention via CI grep gate now shipped; downstream lanes inherit.
3. **Wave-plan substrate-promoting amendments need K-in grep** against canonical `abilities-runtime` before naming new types. Cycle 6's original promotion of `SourceType` was the trigger.
4. **Changelog folds without paired body edits** — recurring author error during L0 cycles 4/5 and L2 cycle 1. K-out: every changelog entry must cite the §N:line edited in the same commit.
5. **L2 reviewer dissent is signal**, not a tie to break. Codex caught AC violations (`complete_run` `DbError`, URL-encoded lex vacuity) that architect + code-reviewer + `/cso` all missed. Per memory `feedback_reviewer_dissent_is_signal`, dissent wins.
6. **W0 reuse-audit must check sibling wave branches' `MIGRATIONS` slice tails**, not just `dev`. Cycle-11 caught the dev v240 ceiling; the cross-version v1.4.4 v245 ceiling was missed because the audit only looked at dev. This is what triggered the PR #345 BLOCKED-ON note.

## What's next

**W2 (DOS-466, 467, 468, 469)** ran on substrate W1 shipped. W2 is also now closed; see `retro-W2.md`.

**W3 (DOS-470, 471, 489)** consumes the `Extractor` and `SignalEmitter` traits + the `WorkspaceClaimProposal`, `FileIdentity`, `RejectionReason`, `SourceType` types W1-A ships in `contracts.rs`. W3-A (claim extraction) merges alone before W3-B + W3-C run in parallel.

## Related

- `HANDOFF-2026-05-21.md` — full session handoff doc
- W1 L0 packets: `L0-packet-W1-A-DOS-463.md`, `L0-packet-W1-B-DOS-464.md`, `L0-packet-W1-C-DOS-465.md`
- W1 L2 reviewer transcripts under `reviews/packet-W1-{A,B,C}-*`
- Proof bundle: `proof-bundle-W1-A.md`
- [DOS-751](https://linear.app/a8c/issue/DOS-751) — path-α nits
