# DailyOS: a trustworthy AI chief of staff, and the primitives behind it

**By James Giroux. Status: Planning / looking for partners.**

**TL;DR:** DailyOS is a working prototype AI chief of staff I've been building since the last week of January. Local-first macOS app, encrypted on device, BYO LLM key, every claim provenanced and trust-scored. For Radical Speed Month I want to make it stable enough to hand to one or two curious Automatticians, write down the substrate primitives (memory, trust, provenance, corrections, the boundary between deterministic code and probabilistic AI) as one-pagers other product teams could adopt, and open a cross-team conversation about whether those primitives could become a shared personal-intelligence layer across Automattic's products.

The longer-form founder note is [here](2026-04-21-rsm-note.md). The first learning (the determinism boundary story) is [here](../learnings/2026-04-21-where-code-ends-and-ai-begins.md).

**Team:** @giroux + 1-2 partners TBD.

## Problem

Every knowledge worker at Automattic starts each morning rebuilding the mental model they had yesterday. Who's on the calendar. Who we promised what. Which threads are about to slip. The rebuild happens by hand, is mostly lost on context-switch, and takes the first half-hour of every day.

AI tools help a little. They also start from zero every session. You type the context back in, the AI replies, the AI forgets by tomorrow. The person is still the integration layer across ten apps.

The AI-for-work tools actually shipping today skip the hard parts:

- **No persistent memory.** What the AI learned yesterday is gone today.
- **No trust signal.** Every output looks equally confident whether it's right or a hallucination.
- **No provenance.** You can't ask "why did you say that" and get a source.
- **No durable corrections.** You fix something, the next enrichment cycle quietly reverts it. Trust dies around week three.
- **No privacy posture.** Customer content ships to vendor servers; at Automattic that's a non-starter for anything real.

Karpathy's LLM Wiki gist last week, GBrain, OpenClaw, Hermes are all wrestling with the same problems. None have shipped answers for users who don't live in a terminal.

## Hypothesis

The harness around the model matters more than the model. If we get persistent memory, trust, provenance, and correction durability right as a shared substrate, every Automattic product that puts AI in front of a user can inherit them without rebuilding the trust infrastructure from scratch.

DailyOS has been the vehicle for working this out in a greenfield codebase for the last three months. The substrate is largely designed (120+ ADRs, v1.4.0 implementation underway). RSM is about proving it holds up in real use and making it adoptable by other teams.

## What we want to build

Not a product launch. Validate a loop: **real daily use → substrate holds → primitives documented → one other team evaluates → we know what's next.**

1. **Ship the v1.4.0 substrate end-to-end on two real abilities** (entity context, meeting prep), so trust, provenance, and correction durability are load-bearing on a real workday.
2. **Harden install and onboarding** so a curious Automattician can try DailyOS without me hand-holding.
3. **Write primitive one-pagers in `.docs/learnings/`.** Five started, one drafted, shareable internally, optionally external after RSM.
4. **Open one cross-team conversation.** me.sh has asked to meet. One more from Jetpack AI, WooCommerce admin AI, VIP tooling, Beeper, or Day One would ideally happen during the month.

## What we'll validate first

- **Does the daily driver stick?** Do I reach for DailyOS every morning? Do I notice when it's broken?
- **Does trust calibration hold?** When the app says "trust this," does the user actually find it correct?
- **Does ghost-resurrection go to zero?** When a user corrects a claim, does it stay corrected across enrichment cycles?
- **Does the privacy boundary hold under real use?** Content stays on device; zero data-boundary surprises.

## What success looks like

**Product:**
- Me, plus at least one curious Automattician, using DailyOS daily by end of month.
- Trust-band calibration strong on the high-confidence band (no silent errors where the app says "trust this").
- Zero silent overwrites of user corrections.
- Zero content-boundary violations (architectural, not just observed).

**Substrate:**
- Five learning one-pagers written and shared internally.
- One cross-team conversation completed with a concrete next step (adopt a primitive, rule out fit, schedule another meeting).
- End-of-month decision: standalone product, shared infrastructure, both, or neither. Any clear answer is a useful answer.

## Team: who I'm looking for

One or two partners for the month. Any of:

- **Rust / Tauri engineer.** The v1.4.0 substrate lanes are explicitly parallelizable. Two Phase 0 blockers (ADR-0104 `ServiceContext`, ADR-0106 `IntelligenceProvider`) can land in separate worktrees without conflict.
- **Designer.** The trust / provenance UX is a genuinely unsolved problem. How do you show "this is current, this is uncertain, this is stale, and here's where each claim came from" without turning the briefing into a wall of footnotes?
- **First curious user.** Someone who wants to be the second daily driver and tell me where the product hurts.

Any of those three makes the month count.

## Immediate next steps

- **Slack:** find us at `#rsm-dailyos`.
- **Read the founder note** at `.docs/strategy/2026-04-21-rsm-note.md` for the longer-form thinking.
- **Read the first learning** at `.docs/learnings/2026-04-21-where-code-ends-and-ai-begins.md` for what the substrate work feels like in practice.
- **Sync with me** if you have a product team that's running into "how do we make the AI trustworthy" problems right now. me.sh is the first conversation; there's room for one more.

## Beyond RSM

Depends on what the month proves. Three honest possible outcomes:

- **Substrate holds, users stick.** Real conversation about DailyOS as a professional tool with work as the commercial wedge and local-first privacy as the moat.
- **Substrate holds, another team adopts primitives.** Different conversation about a shared personal-intelligence layer across the Automattic constellation (me.sh, Day One, Gravatar, Jetpack AI, WooCommerce admin AI, Beeper).
- **First contact with a real user reveals big gaps.** Also useful. Tells us where three months of effort went to the wrong place, faster than any other method.

The goal of RSM is to learn which of those is most real. Pre-deciding is worse than finding out.

## Prior art and kinship

- **Karpathy's LLM Wiki gist.** Validated a lot of what we'd independently built. We've solved several problems the gist comments are still asking about; documenting the answers is half the RSM writing work.
- **GBrain (Garry Tan).** Same shape for a userbase of one, without privacy constraints. We've been making the choices he hasn't had to.
- **OpenClaw, Hermes.** Proactive harnesses for engineer audiences. What they don't solve is the 80% of users who aren't going to live in a terminal.
- **Inside Automattic.** Teams building ad-hoc versions of "AI that knows your work." Everyone is solving trust and memory and corrections independently. The substrate is the part that could compose across.

Feedback welcome, especially if you're building something in this space or you're on a product team with a live AI bottleneck. The point of RSM is to find out things we don't know alone.
