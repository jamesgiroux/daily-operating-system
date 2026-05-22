---
title: "Debug vortex on a user-visible surface — dispatch codex for clean-context diagnosis after 2h, do not keep patching adjacent symptoms"
problem_type: workflow_issue
track: knowledge
module: CLAUDE.md (debugging discipline), .docs/plans/engineering-ladder.md (L4 workflow), engineering-discipline (Rule 6 goal-driven loops)
tags: [debugging, l4, vortex, codex-dispatch, step-back, adjacent-bugs, dos-764, dos-761, dos-762, get-entity-intelligence, producer-attribution]
date: 2026-05-22
related_linear: DOS-764, DOS-761, DOS-762, DOS-168
related_memories: [feedback_parallel_codex_for_diagnosis_when_vortexing, feedback_step_back_after_repeated_patches, feedback_l4_debugging_can_hide_overengineering, feedback_two_similar_bugs_class_review]
---

## Context

v1.4.4 chip-rendering work spanned ~8 hours of debug + substrate revisions across two sessions (2026-05-21 evening + 2026-05-22 morning). The user-visible symptom was constant: WordPress block surfaces (`bring-a-trailer` account page, `meeting-detail` test pages) rendered empty chips or generic "Account identity unavailable" messages.

Adjacent fixes shipped during the vortex (all real, all merged):

| Commit / PR | Fix | Was-it-the-blocker |
|---|---|---|
| PR #351 (`b218d0fa`, `16f5bad3`, `52a2c49f`, `255de03b`) | SurfaceClient audit attribution, envelope.ability.data unwrap, surface-prefix inner block collision-slug rename, WP transport `input` not `payload` | No |
| PR #360 (DOS-762) | Wire 4 live workspace readers + split BridgeSurfaceError to distinct wire codes + log::warn! instrumentation | No |
| PR #358 (DOS-761) | First-party WP loopback as `Actor::User` via new `/v1/local/invoke` route (skip HMAC/session for trusted same-OS-user invokes) | No |
| PR #359 (DOS-168 amended) | MCP v2 substrate at simplified shape; transport-ceremony layer never introduced | No |
| `90af0f16` | WP_Block context propagation for default-template inner blocks (mock-driven path) | No (mock-only; real path still broken) |
| `e0ddd1a3` | Default-template fallback when content empty in meeting-detail / account-detail | No |
| `dcdd0f10` | Restore v1.4.2 account-overview block for compare-against-known-good | Diagnostic only |

After all of the above merged + the WP mock plugin shipped + the substrate trust-model reframe landed cleanly, the real-data path STILL returned the same misleading wire code.

A **separate Codex session, with no conversation context**, traced the call path in ~30 minutes and identified the actual blocker (DOS-764 — `get_entity_intelligence/producer.rs` only attributes `attributes` + `schemaVersion`; every other envelope leaf — `subject`, `sections`, `facts`, `recordEntries`, etc — is un-attributed; `ProvenanceBuilder::finalize()` rejects; `AbilityInvokeError::Ability` propagates; `bridges/types.rs:1003` flattens to `BridgeSurfaceError::AbilityUnavailable` → wire code `ability_not_registered` HTTP 404).

## Why adjacent fixes were not the blocker

Each adjacent fix was a real bug whose absence would have surfaced eventually, but none short-circuited the user-visible symptom because the producer never produced a valid envelope. The wire code was constant from the surface — only the *cause* of `ability_not_registered` changed: it had been "ability literally not registered" (would have been fixed by reader wiring) → "ability registered but producer errors" (still hidden behind same wire code post-PR-360 because the new `ProducerUnavailable` variant wasn't routed in `surface_error()`).

The trust-model reframe (DOS-761) was load-bearing for substrate hygiene + future MCP work, but it was *not* the path to chip rendering. The mock plugin (overnight) made the broken real path harder to diagnose because every chip rendered against canned data, masking the producer failure.

## Resolution

DOS-764 filed (High, Bug, Codebase Maintenance) with the producer-attribution fix scope. A sibling Codex session was already drafting the fix as part of the same diagnosis pass. Real-data chip rendering on `bring-a-trailer` now produces output (mostly raw claim IDs visible — separate downstream renderer work, not blocked on substrate).

## Lesson

When a debug session is **2h+ on the same user-visible symptom** with adjacent fixes that don't move the symptom needle, the correct move is **NOT** to keep patching. The pattern that fails:

1. Find an adjacent bug — real bug
2. Fix it — substrate genuinely improves
3. Symptom still present
4. Find another adjacent bug — also real
5. Loop

Adjacent fixes compound substrate value but they BURY the actual blocker. The agent stuck in the vortex keeps seeing real signal (each fix is genuinely correct) so it doesn't trigger a step-back.

**The intervention that worked: dispatch a fresh Codex agent with the user-visible symptom + the call path from surface to substrate + an explicit "trace, don't patch" instruction.** Codex without conversation context reads the substrate fresh, isn't biased by the hypotheses already explored, and traces the failure point in minutes.

## How to apply

**At ≥2h on the same user-visible symptom without resolution:**

1. **Stop patching.** No more adjacent fixes until a clean diagnosis lands.
2. **Dispatch codex** via `Agent` subagent_type `codex:codex-rescue` with a brief that includes:
   - The user-visible symptom verbatim (no narration)
   - The call path: surface entry point → transport layer → producer/handler
   - The hypotheses I've already explored (so codex doesn't re-walk them)
   - **Explicit instruction: "trace, don't patch — return the failure point with file:line"**
3. **Block on the codex result** before any further code changes on that surface.
4. **File the diagnosis as a ticket** (even if the codex result is incomplete) — separates the symptom from any cleanup work the patches discovered.

## Triggers (predictive signals)

- 2+ failed fixes on the same surface in one session
- Same wire code / error message persists after 2+ "should have fixed it" commits
- Each new fix surfaces another adjacent bug rather than resolving the original
- The session is starting to feel architectural rather than diagnostic
- Memory `feedback_step_back_after_repeated_patches` or `feedback_l4_debugging_can_hide_overengineering` fires in mind but doesn't change the action

If any of the above hits, the dispatch is mandatory — these memories are not advisory, they are STOP signs.

## Anti-pattern surfaced

Building an L0 packet to architecturally re-shape substrate around the wrong premise is the most expensive form of this vortex. The L0 packet for DOS-761/DOS-762/DOS-168 was internally coherent + the work landed cleanly + the substrate is better for it — but the chip rendering it was supposed to unblock did not unblock from any of it. The substrate work and the user-visible blocker were orthogonal axes; the agent treated them as the same axis.

## Companion ADR

None needed yet; this is a workflow discipline, not an architectural decision. If the pattern repeats post-this-entry, file `.docs/decisions/NNNN-mandatory-codex-diagnosis-dispatch.md` to make the dispatch a hard L1 protocol rather than an agent-level discipline.
