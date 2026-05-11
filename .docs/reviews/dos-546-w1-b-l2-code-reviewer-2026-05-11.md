# L2 (Diff) — code-reviewer — DOS-546 W1-B

Commit: `0f873270` — AbilityPolicy schema + `#[ability]` macro compile-error gate + allowlist init.
Verdict bounded to W1-B AC (schema + macro + init); W2-B bridge enforcement and W3-C MCP filtering are carved out by the issue's scope-limits para.

## Verdict: APPROVE

## Findings

1. **Idiomatic.** `required_scopes: &'static [&'static str]` + `required_scopes_typed() -> Vec<SurfaceScope>` is the right split: storage must be `static`-constructible for `inventory::submit!`, typed materialization is a per-call helper. Doc comments name the rationale. Acceptable idiom.

2. **McpExposure.** `#[default] None`, `snake_case` serde, `Copy + Hash + Eq`, `JsonSchema`. All-variant round-trip + literal wire-form assertions both present. Adequate.

3. **Macro gate.** Parse-time error (`expand_ability` early return via `syn::Error`), spanned on `item_fn.sig.ident` — fires before codegen, message names ADR-0102 §7.6 + DOS-546 W1-B + both opt-outs. trybuild `.stderr` is concise and pinned to the function ident. The `ActorArg::SurfaceClient => compile_error!(…)` defense-in-depth at codegen is correctly unreachable but documents the per-invocation-struct constraint clearly. Good.

4. **Call-site plumbing.** Spot-checked bridges/types.rs, bridges/worker.rs, bridges/mcp.rs — all add the three fields with closed defaults (`&[]`, `McpExposure::None`, `false`). No test silently lost a scope set; pre-W1-B fixtures had none. No foot-gun.

5. **Allowlist init.** `from_descriptors_checked` collects union into `BTreeSet` (dedup is set semantics — duplicate scope across abilities is fine), calls `ScopeSet::initialize_allowlist`, swallows `Err` with a documented intentional no-op (second registry build in-process hits initialized `OnceLock`). Behavior matches W1-A.1 contract.

6. **Tests.** 7 new unit tests cover defaults, typed materialization, McpExposure default + round-trip + wire form, and a `const POLICY` static-slice pin. Coverage adequate for schema-level AC.

7. **Path-α.** Pre-existing `clippy::items-after-test-module` in `src/services/people.rs` under `--all-targets` is unrelated to W1-B. File against maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`; do not block this PR.
