# codex challenge — v1.4.4 wave-plan L0 packet cycle 1

**VERDICT: CONDITIONAL APPROVE**

Scoped to 4 attack vectors. Wave structure survives premise-check; the conditions below are pre-W0/pre-W2 plugs, not scope rework.

---

## Finding 1 — W6 parity-proof gate is operationally vague [HIGH]

**Cite:** §5.6 + AC #W1 (§7).

**Concern:** "Every active Tauri surface has a working WP block equivalent" + "user can do the thing the Tauri surface enabled" are subjective. Three slip-throughs the gate as written does not catch:
(a) **Substrate-coverage drift.** A WP block can render real substrate for the *happy path* and fall back to stubbed-empty for `needs_verification` / corrected/superseded / proposal-edited states. AC #W1 references DOS-462's visible-QA-state matrix for W2 but does not require it for W3/W4/W5 surfaces — a Daily Briefing block that has no error-state path passes the literal AC.
(b) **Trust-band fidelity.** "User can do the thing" passes if the action completes; it does not require that the *trust signaling* the user saw in Tauri (band color, freshness chip, provenance drawer evidence) is present and equivalent. Memory `project_memory_plus_judgment_equals_trust` requires no-drift between surfaces — the gate must enforce it.
(c) **Feedback round-trip.** A claim correction submitted from the WP block must reach `services::claims::record_claim_feedback` AND the projection must re-render the corrected state in the same session. Side-by-side screenshots do not prove this.

**Fix:** add to AC #W1 an enumerated checklist per surface: (i) full visible-QA-state matrix from §5.2 reachable in WP equivalent, (ii) trust-band + provenance + freshness signaling matches Tauri reference within tolerance, (iii) at least one feedback round-trip per claim-bearing surface returns projection diff. Each item gets a yes/no in the W6 proof bundle; the gate is unanimous-yes.

---

## Finding 2 — Locked §13 decisions #1 and #2 implicitly constrain each other [HIGH]

**Cite:** §13 decisions #1 (one outer block + N inner blocks) and #2 (server-side cursor pagination).

**Concern:** Decision #1 says Account Detail is one outer block whose ~25 chapters are inner blocks. Decision #2 says list shapes return one page + `next_cursor` and re-invoke the ability on scroll/filter change. These collide for entity-detail composites that include *embedded list chapters* (StakeholderGrid, touchpoints, open-loops, recent claims). The outer block invokes one envelope (DOS-459); the inner list chapter needs its own paginated ability call. Two unresolved questions the packet does not name:
- Does the outer envelope (DOS-459) carry first-page list slices, with inner blocks re-invoking the *list ability* (DOS-460 / list shapes) for subsequent pages? That means inner blocks hold their own SurfaceClient sessions, complicating cache discipline and auth scope.
- Or does the envelope carry only metadata and every list chapter is its own top-level ability call? That breaks "one outer block" framing — the outer is just a layout shell.

This will surface at W1 sub-L0 (envelope contract) AND W2 sub-L0 (block composition) and produce a circular blocker.

**Fix:** add to §13 decision #1 a sub-clause: "Inner list chapters invoke their own paginated ability per decision #2; outer envelope carries first-page slice + cursor for each list chapter so initial render is one round-trip." Or explicitly defer to W1 sub-L0 with this constraint named so W1 packet authors against it.

---

## Finding 3 — C4 (substrate-in-same-wave) vs path-α maintenance offload is genuinely in tension [HIGH]

**Cite:** §7 AC #W5 + §10 invariants row 12 (L2 path-α to maintenance).

**Concern:** AC #W5 says substrate gaps surfaced mid-W2 escalate W1 *reopening*, not deferral. Memory `feedback_no_deferrals_period` reinforces. But memory `feedback_l2_path_alpha_to_maintenance_project` says L2 findings that aren't literal AC violations route to maintenance project. These don't compose cleanly:

A W2 sub-L0 reviewer finds the envelope (DOS-459) does not expose `inclusion_reason` for the StakeholderGrid chapter — the W2 block renders without it but the visible-QA-state matrix for "filtered by subject scope" can't be exercised. Is this:
- A C4 violation (substrate the surface consumes is incomplete) → reopen W1, block W2 PR?
- A path-α finding (theoretical hardening, AC technically met because block renders) → maintenance ticket, W2 unblocks?

The packet doesn't name which class wins. Different reviewers will pick differently, producing inconsistent enforcement and L6 escalations.

**Fix:** add an invariant to §10: "C4 supersedes path-α offload when the substrate gap blocks a sub-L0 packet's *acceptance criteria*. Path-α applies only when the finding is hardening unrelated to a named AC. Cite both memories explicitly so reviewers don't pick." Add a worked example (e.g., "missing `inclusion_reason` for a chapter whose visible-QA-state matrix references subject-scope filtering = C4 reopen, not path-α").

---

## Finding 4 — W0 audit authority has no recovery procedure [MEDIUM]

**Cite:** §5.0 + AC #W1.

**Concern:** Wave packet treats DOS-677's W0 audit as authoritative for "active" surface list. AC #W1 keys parity proof to "every surface on the W0 active list." If W0 misses a surface — reachable by deep-link, only via `cmd+k` palette, only when feature flag is on, only in dev builds — that surface gets no WP equivalent and no one notices until a user hits a dead route after W6 flag-flip (decision #5). The packet has no audit-miss recovery procedure.

**Fix:** add to §5.0 an audit-completeness check: (a) W0 enumerates *all* `createFileRoute` entries in `src/router.tsx` not just nav-reachable, (b) W0 enumerates command-palette entries from `cmd+k` registry, (c) W0 names feature-flagged surfaces explicitly as a 5th list. Add to W6 (§5.6) a pre-flip dry-run procedure: scan Tauri shell for every routed component, cross-check against W0 active+inactive lists, fail W6 if any unaccounted-for route exists. Treats audit as falsifiable, not authoritative.

---

## Out of scope this cycle

Reviewer-matrix completeness, ADR cross-cite accuracy, LOC ballparks, W3 chrome lane K-in correctness — panel covered. Findings 1, 2, 3 are blocking-shape; finding 4 is preventive. Re-review after revision.
