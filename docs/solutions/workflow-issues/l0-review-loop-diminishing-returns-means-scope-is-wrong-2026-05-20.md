---
title: "L0 review-loop diminishing returns (5+ findings/cycle, 80+ line revisions) = ticket scope is wrong, not reviewers — reset scope and split substrate into separate tickets"
problem_type: workflow_issue
track: knowledge
module: .docs/plans/engineering-ladder.md (L0 + path-α framing), CLAUDE.md (L0 protocol)
tags: [l0, review-loop, scope, packet-authoring, substrate-split, dos-576, dos-741, dos-742, dos-743]
date: 2026-05-20
related_linear: DOS-576, DOS-741, DOS-742, DOS-743
related_memories: [feedback_review_loop_diminishing_returns_means_scope_is_wrong, feedback_premise_check_production_vs_dev_friction]
---

## Context

DOS-576 (v1.4.3 W6 audit forensic validation) was scoped as a **validation + migration** ticket against existing W1-A0 substrate. The L0 packet (Packet H) ran three review cycles:

| Cycle | Verdict | Findings | Packet revision |
|---|---|---|---|
| 1 | 2 BLOCK + 1 APPROVE | 1 HIGH AC-violation + 7 path-α + 5 info | V1.0 → V1.1 (~50 line growth) |
| 2 | 2 NEEDS-CHANGES + 1 BLOCK | 3 HIGH AC-violations + 2 MEDIUM scope corrections + path-α | V1.1 → V1.2 (~80 line growth, 287 lines total) |
| 3 | 1 APPROVE + 1 NEEDS-CHANGES + codex no-show | 2 HIGH + 5 MEDIUM/LOW | (reset before cycle 4) |

Each cycle's revisions absorbed substrate work into what was scoped as a validation ticket:

- V1.1 added first-class `request_id` field on `AuditRecord` + `AuditFields` (substrate)
- V1.2 added HMAC canonical signing of `X-DailyOS-Request-Id` (substrate)
- V1.2 added custom clippy lint crate (`tools/clippy-lints-dailyos/`) (substrate)
- V1.2 added `AuditError::SurfaceClientMissingRequestId` contract layer (substrate)
- V1.2 added `emit_pairing_audit_event_legacy` deprecation cascade (substrate)
- V1.2 changed the JSONL hash-chain spec (substrate)

By cycle 3, the discretionary "fold vs path-α" question dominated per-finding triage instead of "AC violation vs not." The packet had drifted from validation-ticket framing into substrate-overhaul territory.

The user called the pattern: **"you need to stop L6ing everything. if this is that bad, go back to the drawing board."**

## Signals to detect

Watch for these during an L0 loop:

1. **Each cycle's V1.N revision is 80+ lines** of packet edits.
2. **Reviewers flag substrate-additions** (new fields, new helpers, contract changes) rather than tightening existing AC bullets.
3. **Each cycle surfaces 5+ net-new findings** without resolving prior ones cleanly.
4. **The discretionary "fold vs path-α" question** dominates per-finding triage instead of "AC violation vs not."

When you see those signals, the ticket scope has grown beyond its framing. The fix is not another cycle; it's a scope reset.

## Resolution

Drawing-board reset 2026-05-20:

- **Packet H reset to V2.0** — went from 287 lines to 77 lines. Scoped back to its original intent: forensic validation + migration on existing substrate. No substrate work.
- **Substrate work split out** into 3 separate tickets with their own L0 cycles:
  - DOS-741: first-class `request_id` field on `AuditRecord`/`AuditFields` + WP `X-DailyOS-Request-Id` header
  - DOS-742: HMAC canonical signing of the header (Rust + PHP signer)
  - DOS-743: MCP invoke audit threads `request_id` via `SurfacePairingAuditEvent.request_id`

All 4 tickets bundled into PR #337 once each was scoped + implemented. Merged 2026-05-20.

## Don't do this

- **Don't L6** when the cycles aren't converging. L6 is for genuine architectural decisions, not for accumulated review fatigue. Three NEEDS-CHANGES cycles is a scope signal, not a deadlock signal.
- **Don't keep folding** when each cycle adds 80+ lines. The packet should converge toward APPROVE, not toward a bigger surface.
- **Don't classify everything as path-α** to escape the loop. Path-α is for L2-cycle findings that aren't AC violations; it's not an escape valve for L0 scope creep.

## Do this

- **Reset ticket scope** to what it actually validates against existing substrate.
- **File substrate gaps as separate tickets** with their own L0 cycles. Cross-link as predecessors.
- **Pair with [[premise-check-production-vs-dev-friction]]**: scope creep often comes from conflating production failure modes with dev-environment friction.

## Cross-references

- Memory: `feedback_review_loop_diminishing_returns_means_scope_is_wrong`
- Wave plan: `.docs/plans/v1.4.3-waves.md` §W6
- Retro: `.docs/plans/v1.4.3-wp-foundation/retro.md` §"DOS-576 L0 review-loop reset"
- Engineering ladder path-α framing: `.docs/plans/engineering-ladder.md`
