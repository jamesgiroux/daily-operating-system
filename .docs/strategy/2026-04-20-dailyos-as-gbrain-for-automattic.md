# DailyOS — Radical Speed Month Pitch

**Date:** 2026-04-20
**Pitch for:** Automattic Radical Speed Month
**Author:** James Giroux
**Status:** Draft — for founder refinement before submission

## TL;DR

Spend radical speed month on **DailyOS**: the native AI chief of staff I've been building for Customer Success. The goal is not a team rollout. The goal is **one person, every day, with a briefing they trust.**

Concrete outcomes at end of month:

1. **A stable daily driver.** DailyOS used every morning by me (and one or two other CSM volunteers opting in late-month if stability holds). The briefing is accurate enough, fast enough, and consistent enough that you feel the loss if it's not there.
2. **Trusted intelligence, measurable.** Briefing claims are provenanced end-to-end. Trust scores correlate with "this is right" on sampled outputs. Ghost-resurrection incidents at zero (substrate is designed to make it structural, not probabilistic).
3. **Substrate shipped on real abilities.** The v1.4.0 substrate (typed ability contract, claim-level trust scoring, provenance envelope, runtime evaluator) lands end-to-end on both a Read ability (`get_entity_context`) and a Transform ability (`prepare_meeting`). Not on fixtures — on my real workday.
4. **Decided on what's next.** With a month of data on what works and what doesn't, we know whether DailyOS earns further investment, and on what axis (quality deepening, scope expansion, or nothing).

What Automattic gets at the end: a proven, measured, single-user AI chief of staff running on real CS work at AI-native velocity. Not a pilot rollout. A thing that works, well enough to judge.

## What DailyOS is

Native macOS app (Tauri + React/Rust). Encrypted local database. It turns the raw stream of a CSM's work day — calendars, emails, transcripts, CRM data, Glean search — into a **believable daily briefing** about accounts, meetings, risks, and commitments.

Three things make it different from a generic "AI assistant":

- **Every claim is provenanced.** Every line in the briefing says where it came from. If the AI says "Alice is the champion at Acme," you can click and see which meeting transcript or email thread that came from and when. No opaque model output.
- **Every claim is trust-scored.** Six factors (source reliability, freshness, corroboration, contradiction, user feedback, meeting relevance) combine into a band: `likely_current` / `use_with_caution` / `needs_verification`. The user sees what to trust, not just what was generated.
- **User corrections are durable.** If you remove a role from a person, the next enrichment cycle cannot silently add it back. The substrate (tombstones, append-only claims, pre-gate rejection) makes ghost-resurrection structurally impossible, not just unlikely.

Today: v1.3.x shipping, I'm the primary user, 113 database migrations, 21 ADRs in the codebase. Architecture just completed a deep overhaul (v1.4.0 substrate) — persona-reviewed, red-team hardened, codex-adversarial-reviewed, founder-signed on three key strategic decisions last week.

It works. It's not a prototype. What it isn't yet is *reliably excellent* — which is exactly what this month is for.

## Why this matters for Automattic

Customer Success is a role where the cost of a missed signal is real — a churned renewal, a surprised stakeholder, a commitment dropped. Every CSM rebuilds the mental model of their accounts manually, every day, and loses most of it on context-switch. The question isn't "can we help a CSM," it's "can an AI chief of staff actually become load-bearing in how a CSM works — trusted enough to be consulted before a call, fast enough to be worth opening, accurate enough not to mislead."

**That question is answered one user at a time, not by a rollout.** A tool that works beautifully for one person can become indispensable for that person. A tool that works okay for ten people is a demo. Radical speed month is the right container for the former.

If DailyOS works beautifully for me — daily, visible, measurable — the case for expansion (whether to other CSMs, other roles, or other product lines) writes itself from real usage data, not speculation. The infrastructure cost is near zero: it runs on my existing Claude Code license and Automattic's existing Glean relationship, on my own laptop.

## Why now

Three things make this the right month:

**The architecture just landed.** Last week I completed the v1.4.0 substrate rebuild — 21 ADRs, 4 strategic decisions, codex-reviewed, persona-reviewed. The design is settled. Implementation is ahead of me. Radical speed month is exactly the window where concentrated build time turns a settled design into shipped product on real workload.

**The category is validated externally.** The broader AI ecosystem (GBrain, OpenClaw, Hermes, Anthropic's Claude Code) is aligned on "harness around a capable model" as the pattern. Personal AI memory is production-real at YC-president-daily-driver scale. The category is moving; DailyOS sits at the CS-vertical specialization of it.

**Single-user quality is the real inflection.** v1.3.x is past "does it work" and approaching "does it work well enough to rely on." The gap between "kind of useful" and "load-bearing" is narrow and specific — it's trust, it's speed, it's the absence of maddening edge cases. That's what this month closes.

## What I'll ship in radical speed month

Concrete deliverables, sized for one month of concentrated work at AI-native velocity. The through-line is: every week the daily driver gets more trustworthy.

### Week 1 — v1.4.0 substrate Phase 0

Two hard blockers land: `ServiceContext` (mode-aware mutation boundary, ADR-0104) and `IntelligenceProvider` trait extraction (ADR-0106). Then the first end-to-end slice of the abilities runtime on `get_entity_context` — the most-invoked Read ability, proof point that the substrate works end-to-end.

By end of week: my daily briefing is served by an ability running through the new substrate. If the substrate has holes, I hit them first.

### Week 2 — Transform slice (`prepare_meeting`)

Second end-to-end slice on `prepare_meeting` — first Transform ability through the full substrate with trust scoring, runtime evaluator, and provenance size cap. Plus: the runtime evaluator pass (ADR-0119) wired so Transform outputs get scored before they hit my screen, not after.

By end of week: meeting prep for my real calendar runs through the hardened substrate, with trust bands visible on every generated claim.

### Week 3 — Quality deepening

No new abilities. Instead: measured, focused work on the things that erode daily-driver trust.

- Retrieval quality pass (foundation of DOS-261 BrainBench-style eval): a small corpus of real queries against my actual data, measure Precision@5 / Recall@5, tune the hybrid-search blend.
- Fail-improve loop (DOS-262) on signal typing: deterministic-rate baseline captured, visible climb over the week as I add regex coverage for phrasings I see in my own prose.
- Ghost-resurrection regression test: tombstone pre-gate property-tested against a corpus of my past enrichment runs.
- Honest bug-fixing: the daily-driver papercuts I've been working around, fixed.

By end of week: I can articulate, with numbers, what got better this month and where the remaining weakness is.

### Week 4 — Measurement + decision

Metrics baseline captured (opt-in anonymous aggregate telemetry per DOS-260). Measurable outcomes reported:

- **Briefing trust calibration.** On a sample of generated claims, does the trust band ("likely_current" / "use_with_caution" / "needs_verification") actually correlate with human judgment of correctness? Target: strong correlation on "likely_current," no silent errors in that band.
- **Ghost-resurrection incidents.** Zero is the target; substrate is designed to make it structural.
- **Runtime evaluator agreement** on a randomly sampled set of Transform outputs.
- **Daily-driver stickiness.** Do I reach for it every day? Do I notice when it's broken? (This is the honest metric — if the answer is "no," nothing else matters.)
- **Retrieval quality** (Precision@5 / Recall@5) before vs after the Week 3 tuning pass.

Founder-level decision at month-end: **deepen quality further, broaden scope (next abilities, adjacent roles), open to one or two more users, or stop.** The decision is driven by data from one user used heavily, not speculation about ten users.

## Why single-user first

Explicit, because it's easy to misread this pitch as "small pilot" when the real frame is different:

- **Single-user is the only way to measure what matters.** Briefing trust, daily stickiness, substrate robustness — these are answered by one person using it intensely for 20 days, not ten people using it casually for 5.
- **Team intelligence is deferred.** ADR-0121 (team-intelligence architecture) stays Open. Shipping shared-account-state across users without solving the local-first boundary cleanly is a foot-gun. We'll answer that question when the single-user product is excellent — not before.
- **The substrate investment pays off on one user first.** Provenance, trust scoring, tombstones, runtime evaluator — these matter most when one person is depending on them every morning. If they hold up for a power user, they'll hold up for more. If they don't, a team rollout would have amplified the problems.
- **Expansion is a next-month decision, informed by this month's data.** Not baked in here.

## What I need

Minimal ask — mostly permissions and availability, not budget:

- **Glean + Google Calendar + Gmail access** continue to work for me as they do today. DailyOS connects to my existing accounts with my credentials.
- **One architectural review slot** with the founder mid-month, 60–90 minutes, on two topics: (a) whether the measured quality bar is being cleared, (b) what the right "next" shape is if we continue — deepen, broaden, or pause.
- **Automattic AI infrastructure alignment.** If there's a Jetpack AI / Claude Code procurement direction I should know about, an hour with the relevant lead to confirm DailyOS sits inside the org's AI infra posture (BYO-key; respects the metadata-only privacy boundary in ADR-0116).
- **Optional late-month:** if by Week 4 the daily-driver bar is cleanly cleared, permission to invite one or two CSM volunteers to start using it — not as a team pilot, as second and third single-users to broaden the signal. Still no team-intelligence features.

No hiring ask. No vendor budget. No infrastructure purchase. No pilot rollout. DailyOS runs on my local device with existing AI relationships.

## What comes out at the end

Three artifacts, one decision:

**Artifact 1 — a measurably-better single-user workflow.** Documented baseline, documented improvement. Briefing trust calibration numbers. Retrieval quality numbers. Ghost-resurrection count (target: zero). Daily-driver stickiness evidence.

**Artifact 2 — v1.4.0 substrate shipped to a real daily driver.** Not as a theoretical rebuild — as the foundation that held up under one person's real workday. The typed ability contract, claim-level trust scoring, provenance envelope, runtime evaluator — all working in production on real CS work.

**Artifact 3 — an honest quality assessment.** Where the tool is load-bearing and where it's still papercut-ridden. What the next month (if there is one) should prioritize.

**The decision at month's end:** does DailyOS earn further investment, and in what shape — deeper on quality, broader on abilities, cautiously open to more single-users, or paused? The single-user month gives us the data to answer. Not speculation; measurement.

## Why me

I've been building DailyOS continuously with full architectural ownership. I know the codebase, the decisions, the trade-offs, the open questions, the pitfalls that have already been found and fixed. At AI-native velocity, the foundational constraint isn't engineering hours — it's judgment. Radical speed month compresses the calendar, and that compression rewards context depth. Every day I don't have to re-read an ADR or re-derive a decision is a day I'm shipping instead of learning.

I am also the primary user. That's the unusual advantage. I feel the papercuts the day they appear. I know which briefings felt true and which felt hallucinated. I know the moments where I trust the tool and the moments where I second-guess it. A month of building with that feedback loop closed is a month a generic "AI feature team" can't replicate.

The architectural reckoning over the past month — persona reviews, red-team passes, codex adversarial reviews, plan-eng reviews, three founder decisions, 21 ADRs — means the design surface is settled. Implementation ahead. This is the exact window where speed month pays off.

## What I'm NOT asking for

A few things worth making explicit so the scope is honest:

- **Not a team pilot.** DailyOS stays a single-user app this month. No shared state, no cross-user features, no rollout.
- **Not team intelligence.** ADR-0121 stays Open. We'll design that when it's time — not while the single-user product is still earning its trust.
- **Not a product launch.** DailyOS stays internal. No marketing. No external customer pilots. No procurement motion.
- **Not a replacement for any existing tool.** I keep Salesforce, keep Slack, keep Gmail. DailyOS sits alongside as my personal briefing and context layer.
- **Not a headcount ask.** I'm the one doing the work. Radical speed month IS the ask.
- **Not a promise to expand.** If the single-user quality bar doesn't clear, we stop, and I'll say so.

## Close

Radical speed month is the right container for DailyOS's next move: run it as the daily driver for one person for a month, ship the substrate that makes it trustworthy, measure whether it's actually load-bearing, and decide based on data whether it earns broader investment.

The product is real. The architecture is settled. The category is proven. The next step is making it genuinely great for one CSM on real Automattic work — and seeing, at the end of the month, whether "genuinely great" is what we got.

That's a decidable question in a month. That's a radical speed month worth running.
