# AI Poller Token Reduction Proposal

Date: 2026-03-29

## Executive Summary

The app is currently burning tokens for two separate reasons:

1. Network pollers are running far more often than configuration suggests.
2. Each background-triggered enrichment can fan out into multiple expensive Sonnet PTY calls, many of which time out.

From the local audit log for 2026-03-29:

- 567 PTY AI calls in a 57.3 minute window
- 448,392 estimated total tokens
- 112 Google calendar syncs in the same window, or about 117/hour
- 25 entity enrichment batches requesting 73 entities
- 258 PTY timeouts, or 45.5% of all AI calls
- 100% of recorded PTY calls used `sonnet`

This is not a single bug. It is a pipeline problem:

- pollers wake too often
- connector enablement is not enforced strongly enough
- calendar polls repeatedly enqueue expensive work
- entity enrichment is too expensive for a background path
- current diagnostics do not identify the real call site, only the model

## Main Findings

### 1. Calendar polling ignores the configured minute interval

Relevant code:

- `src-tauri/src/google.rs`
- `src-tauri/src/activity.rs`

The calendar poller sleeps on `adaptive_network_interval(&state.activity)`, which currently resolves to:

- active: 120s
- idle: 60s
- background: 30s

But the config default is `calendarPollIntervalMinutes = 5`.

There is even a `get_poll_interval()` helper in `google.rs`, but it is not used by the poll loop.

Observed impact:

- 112 `google_calendar_sync` events in 57.3 minutes
- effective cadence about every 30 seconds
- expected cadence at 5 minutes would have been about 12 polls in the same window
- this is about 9.3x more frequent than configured

### 2. Email polling has the same interval defect

Relevant code:

- `src-tauri/src/google.rs`

The email poller also sleeps on `adaptive_network_interval(&state.activity)` instead of the configured `emailPollIntervalMinutes`.

It was not the dominant source of spend in this sample, but it is the same design bug and should be fixed in the same pass.

### 3. Pollers are spawned unconditionally and `google.enabled` is not a hard guard

Relevant code:

- `src-tauri/src/lib.rs`
- `src-tauri/src/google.rs`
- `src-tauri/src/types.rs`

The app starts both calendar and email pollers on startup. The shared `should_poll()` gate checks auth state, not `config.google.enabled`.

Current config on this machine shows:

- `google.enabled = false`
- `calendarPollIntervalMinutes = 5`
- `emailPollIntervalMinutes = 15`

Yet both poller families still ran.

### 4. Every calendar poll can enqueue AI work for upcoming meetings

Relevant code:

- `src-tauri/src/google.rs`
- `src-tauri/src/hygiene/detectors.rs`
- `src-tauri/src/intel_queue.rs`

After each calendar poll, the app calls `check_upcoming_meeting_readiness()`, which scans meetings in the next pre-meeting window and enqueues entity refreshes with `IntelPriority::CalendarChange`.

Important detail:

- `CalendarChange` requests are not debounced in `IntelQueue::enqueue()`
- only `ContentChange` and `ProactiveHygiene` get debounce behavior

So a 30-second calendar loop repeatedly re-runs the same readiness scan and can keep pressure on the enrichment queue.

### 5. Background entity enrichment is too expensive for the trigger frequency

Relevant code:

- `src-tauri/src/intel_queue.rs`

The hot path is:

1. poll calendar
2. scan upcoming meetings
3. enqueue entity refresh
4. run entity enrichment

Entity enrichment uses the parallel dimension path by default:

- 6 dimension-specific PTY calls per entity
- all use `ModelTier::Extraction`
- extraction is currently configured to `sonnet`
- each dimension has a 30 second timeout

This means one background entity refresh can become 6 Sonnet PTY calls. In the audit log, the bursts line up with this pattern exactly: groups of 6 calls, usually with several timeouts mixed in.

### 6. Timeout waste is severe

Relevant code:

- `src-tauri/src/pty.rs`
- `src-tauri/src/intel_queue.rs`

Of 567 PTY calls:

- 309 succeeded
- 258 timed out

That means almost half of all PTY calls still consumed prompt tokens but produced no useful output.

There are also clearly oversized prompts in the audit log. Some timed out calls had prompt estimates above 3,000 tokens, and one exceeded 6,400 prompt tokens.

With a 30-second timeout and Sonnet on every background dimension, this is an expected failure mode, not a rare edge case.

### 7. The current diagnostics are too coarse

Relevant code:

- `src-tauri/src/pty.rs`
- `src-tauri/src/commands/app_support.rs`

`record_ai_usage()` currently records call site as:

- `spawn_claude:<model>`

That means the diagnostics can tell us "Sonnet was called 567 times" but not:

- which subsystem called it
- which queue triggered it
- whether it came from calendar, email, transcript, reports, or manual refresh
- which entity or meeting caused the spend

This investigation had to fall back to the audit log and temporal correlation instead of using first-class diagnostics.

## Why This Blows Through Tokens

The current stack multiplies cost:

- calendar runs about every 30 seconds instead of every 5 minutes
- each poll rechecks upcoming meeting readiness
- readiness enqueues entity enrichment work
- entity enrichment runs 6 Sonnet calls per entity
- many of those calls time out

In practice, this turns "background freshness" into "continuous Sonnet fan-out."

## Proposed Changes

## P0: Stop the obvious over-polling

### Change 1: Respect configured connector intervals

For calendar and email pollers:

- stop using `adaptive_network_interval()` as the primary sleep duration
- use the configured minute interval from `Config.google`
- if activity should affect cadence, it may only slow polling down, never speed it up

Recommended rule:

- `effective_interval = max(configured_interval, activity_backoff_interval)`

Not:

- `effective_interval = activity_interval`

Expected impact:

- calendar poll frequency drops from about 117/hour to about 12/hour at the current 5-minute setting
- about 89% fewer calendar polls immediately

### Change 2: Enforce connector enablement

- do not poll when `config.google.enabled` is false
- ideally do not even start the poller task until enabled
- if tasks remain long-lived, they should idle on a long sleep and re-check config on wake

This should apply to both calendar and email pollers.

## P0: Add backpressure to background AI

### Change 3: Budget gate background AI queues

Before background entity enrichment runs, check:

- daily token budget consumption
- recent timeout rate
- queue backlog

Recommended behavior:

- if daily usage exceeds a threshold, skip background AI and keep manual refresh available
- if recent timeout rate is high, pause background PTY enrichment for a cooldown window
- if the PTY-heavy queue is already saturated, do not add more calendar-driven work

This should degrade gracefully to:

- local/mechanical updates still run
- manual refresh still works
- background Sonnet work pauses

## P1: Reduce enqueue churn from calendar-driven refresh

### Change 4: Debounce `CalendarChange` refreshes

Add a debounce or cooldown for entity refreshes triggered by pre-meeting readiness.

Recommended baseline:

- do not enqueue the same entity more than once every 30-60 minutes for `CalendarChange`
- unless the linked meeting set materially changed

The current 2-hour TTL helps after enrichment succeeds, but it does not prevent the queue from being pressured before that point.

### Change 5: Narrow when pre-meeting refresh runs

Today the scan runs after every calendar poll across the whole pre-meeting window.

Reduce it by:

- only re-evaluating when calendar data changed
- only scanning meetings newly entering the window
- only re-enqueuing entities whose freshness state actually crossed a threshold

## P1: Make background entity enrichment cheaper

### Change 6: Stop using 6-way Sonnet fan-out for routine background refresh

Current behavior is appropriate for an explicit deep refresh, not for a poller-triggered background path.

Recommended split:

- manual refresh: keep rich parallel path if desired
- background refresh: use a cheaper path

Best option:

- background refresh uses one compact prompt, not 6 dimension prompts

Acceptable option:

- background refresh uses 2 phases max
- strategic dimensions stay on Sonnet
- mechanical or summarization dimensions move to Haiku

### Change 7: Add priority-aware model routing

Right now the effective hot path is all Sonnet.

Recommended routing:

- manual deep refresh: Sonnet
- calendar/background refresh: Haiku first, escalate to Sonnet only on failure or low confidence
- email enrichment: move to mechanical/Haiku unless a fallback is needed

This likely requires adding a model-selection layer that considers:

- task type
- trigger source
- priority
- prompt size

instead of just mapping by coarse tier.

## P1: Add prompt size guardrails

### Change 8: Cap prompt size for background jobs

The audit log shows multiple timed out prompts in the 800+ token range and some much larger.

For background PTY jobs:

- enforce a prompt budget cap
- truncate or summarize oversized context before model invocation
- prefer top-k retrieval over full context dumps

Background jobs should aim for predictable, bounded latency. Very large prompts should require a manual path or a staged summarize-first path.

## P2: Improve diagnostics so this is easy to catch next time

### Change 9: Record real PTY call site metadata

Extend `record_ai_usage()` so it includes:

- subsystem, for example `calendar`, `email`, `intel_queue`, `transcript`, `reports`
- operation, for example `dimension:risk`, `email_enrichment`, `briefing_narrative`
- trigger, for example `background`, `manual`, `calendar_change`, `content_change`
- entity or meeting id when available
- queue priority
- prompt token estimate
- timeout flag

Then surface rollups by:

- subsystem
- operation
- model
- success vs timeout

Without this, future diagnostics will still say only "Sonnet was called a lot."

## Suggested Implementation Order

1. Fix calendar and email poll intervals to honor config.
2. Enforce `google.enabled` as a hard gate.
3. Add cooldown or debounce for `CalendarChange` enqueue.
4. Add a background AI budget and timeout-rate circuit breaker.
5. Change background entity refresh to a cheaper path than 6-way Sonnet fan-out.
6. Add PTY call-site diagnostics.

## Expected Outcome

If only P0 lands:

- calendar poll volume should fall by about 89%
- background-trigger pressure on the enrichment queue should drop sharply
- token burn should fall immediately

If P0 and P1 land:

- background refreshes become both rarer and cheaper
- timeout waste should drop materially
- Haiku starts handling routine work instead of Sonnet doing everything

## Recommendation

Treat this as an operational bug, not a tuning preference.

The minimum viable fix is:

- honor configured poll intervals
- stop pollers when connectors are disabled
- add background AI backpressure

But the durable fix is to also change the default background enrichment strategy so routine freshness work is cheap by design.
