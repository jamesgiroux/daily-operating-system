# v1.4.2 W4 L4 take-stock — step-back after L0 V2 BLOCK

Generated 2026-05-16. Closing artifact for two L4-chase sessions on
v1.4.2 W4-A's first-Gutenberg-block rendering acceptance criteria.

## TL;DR

The W4-A L4 render proof has remained structurally uncrossable across two
sessions. A substrate-hardening plan packet (W4-Sub) was authored to
unblock it; both V1 and V2 of the packet were BLOCKED at L0 by
unanimous cross-model verdicts. The deeper L0 lanes audited, the more
substrate-quality issues they surfaced — including ones whose fixes
themselves need their own design review (writer-mutex CAS atomics,
crate-boundary direction between `abilities-runtime` and the app crate,
`commit_composition` version-churn on cache miss, structural-validator
vs byte-allow-list semantics, recurring-actor authorization path that
doesn't exist).

**Decision (James, 2026-05-16):** stop the L4 chase. Wait for v1.4.1 W7
to merge. Rebase onto post-v1.4.1 `dev`. Reconvene with v1.4.1
functionally complete and v1.4.2 at W4 stage-3 to take stock of
direction.

## State snapshot

### v1.4.1
- W7 not yet merged at session close. Functionally complete pending
  the W7 wave landing.

### v1.4.2 W4 stage-3
- 5 of 6 packets merged: DOS-567 (W4-B), DOS-568 (W4-A0), DOS-569
  (W4-C), DOS-570 (W4-D), DOS-571 (W4-E).
- 2 PRs open and L2-passed: PR #291 (DOS-589 W4-B-signals) and PR #292
  (DOS-572 W4-A Gutenberg block).
- L4 acceptance §59 + §60 unmet — no render proof captured.

### Open PRs
- **PR #291 (DOS-589)** — fully green CI, L2 passed two cycles. Awaiting
  decision on whether L4 §60 gap blocks merge.
- **PR #292 (DOS-572 W4-A)** — awaiting CI rerun. Has the same L4 §60
  gap. Last-session L4-unblock fixes in the WP plugin
  (`register_block_category`, `edit.asset.php`, `edit.js` TextControl
  swap) are still local-only on the `wave3-l2-integration` worktree;
  needs backport decision.

### Worktrees
- `/private/tmp/dailyos-dos-589` — branch `dos-589`, tip is PR #291 head.
- `/private/tmp/dailyos-w4-a` — branch `dos-572-w4-a`, tip is PR #292 head.
- `/private/tmp/dailyos-wave3-l2` — branch `wave3-l2-integration`. Contains
  PR #291 + PR #292 + WP plugin L4-unblock fixes + chase-session
  artifacts in `surface_runtime/mod.rs` (rejected by L0; not for cherry-pick)
  + W4-Sub V2 packet + this take-stock doc.

### DB state
- Production DB (`~/.dailyos/dailyos.db`) was corrupted by an overnight
  power outage mid-write on 2026-05-15. Confirmed by `PRAGMA quick_check`
  identifying page 214 b-tree corruption + ~100 downstream unreadable
  pages. The pre-corruption backup at
  `~/.dailyos/dailyos.db.pre-migration.20260515-232932.bak` (304MB,
  May 15 19:29 local) is clean.
- Current live `~/.dailyos/dailyos.db` is the restored backup with
  a manually-applied ALTER for `surface_client_sessions.throttled_until_at`
  (schema drift A3 — never reached deployed DBs via migration).
- Corrupted DB sidelined at `~/.dailyos/dailyos.db.corrupt-20260516-*`
  for forensics.
- Per James 2026-05-16: prod DB damage is acceptable (active build,
  expected) — recovery is not a release-blocker.

### Background runtime state
- Tauri runtime stopped at session close.
- No active polling. Background workers idle.

## What two L4-chase sessions found

### Operational warts (the original blockers from session 1)
1. `dailyos_pairing_marker` stores stale runtime port across restarts
   (DOS-636).
2. HMAC session key not persisted across Tauri restart (DOS-646).
3. Signed-route writes return `pairing_authority_unavailable` silently
   on any SQLITE_BUSY / disk-IO / schema-drift error (DOS-647).

### Substrate parity gaps (uncovered chasing the warts)
4. `is_safe_ability_name` allow-list missing `/` — every well-formed
   ability invoke is rejected at validation. Adding `b'/'` is necessary
   but insufficient — L0 said the patch is structurally wrong
   (still accepts `/leading-slash` and `//double-slash`); needs a
   structural validator (split on `/`, reject empty segments).
5. WP plugin's `project_composition_for_surface()` calls
   `/v1/surface/project-composition` which isn't a registered runtime
   route. The fix is NOT migrating WP to `/v1/surface/invoke` (would
   bypass W4-D projection + W4-C signing + cache). The fix is adding
   the orchestrator route in the runtime — but L0 V2 surfaced 3 deeper
   issues: cache key must be scope-keyed (not token-keyed); cache-miss
   path invokes the producer which calls `commit_composition()` causing
   version churn; `composition_id` → ability resolver has no durable
   table.
6. Schema drift on `surface_client_sessions.throttled_until_at` —
   migration 169 was edited post-deploy without a follow-up ALTER.
   Affects deployed DBs (including James's prod DB). Manual ALTER
   applied to the restored backup; needs proper Rust-gated migration
   for general fix.
7. Silent error swallowing in surface_runtime Err arms — ~9 sites
   where Err maps to `SurfaceHttpError` without `log::warn!`. The
   sweep itself has security constraints: cannot use raw `{error}`
   interpolation in pairing/session/signed-route/session-key arms;
   must use `error.code()` + hashed identifiers.

### Architectural issues (uncovered by L0 review of the W4-Sub packet)
8. **Single writer mutex FIFO model** (per ADR-0067 Stage 2) cannot
   guarantee foreground-write latency under background contention.
   `pause_for(Duration)` cannot preempt in-flight writes or reorder
   queued writes. Real fix is ADR-0067 Stage 3 — DbService priority
   lane / split-pool — which ADR-0067 explicitly deferred. The handshake
   5s timeout we hit IS the latency-target failure that ADR-0067 named
   as the Stage 3 trigger.
9. **Background worker churn** — 12 supervised tasks all start
   clustered with no jitter; `TARGETED_REPAIR_IDLE_POLL_MS = 250`
   busy-loops the writer every 250ms; `adaptive_network_interval`
   has Background = 30s (more polling when less attended) which is
   backwards; email defaults to 5min cadence inappropriate for
   "personal intelligence" signals.
10. **No `BackgroundScheduler` abstraction** — every poller is a
    free-standing `spawn_supervised` call with its own cadence config.
    No central registry, no startup jitter, no pause-for-user-operation
    API. The architecture allows the contention but doesn't provide
    a remediation surface.
11. **`abilities-runtime` crate-boundary** — legacy source pollers
    cannot be migrated to recurring abilities without prior service
    extraction. The app crate depends on `abilities-runtime`, not the
    reverse; current pollers use `AppHandle`, `ActionDb::open`,
    `tauri::*` modules which all live in the app crate. Migration
    requires typed source services + new `AbilityContext` trait
    boundaries before any ability can wrap a poller.
12. **Recurring-actor authorization path doesn't exist** —
    `ensure_required_scopes` at `bridges/surface_client.rs:956` is
    `SurfaceClient`-only; takes `ValidatedSurfaceSession`. A recurring
    background actor has no session. Either a new authorization path
    (`Actor::RecurringSource` with static scope grants and scheduler-side
    validator) needs to exist before recurring abilities can route
    through scope enforcement, or the design needs a different shape.
13. **Actor enum split between registry + provenance** — `Actor` lives
    at both `abilities-runtime/src/abilities/registry.rs:341` and
    `abilities-runtime/src/abilities/provenance/envelope.rs:219` with
    different variants. Any new variant (e.g., `RecurringSource`) needs
    to consider which enum gets the variant; the provenance enum carries
    claim attribution and signed-projection-envelope identity. Currently
    these two enums coexist without a clear authority document.
14. **`commit_composition` version churn** — current implementation
    advances `composition_version` on every successful commit. A cache-miss
    path that invokes a producer ability causes version churn every time
    the cache evicts. Needs split between "compose/update" (advances
    version) and "project existing committed composition" (read-only).

### Substrate-quality concerns
15. **Migration slot conflict** — DOS-589's L0 packet reserves v176-v179.
    Migration v178 already shipped. v179 cannot be claimed by W4-Sub
    without amending DOS-589's reservation. Need slot reservation table
    discipline.
16. **CI lint enforcement mechanism** — shell-grep against Rust syntax
    for the surface-route Err logging sweep is fragile. Multi-line
    arms, nested match, helper functions returning `error_response`
    all confuse it. Needs `cargo clippy` custom lint or AST walker
    for soundness.

## Why the L0 packet kept getting BLOCKED

V1 had 18 findings across 4 lanes; V2 folded them all but BLOCKED again
with 30+ findings (some new, some V2 introduced via restructure). The
pattern is:

- Each "fix" surfaces 2-3 layers of new problems underneath.
- Cross-model consensus (4 different lanes) consistently identifies the
  comprehensive scope as wrong-shaped.
- The codex-consult lane was explicit: "Class C is architectural cleanup,
  not required to unblock W4-A rendering" — i.e. the scope expansion to
  fix substrate writer-contention as part of W4-A L4 is over-reach.
- The packet author (me) was working against a worktree
  (`wave3-l2-integration`) that's downstream of two unmerged PRs.
  Reviewers audit against `dev`, producing systematic +6-line citation
  drift across the packet.

The pattern matches the `feedback_step_back_after_repeated_patches`
rule: 2+ failed reviews on same surface = stop patching, get
independent diagnosis, ask if the mechanism is overengineered for
the use case.

## Reading order for next session

1. **This doc** — the closing artifact.
2. **`tasks/handover-v1.4.2-w4-shipped-l4-blocked.md`** — first-session
   handover capturing the operational warts.
3. **`.docs/plans/dos-546/v1.4.2-project/W4-Sub-L0-packet.md` V2** — the
   BLOCKED packet itself. Read for the architectural findings; do NOT
   implement as written.
4. **The 2 codex L0 review outputs** (in this session's git log, captured
   as commit C below) — the cross-model consensus on what's wrong with
   the V2 packet.
5. **ADR-0067** + its (unfiled but drafted) Amendment 1 — the staged
   split-lock strategy and where W4-Sub would have landed (Stage 2.5)
   vs where the structural fix actually lives (Stage 3).
6. **PR #291 + #292** GitHub state.
7. **v1.4.1 W7 merge state** — must be merged before take-stock begins.

## Take-stock prompt for the next session

When we reconvene (after v1.4.1 W7 merges + rebase onto latest dev):

> **What does the integrated state of v1.4.2 actually do, and is that
> closer to or further from the personal-intelligence outcomes that
> drove v1.4.2 in the first place?**

Specifically, NOT "how do we unblock L4 §59 + §60." That framing was
too narrow — it led to chasing operational warts which led to chasing
substrate writer-contention which led to L0 BLOCK. The right question
is upstream: given what we built, is the path forward to fix the
substrate first (likely a v1.4.3 substrate quality wave with proper
scoping), continue surface work on a different rendering pattern, or
something else we can't see from inside the L4 frame.

Inputs to take-stock:
- Integrated v1.4.2 state on post-v1.4.1 `dev`.
- The 14 architectural findings from this session as the substrate-quality
  backlog.
- ADR-0067 + Amendment 1 draft as the contention-model conversation.
- v1.4.2 project description at
  `.docs/plans/dos-546/v1.4.2-project/01-project-description.md` for
  outcome alignment.

## Authority surface

- **Linear** — W4-Sub L0 BLOCK status filed as a comment on the to-be-filed
  W4-Sub ticket (or as a comment on DOS-546 if W4-Sub isn't filed yet).
- **GitHub** — preservation PR opened from `wave3-l2-integration` →
  `dev` with this take-stock doc, V2 packet, and chase artifacts.
  Marked as draft / preservation-only.
- **Git** — `wave3-l2-integration` branch preserved with this session's
  commits. Rebase onto post-v1.4.1 `dev` discards the worktree.

## Concrete next-session actions

In order of when to do them:

1. Wait for v1.4.1 W7 to merge.
2. Rebase / fresh-clone onto post-v1.4.1 `dev`.
3. Read this take-stock doc end-to-end.
4. Open the take-stock prompt above as a thinking session — not
   implementation.
5. Decide direction based on what take-stock surfaces:
   - If "fix substrate first": file a v1.4.3 wave plan with the 14
     architectural findings as the backlog, do its own L0 properly per
     finding.
   - If "different rendering pattern": revisit W4-A acceptance criteria
     with v1.4.3+ lens — maybe the block doesn't render against a live
     runtime, maybe it renders against a different substrate path that
     doesn't trip the writer-contention surface.
   - If "ship W4-A as-is with documented L4 gap": close out PRs #291
     and #292 with proof bundles that name the L4 gap; file
     substrate-quality findings as separate tickets per finding.
6. Update / close / merge the preservation PR based on the take-stock
   outcome.

## Session boundary

- Two sessions, ~5 hours total elapsed.
- One L4 chase that revealed operational warts.
- One substrate-hardening attempt (W4-Sub V1 + V2 packets) that revealed
  architectural depth.
- Zero rendered blocks.
- Six valuable artifacts: this doc, the V2 packet, the V1 handover, the
  two codex L0 review captures, and the in-flight PRs.

The work was not wasted. The work was the depth charge that surfaced
what the v1.4.2 substrate actually needs.
