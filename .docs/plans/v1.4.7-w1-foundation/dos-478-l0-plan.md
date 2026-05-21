# DOS-478 — MCP v2 tool taxonomy + host-selection contract — L0 plan packet

**Wave:** v1.4.7 W1-B (parallel to W1-A)
**Lane spec:** [DOS-478](https://linear.app/a8c/issue/DOS-478)
**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Agent W1-B — DOS-478"
**Authoring discipline:** narrow-scoped per K-in lessons; substrate gaps file as separate Linear tickets.

## Cycle-2 changelog (2026-05-20)

Cycle-1 verdicts: all 4 reviewers NEEDS-CHANGES (codex challenge, architect, plan-ceo-review, plan-devex-review). 13 findings; 5 convergent (3+/4):
1. Tool inventory ambiguous (challenge + ceo + architect) — fixed by §5 exact required-name table
2. `TaxonomyError` needs parse/invalid variants (challenge + architect + devex) — fixed by §1 #2 expanded variant list + Display impl
3. Fixture density (challenge + ceo) — ADR-0128 §B requires ≥ 2 positive + ≥ 2 negative per tool; fixed by AC-5 update + per-entry YAML fixture-stub fields
4. `Gateway::seal()` pattern (architect + devex) — fixed by Q2 resolution + AC-7
5. AC-4 weak (ceo + challenge) — ADR-0128 needs BOTH broad-corpus AND adjacent-wrong-tool; fixed by AC-4 split

Plus 8 singletons all folded: Side serialization (`SubmitCorrection` not `Submit`); YAML hardening (duplicate-name + unknown-field rejection); cross-conversation continuity in description discipline; Result not panic (matches W1-A); `TaxonomyError::Display` spec'd; nearest-candidate suggestions for typo DX; local-dev FS override env-gated; exact per-tool scope mapping.

## 1. What this lane ships

W1-A already shipped the typed substrate (commit `45b3f53d` + `228a17f3`):
- `src-tauri/src/services/mcp_v2/contracts.rs` — `ToolDescription`, `ScopedName`, `Scope`, `Side` (`Read | SubmitCorrection | Write` — note: NOT `Submit`), `ParamSpec`, `ReturnSpec`, `ToolExample`
- `src-tauri/src/services/mcp_v2/taxonomy.rs` — `TaxonomyCatalog` trait + `TaxonomyError::HandlerCatalogMismatch` + handler-author cursor rustdoc (~130 lines)

W1-B fills:

1. **`src-tauri/resources/mcp_v2/tool_descriptions.yaml`** (NEW) — version-controlled catalog. Entry shape:
   ```yaml
   - name: dailyos.read.account_status
     side: Read                     # MUST be "Read" | "SubmitCorrection" | "Write" — verbatim per Side enum
     summary: "Returns current state of an account the user works with."
     when_to_call: |
       When the user asks about a specific account they have a working
       relationship with — status, recent signals, open commitments,
       champion engagement, contract state. DailyOS persists context
       across conversations and updates as work evolves; the host model
       can rely on this for cross-conversation continuity per ADR-0128 §6.
     when_NOT_to_call: |
       Do NOT call for accounts the user has no working relationship
       with — DailyOS is for the user's working understanding of their
       professional world, not broad enterprise / web corpus search.
       Use enterprise search, web search, or Glean for broad corpus
       lookups. Do NOT call for org-wide policy or how-to questions
       (use Glean / enterprise search).
     scopes_required:
       - dailyos.read.account_status
     parameters:
       - { name: subject, schema: { type: string }, required: true, description: "Account name or handle" }
     returns:
       schema: { type: object, additionalProperties: false }
       description: "Account status payload with claim attribution + freshness per ADR-0105"
     selection_fixtures:                          # ≥ 2 positive + ≥ 2 negative per ADR-0128 §B (cycle-1 ceo + challenge)
       positive:
         - { prompt: "What's going on with Acme?", expected_tool: dailyos.read.account_status }
         - { prompt: "How are things looking with the Hooli deal?", expected_tool: dailyos.read.account_status }
       negative_broad_corpus:                     # MUST select non-DailyOS tool
         - { prompt: "Search the web for SaaS pricing benchmarks", expected_tool_class: external }
         - { prompt: "What's the company's vacation policy?", expected_tool_class: external }
       negative_adjacent_tool:                    # MUST select a DIFFERENT DailyOS tool
         - { prompt: "What did I write about the Q1 readout?", expected_tool: dailyos.search.workspace_memory }
   ```

2. **`src-tauri/src/services/mcp_v2/taxonomy.rs` extensions** — W1-A shipped the trait; W1-B fills the loader implementation. Adds:
   - `TaxonomyError` new variants per cycle-1 architect + challenge + devex: `ParseFailed { error: String }`, `InvalidName { name: String, reason: String }`, `InvalidScope { tool: ScopedName, scope: Scope }`, `DuplicateName { name: ScopedName }`, `SideMismatch { handler: ScopedName, expected: Side, actual: Side }`, `FixtureCoverage { tool: ScopedName, missing: &'static str }`
   - `impl std::fmt::Display for TaxonomyError` — operator-readable messages with concrete fix suggestions per devex cycle-1 #2+#4 (nearest-candidate suggestions on `HandlerCatalogMismatch` typos)
   - `pub struct YamlTaxonomyCatalog { entries: HashMap<ScopedName, ToolDescription>, fixtures: HashMap<ScopedName, SelectionFixtures> }`
   - `pub fn load_embedded() -> Result<YamlTaxonomyCatalog, TaxonomyError>` — reads embedded YAML via `include_str!`
   - `pub fn load_from_path(path: &Path) -> Result<YamlTaxonomyCatalog, TaxonomyError>` — env-gated dev override (cycle-1 devex #5; mirrors `src-tauri/src/presets/loader.rs` precedent)
   - `impl TaxonomyCatalog for YamlTaxonomyCatalog { ... }` — `validate_against_handlers` returns `Result<(), TaxonomyError>` (NOT panic; cycle-1 devex #1)

3. **Gateway integration** — additive line-bounded edits to `gateway.rs`:
   - `pub fn set_taxonomy(&mut self, catalog: Arc<dyn TaxonomyCatalog>)` — register the catalog reference
   - `pub fn seal(&self) -> Result<(), TaxonomyError>` — called by `main.rs` after `Gateway::register()` calls complete; invokes `catalog.validate_against_handlers(&self.handlers_as_slice())` and returns the error structured (NOT panic per cycle-1 devex resolution Q3)
   - main.rs (or wherever Gateway is instantiated in production) MUST call `gateway.seal()?` after registration; missing `seal()` is a startup logic error operators detect via integration test.

4. **`src-tauri/tests/dos478_taxonomy_catalog_test.rs`** (NEW) — integration tests covering AC-1..AC-10.

## 2. What this lane does NOT ship

- Per-tool handler bodies (W2/W3/W4 scope).
- Host-selection eval harness (W5-A / DOS-481 scope; this lane ships fixture STUBS in YAML, DOS-481 implements the eval runner against them).
- ADR-0128 changes (already amended in W0 commit 45b3f53d).
- Production signal bus wiring (W1.5 path-α from W1-A).

## 3. Frozen substrate citations

- **W0 contracts (`contracts.rs` frozen at commit `45b3f53d` + cycle-7/8/9 amendments)**: `ToolDescription`, `Side` (Read | SubmitCorrection | Write — verbatim variant names), `ScopedName`, `Scope`, `ParamSpec`, `ReturnSpec`, `ToolExample`, `CANONICAL_NAMESPACE = "dailyos"`, `Verb` enum (Read | Write | Submit | Search | List | Get | Prepare).
- **W1-A taxonomy.rs (committed `228a17f3`)**: `TaxonomyCatalog` trait + handler-author rustdoc.
- **ADR-0102 §E** — scope namespace freeze.
- **ADR-0128 §3 + §6 + cycle-7 §B amendment** — tool descriptions are product copy; cross-conversation continuity named as affordance; host-selection eval (DOS-481) requires ≥ 2 positive + ≥ 2 negative fixtures per tool with broad-corpus AND adjacent-wrong-tool negatives.
- **ADR-0083** — product vocabulary discipline (warm + specific + self-evident; no system terms).
- **`src-tauri/src/presets/loader.rs`** — embedded + filesystem-override precedent for dev-iteration ergonomics (cycle-1 devex #5).

## 4. K-in citations (present-on-dev)

- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — grep substrate; cycle-2 cites W1-A trait + W0 contracts by file:line.
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` — fail-fast boot via structured `Result` is the structural pattern (cycle-1 devex #1).

## 5. Frozen tool inventory (10 entries — cycle-2 #1)

The catalog ships entries for EXACTLY these 10 tool names. AC-1 enforces the count + identity:

| # | Tool name | Side | Scopes required | Owner wave |
|---|---|---|---|---|
| 1 | `dailyos.read.account_status` | Read | `dailyos.read.account_status` | W2 (out-of-scope handler — discovery surface from DOS-172 ranked entity lookup) |
| 2 | `dailyos.read.daily_briefing` | Read | `dailyos.read.daily_briefing` | W2-A (DOS-175) |
| 3 | `dailyos.read.meeting_briefing` | Read | `dailyos.read.meeting_briefing` | W2-A (DOS-175) |
| 4 | `dailyos.read.portfolio_attention` | Read | `dailyos.read.portfolio_attention` | W2-C (DOS-173) |
| 5 | `dailyos.search.workspace_memory` | Read | `read.workspace_graph` (v1.4.5 grandfathered) | W3-A (DOS-479) |
| 6 | `dailyos.read.workspace_source_provenance` | Read | `dailyos.read.workspace_source_provenance` | W3-A (DOS-479) |
| 7 | `dailyos.write.place_document` | Write | `write.workspace_place_document` (v1.4.5 grandfathered) | W3-B (DOS-480) |
| 8 | `dailyos.submit.note` | SubmitCorrection | `dailyos.submit.note` | W4-A (DOS-167) |
| 9 | `dailyos.submit.action` | SubmitCorrection | `dailyos.submit.action` | W4-B (DOS-169) |
| 10 | `dailyos.submit.action_status` | SubmitCorrection | `dailyos.submit.action_status` | W4-C (DOS-170) |

Note: pagination/list surface (DOS-172) is a discovery refinement applied across read tools, NOT a separate tool name. The frozen 10 is the surface for v1.4.7. Future tools amend the catalog at their own L0.

## 6. Acceptance criteria (cycle-2 tightened)

- **AC-1 Inventory + load.** `YamlTaxonomyCatalog::load_embedded()` succeeds; resulting catalog has EXACTLY the 10 entries from §5 (assert by name + count). Extra or missing entries → test failure.
- **AC-2 Naming convention + Side serialization.** Every entry `name` matches regex `^dailyos\.(read|write|submit|search|list|get|prepare)\.[a-z][a-z0-9_]*$`. Every entry `side` deserializes to `Side::Read | Side::SubmitCorrection | Side::Write` — YAML strings use verbatim variant names (cycle-1 architect #4). Wrong YAML `side: Submit` → `TaxonomyError::ParseFailed`.
- **AC-3 Scope mapping exact** (cycle-1 challenge #2). Each entry's `scopes_required` matches the §5 table verbatim. Test asserts per-tool: catalog scope set equals expected scope set (NOT just "in allowlist").
- **AC-4 Displacement framing** (cycle-1 ceo + challenge convergent). Every entry's `when_NOT_to_call` MUST contain BOTH (a) at least one broad-corpus keyword: `broad`, `enterprise search`, `web search`, `Glean`, `corpus`; AND (b) at least one adjacent-tool framing: `wrong tool`, `different tool`, `instead use`, `use ... for`, or an explicit `dailyos.*` cross-reference. Test fails per-entry if either is missing.
- **AC-5 Fixture coverage** (cycle-1 ceo + challenge convergent). Every entry has `selection_fixtures`: `positive` with ≥ 2 entries, `negative_broad_corpus` with ≥ 2 entries, `negative_adjacent_tool` with ≥ 1 entry (≥ 2 if the tool has an adjacent counterpart in §5; the `dailyos.write.place_document` lone-write tool may have only 1). DOS-481 W5-A will consume these fixtures to run the host-selection eval. `TaxonomyError::FixtureCoverage` on shortfall.
- **AC-6 Continuity affordance** (cycle-1 ceo #5). Every Read entry's `when_to_call` MUST mention continuity using one of: `cross-conversation`, `persists context`, `updates as work evolves`, `working understanding ... over time`. Per ADR-0128 §6 named-affordance contract.
- **AC-7 Boot validation via `Gateway::seal()` returning `Result`** (cycle-1 architect #2 + devex #1). `main.rs` calls `gateway.seal()?` after registration. Mismatch → `Err(TaxonomyError::HandlerCatalogMismatch { handler, catalog_entry, nearest_candidate: Option<ScopedName> })` per devex #4 nearest-suggestion DX. No `panic!()` at startup.
- **AC-8 YAML hardening** (cycle-1 challenge #4). Loader rejects: duplicate `name` entries (`DuplicateName`), unknown YAML fields (serde deny_unknown_fields), empty `summary` / `when_to_call` / `when_NOT_to_call`. Per-failure test asserts the specific variant.
- **AC-9 `TaxonomyError::Display` operator-readable** (cycle-1 devex #2). Each variant's `Display` impl includes: what failed, where (handler name or YAML position), suggested fix. Tests assert the message contains the suggestion substring.
- **AC-10 Local-dev FS override** (cycle-1 devex #5). `YamlTaxonomyCatalog::load_from_path(path: &Path)` reads YAML from filesystem; production uses `load_embedded()`. Env-gate via `DAILYOS_MCP_TAXONOMY_PATH`; mirrors `src/presets/loader.rs` precedent. Override never active in release builds.
- **AC-11 Required checks.** `cargo clippy --lib -- -D warnings && cargo test && pnpm tsc --noEmit` green. `cargo test dos478_taxonomy_catalog_test` passes (AC-1..AC-10 covered).

## 7. Files owned (W1-B)

| File | State | Owner |
|---|---|---|
| `src-tauri/resources/mcp_v2/tool_descriptions.yaml` | NEW (10 entries) | exclusive |
| `src-tauri/src/services/mcp_v2/taxonomy.rs` | W1-A shipped trait; W1-B adds variants + Display + YamlTaxonomyCatalog + load_embedded + load_from_path | shared (additive) |
| `src-tauri/src/services/mcp_v2/gateway.rs` | additive — `set_taxonomy` + `seal` | shared (line-bounded additive) |
| `src-tauri/src/lib.rs` (or main.rs equivalent) | additive — call `gateway.seal()?` after registration | shared (line-bounded additive; verify exact path at L1) |
| `src-tauri/tests/dos478_taxonomy_catalog_test.rs` | NEW | exclusive |
| `src-tauri/Cargo.toml` | additive — `serde_yaml = "0.9"` | shared (additive) |

## 8. Test plan (covers AC-1..AC-10)

- **load_embedded + 10 entries** (AC-1): assert count + name set equals §5.
- **naming + side serialization** (AC-2): per-entry regex match; positive YAML `side: Submit` → ParseFailed.
- **scope mapping** (AC-3): per-tool assert scope set == §5 expected.
- **displacement framing** (AC-4): per-entry assert both keyword groups present in `when_NOT_to_call`.
- **fixture coverage** (AC-5): per-entry assert positive ≥ 2, negative_broad_corpus ≥ 2, negative_adjacent_tool ≥ 1 (≥ 2 where adjacent exists).
- **continuity affordance** (AC-6): per Read entry assert continuity keyword in `when_to_call`.
- **boot validation Result** (AC-7): stub handler set covering 10 → `seal()` returns `Ok(())`. Stub missing one → `Err(HandlerCatalogMismatch { handler: <name>, nearest_candidate: Some(<closest>) })`.
- **YAML hardening** (AC-8): duplicate-name YAML → `DuplicateName`; unknown-field YAML → `ParseFailed`; empty-when_to_call YAML → `ParseFailed`.
- **Display** (AC-9): per-variant assert message contains the suggestion substring.
- **FS override** (AC-10): `load_from_path(temp_yaml)` succeeds; env var triggers override; release build ignores env.

## 9. Security gates

`/cso` not mandatory (no security-annotated work). `/plan-ceo-review` + `/plan-devex-review` mandatory per wave plan (tool taxonomy IS the product-strategy + DX surface).

## 10. Path-α (file as Maintenance Linear tickets)

- Live host-model selection eval — DOS-481 (W5-A).
- Per-tool prompt fixture refinement — DOS-481.
- YAML schema JSON Schema export for IDE tooling — post-v1.4.7.
- Operator runbook for catalog edits — DX docs, ship with W1-B PR but as separate file.

## 11. Depends-on

- **W1-A merged** (mandatory): YAML loader needs `TaxonomyCatalog` trait + ToolDescription types. W1-A at unanimous L2 APPROVE on branch `v1.4.7-w1-foundation` (commits 45b3f53d → dc80ff86, 11 commits). User-validated PR open is the gating step.
- v1.4.5 frozen scopes — NOT a hard dep; v1.4.5 grandfathered scope set is a hardcoded allowlist in §5 table.

## 12. Definition of Done

§6 AC-1..AC-11 met; L0 unanimous APPROVE; L2 unanimous APPROVE bounded by AC; commit-msg `L2-status: passed`; PR opens against `dev` after W1-A PR merges (sequential — W1-B depends on W1-A).

## 13. Open questions resolved in cycle 2

| # | Question | Resolution |
|---|---|---|
| Q1 | YAML library | `serde_yaml = "0.9"` (cycle-1 architect — matches serde_json ergonomics) |
| Q2 | Validation hook point | Explicit `Gateway::seal()` after `register()` calls; main.rs calls `gateway.seal()?` (cycle-1 architect #2 + devex #1) |
| Q3 | Panic vs Result | `Result<(), TaxonomyError>` (cycle-1 devex #1; matches W1-A taxonomy.rs which already shipped no-panic) |
| Q4 | Embedded vs FS | BOTH: `load_embedded()` (production, `include_str!`) + `load_from_path()` (dev override env-gated, mirrors `src/presets/loader.rs`) (cycle-1 devex #5) |
