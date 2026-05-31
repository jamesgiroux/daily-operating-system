# L0 review cycle 3 — DOS-209 plan v3 — architect-reviewer

**Reviewer:** architect-reviewer (substrate/schema profile, cycle 3; cycle-2 was APPROVE)
**Plan revision under review:** v3 (2026-04-29)
**Verdict:** APPROVE

## Cycle-3 delta verification

### NF1 (audit script + snapshot + no-drift test) — architecturally sound: yes

Inspection: The closure mechanism is a three-part construct, all three pieces present and load-bearing.

1. `scripts/dos209-mutation-audit.sh` is a Bash entry that runs an embedded Python program. The script is not a `rg` pipe; it is an actual Rust-syntax parser — `FUNCTION_RE` finds top-level `fn` definitions, `first_code_brace`/`matching_brace` walk the function body with a string/char/raw-string/line-comment/block-comment state machine to identify the function's full body, `cfg_test_spans` excludes `#[cfg(test)] mod` regions, and a per-line attribute check excludes individual `#[test]` / `#[tokio::test]` items. Inside each function body, eight regex `KIND_PATTERNS` are evaluated (`D`, `SQL`, `TX`, `SIG`, `FS`, `BG`, `EXT`, `C`) and the first matching line per kind is captured as evidence. This is materially stronger than a bare `rg` over the file — a `rg` line scan can't tell that a `db.upsert_*` call is inside a `#[cfg(test)]` mod; this scanner can.

2. The committed snapshot `src-tauri/tests/dos209_mutation_catalog.txt` (233 lines, 228 mutator rows + 5 comment headers) is byte-identical to the live script output. I ran `diff <(bash scripts/dos209-mutation-audit.sh) src-tauri/tests/dos209_mutation_catalog.txt` and confirmed exact match. The no-drift CI test (`mutation_catalog_no_drift` in §9) is therefore not stale-on-arrival; it will pass at PR open and any future drift will fail it.

3. The five mutators the cycle-2 challenge cited as missing are all present in v3's snapshot:
   - `accounts::snooze_triage_item:1941` (D+C)
   - `emails::unarchive_email:1124` (D+EXT)
   - `emails::unsuppress_email:1181` (D)
   - `emails::pin_email:1186` (D+SIG)
   - `entity_linking::rules::p2_thread_inheritance::evaluate:9` (D+BG)

   The cycle-2 disagreement is structurally closed — not by re-asserting "exhaustive" prose, but by mechanically generating the catalogue from a parser whose taxonomy directly encodes §3's mutation definition.

Match-to-taxonomy: the eight `KIND_PATTERNS` map 1:1 to §3's mutation taxonomy declaration (`insert/update/upsert/delete` ⇒ D, raw SQL `execute`/`execute_batch` ⇒ SQL, transaction wrapper ⇒ TX, signal emission ⇒ SIG, filesystem write/delete/rename ⇒ FS, background queue ⇒ BG, external side effect ⇒ EXT, direct clock/RNG ⇒ C). The D regex is generous on purpose (every state-changing verb in the codebase's vocabulary including `archive`, `merge`, `tombstone`, `accept_`, `reject_`, etc.); the C regex catches `Utc::now()`, `chrono::Utc::now()`, `rand::thread_rng()`, `thread_rng()`, and `rand::rng()`, which is exactly what §6's no-`Utc::now`/`thread_rng` lint enforces. The `all kinds == C` filter at line 270 correctly drops pure-read functions whose only flag is a clock read used for a return value, without a domain mutation. This taxonomy match is what the cycle-2 challenge was asking for and it is structurally present.

Failure modes for the script approach: an allow-list lint always carries the residual risk that a *new* file in `services/` containing only kinds the regex doesn't recognize would not appear. The current regex set is broad enough that new mutators using existing verbs are caught; entirely novel side-effect kinds (e.g., a new IPC channel) would not be. This is a known property of regex-based catalogue generation and is the same residual risk every grep-CI invariant in the wave plan carries. It is not a v3 defect — it is the structural shape of the closure the plan intends. The §9 `dos209_mode_boundary` runtime test is the second layer that catches mutators the structural lint missed (any `pub` function under `services/` invoked under `Evaluate` that does not return `WriteBlockedByMode` fails). Two layers, regex-based catalogue plus runtime evaluate boundary, are the right composition for this kind of sweep.

Architectural verdict: the closure is sound. The script is the catalogue generator, the snapshot is the reviewable artifact, the no-drift test is the CI gate, and the runtime evaluate test is the second layer. This is the correct architectural shape for a 24-module 228-mutator sweep.

### NF2 (landing order amendment) — preserves end-state alignment: yes

Inspection: §1 includes a verbatim cite block of the 2026-04-29 L6 amendment, not a paraphrase, with the load-bearing line "**Amended landing order (L6, 2026-04-29):** W2-B (DOS-259) lands first. W2-A (this issue) rebases on top once the PTY orchestration extraction has been merged." The Linear ticket cite block format `<issue id="d4e527db-b0d5-4206-bc6f-49ee6c227f84">DOS-259</issue>` is preserved, anchoring the cite to the actual Linear issue, not a free-text reference. §1 also keeps the original "Landing order (original): this issue first" line and adds "Landing order (amended 2026-04-29 per L6 decision):" beneath it, so the contract evolution is auditable.

§7 paragraph 3 restates the same amendment block verbatim and binds it to W2-A's coordination contract: "W2-B-first is now the frozen DOS-209 contract per the 2026-04-29 L6 amendment, not merely coordination guidance." The phrase "frozen contract per the L6 amendment" is the right precision — v2's "per coordination guidance" was the language the cycle-2 challenge correctly flagged as ticket-conflict. v3 fixes this by asserting that the ticket itself was amended, not that the wave plan is overriding the ticket.

Downstream consumer impact: I re-walked the W3/W4/W5 consumer mapping under the W2-B-first ordering.

- W3-A (registry, DOS-210) consumes `AbilityContext` composed over `ServiceContext`. Whether W2-B or W2-A lands first, the eight-carrier `ServiceContext` shape is the same. `IntelligenceProvider` is on `AbilityContext`, supplied by W2-B. W2-B-first means the provider trait exists when W3-A composes its ability context, which is strictly better for W3-A's compile path than W2-A-first.
- W3-B (claims, DOS-211) consumes `ServiceContext` for `commit_claim`. Carriers (`db`, `signals`, `intel_queue`, `mode`, `clock`, `rng`, `external`, `tx`) are the same regardless of landing order. No impact.
- W3-G (`source_asof`/freshness) reads `ctx.clock`. Same regardless. No impact.
- W4-A (Trust Compiler) reads `ctx.clock` and `ctx.services.external.*`. Same regardless. No impact.
- W4-C (`invoke_ability` bridge) consumes both `PlannedMutationSet` (from W3-A) and `ProvenanceRef` (from W3-B). Independent of W2 landing order.
- W5 pilots consume Live-only construction. Independent of landing order.

The flip is architecturally net-positive (smaller mutation surface for the catalogue sweep after PTY orchestration extraction) and downstream-neutral (no consumer's compile dependency cares which W2 PR lands first — both shapes are frozen). The amendment preserves end-state alignment.

§7 paragraph 4 also retains the cross-file guard-preservation protocol from cycle 2: "If W2-B moves any mutation path, W2-A updates the catalogue and the moved function must keep the guard before L2." This is the right escape hatch — if W2-B's PTY extraction touches a function in the catalogue (e.g., the residual mutator list `enrich_entity`, `persist_entity_keywords`, etc.), W2-A re-runs the audit script, regenerates the snapshot, and the moved function's first line still reads `ctx.check_mutation_allowed()?`. Mechanically enforceable via the no-drift test.

### NF3 (full-suite CI command) — no architectural conflicts: yes

Inspection: §9 final paragraph reads verbatim:

> CI command: `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml && pnpm tsc --noEmit`. This restores the full regression suite and closes cycle-2 challenge NF3. Additional DOS-209 evidence command: `cargo test --manifest-path src-tauri/Cargo.toml dos209`; this targeted invocation supplements the full regression suite and is not a replacement.

This matches CLAUDE.md's "Definition of Done" exactly (`cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit`) and matches the W2 merge gate line "`cargo clippy -D warnings && cargo test && pnpm tsc --noEmit` green" in `v1.4.0-waves.md` §"Wave 2". The targeted `cargo test ... dos209` invocation is correctly framed as "supplements the full regression suite and is not a replacement," which is the right relationship — the targeted test set is evidence for the catalogue/mode-boundary/trybuild work, not a substitute for full regression.

No conflict with the W2 merge gate (the gate command is the same string), no conflict with §6's clippy budget (the lint command in §6 is a stricter `--workspace --all-features --lib --bins` variant; the §9 CI command is the floor that ships in PR CI). No conflict with any other section.

## End-state alignment (cycle 3)

v3 freezes the same seam shape v2 froze (eight carriers, `IntelligenceProvider` excluded from `ServiceContext` and on `AbilityContext`, concrete `ExternalClients` with named fields, `TxCtx` with no external/provider) and adds three structural reinforcements: a programmatic catalogue generator + reviewable snapshot + no-drift CI test (NF1), a verbatim L6 amendment cite that aligns ticket and wave plan (NF2), and the full regression suite restored to the §9 CI line (NF3). None of the three changes alters a carrier, renames a type, or shifts an ownership boundary. They add enforcement and align contract truth — exactly the cycle-3 mandate.

Downstream W3/W4/W5 consumers can adopt v3's frozen seam without breaking changes: W3-A's registry composes `AbilityContext` over the same eight carriers, W3-B's `commit_claim` writer takes the same `&ServiceContext`, W3-G consumes `ctx.clock`, W4-A consumes `ctx.clock` and `ctx.services.external.*`, W4-C consumes `PlannedMutationSet`/`ProvenanceRef` from W3-A/W3-B over the same `ServiceContext`/`TxCtx`, and W5 pilots use `new_live` exclusively. The W2-B-first landing order is downstream-neutral; W2-B opens a smaller mutation surface for W2-A but the merged W2 outcome (carrier shape) is identical regardless of order. No new architectural drift was introduced by cycle-3's narrow scope.

## Fresh findings (if any)

None at any severity.

One informational note (not blocking, not action-required): the Python-embedded-in-Bash structure of `scripts/dos209-mutation-audit.sh` will need a `python3` interpreter on the PR CI runner. The CI runner already has Python available for other scripts, so this is not a CI surprise; mentioning it for awareness only.

## Verdict rationale

APPROVE. All three L6-authorized cycle-3 changes are mechanically realized and architecturally sound.

NF1's audit script is a real Rust-syntax parser plus a deterministic taxonomy regex pass, not a `rg` veneer. The committed snapshot is byte-identical to live script output (verified by `diff`), so the no-drift test will pass at PR open and break on any future drift. All five mutators the cycle-2 challenge cited as missing are now in the snapshot. The taxonomy regex set maps 1:1 to §3's mutation definition. The two-layer composition (structural lint + runtime evaluate boundary test) is the correct architectural shape for catching missed mutators; the residual allow-list-lint risk is bounded by the runtime layer.

NF2's amendment cite is verbatim, anchored to the Linear issue id, preserves the original-vs-amended audit trail, and asserts "frozen contract per the L6 amendment" rather than v2's "per coordination guidance" (the language the cycle-2 challenge correctly flagged). The W2-B-first landing order is architecturally net-positive (smaller mutation surface after PTY extraction) and downstream-neutral for W3/W4/W5 consumers. The cross-file guard-preservation protocol is retained.

NF3's CI command matches CLAUDE.md's Definition of Done and the W2 merge gate. The targeted `dos209` test invocation is correctly framed as supplementary evidence, not a replacement. No conflict with any other section.

The seam shape from v2 is unchanged, so v2's APPROVE-level analysis still holds: eight carriers frozen, `IntelligenceProvider` exclusion explicit, `ExternalClients` concrete, `TxCtx` enforcing the ADR-0104 transaction ban architecturally, capability boundary enforced by trybuild, performance budget falsifiable, fallback paths named. The cycle-3 changes are additive enforcement and contract alignment, not architectural rework.

## If APPROVE

All three L6-authorized changes preserve architectural soundness + downstream consumer compatibility. Plan is frozen.
