# DOS-478 — MCP v2 tool taxonomy + host-selection contract — L0 plan packet

**Wave:** v1.4.7 W1-B (parallel to W1-A)
**Lane spec:** [DOS-478](https://linear.app/a8c/issue/DOS-478) — Define MCP v2 tool taxonomy + host-selection contract
**Wave plan reference:** `.docs/plans/v1.4.7-waves.md` §"Wave 1 — Contract & Policy" → "Agent W1-B — DOS-478"
**Authoring discipline:** narrow-scoped per `docs/solutions/workflow-issues/l0-review-loop-diminishing-returns-means-scope-is-wrong-2026-05-20.md`. Substrate gaps file as separate Linear tickets.

## 1. What this lane ships

Tool taxonomy as the versioned product-copy surface: a YAML catalog that names every v1.4.7 MCP tool, the host-selection language that determines which tool a host model picks, and the loader + boot-time validation seam W1-A's gateway consumes.

W1-A already shipped the typed substrate (commit `45b3f53d`):
- `src-tauri/src/services/mcp_v2/contracts.rs` — `ToolDescription`, `ScopedName`, `Scope`, `Side`, `ParamSpec`, `ReturnSpec`, `ToolExample` (all frozen, serde-tested)
- `src-tauri/src/services/mcp_v2/taxonomy.rs` — `TaxonomyCatalog` trait + `TaxonomyError::HandlerCatalogMismatch` + handler-author cursor rustdoc

W1-B fills:

1. **`src-tauri/resources/mcp_v2/tool_descriptions.yaml`** (NEW) — version-controlled catalog with one entry per tool W2/W3/W4 ships. Entry shape:
   ```yaml
   - name: dailyos.read.account_status
     summary: "Returns current state of an account the user works with."
     when_to_call: |
       When the user asks about a specific account they have a working
       relationship with — status, recent signals, open commitments,
       champion engagement, contract state. Personal working context, not
       broad corpus.
     when_NOT_to_call: |
       Do NOT call for accounts the user has no working relationship with.
       Do NOT call for broad enterprise search or web search — use other
       tools for those.
     side: Read
     scopes_required:
       - dailyos.read.account_status
     parameters:
       - name: subject
         schema:
           type: string
         required: true
         description: Account name or handle
     returns:
       schema:
         type: object
         additionalProperties: false
       description: Account status payload with claim attribution + freshness
     examples:
       - prompt: "What's going on with Acme?"
         invocation:
           name: dailyos.read.account_status
           arguments:
             subject: acme
         expected_response_shape:
           account_id: <opaque>
           status: <enum>
           as_of: <iso8601>
   ```
   Initial entries cover every tool the v1.4.7 W2-W4 lanes name (10 tools): `dailyos.read.daily_briefing`, `dailyos.read.meeting_briefing`, `dailyos.read.portfolio_attention`, `dailyos.search.workspace_memory`, `dailyos.read.workspace_source_provenance`, `dailyos.write.place_document`, `dailyos.submit.note`, `dailyos.submit.action`, `dailyos.submit.action_status`, plus entry-list discovery surfaces from DOS-172 (entity list, person list, project list).

2. **`src-tauri/src/services/mcp_v2/taxonomy.rs`** — fill the `TaxonomyCatalog` trait with a concrete `YamlTaxonomyCatalog` implementation. Public API:
   - `pub struct YamlTaxonomyCatalog { entries: HashMap<ScopedName, ToolDescription> }`
   - `pub fn load() -> Result<YamlTaxonomyCatalog, TaxonomyError>` — reads embedded YAML resource via `include_str!`, parses with `serde_yaml`, validates every entry's `name` conforms to `dailyos.<verb>.<noun>` per ADR-0102 §E (verbs: read | write | submit | search | list | get | prepare), validates every `scopes_required` Scope is in the canonical allowlist (W0-shipped scope namespace + v1.4.5 grandfathered scopes).
   - `impl TaxonomyCatalog for YamlTaxonomyCatalog { ... }` — `validate_against_handlers` returns `HandlerCatalogMismatch` for first registered handler whose `description().name` has no catalog entry (or catalog entry's `side` mismatches handler's `description().side`); `side_for` returns `Option<Side>` from the entry map.

3. **Build-time YAML inclusion** — `taxonomy.rs` uses `include_str!("../../../resources/mcp_v2/tool_descriptions.yaml")` so the catalog is compiled into the binary (no runtime filesystem dependency, no separate distribution step).

4. **`src-tauri/tests/dos478_taxonomy_catalog_test.rs`** (NEW) — integration test that loads the embedded YAML, runs `validate_against_handlers` against a stub handler set, and asserts:
   - All 10 v1.4.7 tool names present
   - Every name conforms to canonical convention
   - Every scopes_required is in allowlist
   - Every entry has non-empty `when_to_call` and `when_NOT_to_call`
   - Every entry has ≥ 1 example
   - HandlerCatalogMismatch fires on a missing-handler stub

## 2. What this lane does NOT ship

- Per-tool handler bodies — that's W2/W3/W4 scope.
- The host-selection eval harness — that's W5-A (DOS-481) scope.
- New tool descriptions for W2+ tools not in the original v1.4.7 surface — those tools may amend the catalog at their own L0 plan.
- ADR-0128 product-surface frame changes — already amended in W0 (commit `45b3f53d` includes ADR-0128 cycle-7).
- Production signal bus wiring (filed as W1.5 path-α from W1-A L2 cycle).

## 3. Frozen substrate citations

- **W0 contracts (`src-tauri/src/services/mcp_v2/contracts.rs`, frozen at commit `45b3f53d` + cycle-7/8/9 amendments)**: `ToolDescription`, `Side` (Read | SubmitCorrection | Write), `ScopedName`, `Scope`, `ParamSpec`, `ReturnSpec`, `ToolExample`, `CANONICAL_NAMESPACE = "dailyos"`, `Verb` enum (Read | Write | Submit | Search | List | Get | Prepare).
- **W1-A taxonomy.rs (committed `228a17f3`)**: `TaxonomyCatalog` trait + `TaxonomyError::HandlerCatalogMismatch` + handler-author rustdoc (~130 lines).
- **ADR-0102 §E** — scope namespace freeze: new v1.4.7 scopes use `dailyos.<verb>.<noun>`; v1.4.5-shipped scopes stay unprefixed verbatim (`write.workspace_place_document`, `read.workspace_graph`, `read.entity_names`).
- **ADR-0128 §3 + cycle-7 §B amendment** — tool descriptions are product copy; YAML catalog is the version-controlled source; host-selection eval (DOS-481) is the acceptance test.
- **`docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md`** — grep by substrate type; applied here by reading W1-A taxonomy.rs trait surface before drafting the YAML loader.

## 4. K-in citations (present-on-dev)

- `docs/solutions/workflow-issues/k-in-grep-substrate-type-not-proposed-name-2026-05-19.md` — grep substrate; this packet cites W1-A trait + W0 contracts verbatim with line refs.
- `docs/solutions/architecture-patterns/capability-boundary-needs-crate-split-not-grep-2026-05-18.md` — informs the boot-time fail-fast pattern: catalog mismatch is a panic-at-init (structural), not a grep lint (porous).

## 5. Acceptance criteria

- **AC-1 YAML loads** — `YamlTaxonomyCatalog::load()` succeeds on the embedded catalog; returns `Ok(YamlTaxonomyCatalog)`.
- **AC-2 Naming convention enforced at load** — every entry `name` matches regex `^dailyos\.(read|write|submit|search|list|get|prepare)\.[a-z][a-z0-9_]*$`. Out-of-allowlist tool name → `TaxonomyError::HandlerCatalogMismatch` at load time.
- **AC-3 Scope allowlist enforced at load** — every `scopes_required` is one of: `dailyos.<verb>.<noun>` (new v1.4.7 scopes) OR the explicit v1.4.5 grandfathered set (`write.workspace_place_document`, `read.workspace_graph`, `read.entity_names`, `submit.feedback`, `submit.correction`, `submit.dismissal`). Out-of-allowlist scope → load-time error.
- **AC-4 Content discipline** — every entry has non-empty `summary`, `when_to_call`, `when_NOT_to_call`, `returns.description`. `when_NOT_to_call` MUST mention either "broad corpus", "enterprise search", "web search", OR "not for the wrong tool" pattern per ADR-0128 §B displacement framing.
- **AC-5 Example coverage** — every entry has ≥ 1 `ToolExample` with non-empty `prompt`, `invocation.name`, `expected_response_shape`.
- **AC-6 Handler binding contract** — `validate_against_handlers(handlers: &[&dyn McpToolHandler])` returns `Ok(())` if every handler's `description().name` matches a catalog entry AND `description().side == catalog_entry.side`. Returns `HandlerCatalogMismatch` on first mismatch (handler name absent in catalog, OR side mismatch).
- **AC-7 Boot fail-fast** — `validate_against_handlers` is called from gateway boot (W1-A wires this via `Gateway::with_taxonomy(catalog)` constructor, NEW in this lane). Mismatch → `panic!()` at startup with operator-readable error per `TaxonomyError::Display`.
- **AC-8 Required checks** — `cargo clippy -- -D warnings && cargo test && pnpm tsc --noEmit` green. `cargo test dos478_taxonomy_catalog_test` passes.

## 6. Files owned (W1-B exclusive)

| File | State | Owner |
|---|---|---|
| `src-tauri/resources/mcp_v2/tool_descriptions.yaml` | NEW | exclusive |
| `src-tauri/src/services/mcp_v2/taxonomy.rs` | trait + rustdoc shipped by W1-A; W1-B fills `YamlTaxonomyCatalog` impl below the trait declaration | shared (W1-B fills) |
| `src-tauri/src/services/mcp_v2/gateway.rs` | additive — add `Gateway::with_taxonomy(catalog: Arc<dyn TaxonomyCatalog>)` constructor + call `catalog.validate_against_handlers(handlers)` at registration boot | shared (line-bounded additive) |
| `src-tauri/tests/dos478_taxonomy_catalog_test.rs` | NEW | exclusive |
| `src-tauri/Cargo.toml` | additive — add `serde_yaml = "0.9"` if not already present | shared (additive) |

## 7. Test plan

- **Load + naming + scope allowlist** (AC-1/2/3): `tests/dos478_taxonomy_catalog_test.rs` loads embedded YAML, asserts ≥ 10 entries present, asserts every name passes the convention regex, asserts every scope is in allowlist.
- **Content discipline** (AC-4/5): for-each entry assertion that `summary`, `when_to_call`, `when_NOT_to_call` are non-empty + `when_NOT_to_call` contains one of the displacement-framing keywords + ≥ 1 example.
- **Handler binding** (AC-6/7): stub `McpToolHandler` set covering 10 names → `validate_against_handlers` returns `Ok(())`. Stub set with synthetic name not in catalog → returns `HandlerCatalogMismatch { handler: ..., catalog_entry: None }`. Stub set with name in catalog but wrong side → returns `HandlerCatalogMismatch` with catalog_entry's side.
- **Required checks** (AC-8): cargo clippy clean + cargo test --lib clean.

## 8. Security gates

`/cso` not mandatory for this lane (no security-annotated work — pure catalog + validation). `/plan-ceo-review` and `/plan-devex-review` are the gating reviewers per wave plan: taxonomy text IS the product-strategy artifact + every-lane DX surface.

## 9. Path-α (file as Maintenance, not blocking PR)

- Live host-model selection eval — W5-A (DOS-481) scope; this lane ships the catalog structure that DOS-481 evaluates against, not the eval harness itself.
- Per-tool prompt fixtures — DOS-481 scope.
- YAML schema JSON Schema export for IDE tooling — DX nice-to-have, post-v1.4.7.

## 10. Depends-on

- **W1-A merged** (mandatory): YAML loader needs `TaxonomyCatalog` trait + `ToolDescription` types. Currently W1-A is at unanimous L2 APPROVE on branch `v1.4.7-w1-foundation` (commits 45b3f53d → dc80ff86, 10 commits, awaiting user-validated PR open).
- v1.4.5 frozen scopes — NOT a hard dep; the v1.4.5 grandfathered scope set is a hardcoded allowlist in AC-3.

## 11. Definition of Done

§5 AC-1..AC-8 met; L0 unanimous APPROVE; L2 unanimous APPROVE bounded by AC; commit-msg `L2-status: passed`; PR opens against `dev` after W1-A PR merges (sequential — W1-B depends on W1-A) per `feedback_l2_first_then_pr_not_intermediate_pushes`.

## 12. Open questions (cycle-1 dispatch)

| # | Question | Default | Resolution path |
|---|---|---|---|
| Q1 | YAML library choice — `serde_yaml` (0.9) vs `yaml-rust2` | serde_yaml; matches `serde_json` ergonomics + ADR-0125 conventions | architect at L0 plan |
| Q2 | Where in gateway boot does `validate_against_handlers` fire — `Gateway::new()` or `Gateway::register()` or new `Gateway::seal()` method | `Gateway::seal()` called explicitly by main; allows test setup to register handlers without booting taxonomy | architect at L0 plan |
| Q3 | Should validation be `panic!()` or `Result` return | panic!() per L0 packet AC-7 (boot fail-fast); operator-readable Display impl on TaxonomyError | devex at L0 plan |
| Q4 | Embedded vs filesystem YAML | embedded via `include_str!` (no runtime FS dep, no separate distribution) | architect at L0 plan |
