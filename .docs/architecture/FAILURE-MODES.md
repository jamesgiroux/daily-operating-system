# Failure Modes

**Purpose:** Name what fails, what still works, what the user sees, and how we recover. A one-page matrix so incident investigation starts here, not at log-grep.
**Date:** 2026-04-20 | **Reflects:** ADRs 0100–0120 + R1/R2 amendments
**Audience:** Anyone debugging an issue or designing a new failure boundary.

## The matrix

| Failure | What still works | What user sees | Recovery |
|---|---|---|---|
| **Glean down** | PTY Claude Code fallback ([ADR-0100](../decisions/0100-glean-first-intelligence-architecture.md)) | Latency warning; enrichment takes 60–180s instead of 10–30s | Auto-recover when Glean health returns |
| **PTY unavailable** (Claude Code not installed or crashed) | Glean-first path continues for Glean-connected users | Non-Glean users: error on enrichment; existing content unchanged | Restart Claude Code or reinstall; user-triggered retry |
| **Both Glean and PTY down** | Reads of cached data continue; no new enrichment | "Intelligence unavailable, check connectivity" banner; briefings show last-known state with staleness marker | Auto-recover when either provider returns |
| **SQLite locked for writes** (another process holding the lock) | All reads continue | Writes fail with `DbError::Locked`; UI shows retry prompt | Retry on user action; if chronic, surface to user for manual restart |
| **Claim commit lock timeout** ([ADR-0113 R2](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#revision-r2--2026-04-20--pessimistic-row-lock-on-commit_claim)) | All other claim writes continue; this specific field blocked | Specific field's update shows "hot contention, please retry"; other fields update normally | Caller retries; if user-initiated, usually succeeds on retry |
| **Consistency check fails** ([`consistency.rs`](../../src-tauri/src/intelligence/consistency.rs)) | Ability returns `AbilityOutput` with `Provenance::warnings` populated | Ability output renders with a "verification issue" marker per [ADR-0108](../decisions/0108-provenance-rendering-and-privacy.md); can be expanded to see the specific finding | User clicks "refresh" to re-trigger enrichment; unresolved high-severity → re-prompt with compact repair ([ADR-0118 note](../decisions/0118-dailyos-as-ai-harness-principles-and-residual-gaps.md)) |
| **Validation check fails** ([`validation.rs`](../../src-tauri/src/intelligence/validation.rs)) | Output blocked from commit; prior state preserved | No change to visible state; background log entry | Logged; retry on next enrichment cycle |
| **Runtime evaluator low score** ([ADR-0119](../decisions/0119-runtime-evaluator-pass-for-transform-abilities.md)) | Single retry with critique attached | User may see slightly longer latency (retry adds ~2× LLM call); output annotated with `output_quality_score` in provenance | Automatic; no user action |
| **Runtime evaluator timeout/error** | Primary output ships unevaluated with `SkipReason` in provenance | Output renders normally; evaluator annotation marked "skipped" in debug trace | Logged; next invocation may succeed |
| **Invalidation queue full** ([ADR-0115 §6](../decisions/0115-signal-granularity-audit.md)) | Emits still commit (aggressive coalescing first); after 30s cap, new non-coalescable emissions error | New mutations fail with `SignalError::PropagationQueueFull`; existing data unchanged | Queue processes; auto-recovers as workers drain |
| **Invalidation job cycle detected** ([ADR-0115 R1.6](../decisions/0115-signal-granularity-audit.md#r16-cycle-detection-needs-ancestry--expand-the-job-shape)) | Other jobs continue; cycle job transitions to `CycleDetected` | Affected outputs marked stale; downstream surfaces render with `last_known_good_as_of:` marker | Investigate the signal chain; usually a design bug |
| **Invalidation job dead-letters** (retry-exhausted) | Other jobs continue | Affected outputs stale-marked; admin surface (v1.5.0+) shows dead-letter count | Manual intervention; investigate upstream failure |
| **Prompt template hash mismatch** ([ADR-0106](../decisions/0106-prompt-fingerprinting-and-provider-interface.md)) | Eval regression classification flags as `PromptChange` | PR-time: fail-soft merge gate; dev sees expected output delta | Reviewer rebaselines or rejects |
| **Provenance envelope too large** ([ADR-0108 Amendment](../decisions/0108-provenance-rendering-and-privacy.md#amendment--2026-04-20--enforce-64-kb-provenance-size-cap)) | Ability returns `Err(AbilityError::ProvenanceTooLarge)` | Hard error on this specific ability invocation; surface renders per [ADR-0108](../decisions/0108-provenance-rendering-and-privacy.md) | Ability implementation is redesigned (summarize depth, shallower composition) or cap raised via ADR amendment |
| **Agent trust below floor** ([ADR-0113 §6](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md)) | All other agents continue; this agent's claims go to Analysis Inbox | Low-trust agent's claims appear in Analysis Inbox for review; not rendered in briefings | Shadow sampling ([R1.4](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md#r14-trust-ratchet--shadow-sampling-prevents-permanent-quarantine)) surfaces some claims to user; acceptances recover trust |
| **User tombstone blocks agent repopulation** ([ADR-0113 §5](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md)) | Agent claim rejected at commit gate | No user-visible error; agent's attempt silently declined | Works as intended; agent with 3+ independent corroborations within 7 days can override |
| **Claim contradiction detected** ([ADR-0113 §7](../decisions/0113-human-and-agent-analysis-as-first-class-claim-sources.md)) | Both claims stay committed; contradiction row written | Both values render with a contradiction flag | User resolves via Analysis Inbox; resolution writes a new superseding claim |
| **Config file malformed** ([ADR-0120 + D3 decision](../decisions/0120-observability-contract.md)) | Defaults compiled into binary apply | WARN logged on boot; app continues with defaults | User fixes config file; restart |
| **Config weights don't sum to 1.0** (Trust Compiler) | Trust computation rejects | Panic at boot with clear error message; app fails to start | User fixes config; restart |
| **Clock skew / future-dated input** (user device clock wrong) | Time-based checks (freshness decay, tombstone window) compute with negative age | Freshness clamped at 1.0 for negative ages per [DOS-10](https://linear.app/a8c/issue/DOS-10); tombstone window still applies forward from `now` | User fixes system clock; data recovers on next computation |
| **Log record buffer full** (Evaluate mode ring buffer) | Oldest records evicted | Test-time assertion on recent events still works; distant-past events unavailable | Test restructures to assert closer to emission |
| **DB key unavailable** (Keychain denied or control-plane revoked) | App refuses to open DB; existing data remains encrypted at rest | Login failure or "session revoked" state | User re-auths via Keychain / control plane restores access |
| **Panic in mutation service function** | Transaction rolls back; DB integrity preserved | Generic error to user with retry affordance | Bug; logged; requires code fix |
| **Background worker crash** (invalidation worker, publish worker, etc.) | Other workers continue; failed worker restarts from durable job queue | No user-visible failure if restart is fast; latency increase on affected work class | Automatic restart; investigate if crash recurs |
| **Process crash mid-publish** (between commit_publish and outbox delivery) | On restart, outbox worker picks up Pending entry and delivers | User sees "pending delivery" in surface until delivery completes | Automatic |
| **Destination unreachable** (publish target down) | Outbox marks `FailedRetryable`; exponential backoff | User sees "delivery pending" marker | Automatic retry with backoff; eventually dead-letters if permanent |
| **Destination idempotency conflict** ([ADR-0117 R1.4](../decisions/0117-publish-boundary-pencil-and-pen.md#r14-idempotency-key-fix)) | Outbox treats as successful (idempotent replay) | No duplicate publish | Automatic |

## Cross-cutting observations

**No silent data loss.** Every failure class above is either logged, surfaced via warning markers, or produces a visible error state. "Log and proceed without telling anyone" is explicitly prohibited ([ADR-0102 Amendment A](../decisions/0102-abilities-as-runtime-contract.md#a-error-warning-and-soft-degradation-contract-addresses-s2)).

**No silent staleness.** Outputs that depend on failed invalidations carry `last_known_good_as_of:` markers rendered by the surface. The user sees when data is stale.

**Retry everywhere reasonable.** Transient failures (network, provider, lock contention) retry with backoff. Permanent failures surface once and don't churn.

**Content never leaves the device during failure.** Even in error paths, content stays encrypted locally. Errors carry typed enum codes, not free-text ([ADR-0120 §6](../decisions/0120-observability-contract.md)) — the log records shape, not content.

## When a new failure class emerges

If you find a failure that doesn't fit this matrix:

1. Determine what still works vs what breaks — failure domain boundaries matter.
2. Determine what the user sees — should it be silent, warning-marker, or hard-error?
3. Determine the recovery path — automatic, user-triggered, or ops intervention?
4. Add a row to this matrix.
5. If the failure reveals a design gap (e.g., no retry mechanism where there should be one), file an ADR amendment or issue for it.

## Not in this document

- **Performance degradation scenarios.** Not failures; those belong in a future `PERFORMANCE.md`.
- **Multi-user / distributed failures.** DailyOS is single-user. If that changes, this doc gets a section.
- **Enterprise BYOK key-availability scenarios.** Covered by [ADR-0116](../decisions/0116-tenant-control-plane-boundary.md); specifics land when v2.x activates.
