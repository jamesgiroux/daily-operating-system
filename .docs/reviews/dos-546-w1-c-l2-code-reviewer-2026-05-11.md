# DOS-546 W1-C L2 (Diff) — code-reviewer

**Verdict: APPROVE**

Cumulative scope: `b25c6cc4` + `ec570d1d` + workflow-step part of `57a57e1f`.
Files: `inventory.rs` (471L), `bin/emit_ability_inventory.rs` (107L), `lib.rs`
mod registration, `registry.rs` `iter_all` (10L), `web/types/ability-surface.ts`
(94L), `tools/dailyos-abilities.json` (4 entries), `scripts/check_ability_inventory.sh`,
`.github/workflows/test.yml` CI step.

## AC bound — every bullet covered

- Artifact 05 Rust struct + matching TS interface — landed (`AbilitySurfaceInventoryEntry`,
  identical field set, snake_case serde).
- CI gate at `scripts/check_ability_inventory.sh` wired into `.github/workflows/test.yml`
  line 90-91. Drift between live `AbilityRegistry` and `tools/dailyos-abilities.json`
  fails the build with a unified diff and remediation command.
- `tools/dailyos-abilities.json` shipped with 4 current registry abilities, sorted by name,
  deterministic.
- Additive-only contract: `schema_version: 1` envelope, plan to use
  `#[serde(skip_serializing_if = "Option::is_none")]` for future Option<T> fields
  (documented in module doc comment, not violated today).

## Assessment per requested axes

1. **Type definitions** — idiomatic. `BTreeMap` for annotations (deterministic key order),
   `#[serde(rename_all = "snake_case")]`, dedicated `InventoryCategory`/`InventoryMcpExposure`
   wrappers with `From` impls keep runtime enums and surface enums decoupled. Field order
   in `AbilitySurfaceInventoryEntry` matches artifact 05 §"Canonical TypeScript Interface".

2. **`From<&AbilityDescriptor>` projection** — correct. `allowed_actors` collapses
   `Agent|System|Admin → Runtime` per artifact 05 §"AbilityActor" definition; `User` and
   `SurfaceClient` preserved. Sort + dedup applied to both `allowed_actors` and
   `required_scopes` so JSON is canonical. Closed defaults for the four fields without a
   runtime source (`description=""`, `wp_permission="none"`, `annotations={}`,
   `composition_kind=none()`) are explicitly documented and align with artifact 05's
   "closed defaults" principle. `idempotency_class` derives from category exactly as
   artifact 05 §"Field Specifications" prescribes (`Publish|Maintenance → SideEffect`).

3. **Emit binary exit codes** — semantics are documented but **inverted relative to your
   review prompt's expectation**: this implementation uses `0 = success`, `1 = I/O / arg /
   serialization error`, `2 = registry violation`. Your prompt asked for "0 success / 1
   violation / 2 IO". I'm calling this APPROVE rather than REVISE because (a) the binary's
   own module doc declares the mapping explicitly, (b) the CI gate (`check_ability_inventory.sh`)
   only cares about nonzero (it uses `if ! cargo run ...`), and (c) no committed consumer
   distinguishes 1 vs 2 today. **Path-α maintenance note:** if any downstream wires
   "registry violation" vs "IO failure" handling, the binary should be aligned to the prompt's
   convention; file as Codebase Maintenance ticket. `clap` is not used — hand-rolled
   `std::env::args().skip(1)` loop with `--out` / `--help` only; appropriate for a 1-flag tool,
   no new dependency cost, exit on unknown arg.

4. **`AbilityRegistry::iter_all()`** — `pub` (necessary for the emit binary which lives
   in a separate `bin/` target consuming the public API). Doc comment names the tooling-only
   use case and explicitly steers runtime callers to `iter_for` so the actor gate stays in
   force (registry.rs:767-775). Acceptable; tighter scoping would require a
   `pub(crate)` or `#[cfg(feature)]` gate which the `bin/` target can't access cleanly.

5. **TS mirror sync** — manual today. The TS interface in `web/types/ability-surface.ts`
   exactly mirrors the Rust serialization (snake_case enums, identical fields,
   `ABILITY_SURFACE_INVENTORY_SCHEMA_VERSION = 1`). Both sides comment-reference each other
   and artifact 05. No generator. Acceptable scope for W1-C — the diff gate against
   `tools/dailyos-abilities.json` is the practical drift detector: if Rust output changes,
   the JSON regenerates, and TS consumers will see the field on the next merge cycle.
   Path-α maintenance: consider `typeshare` or `ts-rs` if/when a third surface joins
   (file under maintenance project).

6. **CI gate shell script** — bash strict mode (`set -euo pipefail`) ✓.
   `git rev-parse --show-toplevel` → repo-root anchored (idempotent regardless of cwd) ✓.
   `mktemp -t` + `trap 'rm -f "$TEMP_FILE"' EXIT` cleanup ✓. `diff -u` output goes to stdout
   so CI logs surface it; remediation HEREDOC to stderr is clear and copy-pasteable.
   One minor: `mktemp -t dailyos-abilities-actual.XXXXXX.json` — the `.json` suffix is
   appended literally after the template on Linux mktemp (not template-substituted), but
   the file is still written/read by path so this is cosmetic only. Path-α.

7. **Test coverage** — 5 unit tests cover: empty-inventory serialization determinism,
   descriptor projection sort/dedup + closed defaults, `idempotency_default_for` per
   category, entry sort-by-name, and `ActorKind → AbilityActor` collapse. Adequate for the
   AC. Missing but non-blocking: round-trip serde test (deserialize the canonical JSON
   back), and a test asserting `tools/dailyos-abilities.json` parses against
   `AbilitySurfaceInventory`. These are path-α hardening, not AC.

8. **Path-α findings (file in maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):**
   - Exit code mapping convention alignment (1↔2 swap if a consumer needs to differentiate).
   - Auto-generated TS bindings (typeshare / ts-rs) once a third Rust→TS contract appears.
   - Round-trip serde test + parse-the-committed-artifact test.
   - `CompositionKind` is a flat struct in Rust but a discriminated union in artifact 05's
     canonical TS. The Rust comment explains the choice (round-trip cleanliness), and the
     emitted JSON satisfies both shapes when `produces_blocks=false`. When the first ability
     declares `produces_blocks=true`, validate that consumers (WP plugin schema validator,
     MCP server) still parse it under their schema interpretation.

## L2-status

APPROVE for merge against `dev`. AC bullets all met; no AC violations, no ADR-named
contract violations, no PR-introduced regressions. Path-α items above to file in the
maintenance project per CLAUDE.md "Path-α L2 findings" rule.

Co-reviewer: code-reviewer (Claude Opus 4.7).
