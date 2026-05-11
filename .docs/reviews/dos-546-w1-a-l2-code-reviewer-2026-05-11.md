# DOS-546 W1-A — L2 (Diff) Code Reviewer

**Commit:** `7fba6a22` — "DOS-546 W1-A: SurfaceClient as the fourth actor class"
**Reviewer:** code-reviewer
**Date:** 2026-05-11
**Scope:** L2 (Diff) bounded by W1-A acceptance criteria in `.docs/plans/dos-546/v1.4.2-project/02-issues.md` §345-422
**Files reviewed:** 8 (+231/-13)

---

## Verdict

**APPROVE** with one acceptance-criterion delta surfaced for the wave plan (not a blocker), and three path-α observations filed for the maintenance project.

The diff is a clean stage-1a substrate landing. Newtype design, derives, plumbing discipline, and test coverage are idiomatic and intentional. `Copy` removal is justified and call sites are mechanical. Tests are meaningful (not coverage padding); each pins a specific invariant the downstream W1-B / W1-A0 bridges will rely on.

---

## 1. Correctness

**Actor enum extension.** `Actor::SurfaceClient(SurfaceClientId)` is added to the existing enum (registry.rs:127-131) with `Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema` derives. `Copy` is correctly dropped (carrying `String`). Exhaustive matches across the eight touched files all gain a `SurfaceClient(_) => todo!(...)` arm — no `_` wildcards anywhere, so the compiler will block W1-B authors who forget to revisit a site. Good.

**Newtype derives.** Both `SurfaceClientId` and `SurfaceScope` derive `Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, JsonSchema` and carry `#[serde(transparent)]`. `Display` is hand-written and writes the inner string. `as_str() -> &str` and `new(impl Into<String>)` are the right minimal surface. No `From<String>` or `From<&str>` impls; not strictly needed at this stage and good restraint (forces callers through `::new` which is greppable for W1-B).

**Hash / Eq round-trip in HashSet** is validated by the dedicated tests.

**`iter_for(actor: Actor)` and `validate_invocation_policy` (registry.rs:363, 902)** still take `actor` by value / by reference respectively. Now that `Actor` is no longer `Copy`, `iter_for` moves; this is fine because the filter closure captures `move` and the actor is used via `&actor` inside. Callers that previously relied on `Copy` to call `iter_for(actor)` twice in scope must now clone — verified compiles (L1 reports `cargo clippy -- -D warnings` and `cargo test --lib` exit 0; 255 tests pass).

## 2. Plumbing discipline

**Clone-at-move-sites are necessary, not gratuitous.** I checked each `.clone()` introduction:

- `registry.rs:253` (`AbilityContext::with_*` rebuild) — required: source `&self.actor` consumed into new owned context.
- `get_entity_context.rs:82, 396` — required: `filter_claims_for_actor` and `provenance_actor` both take `Actor` by value.
- `synthesis.rs:1552` — same pattern, required.
- `temporal/mod.rs:341` — same pattern, required.
- `mcp.rs:713`, `tauri.rs:613` — both inside `#[cfg(test)]` fixture builders, required for same reason.
- `operations/mod.rs:251` — `OperationInvocation { actor, ... }` takes by value, required.

No unnecessary clones. The single non-clone change is `render_actor_for_context` (`get_entity_context.rs:460`) which switches `match ctx.actor` → `match &ctx.actor` — the correct fix (borrow instead of clone when the arms don't need ownership).

**`todo!()` arms are appropriately scoped.** All seven `todo!("W1-B+ wiring for Actor::SurfaceClient")` arms sit in code paths that today are unreachable because no current invocation site constructs `Actor::SurfaceClient` (no producer exists yet — the bridge lands in W1-B). The comments above each `todo!()` name the downstream issue that will replace it. `todo!()` panics at runtime if hit; given there is no producer in this commit, this is safe **for this stage only** — see Finding 3 below.

**`Copy` removal on `TauriInvokeContext` and `OperationBridgeContext`** is correctly justified by inline comments at `tauri.rs:88` and `operations/mod.rs:65`. Both structs now `#[derive(Clone)]`; the comments name W1-A as the reason. Good documentation hygiene.

## 3. Test coverage

The 10 new tests at `registry.rs:1794-1899` (per commit message; actual line range `1900-2010` in committed file) are meaningful:

| Test | Invariant pinned |
|---|---|
| `surface_client_id_round_trip_preserves_value` | `as_str` + `Display` + `Debug` representation |
| `surface_client_id_serde_round_trip_is_transparent` | `#[serde(transparent)]` wire shape is bare string |
| `surface_client_id_hash_eq_match_inner_string` | `HashSet` semantics for de-dup at the W1-B scope-grant cache |
| `surface_scope_round_trip_preserves_value` | mirror for `SurfaceScope` |
| `surface_scope_serde_round_trip_is_transparent` | mirror |
| `surface_scope_hash_eq_match_inner_string` | mirror |
| `actor_surface_client_round_trip_preserves_identity` | variant + clone preserve identity |
| `actor_surface_client_serde_round_trip` | full Actor enum serde round-trip including new variant |
| `actor_surface_client_distinct_instances_are_not_equal` | per-instance discrimination (the audit-attribution invariant) + non-equality vs unit variants |
| `actor_surface_client_not_in_user_agent_allowed_actors` | AC-named negative test: `[User, Agent].contains(&SurfaceClient(_))` returns false |

The last test pins the exact `.contains()` semantic that `iter_for` and `validate_invocation_policy` rely on — this is the load-bearing one for the AC line "Negative test: an ability marked `allowed_actors: [User, Agent]` rejects a `SurfaceClient` invocation at the registry boundary". The test asserts the slice-contains semantic, not the registry rejection itself; the registry rejection is one call-site away and is exercised by every existing `iter_for` / `validate_invocation_policy` test once `Actor::SurfaceClient` flows through (W1-B). For stage-1a this is the correct depth.

**Debug-format brittleness.** `assert_eq!(format!("{id:?}"), "SurfaceClientId(\"wp-instance-alpha\")")` (line 1914) pins the derived `Debug` output. If a future maintainer hand-implements `Debug` to scrub the inner string, this test will fail loudly — which is the right behavior: scrubbing belongs in a separate `Display`/audit codec, not in `Debug`. Acceptable.

## 4. Idiomaticity

- Newtype pattern: correct.
- `impl Into<String>` constructor: idiomatic.
- Hand-rolled `Display`: idiomatic (no `derive_more` dep introduced — good restraint).
- `#[serde(transparent)]`: correct for a single-field tuple struct that should serialize as the inner value.
- `JsonSchema` derive: correct for the registry's schema export path.
- `formatter` as parameter name in `Display::fmt`: rust-style allowed; rustfmt-stable.

Anti-idiom check: no `unwrap()`, no `panic!()`, no `expect()` in production paths (the `.expect()` calls are inside tests). Pattern matches are exhaustive everywhere (compiler-enforced by the absence of `_` wildcards). Matches AC line 371.

## 5. Scope discipline

The commit ships **exactly** the actor-class types. It does not:

- touch `AbilityPolicy` (W1-B owns `required_scopes`, `mcp_exposure`, `client_side_executable`)
- touch the `#[ability]` macro
- introduce an `audit_log` module (W1-A0 owns that)
- introduce `SurfaceClientBridge` (W1-B / W2)
- alter `Actor::Agent` / `Actor::System` semantics
- modify any frontend code

The `SurfaceScope` newtype lands here even though no producer/consumer in this commit reads it. This is the only "scope creep" candidate, and the issue spec sanctions it: AC line 365-367 names `SurfaceClientId` as the typed wrapper that must land, and the doc-comment at registry.rs:67-78 explicitly states `SurfaceScope` is shipped now so W1-B's `required_scopes: Vec<SurfaceScope>` field can compile against an existing type. This matches the "Don't swing past center" / "standards precede consumers" guidance. Acceptable.

## 6. PII / customer data

Clean. Identifiers in tests: `wp-instance-alpha`, `alpha`, `beta`, `gamma`, `read.account_overview`, `write.feedback`, `read.x`, `write.y`. Doc-comments mention `WordPress site GUIDs, Obsidian vault IDs, browser-extension installation IDs` as illustrative — these are tool/category names, not customer data. The commit message contains no PII.

---

## Findings

### Finding 1 — Acceptance-criterion shape delta (informational, not blocking)

**AC line 365** specifies:

> `Actor::SurfaceClient { instance: SurfaceClientId, scopes: ScopeSet }` lands in the abilities-runtime crate.

**Committed shape** is the tuple variant `Actor::SurfaceClient(SurfaceClientId)` — identity only; scopes are routed through `SurfaceClientBridge` request context per the W1-A doc-comment at registry.rs:113-118.

The commit message is explicit about this and the W0-D ADR-0102 amendments support routing scopes at the bridge boundary rather than embedding them in the actor variant (per-request grant lookup, revocation freshness). The plan-vs-implementation delta is a reasonable refinement, but it is a literal departure from the AC string.

**Recommendation:** not a blocker. The delta is principled (avoids stale scopes embedded in long-lived actor values) and downstream issues (W1-B, W1-A0) can absorb it. Update the W1-A issue's AC line on Linear to reflect the chosen shape, or add a one-line note in the wave plan saying "AC line 365 amended: scopes at bridge, not on actor; rationale in commit `7fba6a22`". This protects future L0 reviewers from flagging the same delta against W1-B.

### Finding 2 — `todo!()` arms are a wave-coupling tripwire (path-α, file to maintenance)

The seven `todo!()` arms are unreachable today, but they will panic at runtime the first time anything constructs `Actor::SurfaceClient` in a context that flows through any of `provenance_actor`, `render_actor_for_context`, the prepare_meeting `provenance_actor`, the `mcp.rs` / `tauri.rs` test fixtures, the `BridgeActor::from`, or `temporal/mod.rs`. If a W1-A0 / W1-B implementer constructs the variant before wiring all seven sites, the failure mode is a panic-on-invoke rather than a compile error.

**Mitigation already in place:** every `todo!()` has a comment naming the downstream issue. The compiler will also force any new match site to handle the variant.

**Path-α recommendation (not blocking):** file a maintenance ticket to add a `cargo test --test surface_client_todo_audit` integration test (or a `grep -r 'todo!("W1-B+ wiring' src-tauri/` CI check) that fails on `dev` when the number of `todo!()` arms is non-zero AND any commit on the WP-foundation branch declares "W1-B complete". This is a wave-coupling gate, not an AC violation. File to maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`.

### Finding 3 — `Actor::SurfaceClient` debug-format leaks raw identifier into logs (path-α, file to maintenance)

`SurfaceClientId(String)` derives `Debug`, which produces `SurfaceClientId("wp-instance-alpha")`. If the inner string ends up containing PII (a user-chosen vault name, a hostname like `acme-corp-research.wp.test`), it will leak into any `tracing::debug!(?actor, ...)` call. The doc-comment at registry.rs:42-43 says "Callers are expected not to embed PII in the identifier itself; the type does no scrubbing." That's a discipline contract, not an enforcement.

This is **explicitly out of W1-A AC scope** (AC line 366 says only "debug-printing it produces a stable, non-PII representation" — derived Debug satisfies "stable", and "non-PII" is shifted to the caller). It is not an AC violation. It is a substrate hardening opportunity for W2 when real `SurfaceClientId` values flow.

**Path-α recommendation:** file a maintenance ticket to add a hand-rolled `Debug` impl that hashes or truncates the inner string (e.g. `SurfaceClientId(sha256-prefix-12)`), and add a `tracing`-filter integration test asserting the raw identifier does not appear in formatted log output. File to maintenance project. Not blocking.

### Finding 4 — `iter_for(actor: Actor)` by-value after `Copy` removal (path-α, file to maintenance)

With `Copy` gone, `registry.iter_for(actor)` now moves the actor into the filter closure. Existing callers in tests (e.g. `registry_iter_for_agent_hides_*` at registry.rs:1599+) pass `Actor::Agent` literally, which is fine. But any future caller that wants to call `iter_for` twice will need to `.clone()` the actor. Switching the signature to `iter_for(&self, actor: &Actor)` would be a minor ergonomic improvement.

**Outside W1-A AC scope.** L1 confirms the current signature compiles and tests pass. File to maintenance for a future ergonomics pass, do not gate W1-A on it.

---

## L2-status acknowledgment

The commit carries `L2-status: not-run-acknowledged`. This review records the L2 (Diff) code-reviewer lane as **APPROVE**. Per the ladder, Codex review and (per W1-A's `trust-boundary` label) `security-auditor` lanes are independent and must also approve before merge; their verdicts land on the Linear ticket. The substrate PR is unblocked from code-reviewer's lane.

---

**End of L2 code-reviewer verdict.**
