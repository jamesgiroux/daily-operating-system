# L0 Packet C1 — ADR + Wave-Plan Topology Retrofits

**Status:** SUPERSEDED 2026-05-21 — L0 panel surfaced 14 findings; ADR amendments + wave retrofits are premature. Deferred until after the minimal trim PR ships and the actual shape is known. PR 347 (MCP v2 substrate) is queueing ADR-0102 cycle-7/8/9 amendments to dev — let those land first so we know what we'd be amending against.
**Scope:** ADR amendments + wave-plan §1 topology classifier sweep + memory promotion
**Author:** James + Claude Opus 4.7
**Drafted:** 2026-05-21
**Engineering Ladder rung:** L0 (Plan); panel = `architect-reviewer` + `/codex consult` + `/ce-doc-review` (light panel — docs-only; no security-auditor since no code paths; codex challenge optional but recommended for adversarial reading of ADR amendments)
**Blocked by:** [`L0-packet-B-substrate-trim.md`](./L0-packet-B-substrate-trim.md) must merge first so ADR amendments cite what shipped.
**Pairs with:** parent `L0-packet.md`, C0, B

---

## §1 Trust Topology

**This packet's deployment topology: n/a — process change (docs-only).**

C1 modifies ADRs, wave-plan documents, and a memory entry. No runtime code, no transport boundaries, no data stores. Same posture as C0.

---

## §2 Goal

Bring durable architectural docs and historical wave plans into alignment with the topology framing now operational in engineering-ladder.md §37 (PR 348) and the substrate behavior shipped in B. Future packets and future readers should encounter consistent framing across CLAUDE.md → engineering-ladder.md → ADRs → wave plans → memory.

**Success criteria:**

1. ADR-0102, ADR-0111, ADR-0129 each carry an amendment block explicitly addressing local-to-local single-user SurfaceClient posture, with cross-references to engineering-ladder.md §37 and the B PR.
2. Every wave plan (v1.4.0-waves.md, v1.4.1-waves.md, v1.4.3-waves.md, v1.4.4-waves.md) carries a retroactive §1 topology classifier (one line — most are `local-to-local single-user`).
3. The canonical memory entry on local-to-local security overreach is promoted to a reference doc; related per-wave feedback memos link to it.
4. No code paths touched; CI gates pass on documentation-only diff.

---

## §3 Inventory — Concrete Deliverables

### §3.1 ADR amendments

| File | Change |
|---|---|
| `.docs/decisions/0102-abilities-as-runtime-contract.md` | Add amendment block: "Local-to-local single-user SurfaceClient carve-out." Document that `Actor::SurfaceClient` sessions whose `ValidatedSurfaceSession.topology == LocalToLocalSingleUser` bypass `required_scopes` and `requires_confirmation` for `AbilityCategory::Read`. Third-party `Actor::Mcp` path unaffected. Cross-reference engineering-ladder.md §37 and the B PR commit. |
| `.docs/decisions/0111-surface-independent-ability-invocation.md` | Add amendment block: "Topology dimension on session, not actor." Document that the channel-property `ValidatedSurfaceSession.topology` carries trust topology; `Actor` enum stays wire-level. Symmetric framing across SurfaceClient and McpClient preserved per ADR-0111's original §8. |
| `.docs/decisions/0129-composable-surfaces-wordpress-studio-as-primary-surface.md` | Rewrite §threat-model (or add explicit §threat-model section if missing) to state the local-to-local single-user posture: OS account boundary is the trust boundary; HMAC defends in-flight replay/tampering; same-OS-user impersonation is out-of-topology by framing. Cross-reference engineering-ladder.md §37. |

### §3.2 Wave-plan §1 topology classifier sweep

For each wave plan, add a one-line `**Trust topology:** <classifier>` to the §1 header (or top-of-file metadata block if no §1 exists).

| File | Classifier |
|---|---|
| `.docs/plans/v1.4.0-waves.md` | local-to-local single-user |
| `.docs/plans/v1.4.0-waves-amendments.md` | local-to-local single-user |
| `.docs/plans/v1.4.1-waves.md` | local-to-local single-user |
| `.docs/plans/v1.4.1-waves-amendments.md` | local-to-local single-user |
| `.docs/plans/v1.4.3-waves.md` | local-to-local single-user |
| `.docs/plans/v1.4.4-waves.md` | local-to-local single-user |

Note: every shipped DailyOS wave to date has been local-to-local single-user; the sweep mostly documents this rather than discovering new framings.

Plans whose topology was over-scoped (defended multi-actor when the deployment was single-actor) get a `<!-- topology drift — substrate hardened multi-actor when shape was single-actor; addressed in B -->` annotation in the affected wave's relevant section, NOT a rewrite. The retrofit is documentary; B already fixed the substrate.

### §3.3 Memory promotion + consolidation

| File | Change |
|---|---|
| `/Users/jamesgiroux/.claude/projects/-Users-jamesgiroux-Documents-dailyos-repo/memory/feedback_local_to_local_security_overreach_primary_concern.md` | Promote to canonical reference; rewrite body to point at engineering-ladder.md §37 as the now-load-bearing source. Memory entry becomes a pointer + the 2026-05-21 reinforcement context, not a parallel rule restatement. |
| Related memos (`feedback_wp_is_local_surface_not_remote`, `feedback_premise_check_production_vs_dev_friction`, `feedback_canonical_signing_changes_invalidate_pairings`, `feedback_dont_swing_past_center_when_correcting`) | Add cross-link `[[local-to-local-security-overreach-primary-concern]]` at the end of each. No rewrites. |
| `MEMORY.md` (index) | Update the local-to-local entry's hook to note it's the canonical reference now. |

### §3.4 README / cross-doc references

| File | Change |
|---|---|
| `.docs/plans/README.md` (if exists) | Add reference to engineering-ladder.md §37 and the trust-topology-realignment packet trio (C0, B, C1). |
| `docs/solutions/README.md` | One-line note: "Topology-scoped findings now distinguish in-topology vs out-of-topology per engineering-ladder.md §37." |

---

## §4 Implementation

Single phase, single PR. Docs-only diff.

**Steps:**

1. Read each ADR top-to-bottom before drafting amendments. Amendment blocks at end of file with date + cross-reference; do not rewrite existing content unless §threat-model needs replacement in ADR-0129.
2. Sweep wave plans — single-line addition each.
3. Memory promotion: read existing entry, rewrite to canonical-reference shape, update MEMORY.md index hook.
4. Cross-link sweep across related memos.
5. CI run for hygiene (no code changes expected to fail).
6. Commit + L2 panel.

---

## §5 Acceptance Criteria

1. **ADR amendments exist.** `grep -l "engineering-ladder.md §37\|engineering-ladder.md#37" .docs/decisions/0102*.md .docs/decisions/0111*.md .docs/decisions/0129*.md` returns all three files.
2. **Wave-plan sweep complete.** `grep -l "Trust topology:" .docs/plans/v1.4.*-waves*.md` returns at least 6 files (the six listed in §3.2).
3. **Memory promoted.** `feedback_local_to_local_security_overreach_primary_concern.md` body length reduced (canonical-pointer shape, not parallel rule); cross-references engineering-ladder.md §37; MEMORY.md hook updated.
4. **No code changes.** `git diff --name-only origin/dev...HEAD` returns only `.md` paths (no `.rs`, `.ts`, `.php`, `.json`, `.yml` outside frontmatter).
5. **CI gates pass.** `cargo clippy -- -D warnings && cargo test --workspace && pnpm tsc --noEmit && pnpm test` all green (no-op for this PR but run for hygiene).
6. **ADR slug consistency** (per parent doc-review #2): every ADR-0129 reference in the new amendments uses the full slug `ADR-0129-composable-surfaces-wordpress-studio-as-primary-surface`, not bare `ADR-0129`.

---

## §6 Out-of-Scope

- **Substrate changes** — already in B.
- **Reviewer-prompt updates** — already in C0.
- **New ADRs** — C1 amends existing ADRs only; no new ADRs introduced.
- **L0 packet template** — already in C0.
- **HMAC canonical rollover** (DOS-746) — separate track.
- **Refresh-endpoint hardening** — filed as Linear maintenance ticket in B §9.
- **Engineering-ladder.md changes** — PR 348 already shipped §37; C1 only references it.

---

## §7 Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| ADR amendment contradicts the original ADR rather than amending it | Low | Amendment block at end of file with explicit "Amendment YYYY-MM-DD" header; original body untouched. Reviewer panel reads original first to confirm coherence. |
| Wave-plan sweep mis-classifies a historical wave's topology | Low | All v1.4.x waves to date are local-to-local single-user. Sweep is mostly mechanical. If a reviewer flags a mis-classification, easy single-line fix. |
| Memory promotion loses important nuance from the original feedback entry | Low | Keep the 2026-05-21 reinforcement context inline; canonical pointer is to engineering-ladder.md §37 but the lived context stays. |
| Future docs-only sweeps in other repos cause merge conflicts | Low | C1 lands quickly; conflict window is small. |

---

## §8 L0 Panel — Reviewer Matrix

| Reviewer | Lens |
|---|---|
| `architect-reviewer` | Cross-cutting pattern compliance — do the three ADR amendments stay coherent with their original framing? Wave-plan sweep mechanical and consistent? |
| `/codex consult` | Open-ended — is C1 sized right? Anything in C1 that should be deferred or merged into another track? |
| `/ce-doc-review` | Coherence — cross-refs, ADR slug consistency, terminology drift, ambiguity in amendment language. |
| `/codex challenge` (optional) | Adversarial reading of ADR amendments — does the carve-out language leave room for misuse? |

`security-auditor` does NOT trigger — no code paths touched.

K-in obligation: each reviewer reads engineering-ladder.md §37, B's amendment block citations, and the three ADRs in current state before scoring.

Unanimous required.

---

## §9 Linear

Sub-issue of parent initiative "Trust Topology Realignment." Tag `docs-only`, `adr`, `memory`, `blocked-by:B`. Estimated L1 wall-clock: 60–90 min (3 ADR amendments + 6 wave-plan one-liners + 1 memory promotion + small sweep).

---

## §10 Reading Order for the Panel

1. This packet §1 + §2 + §3 (frame, goal, exact deliverables)
2. `.docs/plans/engineering-ladder.md` §37 — the load-bearing rule
3. ADR-0102, ADR-0111, ADR-0129 in current state — to confirm amendments don't contradict
4. Original L0 panel verdict (in-conversation 2026-05-21) for findings being addressed
5. C0 and B packets — to confirm C1 closes the loop without overlap

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
