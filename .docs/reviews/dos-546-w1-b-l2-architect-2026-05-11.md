# L2 (Diff) Architecture Review — DOS-546 W1-B (commit 0f873270)

**Reviewer:** architect-reviewer
**Date:** 2026-05-11
**Scope:** AbilityPolicy canonical schema + macro compile-error gate + allowlist init
**Verdict:** **APPROVE**

## Findings

### 1. Substrate-contract fidelity — PASS
`AbilityPolicy` carries `allowed_actors`, `allowed_modes`, `requires_confirmation`, `may_publish`, `required_scopes`, `mcp_exposure`, `client_side_executable`. Field names match ADR-0102 §7.1 verbatim (W0-D cycle-3 form). `Default` returns the closed-by-default policy. The AC's `Vec<SurfaceClientScope>` nomenclature is reconciled to the W1-A.1 `SurfaceScope` newtype via `required_scopes_typed()` — documented in the docstring and pinned by `ability_policy_required_scopes_storage_is_static_slice` const test. Reconciliation is sound.

### 2. McpExposure tri-state — PASS
Variants are `None | MetadataOnly | Invocable` per ADR-0102 §7.1 cycle-3 amendment. `#[default] = None` (least-privilege). Serde wire form is `snake_case` (`"none" | "metadata_only" | "invocable"`) — matches Phase 0 artifact 05's TS interface contract. Round-trip and wire-form tests cover all three variants.

### 3. required_scopes storage shape — PASS
`&'static [&'static str]` is the only shape compatible with `inventory::submit!`'s static context — confirmed by inspecting the macro emission (`scopes_ident: [&'static str; #scope_count]`). The typed accessor `required_scopes_typed()` materializes `Vec<SurfaceScope>` per call. W2-B's bridge can call this once per request and feed each `SurfaceScope` to `ScopeSet::contains` — the per-request allocation is bounded by the (small) declared scope set and is on the cold path. No ergonomic break for W2-B.

### 4. Compile-error gate — PASS
Gate at `abilities-macro/src/lib.rs:96-112` fires on `exposes_to_surface_client && required_scopes.is_empty() && !no_scope_required` — gates strictly on actor membership, ignores `client_side_executable` and `mcp_exposure`. Matches ADR-0102 §7.6 exactly. The `ActorArg::SurfaceClient::registry_expr()` defensive `compile_error!` is correct: `Actor::SurfaceClient` is a struct variant carrying owned `ScopeSet`, so it cannot appear in a `&'static [Actor]`; the macro emits a clear diagnostic instead of leaking codegen confusion. Trybuild fixture `surface_client_without_scopes_fails` exercises the actor+empty-scopes+no-opt-out path; stderr asserts the named error message. Other four existing fixtures rebless with the three new fields — round-trip green at 9/9.

### 5. Allowlist init wiring — PASS WITH NOTE
`from_descriptors_checked` collects the union of `required_scopes` across all registered descriptors and calls `ScopeSet::initialize_allowlist`. `OnceLock::set` is idempotent (second call returns `Err` and the code intentionally drops it). This is correct for the production single-init path. Dynamic post-init registration is not supported by the registry shape (`Self { by_name }` is consumed once), so the "abilities register dynamically post-init" failure mode does not exist in the current substrate. Duplicate scopes across abilities collapse via `BTreeSet`. **Note (not blocking):** the silent `Err` drop in `initialize_allowlist(...).is_err()` could surface a `tracing::debug!` for observability under test harnesses that build the registry twice — file as path-α maintenance, not blocking.

### 6. Scope discipline — PASS
Diff is strictly: (a) schema fields on `AbilityPolicy`, (b) macro attribute parsing + compile-error gate, (c) allowlist seeding at registry boot, (d) literal call-site plumbing (8 production + 6 test sites). No SurfaceClientBridge enforcement (W2-B). No MCP `list_tools` filtering (W3-C). No `emit_surface_audit` consumer (W1-A0). Commit message acknowledges this explicitly and matches diff reality.

### 7. Cross-ADR invariants — PASS
- ADR-0105 (provenance lives once): untouched — no provenance fields added.
- ADR-0108 (envelope cap): untouched — `mcp_exposure` does not compose with sensitivity at this layer; the ADR-0102 §7.6 note that "ADR-0108 sensitivity gating composes with `mcp_exposure`" is a W3-C wiring concern.
- ADR-0111 §8 (SurfaceClient class): the macro's defensive `compile_error!` in `ActorArg::SurfaceClient::registry_expr()` correctly enforces the struct-variant invariant — `Actor::SurfaceClient { instance, scopes }` is per-invocation, never policy-static.
- ADR-0130 (composition contract): untouched.

### 8. L1 evidence — TRUSTED
Commit reports `cargo clippy -- -D warnings` clean, `cargo test --lib` 2162/0/11, trybuild 9/9. The pre-existing `clippy::items-after-test-module` lint in `services/people.rs` under `--all-targets` is correctly identified as not introduced by this commit. Trust the implementer's discipline per L1 protocol.

## Acceptance Criterion Coverage (issue lines 446-456)

| AC | Status |
|----|--------|
| 446 — three new fields on AbilityPolicy | met (with documented nomenclature reconciliation) |
| 447 — McpExposure enum + client_side_executable separation | met |
| 448 — macro attribute parsing + defaults | met |
| 449 — compile-error invariant | met (trybuild verified) |
| 450 — SurfaceClientBridge required_scopes | **W2-B scope** (explicitly deferred) |
| 451 — SurfaceClientBridge client_side_executable | **W2-B scope** (explicitly deferred) |
| 452 — MCP introspection filter | **W3-C scope** (explicitly deferred) |
| 453 — neg test mcp_exposure None | **W3-C scope** |
| 454 — neg test scope mismatch | **W2-B scope** |
| 455 — neg test compile-time gate | met (trybuild) |
| 456 — audit policy_check_result | **W1-A0 scope** |

W1-B's contracted slice (446-449, 455) is fully met. Deferrals match the W1 lane plan and are explicitly named in the commit body.

## Recommendation

**APPROVE for merge into the W1-B branch.** No blocking findings. The one observability note in §5 (silent `OnceLock::set` failure) is path-α — file as a maintenance ticket under `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb` and unblock this PR.
