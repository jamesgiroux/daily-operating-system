# DailyOS — Radical Speed Month Pitch

**Date:** 2026-04-20
**Pitch for:** Automattic Radical Speed Month
**Author:** James Giroux
**Status:** Draft — for founder refinement before submission

## TL;DR

Spend radical speed month on **DailyOS**: the native AI chief of staff I've been building for Customer Success at Automattic.

Concrete deliverables in the month:

1. **Team intelligence pilot.** 10 CSMs across 2 product lines using DailyOS daily, with shared operational truth about accounts they co-own. No more "what's actually true about Acme right now" being three slightly-different mental models across three teammates.
2. **Measured quality lift.** Before/after metrics on meeting prep time, renewal-risk catch rate, ghost-resurrection incidents (AI silently undoing user corrections). Published numbers, not vibes.
3. **Architecture foundation shipped.** The v1.4.0 substrate — typed capability contract, claim-level trust scoring, provenance on every AI output — lands end-to-end on at least one real Transform ability (`prepare_meeting`).
4. **Team intelligence architecture decided.** The open strategic question (how 30+ CSMs across products see shared account state without violating DailyOS's local-first boundary) — chosen answer, ADR committed.

What Automattic gets at the end: a proven, measured, Customer-Success-vertical AI chief of staff running on real team workload — ready to expand beyond the pilot.

## What DailyOS is

Native macOS app (Tauri + React/Rust). Encrypted local database. It turns the raw stream of a CSM's work day — calendars, emails, transcripts, CRM data, Glean search — into a **believable daily briefing** about accounts, meetings, risks, and commitments.

Three things make it different from a generic "AI assistant":

- **Every claim is provenanced.** Every line in the briefing says where it came from. If the AI says "Alice is the champion at Acme," you can click and see which meeting transcript or email thread that came from and when. No opaque model output.
- **Every claim is trust-scored.** Six factors (source reliability, freshness, corroboration, contradiction, user feedback, meeting relevance) combine into a band: `likely_current` / `use_with_caution` / `needs_verification`. The user sees what to trust, not just what was generated.
- **User corrections are durable.** If you remove a role from a person, the next enrichment cycle cannot silently add it back. The substrate (tombstones, append-only claims, pre-gate rejection) makes ghost-resurrection structurally impossible, not just unlikely.

Today: v1.3.x shipping, small internal user base dogfooding, 113 database migrations, 21 ADRs in the codebase. Architecture just completed a deep overhaul (v1.4.0 substrate) — persona-reviewed, red-team hardened, codex-adversarial-reviewed, founder-signed on three key strategic decisions last week.

It works. It's not a prototype.

## Why this matters for Automattic

Customer Success at Automattic is a distributed team sport. WooCommerce, WordPress.com, WPVIP, Pressable, Jetpack, Akismet — each has its own CS surface, its own account shapes, its own workflows. A CSM prepping for a call with an Acme Corp contact needs:

- What's happening with this account this week.
- Who on my team talked to them last and what did they say.
- What commitments did we make; are we on track.
- What risks are trending; should I bring them up.
- What's changed since the last time I looked.

Every CSM builds this mental model manually today, reconstructed on every interaction, lost when they context-switch, invisible to their teammates. Multiply by ~30 CSMs across product lines and the cost is enormous — not in minutes per day, but in missed signals, repeated work, and accounts that fall through the cracks between teammates.

**DailyOS solves this job specifically.** It's not a generic productivity brain; it's a CS harness tuned for renewal windows, health dimensions, stakeholder coverage, champion tracking, engagement cadence — the actual dimensions CSMs reason about.

The opportunity: Automattic has ~30 CSMs × many product lines. A measurable time-saving or risk-catch-rate lift per CSM compounds immediately into customer outcomes. And the infrastructure cost is near zero — each CSM uses their existing Claude Code license and Automattic's existing Glean relationship.

## Why now

Three things make this the right month:

**The architecture just landed.** Last week I completed the v1.4.0 substrate rebuild — 21 ADRs, 4 strategic decisions, codex-reviewed, persona-reviewed. The design is settled. Implementation is ahead of me. Radical speed month is exactly the window where concentrated build time turns a settled design into shipped product.

**The category is validated externally.** The broader AI ecosystem (GBrain, OpenClaw, Hermes, Anthropic's Claude Code) is aligned on "harness around a capable model" as the pattern. Personal AI memory is production-real at YC-president-daily-driver scale. The category is moving; DailyOS sits at the CS-vertical specialization of it.

**Pilot-scale is the right next step.** v1.3.x is past "does it work" and ready for "does it help a team." Moving from 1 user to 10 users across 2 product lines is the inflection point — we learn whether the team-intelligence thesis (shared operational truth across teammates) is real, with enough signal to decide what to build next.

## What I'll ship in radical speed month

Concrete deliverables, sized for one month of concentrated work at AI-native velocity:

### Week 1 — v1.4.0 substrate Phase 0 + first slice

Two hard blockers land: `ServiceContext` (mode-aware mutation boundary) and `IntelligenceProvider` trait extraction. Then the first end-to-end slice of the abilities runtime on `get_entity_context` — the most-invoked Read ability, proof point that the substrate works end-to-end.

### Week 2 — Transform slice + pilot onboarding

Second end-to-end slice on `prepare_meeting` — first Transform ability through the full substrate with trust scoring, runtime evaluator, and provenance size cap. In parallel, onboard first pilot CSMs: 2-3 from one product line, covering 20-30 accounts.

### Week 3 — Team intelligence design + pilot expansion

Focused design cycle on the team-intelligence architecture question (ADR-0121, currently Open). Six option classes already scoped; pick one with founder sign-off, commit the ADR, scope the implementation. Pilot expands to 10 CSMs across 2 product lines.

### Week 4 — Measurement + decision

Metrics dashboard wired (opt-in anonymous aggregate telemetry). Pilot baseline captured. Measurable outcomes reported:

- Minutes per day saved on meeting prep (before/after).
- Renewal-risk catches vs baseline manual review.
- Ghost-resurrection incidents (zero is the target; substrate is designed to make it structural).
- Runtime evaluator correlation vs CSM judgment on randomly sampled outputs.
- Qualitative: do CSMs trust it enough to keep using it after the month ends.

Founder-level decision at month-end: **expand the pilot, hold at 10, or stop.**

## What I need

Minimal ask — mostly permissions and availability, not budget:

- **10 CSM volunteers** across 2 product lines (WooCommerce + WPVIP recommended; open to any combination the founder prefers based on current priorities). Commitment per CSM: use DailyOS daily, complete a weekly 15-minute feedback survey, let me pair with them once during the month.
- **Glean + Google Calendar + Gmail access** for the pilot cohort. These are already the enterprise tools they use; DailyOS connects to their existing accounts with their credentials.
- **One architectural review slot** with the founder mid-month on the team-intelligence ADR decision. 60-90 minutes.
- **Automattic AI infrastructure alignment.** If there's a Jetpack AI / Claude Code procurement direction I should know about, an hour with the relevant lead to confirm DailyOS sits inside the org's AI infra posture (BYO-key; respects the metadata-only privacy boundary in ADR-0116).

No hiring ask. No vendor budget. No infrastructure purchase. DailyOS runs on each CSM's local device with their existing AI relationships.

## What comes out at the end

Three artifacts, one decision:

**Artifact 1 — a measurably-better CS workflow** for 10 CSMs across 2 product lines. Documented baselines, documented improvement. Either the numbers justify expanding; or they don't, and we learn why.

**Artifact 2 — a decided team-intelligence architecture.** The ADR-0121 open question closed. One option chosen, committed, scoped. If we expand DailyOS beyond the pilot, we know the shape we're building.

**Artifact 3 — v1.4.0 substrate shipped to real users.** Not as a theoretical rebuild — as the foundation that made the pilot possible. The typed ability contract, claim-level trust scoring, provenance envelope, runtime evaluator — all working in production on real CS work, not just on fixtures.

**The decision at month's end:** does Automattic invest in DailyOS as the CS brain for the broader CS org? The pilot gives us the data to answer. Not speculation; measurement.

## Why me

I've been building DailyOS continuously with full architectural ownership. I know the codebase, the decisions, the trade-offs, the open questions, the pitfalls that have already been found and fixed. At AI-native velocity, the foundational constraint isn't engineering hours — it's judgment. Radical speed month compresses the calendar, and that compression rewards context depth. Every day I don't have to re-read an ADR or re-derive a decision is a day I'm shipping instead of learning.

The architectural reckoning over the past month — persona reviews, red-team passes, codex adversarial reviews, plan-eng reviews, three founder decisions, 21 ADRs — means the design surface is settled. Implementation ahead. This is the exact window where speed month pays off.

## What I'm NOT asking for

A few things worth making explicit so the scope is honest:

- **Not a product launch.** DailyOS stays internal. No marketing. No external customer pilots. No procurement motion.
- **Not a replacement for any existing tool.** CSMs keep Salesforce, keep Slack, keep Gmail. DailyOS sits alongside as their personal briefing and context layer.
- **Not a headcount ask.** I'm the one doing the work. Radical speed month IS the ask.
- **Not a promise to expand.** The pilot measures. If the numbers don't clear the bar, we stop, and I'll say so.

## Close

Radical speed month is the right container for DailyOS's next move: pilot it with a real CS team, measure it with real metrics, ship the substrate that makes it scalable, and decide based on data whether it earns broader investment.

The product is real. The architecture is settled. The category is proven. The next step is putting it in front of 10 people who actually do Customer Success at Automattic and seeing if they want it to stay in their workflow after the month ends.

That's a decidable question in a month. That's a radical speed month worth running.
