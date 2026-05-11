# L2 (Diff) — architect-reviewer — DOS-546 W1-C cumulative

**Date:** 2026-05-11
**Commits:** `b25c6cc4` (inventory + gate) + `ec570d1d` (iter_all) + `57a57e1f` (workflow wiring)
**Verdict:** APPROVE

## Assessment

1. **Schema fidelity to artifact 05.** Field-for-field match: `name`, `description`, `category`, `annotations`, `wp_permission`, `allowed_actors`, `required_scopes`, `mcp_exposure`, `client_side_executable`, `idempotency_class`, `composition_kind` — all present, serde snake_case matches the TS union tags verbatim, sort/dedup invariants honored in `from_descriptor`. `schema_version` envelope is a substrate-improving addition over the artifact (additive-only contract per AC bullet 7 needs this; artifact didn't mandate it, but it's the right call). Issue AC line 546 lists a `display` field — not in artifact 05 §"Canonical TypeScript Interface". Implementation correctly follows the artifact (canonical), not the issue body (stale). Path-α drift, not a blocker.

2. **Type taxonomy.** `AbilityActor`/`InventoryCategory`/`InventoryMcpExposure` aliasing the runtime enums via `From` projections keeps the wire form decoupled from runtime evolution — the right call. `ActorKind::Agent | System | Admin → Runtime` collapse matches artifact 05 §47-60. `AnnotationValue` untagged union (Null/Bool/Int/String) loses `f64` vs `i64` distinction the artifact's JSON Schema permits (`"type": ["string","number","boolean","null"]`); for `surface_priority: 0-100` and other reserved keys this is fine. `CompositionKind` as flat struct (not TS-style discriminated union) is justified in doc comment — serde round-trip cleanliness. Defensible.

3. **TS mirror SSoT.** Rust is source, TS is hand-mirrored. CI gate diffs the JSON artifact, NOT the TS types. A developer adding a Rust field who forgets the TS update will drift silently until a downstream TS consumer breaks. Recommend a follow-up maintenance ticket: `emit_ability_inventory --emit-ts` or a `ts-rs`/`schemars` codegen step gated in CI. Path-α — file to maintenance project, not a blocker.

4. **CI gate UX.** `diff -u` failure with remediation hint (`cargo run … --out tools/dailyos-abilities.json`) is the correct shape — drift detection + forward-roll path is one command. Wired in `.github/workflows/test.yml` after W1-D. Workflow integration in `57a57e1f` correctly placed after description lint so a tripping description fails first with a clearer message.

5. **`iter_all` policy-bypass naming.** Names match intent — `iter_all` (tooling-facing, no actor gate) vs `iter_for(actor)` (runtime gate enforced). Doc comment explicitly warns runtime callers off. The `pub` visibility is broader than ideal (a `pub(crate)` plus a re-export through a `tooling` module would tighten the surface), but the doc-comment warning is the documented contract. Acceptable.

6. **ADR-0129 §4 consistency.** §4 names "WP Abilities API + MCP Adapter" as the WP-side consumers; §86 names a "custom MCP server configuration with an explicit ability allowlist" — the inventory's `mcp_exposure` and `wp_permission` fields are the right shape to drive both. ADR-0129 doesn't restate the inventory schema (correctly defers to artifact 05). No drift.

## Findings filed as path-α (maintenance project)

- TS mirror codegen (item 3 above) — Linear maintenance ticket recommended.
- Issue body AC line 546 references a `display` field absent from artifact 05 — reconcile the issue body, not the code.
- `iter_all` visibility tightening (`pub(crate)` + tooling re-export) — optional ergonomic.

## Acceptance criteria check

- [x] Phase 0 artifact 05 TS interface + matching Rust struct land in `abilities-runtime`.
- [x] `tools/dailyos-abilities.json` artifact consumed by WP plugin / MCP server / SurfaceClient introspection (shape lands; consumers wire in W3).
- [x] CI gate at `scripts/check_ability_inventory.sh` wired in `test.yml`.
- [x] Additive-only contract: `schema_version: 1` envelope + serialized field order locked.
- [~] Per-ability inventory entry assertion: artifact gate is drift-based (regenerate + diff), not a per-`#[ability]` macro requirement. AC line 541 reads "each has an inventory entry. Missing entry = build failure." The implementation satisfies the intent (any unrepresented ability would show in the diff) but does not enforce a per-macro inline declaration. Acceptable substrate decision — entries are projected from the runtime descriptor today; per-macro inline metadata lands in a later wave per `inventory.rs` doc comment lines 33-37.

## Verdict

**APPROVE.** Schema is faithful to the canonical artifact, the CI gate prevents silent drift, the `iter_all`/`iter_for` split correctly names the trust boundary, and cross-ADR consistency holds. Path-α findings (TS codegen, issue-body reconciliation, visibility tightening) file to the maintenance project.
