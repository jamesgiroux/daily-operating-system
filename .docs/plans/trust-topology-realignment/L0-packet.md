# L0 Packet — Trust Topology Realignment

**Status:** L0 DRAFT — not yet panel-reviewed
**Scope:** Codebase + plan-document sweep
**Author:** James + Claude Opus 4.7
**Drafted:** 2026-05-21
**Engineering Ladder rung:** L0 (Plan); panel = `/codex challenge` + `architect-reviewer` + `/cso` + `/codex consult`
**Pairs with memory:** `feedback_local_to_local_security_overreach_primary_concern`, `feedback_wp_is_local_surface_not_remote`, `feedback_premise_check_production_vs_dev_friction`, `feedback_canonical_signing_changes_invalidate_pairings`, `feedback_dont_swing_past_center_when_correcting`

---

## §1 Trust Topology Framing

DailyOS today ships in one deployment shape:

> **local-to-local single-user** — the user runs the Tauri runtime on their own machine; the WordPress surface (when present) runs in their local Studio (or future packaged WP) on the same machine; both surfaces represent the same human user; the only transport between them is loopback HTTP.

The codebase, however, is hardened as if it ships in four shapes — the most permissive of which it claims to want to defend (remote third-party multi-user agents reading data through the same APIs the user's own React window uses). That divergence between *actual deployment topology* and *threat model encoded in code* is the root cause of the L4 friction surfaced on 2026-05-21: every empty render on `/accounts/bring-a-trailer/` traced back to a security layer (capability check on the credential store, narrow `DEFAULT_GRANTED_SCOPES` from v1.4.2, missing `SurfaceClient` in `allowed_actors` for read abilities, attestation prompt awaiting a UI that hasn't been built) — each gate defensible alone, additively unusable.

**This packet's job:** realign the substrate's threat model to the actual deployment topology, trim the gates that only defend against fantasy shapes, and put a classifier in place so new packets don't repeat the mistake.

### Trust topology taxonomy (proposed for §1 of every future L0 packet)

| Topology | Who is the caller? | Transport | Threat model |
|---|---|---|---|
| **local-to-local single-user** | The same human, same machine, signed pairing | Loopback HTTP / Tauri invoke | Same as a desktop app: integrity of binaries, file permissions, OS account boundary. Cross-surface gates inside this boundary are theatre. |
| **local-to-local multi-user** | Same machine, different OS users sharing one Tauri runtime | Loopback HTTP | Real but uncommon. Per-user pairing isolates this. |
| **remote-to-local** | A remote MCP client (third-party agent) | HTTPS over the public internet | Real threat. Full HMAC + scope sets + attestation + audit log warranted. |
| **remote-to-remote** | Hypothetical multi-tenant cloud DailyOS | HTTPS | Not currently a deployment shape. Defer hardening until it is. |
| **untrusted-content** | The user's own pipelines ingesting external data (Drive docs, transcripts, AI-generated text) | n/a — orthogonal to caller | Always real (ADR-0093 indirect prompt injection, ADR-0108 sensitivity redaction). Independent of caller topology. |

**Rule the substrate must encode:** every gate, every allowlist, every confirmation prompt, every scope set should declare which topology it defends against. Gates that defend only the third-party shapes should be inactive when the caller is local-to-local single-user.

---

## §2 Goal

Make the user's experience of WordPress-as-primary-surface (ADR-0129) match the user's experience of Tauri-as-surface — same data, same speed, same trust surface — without weakening the gates that defend the genuinely-remote and untrusted-content cases.

**Success criteria:**

1. Every W2/W3 entity-detail surface (account / project / person / meeting / briefing) renders end-to-end through the WP loopback path with the same data fidelity as the equivalent React route, with no manual re-pairing required after substrate adds new abilities.
2. Every gate left in the codebase has a documented topology classification in its source comment or ADR (`# topology: local-to-local single-user | local-to-local multi-user | remote-to-local | untrusted-content`).
3. The `DEFAULT_GRANTED_SCOPES` model is either eliminated for local-to-local pairings (the WP loopback becomes a User-equivalent actor with no separate scope set) OR replaced with auto-expanding defaults that match the substrate's current ability set without manual updates per wave.
4. ADRs that encoded the old "WP is a third party" framing are rewritten or supplemented with the new framing; downstream packets reference the topology classifier in their §1 header.
5. No regression in the third-party MCP path — `mcp_exposure = Invocable` abilities still gate scopes, attestation, rate limits.

**Non-goals:**

- Adding new abilities or surfaces.
- Reshaping the abilities-runtime contract for non-security reasons.
- Touching the untrusted-content boundary (sensitivity / indirect prompt injection / Drive ingest). Those remain as-is per the reinforcement note in the security-overreach memory.
- Refactoring the HMAC signing canonical or the pairing handshake itself (still warranted as transport-layer defense; see `feedback_canonical_signing_changes_invalidate_pairings` for the separate fix track on canonical churn).

---

## §3 Inventory — Current Gates and Their Topology Classification

This is the audit-first checklist. Each gate gets reviewed during L0 panel; classification (warranted / trim / eliminate) decided unanimously.

### §3.1 Tauri runtime (`src-tauri/`)

| Gate | Location | Currently defends against | Topology classification | Proposed action |
|---|---|---|---|---|
| `allowed_actors` includes `SurfaceClient` | every `#[ability(...)]` in `abilities-runtime/src/abilities/*` | Third-party MCP / unauthorized surface impersonation | Remote-to-local. For local-to-local SurfaceClient = same user, gate is theatre. | **Trim:** SurfaceClient default-allowed on Read category. Keep gate for Write / Maintenance / experimental. |
| `required_scopes = ["read.X"]` per ability | every `#[ability(...)]` | Surface only granted a subset of capabilities | Remote-to-local (scope as least-privilege). For local-to-local same user, the user already has all capabilities. | **Eliminate for local-to-local:** SurfaceClient.actor when pairing is local-to-local should bypass scope check entirely. Keep for `Actor::Mcp` (third-party) and `Actor::Agent` (sub-agent). |
| `requires_confirmation = true` + `UserAttestationHost::request_user_attestation` | `bridges/tauri.rs:33`, `state.rs:1054` | User confirms before each cross-surface invoke | Remote-to-local (UI consent for third-party). Local-to-local single-user = the user already invoked. | **Trim:** skip attestation on local-to-local read paths; keep for Write + remote MCP. Per-ability `requires_confirmation` declaration becomes a hint, not a hard gate when topology is local-to-local. |
| `descriptor.experimental` blocks SurfaceClient | `bridges/surface_client.rs:347` | Experimental abilities not exposed to surfaces | Generic — applies to all topologies | **Keep** — experimental discipline is orthogonal. |
| `client_side_executable = false` + `BrowserDirectJs` rejection | `bridges/surface_client.rs:375` | Tauri-only abilities not callable from browser context | Remote-to-local for browser-side JS. WP block render is PHP, not browser JS. | **Keep gate for browser-direct; clarify in code that PHP server-side render is not browser-direct.** |
| `SurfaceClientRateLimitRequest::check_and_consume` | `bridges/surface_client.rs:407+` | Per-surface rate cap | Remote-to-local. Local-to-local single-user the rate cap is unnecessary friction. | **Trim:** orders-of-magnitude wider limits for local-to-local; keep current ceilings for remote MCP. |
| `DEFAULT_GRANTED_SCOPES` hardcoded v1.4.2 set | `services/surface_pairing.rs:33` | Surface scope-set granted at pairing time | Remote-to-local | **Eliminate** for local-to-local: pairing grants `*` (or User-actor equivalence). For remote MCP keep explicit scope-set grant. |
| `request_confirmation_attestation` awaits Tauri UI prompt that doesn't exist | `state.rs:1059` (TODO W5/W6) | Per-invocation user click | Remote-to-local | **Eliminate the W5/W6 UI requirement** for local-to-local; the W5/W6 attestation UI is only built if remote MCP needs it. |

### §3.2 WordPress plugin (`wp/dailyos/`)

| Gate | Location | Currently defends against | Topology classification | Proposed action |
|---|---|---|---|---|
| `current_user_can('manage_options')` capability check before retrieving session key | `includes/transport/class-dailyos-credential-store.php:183` | Non-admin WP users reading the user's runtime data | Local-to-local multi-user (admin protects from logged-in non-admin sharing the same Studio) | **Re-scope:** if Studio runs single-user (no other WP users), check is theatre. For real multi-WP-user case (future shared install), keep it. Make conditional on `is_multisite() || count(wp_users) > 1`. |
| `dailyos_invoke_mcp_ability` custom capability | `includes/transport/class-dailyos-credential-store.php:185` | MCP runtime can invoke as substrate user | Remote-to-local (MCP) | **Keep** — this is the third-party path. |
| HMAC-signed POST with replay protection | `includes/transport/class-dailyos-runtime-client.php` | Same-machine port-spoofing / replay across sessions | Generic transport defense | **Keep** — transport layer is genuinely useful even local-to-local (prevents another process on the same machine from impersonating the WP plugin). |
| Re-pairing required after HMAC-canonical change | `feedback_canonical_signing_changes_invalidate_pairings` | Signing field drift | Same-class shape as scope set drift | **Address in this initiative:** canonical version bumps should auto-roll the pairing forward, not silently invalidate. (DOS-746 is the partial fix track; supersede.) |
| `apply_filters('dailyos_surfaceclient_resolved_scopes', [])` passed to runtime | `blocks/*/render-functions.php` | Surface tells runtime what scopes it has | Remote-to-local convention | **Make advisory-only:** the WP block sends its scope set as a hint; runtime authoritatively decides based on pairing-side identity (which it already does). When pairing is local-to-local, scope set is unused. |
| `method_exists()` on runtime client in render-functions | every W2 inner-block render | Defensive check against composite/wrapper clients | Implementation defense, not security | **Replace with `is_callable()`** so `__call`-handled methods (showcase mu-plugin, future proxies) work. Done in account-detail today; needs sweep across all blocks. |
| `mu-plugins/dailyos-block-showcase.php` wraps client in composite | Studio-side dev shim, not shipped | v1.4.2 showcase rendering | Pre-v1.4.4; broken under current model | **Either rewrite to v1.4.4 envelope shape, or delete.** Decision in this initiative. |

### §3.3 Plan documents and ADRs

| Document | What it codifies | Action |
|---|---|---|
| `.docs/decisions/ADR-0129-wordpress-as-primary-surface.md` | WP is primary surface representing the user | **Update §threat-model:** explicitly state the local-to-local single-user posture; supersede the parts that encoded "treat WP as remote." |
| `.docs/decisions/ADR-0102-ability-policy.md` (if exists; referenced by macro error message) | SurfaceClient must declare required_scopes | **Update:** add carve-out for local-to-local SurfaceClient. |
| `.docs/decisions/ADR-0093-indirect-prompt-injection.md` | Untrusted-content topology | **Reaffirm — leave as-is.** This is orthogonal to caller topology. |
| `.docs/decisions/ADR-0108-sensitivity-redaction.md` | Log/screenshot data hygiene | **Reaffirm — leave as-is.** Orthogonal. |
| `.docs/plans/engineering-ladder.md` | L0–L6 process | **Add §Trust Topology Framing:** every L0 packet §1 declares topology; reviewer panels scope their threat model to that topology. |
| `.docs/plans/v1.4.x-waves.md` (all) | Wave plans for v1.4.0–v1.4.6 | **Add topology classifier retroactively** to each wave's §1; flag wave plans whose threat model was over-scoped. |
| `wp/dailyos/L0-*` packets | Per-packet plans for WP work | **Sweep:** add §1 topology classifier; remove gates that only defend against fantasy shapes. |

### §3.4 Reviewer infrastructure

| Reviewer | Current behavior | Action |
|---|---|---|
| `/cso` (CSO mode security audit) | Applies remote-actor threat model uniformly | **Update prompt:** scope threat model to packet's §1 topology classifier. For local-to-local single-user, focus on integrity / supply chain / untrusted-content; deprioritize multi-actor confidentiality. |
| `codex challenge` | Adversarial gate-construction | **Update prompt:** topology-aware threat construction; don't synthesize multi-actor gates for single-actor surfaces. |
| `architect-reviewer` | Cross-cutting pattern review | **Update prompt:** check that gates' topology classification matches the packet's stated topology. |
| `/plan-eng-review` | Plan-level architecture | **Update:** require §1 topology classifier before approving any L0. |

---

## §4 Implementation Phases

Three serial phases. Phase A unblocks the current wave (v1.4.4 L4) within hours; Phase B normalizes the substrate over days; Phase C updates plans/ADRs/reviewers over a week.

### Phase A — Unblock v1.4.4 L4 (minimum patch, already in flight 2026-05-21)
- Add `SurfaceClient` to `allowed_actors` on 8 read abilities ✅ (done in session)
- Expand `DEFAULT_GRANTED_SCOPES` to cover the W2 read set ✅ (done in session)
- Disable the v1.4.2 showcase mu-plugin so it doesn't wrap the client incompatibly ✅ (done in session)
- Replace `method_exists` → `is_callable` in account-detail outer block ✅ (done in session)
- Re-pair WP→Tauri (user manual click) to pick up the new defaults
- **Defines the floor.** Everything in Phase B/C builds on this; nothing here is the final design.

### Phase B — Substrate trim
- Topology classifier as data on `Actor::SurfaceClient { topology: TrustTopology, ... }`; pairing flow records the topology at pair time
- Scope checks bypass when `topology = LocalToLocalSingleUser` on Read category abilities
- Attestation host bypassed on local-to-local Read
- Rate limits widened (or eliminated) for local-to-local SurfaceClient
- Sweep `method_exists` → `is_callable` across all inner-block render-functions
- WP `current_user_can('manage_options')` gate becomes conditional on multi-user detection
- Cap `DEFAULT_GRANTED_SCOPES` issue at the root: either eliminate scope set for local-to-local or auto-derive from registered Read abilities at pair time
- HMAC-canonical bumps roll pairings forward (DOS-746 absorption)
- Decide: rewrite block-showcase mu-plugin for v1.4.4 envelope shape, or delete

### Phase C — Plan + ADR + reviewer alignment
- ADR-0129 §threat-model rewrite
- ADR-0102 carve-out
- engineering-ladder.md §Trust Topology Framing
- v1.4.x-waves.md retroactive topology classifier
- `/cso`, `codex challenge`, `architect-reviewer`, `/plan-eng-review` prompts updated
- New L0 template includes §1 topology classifier as required field
- Memory entry promoted: `feedback_local_to_local_security_overreach_primary_concern` becomes the canonical reference; old wave-specific feedback memos linked

---

## §5 Acceptance Criteria

1. Every W2/W3 entity-detail surface renders end-to-end through WP loopback against a fresh pairing, no manual scope grant required.
2. Adding a new Read ability does not require re-pairing every existing WP surface.
3. Every gate in `src-tauri/src/bridges/`, `src-tauri/src/surface_runtime/`, `wp/dailyos/includes/transport/` carries a `# topology:` comment declaring which trust topology it defends.
4. `/cso` audit run on the substrate post-trim produces no high/critical findings against the local-to-local topology (genuine findings against remote-to-local / untrusted-content topology preserved).
5. ADR-0129 explicitly states local-to-local single-user posture; ADR-0102 carve-out for SurfaceClient documented.
6. New L0 packet template requires §1 topology classifier; engineering-ladder.md documents the framework.
7. Three follow-up wave packets (v1.4.5 onwards) demonstrate the topology classifier in their §1 with reviewer panels scoping accordingly.
8. No regression in remote MCP path — third-party invocations still gate scope, attestation, rate-limit.

---

## §6 Out-of-Scope (Defer or Separate Track)

- **Untrusted-content boundary** (ADR-0093 indirect prompt injection, ADR-0108 sensitivity redaction). These are orthogonal to caller topology and remain as-is.
- **HMAC canonical-version migration** (DOS-746). Adjacent — this packet calls out the same-class problem (substrate evolution silently invalidating pairings) but the canonical-version fix is its own scope.
- **Multi-user Studio support.** If/when DailyOS supports multiple WP users sharing one Tauri runtime, that's `local-to-local multi-user` and the gates re-activate accordingly. Defer until the deployment shape is real.
- **Remote-to-remote (multi-tenant cloud DailyOS).** Not a deployment shape. Do not harden.
- **MCP exposure rules** for third-party agents. Stay as-is; this initiative does not touch the `mcp_exposure = Invocable` lattice.

---

## §7 Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Over-trim removes a gate that DOES defend a real local-to-local threat | Medium | Phase A is reversible; each Phase B gate change reviewed under updated `/cso` + architect-reviewer prompts with topology in mind |
| Topology classifier becomes another vocabulary nobody uses | Medium | Engineering-ladder.md makes it a required field; reviewer panels enforce |
| ADR rewrites contradict existing in-flight v1.4.x packets | Low | Phase C runs AFTER v1.4.4 / v1.4.5 close; coordinate with wave leads |
| Third-party MCP path quietly affected | Low | Acceptance §8 explicit; regression suite covers MCP |
| `cargo test` requires updates to assertion-bake-in tests that assumed narrow scope sets | Medium-High | Treat as expected scope cost; budget L1 for sweep |

---

## §8 L0 Panel — Reviewer Matrix

Per engineering-ladder.md §Skill matrix, L0 unanimous required:

| Reviewer | Lens |
|---|---|
| `/codex challenge` | Adversarial — what's the gate this packet removes that I can exploit? Construct local-to-local single-user attack scenarios; if any are realistic, the trim is wrong. |
| `architect-reviewer` | Cross-cutting pattern compliance — does the topology classifier propagate cleanly? Are gates consistent within a topology? |
| `/cso` | Security lens — Amendment 3 triggers (auth, trust boundary, scope). Honest comparison: what does each removed gate actually defend? |
| `/codex consult` | Open-ended scope review — is this packet trying to fix too much / too little? |
| `/plan-eng-review` | Plan-level architecture — does Phase A → B → C ordering survive contact with reality? |

K-in obligation: every reviewer greps `docs/solutions/` + `.docs/decisions/` (especially ADR-0129, ADR-0102, ADR-0093, ADR-0108) before scoring.

---

## §9 Linear

- Open Linear initiative: **"Trust Topology Realignment — local-to-local single-user posture across substrate"**
- Phase A → existing v1.4.4 wave (already in flight; L4 unblock)
- Phase B → new initiative-level milestone, ~1–2 weeks of substrate work
- Phase C → new initiative-level milestone, plan/ADR/reviewer documentation, ~1 week
- Path-α follow-ups for any gates the panel keeps but flags for revisit

---

## §10 Reading Order for the Panel

1. **This packet §1 + §2** — frame and goal
2. **Memory `feedback_local_to_local_security_overreach_primary_concern`** (including the 2026-05-21 reinforcement note from v1.4.5 W2-C L0 cycle 3) — durable context on why we're here
3. **ADR-0129** — the canonical "WP is primary surface" decision
4. **`src-tauri/src/bridges/surface_client.rs:311-420`** (`authorize_for_path`) — the actual gate machinery
5. **`src-tauri/src/services/surface_pairing.rs:2346`** (`default_granted_scopes`) — the silent v1.4.2-era constant
6. **`wp/dailyos/includes/transport/class-dailyos-credential-store.php:179-189`** — the WP-side capability gate
7. **2026-05-21 session transcript / HANDOFF** — concrete example of how the gates compose into "WP shows nothing while React shows everything"

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
