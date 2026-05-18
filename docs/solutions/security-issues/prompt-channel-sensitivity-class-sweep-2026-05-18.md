---
title: Prompt-channel sensitivity gate — centralize at services/claims.rs, sweep all 5 claim-bearing channels
problem_type: security_issue
track: bug
module: src-tauri/src/services/claims.rs (consumers: prepare_meeting, get_entity_context, all prompt-channel readers)
tags: [sensitivity, prompt-channel, trust-boundary, class-sweep, w5, cycle-7, adr-0108]
date: 2026-05-18
related_adr: ADR-0108
related_linear: DOS-219, DOS-412
---

## Problem

Sensitivity-bearing claims (`Public` / `Internal` / `Confidential` / `Secret`) were being passed into prompt channels (LLM input contexts) without consistent filtering. Across W5 cycles 2–6, reviewers found leaks on individual channels and patched them one channel at a time. Cycles 2, 3, 5, and 6 each closed a single channel; cycle 7 reviewers caught that the same shape was about to recur on a fifth channel.

## Symptoms

- `prepare_meeting` could emit `Confidential` source claims into the prompt input when source-ref-matched claims weren't subject-filtered.
- `get_entity_context` Agent-actor path could include claims above the actor's clearance.
- Composed `get_entity_context` children weren't sensitivity-gated before `PromptContext::from_context`.
- Linked-meeting subjects weren't on the source allowlist, leaking adjacent-account context.

## What Didn't Work

Patching one channel per cycle. Cycle 2 fixed `source_subject_allowed`. Cycle 3 extended the meeting-scope allowlist. Cycle 5 filtered source-ref-matched claims. Cycle 6 sensitivity-gated composed `get_entity_context` children. Each fix was correct for its channel but the pattern's shape was unchanged, and reviewer fatigue meant cycle 7 was on a path to find a fifth channel.

## Solution

**Centralize the Public/Internal-only gate in `services/claims.rs` and apply uniformly across all five claim-bearing prompt channels in one commit** (W5 commit `055516ea`, Track Q):

1. Subject-ref claims (`load_claims_active`)
2. Source-ref claims (`load_claims_active_by_source_ref`)
3. Composed `get_entity_context` children (already gated; verified)
4. Source-subject scope claims (allowlist + sensitivity)
5. Linked-meeting-derived claims (allowlist extension + sensitivity)

The gate lives once in `services/claims.rs` and is composed into every channel's reader. Adding a new prompt channel reaches for the same gate by name.

## Why This Works

- **One source of truth:** sensitivity policy can be revised in one file; all five channels pick it up.
- **Enumerable surface:** the channel list is finite and reviewable. New channels added to the list go through review.
- **CI-enforceable:** a structural check can grep prompt-input construction sites and assert the gate is composed.

## Prevention

Per memory `feedback_enumerate_channels_before_patching`: when 2+ channels have been found leaking via the same boundary class, audit ALL channels into that boundary, centralize the gate, add a sweep regression test that covers every channel.

L0 reviewers now grep `docs/solutions/security-issues/` for trust-boundary findings before scoring; future plans that touch new prompt channels must wire through the centralized gate or score BLOCKED.

## Related

- Memory: `feedback_enumerate_channels_before_patching` (the operational rule)
- Memory: `feedback_zoom_out_for_class_pattern_in_l2_loop`
- ADR-0108 (sensitivity rendering audit, OUTPUT boundary mirror at DOS-412 W6)
