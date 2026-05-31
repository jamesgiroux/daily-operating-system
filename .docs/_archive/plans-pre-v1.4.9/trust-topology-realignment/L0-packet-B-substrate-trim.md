# L0 Packet B — Substrate Trim (Topology-Aware Gate Bypass)

**Status:** SUPERSEDED 2026-05-21 — L0 panel surfaced 26 findings; convergent root cause: `ValidatedSurfaceSession.topology` + category-keyed bypass + Default impl was over-engineered for the actual L4 unblock (Phase A already solved it). Replaced by minimal change: auto-derive `DEFAULT_GRANTED_SCOPES` from registry at pair time (preserves `submit.feedback` as explicit grant). No new enum, no `Actor` change, no bypass branch in `authorize_for_path`. Topology stays as documentation (engineering-ladder.md §Threat-topology framing, PR 348) — not a runtime data type yet.
**Scope:** Rust substrate — `ValidatedSurfaceSession.topology` field + category-keyed bypass logic in `authorize_for_path` + refresh-endpoint rhetoric fix
**Author:** James + Claude Opus 4.7
**Drafted:** 2026-05-21
**Engineering Ladder rung:** L0 (Plan); panel = `/codex challenge` + `architect-reviewer` + `security-auditor` (Amendment 3 — auth, trust boundary, scope) + `/codex consult` + `/ce-doc-review`
**Blocked by:** [`L0-packet-C0-reviewer-scoping.md`](./L0-packet-C0-reviewer-scoping.md) must merge first so reviewer prompts are topology-aware before B's L2 panels run.
**Pairs with:** parent `L0-packet.md` (Trust Topology Realignment), revised against original L0 panel findings (2026-05-21)

---

## §1 Trust Topology

**This packet's deployment topology: local-to-local single-user.**

Same human, same machine, signed pairing handshake, loopback HTTP between WordPress Studio (PHP process) and Tauri runtime. OS account boundary is the trust boundary.

**Explicit prerequisites for the "same as a desktop app" framing** (per codex consult #3 from parent panel):
1. Trusted binaries (DailyOS Tauri runtime, WP Studio runtime, WP plugin) — out of scope; integrity guaranteed by OS package + signing
2. Same OS user owns both DailyOS process and WP Studio process
3. Signed pairing handshake completed (the only inbound trust event)
4. Transport is loopback only (no remote WP MCP exposure on this path; remote MCP path stays `Actor::Mcp` with all gates intact)
5. WP plugin and Studio MySQL run under the same OS user as DailyOS

Inside these prerequisites, multi-actor gates (per-ability `required_scopes`, attestation prompts, narrow `DEFAULT_GRANTED_SCOPES`) are theatre. They defend a differentiated-principal model that does not exist.

**Out-of-topology by §37 framing — do NOT defend against:**
- Same-OS-user malware running as the user — that's an OS account compromise; OS account boundary is the gate, not application-level scope checks
- Third-party remote MCP — preserved by separate code path (`Actor::Mcp`); gates stay
- Untrusted content (ADR-0093, ADR-0108) — orthogonal; not touched

---

## §2 Goal

Eliminate the v1.4.2-era scope-set hardening for local-to-local single-user SurfaceClient sessions, without weakening the third-party MCP path. Adding new Read abilities should not require re-pairing every WP surface.

**Success criteria:**

1. New Read abilities admit `Actor::SurfaceClient` whose session topology is `LocalToLocalSingleUser` without per-ability `required_scopes` lookup against `DEFAULT_GRANTED_SCOPES`.
2. Third-party MCP path (`Actor::Mcp`) retains scope checking, attestation, and rate limiting — verified by named regression test.
3. WP `current_user_can('manage_options')` capability check becomes conditional on multi-WP-user detection (single-user Studio: skip; multi-user: enforce).
4. Refresh-endpoint rhetoric in parent §3.2 row 3 is corrected — packet documents honestly what HMAC defends (replay/tampering of in-flight requests) vs what the OS account boundary defends (same-OS-user impersonation).
5. `cargo clippy -- -D warnings && cargo test --workspace && pnpm tsc --noEmit` all pass.

---

## §3 Inventory — Substrate Changes

### §3.1 Topology classifier as a session-level property

Per architect-reviewer #2 from parent L0 panel: topology rides on `ValidatedSurfaceSession`, NOT on `Actor::SurfaceClient` enum variant. Channel-property at pair time, never changes for session lifetime, no Actor-enum churn through audit_log / macro / trybuild.

| File | Change |
|---|---|
| `src-tauri/src/services/surface_pairing.rs:205` | Add `pub topology: TrustTopology` field to `ValidatedSurfaceSession`. |
| `src-tauri/src/services/surface_pairing.rs` (new types module or top of file) | Define `pub enum TrustTopology { LocalToLocalSingleUser, LocalToLocalMultiUser, RemoteToLocal }` (3 variants — `RemoteToRemote` not a deployment shape per parent §6; can be added later without breaking). |
| `src-tauri/src/services/surface_pairing.rs` `validate_signed_session_*` (existing fn that returns `ValidatedSurfaceSession`) | Populate `topology` field. Initial implementation: always `LocalToLocalSingleUser` for `Actor::SurfaceClient` sessions (current deployment shape); `RemoteToLocal` for `Actor::Mcp`. Future deployment shapes (multi-WP-user Studio, cloud) populate the discriminator from pair-handshake input. |

### §3.2 Category-keyed bypass in `authorize_for_path`

Per architect-reviewer #4: single runtime layer interprets `descriptor.category` per-topology; ability declarations stay truthful.

| File | Change |
|---|---|
| `src-tauri/src/bridges/surface_client.rs:311` (`authorize_for_path`) | After existing `experimental` + `BrowserDirectJs` checks (those stay), branch on `session.topology` AND `descriptor.category`. **For `LocalToLocalSingleUser` + `AbilityCategory::Read`: bypass `ensure_required_scopes` AND skip `requires_confirmation` attestation.** Keep rate-limit check (with widened ceiling per §3.3). For all other (`LocalToLocalSingleUser` + Write/Maintenance/Publish/Transform; any `RemoteToLocal`; any `LocalToLocalMultiUser`): existing gates fire unchanged. |
| `src-tauri/src/bridges/surface_client.rs:407+` (rate limiter) | Widen `SurfaceClientRateLimitRequest` ceiling for `LocalToLocalSingleUser` (e.g., 10k/min — sanity floor per security-auditor advisory; not eliminated). |

**Critical implementation constraint** (security-auditor caveat from parent panel): the bypass is a topology-keyed BRANCH on `session.topology`, NOT removal of the `ensure_required_scopes` call site. The `Actor::Mcp` path must not accidentally inherit the bypass.

### §3.3 `DEFAULT_GRANTED_SCOPES` auto-derivation

| File | Change |
|---|---|
| `src-tauri/src/services/surface_pairing.rs:38-49` | Replace hardcoded constant. For `LocalToLocalSingleUser` pairings: at pair-handshake time, derive granted scopes from the registered `AbilityRegistry` (every Read ability's declared scopes). Auto-rolls forward as new Read abilities ship; no wave-by-wave constant update. For `RemoteToLocal` (Mcp): explicit scope-set grant remains. |

### §3.4 WP capability gate conditional on multi-user detection

Per security-auditor advisory + parent §3.2 row 1:

| File | Change |
|---|---|
| `wp/dailyos/includes/transport/class-dailyos-credential-store.php:183` | Replace bare `current_user_can('manage_options')` with: if `is_multisite() || count_total_users() > 1` → enforce capability check; else → skip (single-user Studio is the deployment shape per §1). Use cheap query `get_users(['count_total' => true, 'number' => 2])` per security-auditor implementation note. |

### §3.5 Parent-packet rhetoric correction (refresh-endpoint claim)

Per codex challenge BLOCKED finding from parent panel:

| File | Change |
|---|---|
| `.docs/plans/trust-topology-realignment/L0-packet.md` §3.2 row 3 | Rewrite the "Currently defends against" cell. Honest framing: HMAC defends replay + tampering of in-flight signed requests. HMAC does NOT defend against same-OS-user impersonation (the refresh endpoint at `src-tauri/src/surface_runtime/mod.rs:1497` bootstraps the HMAC key from publicly readable marker fields). Same-OS-user impersonation is **out-of-topology** by the OS-account-boundary framing in §1 — not defended at the application layer. |

This is a docs-only change to the parent packet; ships in this B PR for traceability.

### §3.6 Phase A (already shipped) — confirm additive, not replacement

Per architect-reviewer #3: the v1.4.4-session expanded `DEFAULT_GRANTED_SCOPES` constant + 8 `allowed_actors` widenings on Read abilities are the conservative floor. B layers topology-keyed bypass on top. After §3.3 auto-derivation lands, the expanded constant becomes dead code and is removed in the same PR; the ability-level `allowed_actors` widenings stay (they declare which actor kinds the ability admits, orthogonal to topology bypass).

---

## §4 Implementation Phases

Three sub-phases within this packet. All ship in one PR, but ordering matters for review.

### Phase B1 — Data model (`TrustTopology` enum + `ValidatedSurfaceSession.topology`)

- Define enum, add field, populate at session validation site
- No behavior change — bypass logic not wired yet
- `cargo test` passes (field is unused but compiles)

### Phase B2 — Bypass logic + auto-derivation

- Wire `authorize_for_path` topology+category branch
- Replace `DEFAULT_GRANTED_SCOPES` with registry-derived set
- Widen rate-limit ceiling for `LocalToLocalSingleUser`
- WP capability conditional
- Remove now-dead expanded `DEFAULT_GRANTED_SCOPES` literal from Phase A
- Update parent packet §3.2 row 3 rhetoric (§3.5)
- New unit tests:
  - `bypass_applies_only_to_local_to_local_single_user_read` (positive)
  - `mcp_actor_still_requires_scopes_under_local_to_local_topology` (regression — bypass MUST be topology-keyed branch, not removal)
  - `local_to_local_write_category_still_gates` (positive — Write/Maintenance/Publish/Transform unaffected)
  - `default_granted_scopes_auto_derives_from_registry` (positive)
  - WP PHPUnit: `multi_user_studio_still_enforces_manage_options_check`

### Phase B3 — Regression sweep + proof bundle

- Run full `cargo test --workspace` and `pnpm tsc --noEmit` and `pnpm test`
- Re-pair WP→Tauri locally; render every W2 surface (account / project / person / meeting / briefing) and confirm data flows
- MCP smoke: third-party MCP invocation against a non-default scope is REJECTED (regression test for AC #2)
- Capture proof bundle per L1 self-validation: screenshots, test output, MCP rejection log

---

## §5 Acceptance Criteria (Concrete + Verifiable)

Replacing the parent packet's AC #3/4/7/8 which codex consult flagged as too vague:

1. **Topology field exists.** `grep -n "pub topology: TrustTopology" src-tauri/src/services/surface_pairing.rs` returns exactly one line in the `ValidatedSurfaceSession` struct.
2. **Bypass is topology-keyed branch, not call-site removal.** `grep -n "ensure_required_scopes" src-tauri/src/bridges/surface_client.rs` returns the same number of call sites as on `dev` (the call site is preserved; only the path through it is conditional).
3. **MCP regression test passes.** Named test `mcp_actor_still_requires_scopes_under_local_to_local_topology` exists in `src-tauri/src/bridges/surface_client.rs` (or adjacent test module) and asserts a `Actor::Mcp` session is rejected when invoking an ability whose scopes it lacks, EVEN IF a co-resident `LocalToLocalSingleUser` SurfaceClient would have been allowed.
4. **Read-bypass positive test passes.** Named test `bypass_applies_only_to_local_to_local_single_user_read` exists and asserts the four cells of (topology × category) — only `LocalToLocalSingleUser × Read` bypasses.
5. **Auto-derivation works.** Named test `default_granted_scopes_auto_derives_from_registry` exists and asserts the granted-scope set equals the set of scopes declared by all registered `AbilityCategory::Read` abilities — without any hardcoded list.
6. **WP capability conditional works.** Named PHPUnit test `multi_user_studio_still_enforces_manage_options_check` exists in `wp/dailyos/tests/` and asserts the cap check fires when `count_total_users() > 1`, and is skipped otherwise.
7. **End-to-end render works against fresh pairing.** Manual L4 step: revoke existing pairing, re-pair from a fresh state, render `/accounts/<slug>/`, `/projects/<slug>/`, `/people/<slug>/`, `/meetings/<slug>/`, `/briefings/today/` — each surface populated with the same data as the equivalent React route. Screenshot bundle attached to PR.
8. **No new Read ability requires re-pairing.** Add a throwaway Read ability under a feature flag; without re-pairing, it's invokable by the existing WP session. Captured as a one-shot reproduction script, not a permanent test.
9. **CI gates pass.** `cargo clippy -- -D warnings && cargo test --workspace && pnpm tsc --noEmit && pnpm test` all green.

---

## §6 Out-of-Scope (Defer)

- **ADR-0102 / ADR-0111 / ADR-0129 amendments** — C1 packet.
- **Wave-plan retroactive topology classifier sweep** — C1 packet.
- **HMAC-canonical rollover absorption** (DOS-746) — separate track, called out as adjacent in parent §6. B does not touch canonical signing.
- **Refresh-endpoint hardening** (moving marker to OS keychain or requiring attestation token) — out-of-topology by §1 framing; if user accepts OS-account-boundary as trust boundary, no fix needed at application layer. If user wants defense-in-depth against same-OS-user attack, that's a follow-up *defensive* track filed against the maintenance project.
- **`Actor::SurfaceClient` enum variant change** — explicitly NOT done per architect-reviewer #2; topology rides on session, not actor.
- **Block-showcase mu-plugin decision** (parent §3.2 row 8) — file as Linear maintenance ticket; not on B critical path.
- **method_exists → is_callable sweep across all blocks** (parent §3.2 row 7) — file as Linear maintenance ticket.

---

## §7 Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Bypass accidentally inherited by `Actor::Mcp` path | Medium | AC #3 regression test catches this. Topology-keyed branch on `session.topology == LocalToLocalSingleUser` is the structural guard. |
| Auto-derived scope set differs from existing `DEFAULT_GRANTED_SCOPES` constant in a way that breaks live pairings | Medium | Phase B2 ships the auto-derivation behind a session-version bump if needed; alternatively, the existing pairings keep their hardcoded scope set until next pair (rollback path: revert auto-derivation; explicit scopes still work). |
| Rollback story unclear if B ships and produces a regression | Medium | The bypass branch is gated on `session.topology` — revert path is feature-flag-style: set the topology classifier to a sentinel (`Unset`) and the branch falls through to existing gate logic. Document the revert SHA in the proof bundle. |
| Effect on in-flight v1.4.4 W2-C and v1.4.5 packets | Medium | v1.4.4 W2-C is the L4 unblock that motivated this work; B replaces Phase A's expanded constant — verify W2-C still renders post-B. v1.4.5 work hasn't started yet; B lands cleanly before W1. |
| Refresh-endpoint rhetoric correction in parent §3.2 row 3 misread as a fix promise rather than an out-of-topology acknowledgment | Low | §3.5 explicitly classifies it as out-of-topology per §1 OS-account-boundary framing. Reviewer panel reads §1 first. |
| Cargo-level type churn: adding field to `ValidatedSurfaceSession` ripples through 12+ construction sites | Medium-High | Provide a `Default::default()` impl for `TrustTopology` (defaults to `LocalToLocalSingleUser` since that's the actual deployment shape); construction sites that don't care get the default. New code paths in MCP set it explicitly. |
| DOS-746 (HMAC canonical migration) lands concurrently and causes pairing-record schema drift | Low | B does not touch canonical signing or pairing schema; B and DOS-746 are orthogonal. Coordinate merge order: whichever ships second rebases. |
| Same-OS-user attack (codex challenge BLOCKED finding from parent) re-raised as a B blocker | Low | §1 + §3.5 explicitly classify it as out-of-topology. If reviewer panel disputes the topology framing itself, that's a §1 dispute and should L6-escalate before B implementation; not a B-level blocker. |

---

## §8 L0 Panel — Reviewer Matrix

| Reviewer | Lens |
|---|---|
| `/codex challenge` | Adversarial — construct in-topology same-machine attacks against the trim. Specifically test: can the `Actor::Mcp` path accidentally inherit the bypass? Can a malformed `topology` field smuggle a `LocalToLocalSingleUser` discriminator into a `RemoteToLocal` session? |
| `architect-reviewer` | Cross-cutting pattern compliance — is `ValidatedSurfaceSession.topology` the right seam? Does the category-keyed bypass propagate cleanly? Audit construction sites of `ValidatedSurfaceSession` (>12 expected). Cite file:line. |
| `security-auditor` | Amendment 3 (auth, scope, trust boundary). Honest evaluation: each trimmed gate, what does it actually defend in-topology? K-in `.docs/decisions/0102-abilities-as-runtime-contract.md` + `.docs/decisions/0111-surface-independent-ability-invocation.md` for prior policy. |
| `/codex consult` | Open-ended — phase ordering B1→B2→B3, AC realism, risk completeness. Anything in B that should be in C1 (or vice versa)? Does §3.5 (parent-rhetoric correction) belong here or in C1? |
| `/ce-doc-review` | Coherence — cross-refs, ambiguity, terminology drift. Confirm ADR slugs use full names (per parent doc-review #2). |

K-in obligation: every reviewer greps `docs/solutions/security-issues/` and `docs/solutions/architecture-patterns/` (correct directory names per parent codex consult). Specifically check for prior `ValidatedSurfaceSession` modifications and `authorize_for_path` change history.

Unanimous required.

---

## §9 Linear

Sub-issue of parent initiative "Trust Topology Realignment." Tag `substrate`, `auth`, `trust-boundary`, `blocked-by:C0`. Coordinates with v1.4.4 W2-C (the L4 unblock motivating this work). Estimated L1 wall-clock: 4–8h (substrate change + tests + L4 re-pair + proof bundle).

Path-α follow-ups (file as separate Linear tickets in maintenance project `b8e6aea4-d47e-4f3a-b03d-a05bec914aeb`):
- Block-showcase mu-plugin decision (parent §3.2 row 8)
- method_exists → is_callable sweep (parent §3.2 row 7)
- Refresh-endpoint defense-in-depth (out-of-topology but James may want it anyway)

---

## §10 Reading Order for the Panel

1. This packet §1 (with explicit prerequisites) — confirm topology framing
2. Parent `L0-packet.md` — historical context only; this packet supersedes parent §4 Phase B
3. `.docs/plans/engineering-ladder.md` §37 — load-bearing topology-scoping rule
4. `src-tauri/src/services/surface_pairing.rs:205-217` (`ValidatedSurfaceSession` struct) — the seam
5. `src-tauri/src/bridges/surface_client.rs:311-420` (`authorize_for_path`) — the gate machinery
6. `src-tauri/src/surface_runtime/mod.rs:1497-1570` (`surface_session_refresh_response`) — the endpoint codex challenge flagged in parent panel; confirm it's out-of-topology per §1
7. Original L0 panel verdict (in-conversation, 2026-05-21) for findings being addressed

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
