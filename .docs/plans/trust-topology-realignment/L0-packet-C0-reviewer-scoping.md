# L0 Packet C0 — Reviewer + Template Topology Scoping

**Status:** L0 DRAFT — not yet panel-reviewed
**Scope:** Reviewer-prompt updates + L0 packet template + AUTHORING.md / CLAUDE.md stubs
**Author:** James + Claude Opus 4.7
**Drafted:** 2026-05-21
**Engineering Ladder rung:** L0 (Plan); panel = `/codex challenge` + `architect-reviewer` + `/codex consult` + `/ce-doc-review` (security-auditor not triggered — no code paths touched)
**Pairs with:** parent `L0-packet.md` (Trust Topology Realignment), which split into C0/B/C1 per the original L0 panel
**Pre-reads:** `.docs/plans/engineering-ladder.md` §37 (already shipped at PR 348), parent packet §1 + §3.4

---

## §1 Trust Topology

**This packet's deployment topology: n/a — process change.**

This packet does not modify runtime code paths, transport boundaries, or data stores. It updates documentation, agent prompts, and a planning-doc template. The reviewer-scoping rule it operationalizes (engineering-ladder.md §37) is itself topology-agnostic — the rule is *every L0 packet declares §1 topology and reviewers scope to it*; the packet enforcing that rule has no topology of its own to defend.

For the surfaces whose threat models the updated reviewer prompts will scope against, the canonical taxonomy lives in engineering-ladder.md §37 — five rows: `local-to-local single-user | local-to-local multi-user | remote-to-local | remote-to-remote | untrusted-content`.

---

## §2 Goal

Make engineering-ladder.md §37's threat-topology framing operational — load the rule into every L0/L2/L3 reviewer prompt and into the L0 packet template so future packets can't ship without declaring §1 topology.

**Success criteria:**

1. Every reviewer prompt in `.github/reviewer-prompts/` cites engineering-ladder.md §37 and instructs the reviewer to scope threat-model claims to the PR's declared topology — refusing to synthesize multi-actor gates against a single-actor surface.
2. A new `.docs/plans/_templates/L0-packet.md` template exists with §1 trust-topology declaration as a required field; AUTHORING.md references it.
3. CLAUDE.md's L0 description names the §1 topology requirement (≤2 lines).
4. The parent packet's Phase B (substrate trim) can ship behind this packet without re-hitting the local-to-local-overreach failure mode at L2.

---

## §3 Inventory — Concrete Deliverables

This packet ships exactly seven file changes. No more, no less.

| # | File | Change |
|---|---|---|
| 1 | `.github/reviewer-prompts/security-auditor.md` | Add §0 "Threat-topology scoping" — cite engineering-ladder.md §37; instruct reviewer to read packet/PR header for declared topology; explicit rule "do NOT synthesize multi-actor gates against a single-actor surface; if attempting to do so, STOP and re-scope." |
| 2 | `.github/reviewer-prompts/architect-reviewer.md` | Same §0 addition, scoped to architectural concerns (cross-cutting pattern compliance MUST respect declared topology). |
| 3 | `.github/reviewer-prompts/code-reviewer.md` | Lighter §0 — reviewer notes the declared topology but defers topology-specific judgments to security-auditor / architect-reviewer; flags only when the diff contradicts the declared topology. |
| 4 | `.github/reviewer-prompts/l3-codex-challenge.md` | Same §0 addition; L3 adversarial scope inherits topology from the wave's plan. |
| 5 | `.github/reviewer-prompts/l3-architect-reviewer.md` | Same §0 addition; L3 integrated review inherits topology from the wave's plan. |
| 6 | `.docs/plans/_templates/L0-packet.md` (NEW) | Skeleton packet with §1 topology field marked `<!-- REQUIRED: declare one of the five topologies from engineering-ladder.md §37 -->`; includes the rest of the canonical L0 structure (§2 goal, §3 inventory, §4 phases, §5 AC, §6 out-of-scope, §7 risks, §8 panel, §9 Linear, §10 reading order). |
| 7 | `.docs/plans/AUTHORING.md` | Add brief subsection pointing to the new L0 template; note that L0 packets are Tier 3 (markdown-only). |

CLAUDE.md update (stub): in the L0 description under "## The Engineering Ladder," add one line: `Every L0 packet declares §1 trust topology (see engineering-ladder.md §37); reviewer panels scope threat models to that topology.`

---

## §4 Implementation

Single phase. All seven file changes land in one PR. No code paths touched.

**Steps:**

1. Draft a reusable "§0 Threat-topology scoping" block (~10 lines) referencing engineering-ladder.md §37 and listing the do-NOT pattern. Inline-paste into the five reviewer-prompt files with minor framing per reviewer's lens.
2. Write `.docs/plans/_templates/L0-packet.md` based on this packet's structure (it's a good reference shape — concrete inventory, single-phase, tight AC).
3. Add the one-line CLAUDE.md stub.
4. Add the AUTHORING.md template reference.
5. Run `pnpm tsc --noEmit && cargo clippy -- -D warnings && cargo test` — should all pass (no code touched, but run for hygiene).
6. Commit + L2 panel.

---

## §5 Acceptance Criteria

Concrete, verifiable per codex consult #4 from the parent packet's L0 panel:

1. `grep -l "engineering-ladder.md" .github/reviewer-prompts/*.md` returns at least 5 file paths (the 5 reviewer prompts listed in §3).
2. `grep -l "topology\|local-to-local" .github/reviewer-prompts/*.md` returns at least 5 file paths.
3. `.docs/plans/_templates/L0-packet.md` exists and contains the string `REQUIRED` adjacent to `topology` in its §1 header.
4. CLAUDE.md contains the exact phrase "Every L0 packet declares §1 trust topology" (case-insensitive grep).
5. AUTHORING.md contains a reference to `_templates/L0-packet.md`.
6. `pnpm tsc --noEmit` and `cargo clippy -- -D warnings` and `cargo test --lib` all pass on the PR branch.
7. No file outside the seven named in §3 (plus the CLAUDE.md + AUTHORING.md stubs) is modified by this PR.

---

## §6 Out-of-Scope (Defer to B and C1)

- **ADR-0102 / ADR-0111 / ADR-0129 amendments** → C1 packet.
- **Wave-plan topology classifier retroactive sweep** (v1.4.0-waves.md, v1.4.1-waves.md, etc.) → C1 packet.
- **Substrate code changes** (`ValidatedSurfaceSession.topology`, category-keyed bypass in `authorize_for_path`, refresh-endpoint rhetoric fix or hardening) → B packet, blocked-by this packet.
- **Memory promotion** (`feedback_local_to_local_security_overreach_primary_concern` → canonical) → C1 packet.
- **Reviewer-prompt updates to gstack `/cso` / `/plan-eng-review` / `/codex` skill prompts** — these live outside `.github/reviewer-prompts/` (under `.claude/skills/gstack/`) and are user-invoked, not CI-invoked. Out of scope for C0; revisit if friction surfaces in B's L0 panel.

---

## §7 Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| Reviewer-prompt update misses an edge case ("trust topology" interpreted differently across reviewers) | Low | The §0 block is identical across files; only framing varies. Engineering-ladder.md §37 is the canonical source of truth referenced by all five. |
| L0 packet template too prescriptive (forces unnecessary structure on small packets) | Low-Medium | Template is a reference shape, not a CI gate. Smaller packets can drop optional sections; only §1 topology is structurally required. |
| Future contributors don't read the template and ship packets without §1 topology | Medium | C1 follow-up adds a `/plan-eng-review` skill check that fails if §1 topology is missing. Out of C0 scope; tracked. |
| CLAUDE.md stub causes friction with the CLAUDE.md ~5-line budget for the Engineering Ladder section | Low | One line added; section stays under budget. |

---

## §8 L0 Panel — Reviewer Matrix

| Reviewer | Lens |
|---|---|
| `/codex challenge` | Adversarial — is there an attack on the topology-scoping rule itself? Reviewer-prompt injection? Topology-evasion via creative packet framing? |
| `architect-reviewer` | Cross-cutting pattern compliance — do the five §0 inserts propagate consistently? Is the template structurally coherent with existing packets (parent L0, v1.4.4 wave packets)? |
| `/codex consult` | Open-ended — is C0 sized right? Anything missing that would block B? Anything in C0 that should be in C1 (or vice versa)? |
| `/ce-doc-review` | Coherence — cross-refs (engineering-ladder §37, ADR-0093/0108) resolve; terminology drift; ambiguity. |

`security-auditor` does NOT trigger — no code paths touched, no Amendment 3 triggers.

K-in obligation: each reviewer greps `docs/solutions/workflow-issues/` and `docs/solutions/architecture-patterns/` (note: directory names are `*-issues` and `*-patterns`, not `security/` and `architecture/` — caught by codex consult in parent L0 panel).

Unanimous required per engineering-ladder.md L0 Plan pass rule.

---

## §9 Linear

Sub-issue of parent initiative "Trust Topology Realignment." Tag `topology`, `reviewer-prompts`, `docs-only`, `unblocks:trust-topology-realignment-B`. No L4 (no user-facing surface). No migration slot needed. Estimated L1 wall-clock: 30–60 min (5 small reviewer-prompt edits + 1 template + 2 stubs).

---

## §10 Reading Order for the Panel

1. This packet §1 + §2 + §3 (frame, goal, exact deliverables)
2. `.docs/plans/engineering-ladder.md` §37 — the rule being operationalized
3. Parent packet `.docs/plans/trust-topology-realignment/L0-packet.md` — context for why C0 was split out (specifically §3.4 reviewer-infrastructure row, which C0 implements)
4. `.github/reviewer-prompts/security-auditor.md` and `.github/reviewer-prompts/architect-reviewer.md` — current state of the two anchor reviewer prompts
5. Original L0 panel verdict (in-conversation, 2026-05-21 session) — codex consult #2 and architect-reviewer #5 both required this ordering inversion

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
