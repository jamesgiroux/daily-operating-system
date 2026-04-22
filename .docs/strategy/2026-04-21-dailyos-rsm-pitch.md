# DailyOS: a trustworthy AI chief of staff, and the primitives behind it

**By James Giroux. Status: Planning / looking for partners.**

**TL;DR:** DailyOS is a working prototype AI chief of staff I've been iterating on for a while. It started as a CLI tool and has become a local-first macOS app. For Radical Speed Month I want to make it stable enough to hand to a few curious Automatticians, validate a few AI primitives (memory, trust, provenance, corrections, etc.), and open a cross-team conversation about whether those primitives could become a shared personal-intelligence layer across Automattic's products.

## What DailyOS is today

You sync your calendar and email, connect to Glean, Mesh, Gravatar, Linear, Quill, and Granola (with your Claude Code subscription), and the app produces high-quality meeting briefings about your customers and projects. It builds profiles on the people in your network, keeps a dossier for each customer (or partner, affiliate, lead), and suggests and tracks actions from your meetings.

It's admittedly flaky. Typical bugs, and a healthy amount of AI bluster. The substrate underneath is the bet.

## Problem

Parts of this are specific to my day as a CSM. I think most of it generalizes to anyone whose work depends on reasoning about people, accounts, and commitments over time (AM, sales, SE, PM, support, project leads). If your work is deep focus on a single codebase or canvas, this pitch probably isn't for you.

Every morning I rebuild the mental model I had yesterday. Who's on the calendar, who I promised what, which threads are about to slip. The rebuild happens by hand, is mostly lost on context-switch, and takes the first half-hour of the day.

AI tools help with pieces of this, and they've come a long way. ChatGPT remembers things across sessions now. Claude Projects hold persistent context. Copilot reads my email. Perplexity cites sources. Glean indexes the company corpus. Each of these does something real. None of them does all of it, and none of them does it on my device.

The gap I keep hitting is integration, not absence. No single tool combines:

- **Persistent structured memory** that survives sessions, upgrades, and vendor changes
- **Trust calibration**, not just citation. Confidence as a visible band, not a hidden number or a footnote
- **Field-level provenance**: which specific claim in a synthesized paragraph came from which source sentence
- **Durable user corrections** that survive re-enrichment. You fix it once, it stays fixed
- **Architectural privacy**: content never leaves the device. Not "we promise not to train." A literal architectural guarantee
- **A native UI** a non-engineer can use without opening a terminal

Individually, each of those is in some shipped product today. Together, in one tool, for a non-engineer user, is what I haven't found. Karpathy's LLM Wiki gist, GBrain, OpenClaw, and Hermes are all wrestling with variants of the same integration problem. They're all shipping for engineers in terminals. That's the gap I've been trying to close in DailyOS.

## Hypothesis

The harness around the model matters as much as the model. If we get persistent memory, trust, provenance, and correction durability right, those become primitives that could support every Automattic product putting AI in front of a user.

I've got theories on how to tackle them. RSM is about proving the theories hold up in real use and making the thinking available to other teams.

## What I want to build

I want to validate a loop: **real daily use → substrate holds → primitives documented → we know what's next.**

1. **Ship the theory end-to-end on two real abilities** (entity context, meeting prep), so trust, provenance, and correction durability are load-bearing on a real workday.
2. **Harden install and onboarding** so a curious Automattician can try DailyOS without me hand-holding.
3. **Document the learnings** so others can lean in.
4. **Open a cross-team conversation** as I think Cosmos is a place the learnings could land and I'm keen to chat.

## What we'll validate first

- **Does the daily driver stick?** Do I reach for DailyOS every morning? Do I notice when it's broken?
- **Does trust calibration hold?** When the app says "trust this," does the user actually find it correct?
- **Does ghost-resurrection go to zero?** When a user corrects a claim, does it stay corrected across enrichment cycles?
- **Does the privacy boundary hold under real use?** Content stays on device; zero data-boundary surprises.

## What success looks like

The end-of-RSM slide needs a number on it. Here's the number I want to be defending.

**North star: trust calibration score.** On a sampled 100 claims from my own briefings, hand-rated for correctness, the app's trust bands should match reality:

- `likely_current` ≥ 90% accuracy. No silent errors in the high-confidence band is the non-negotiable one.
- `use_with_caution` in the 50-75% range. It should genuinely mean "go verify."
- `needs_verification` < 40% accuracy. If it's confident here, we've mis-calibrated.

This is the single chart that says "the substrate works." If it holds, everything else in the product is credible. If it doesn't, nothing else matters.

**Product leading indicators:**

- Days DailyOS opened before 9am out of 30. Target: ≥ 22 for me, ≥ 15 for each additional daily driver.
- Briefings generated per day. Target: 1 per weekday, reliable.
- Ghost-resurrection incidents. Target: 0. Counted, instrumented number.
- Content-boundary violations. Target: 0. Architectural, not just observed.
- Crashes per week. Target: < 1 on my machine, tolerable on partner machines.
- Runtime evaluator agreement with human judgment on sampled Transform outputs. Target: ≥ 75%.

**Retrieval benchmark (BrainBench-style, [DOS-261](https://linear.app/a8c/issue/DOS-261) permitting):**

- Precision@5 and Recall@5 on a fixed corpus of real queries. Numbers before and after a focused tuning pass mid-month.
- Graph-only vs hybrid F1 ablation. GBrain saw 86.6% vs 57.8%. I want to see the same order-of-magnitude lift once claim edges are populated.

**Substrate extraction:**

- 4 of 6 learning one-pagers written (1 drafted, 5 stubbed today). Publishable quality, internally.
- ≥ 1 cross-team conversation completed with a named next step (adopt a primitive, schedule a second meeting, rule out fit).
- End-of-month decision: standalone product, shared infrastructure, both, or neither. Any clear answer is a useful answer.

**Aspirational (self-reported, honest):**

- Meetings I walked into prepared because of DailyOS vs meetings I prepped manually. Rough tally, kept in a log I actually keep.
- Commitments caught and closed because DailyOS surfaced them before I forgot. Same tally.

These last two are anecdotal. Keeping them on the slide anyway. If the first eight numbers are green and these two are green, that's the story. If the first eight are green and these two aren't, that's an honest "the substrate works but the product isn't yet load-bearing on my day," which is also useful.

## Team: who I'm looking for

One or two partners for the month. Any of:

- **Engineer.** I could use a colleague to challenge the thinking, poke holes in the code and push it to be better.
- **Designer.** Pretty apps are more fun to use and this app is, well, in need of some help.
- **A few curious users.** Some folks who want to be the first few daily drivers and tell me where the product hurts.

Any of those three makes the month count.

## Immediate next steps

- **Slack:** find us at `#dailyos`.
- **Sync with me** if you have a product team that's running into "how do we make the AI trustworthy" problems right now. 

## Beyond RSM

Depends on what the month proves. Three honest possible outcomes:

- **Substrate holds, users stick.** Real conversation about DailyOS as a professional tool with work as the commercial wedge and local-first privacy as the moat.
- **Substrate holds, another team adopts primitives.** Different conversation about a shared personal-intelligence layer across the Automattic constellation.
- **First contact with a real user reveals big gaps.** Also useful. Tells us where three months of effort went to the wrong place, faster than any other method.

The goal of RSM is to learn which of those is most real. Pre-deciding is worse than finding out.

## Prior art and kinship

- **Karpathy's LLM Wiki gist.** Validated a lot of what we'd independently built. We've solved several problems the gist comments are still asking about; documenting the answers is half the RSM writing work.
- **GBrain (Garry Tan).** Same shape for a userbase of one, without privacy constraints. We've been making the choices he hasn't had to.
- **OpenClaw, Hermes.** Proactive harnesses for engineer audiences. What they don't solve is the 80% of users who aren't going to live in a terminal.
- **Inside Automattic.** Teams building ad-hoc versions of "AI that knows your work." Everyone is solving for this independently. A primitive everyone could start from is a neat outcome.
