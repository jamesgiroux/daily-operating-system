---
title: "K-in grep should query substrate-type, not proposed-name — V1.0 packets miss existing services when the query mirrors the new module name"
problem_type: workflow_issue
track: knowledge
module: .docs/plans/v1.4.0-waves.md (L0 K-in obligation), CLAUDE.md (Knowledge store discovery), .docs/plans/engineering-ladder.md
tags: [k-in, l0, substrate-discovery, packet-authoring, grep-discipline, w4, dos-683]
date: 2026-05-19
related_linear: DOS-683
related_memories: [check-substrate-before-authoring-primitives, ground-first-drafts-in-real-codebase]
---

## Context

L0 Packet F (v1.4.3 W4 Feedback Write Infrastructure, DOS-683) authored a parallel `services::surface_feedback` service + migration v181 `surface_feedback_nonces` table + new audit event family `pairing.feedback.*`. Cycle-1 reviewers BLOCKed twice because the substrate already existed:

- `src-tauri/src/services/surface_nonce.rs` — full 2-phase nonce lifecycle (`issue_nonce` / `verify_nonce`), HKDF digest key, TTL, rate-limit budgets, audit events (`presence_nonce_issued` / `_verified` / `_rejected`), replay-rejection via `verify_and_consume`, action binding (`PresenceNonceAction::{Correct, Dismiss, Corroborate, Contradict}`).
- HTTP routes `POST /v1/surface/nonce/issue` + `POST /v1/surface/nonce/verify` live at `src-tauri/src/bridges/surface_client.rs:19-26` and `surface_runtime/mod.rs:2257, 2557`.
- WP plugin transport already calls them at `wp/dailyos/includes/transport/class-dailyos-runtime-client.php:172, 188`.

The actual W4 gap is small: wire `verify_nonce` → `record_claim_feedback` + extend `PresenceNonceAction` 4 → 9 variants to match ADR-0123. Everything else in V1.0 was reinvention.

## Root cause

The K-in grep query used **the proposed module name** (`surface_feedback`), which returned only the brand-new packet itself + zero prior art:

```
grep -rli "surface_feedback" .docs/decisions/ docs/solutions/
→ (none in .docs/decisions/)
→ packet conclusion: "net-new"
```

But the substrate that does the work was named differently. The right query was a **substrate-pattern type** search:

```
grep -rln "nonce" src-tauri/src/services/ wp/dailyos/includes/
grep -rln "issue_nonce\|verify_nonce" src-tauri/src/
grep -rln "PresenceNonce\|presence_nonce" .
```

— any of which immediately surfaces `surface_nonce.rs`.

This is the second K-in defect in v1.4.3 (W3 V1.0 deferred substrate audits to "cycle 2", retro-captured in W3 cycle-2 fold). The class pattern recurs — name the gate.

## Pattern: query by substrate-type, not proposed-name

When authoring an L0 packet that introduces "new" substrate:

1. **Don't grep for the name you're about to give it.** A V1.0 packet's chosen name + a yet-to-exist module = empty result by construction.
2. **Grep for the substrate-type primitives.** For a nonce service: `nonce`, `issue/verify`, `consume`. For a feedback writer: `record_claim_feedback`, `ClaimFeedbackInput`, `FeedbackAction`. For a CPT: `register_post_type`. For a migration: scan `src-tauri/src/migrations/` end-to-end.
3. **Grep the existing service surface area + transport layer in parallel.** `src-tauri/src/services/*`, `src-tauri/src/bridges/*`, `src-tauri/src/surface_runtime/mod.rs`, `wp/dailyos/includes/transport/*`. Substrate that's plumbed end-to-end is the highest reuse value.
4. **List `src-tauri/src/services/*.rs` directly.** If the file name shape resembles your proposed module name (`surface_feedback.rs` proposed; `surface_nonce.rs` exists), the prior art is staring at you.

## Cost

- W4 V1.0 packet authoring: ~45 min effort.
- 5 cycle-1 reviewers × 5-15 min each = ~50 min reviewer dispatch + waiting.
- V1.1 rewrite (still pending; ~60 min estimated): drop migration v181/v182/v183, drop the parallel service, reframe scope as "extend `PresenceNonceAction` 4→9 + wire `verify_nonce` → `record_claim_feedback` + add `wp_user_id` binding + `payload_json` plumbing".

Compared to honoring K-in up front: ~10 min broader grep would have surfaced `surface_nonce.rs` at packet outline, V1.0 would have been written as "extend, not parallel" from the start. **Recovery cost ≈ 15× the up-front discipline cost.**

## Fix forward

- **L0 packet template** should require a "substrate-type grep evidence" subsection separate from "proposed-name grep evidence". Both empty → suspicious.
- **K-in reviewer obligation** should explicitly verify: "did the K-in grep cover the substrate-type primitives, not just the proposed module name?" If only proposed-name was grep'd, BLOCK at L0 cycle 1.
- **Memory `check-substrate-before-authoring-primitives.md`** captures the principle. This entry captures the **specific failure mode** (query-shape) so future agents can recognize it before dispatching reviewers.

## Cross-references

- `.docs/plans/engineering-ladder.md` § "K-in mandatory at L0"
- CLAUDE.md § "Knowledge store discovery" (Critical Rules)
- Memory: `feedback_check_substrate_before_authoring_primitives.md`
- Memory: `feedback_ground_first_drafts_in_real_codebase.md`
- W4 Packet F V1.0: `.docs/plans/v1.4.3-wp-foundation/L0-packet-F-feedback-write-infrastructure.md` (branch `docs/v143-l0-packet-f` HEAD `55fadcee`)
- Existing substrate this missed: `src-tauri/src/services/surface_nonce.rs`
