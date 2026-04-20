# DailyOS: what if your AI chief of staff just knew?

**Date:** 2026-04-20. **Author:** James Giroux. **For:** Automattic Radical Speed Month.

Looking for one or two partners for the month.

## The problem

Everyone's trying to get AI into their workday in a way that actually helps. Schedule agents. Meeting prep agents. "Who's waiting on me" agents. The longer you're in a role, the more institutional knowledge you hold, and the more context has to be re-assembled before every meeting, every project, every decision.

The tools we use to manage all that were built for a different era — one where the human did the work. You marked the task complete. You moved it through the status columns. You wrote the meeting summary. You published the report. You were the integration layer between the ten apps that held your life.

AI is making the busy work faster. But not always better. Single-session. Missing context. Half right. It's good at telling you what you've already done. What you actually want is help figuring out what comes next.

And all of it is prompt-driven. Every morning, you type context back in. Who you are. What project. What account. Which meeting. The AI starts from zero. You're the one stitching together the world it needs.

**What if it just knew?**

## The category exists — mostly

You've probably heard of second brains — Notion, Obsidian, Garry Tan's GBrain. Tools to help you master the mess. Your AI chief of staff that tells you where to be, what's important, and what could be coming.

Most of them are a terminal and a folder of markdown files. Not bad — we're all adults, we can read hashes. But it could be better. It could be proactive. It could be doing things before you ask. OpenClaw and Hermes gesture at that version.

We're Automattic. Pioneers and flag-bearers of open source and privacy. Avid builders. Could we not have something like this, shaped for our teams?

## Where I think it gets interesting

DailyOS is a native macOS app I've been building. It takes the raw stream of your work day — calendar, email, transcripts, CRM, Glean search — and turns it into a briefing you can actually trust. No prompt to start. No context to paste in. It was already paying attention.

- **Prepared, not empty.** Open the app and your day is already there. No dashboards to configure. No statuses to move. Skip a day, nothing breaks.
- **Every claim is provenanced.** Click it, see the transcript or email it came from and when. No opaque model output.
- **Every claim has a visible trust score.** Six factors compile into `likely_current` / `use_with_caution` / `needs_verification`. You see what to rely on, not just what was generated.
- **Corrections stick.** Tombstones + append-only claims make ghost-resurrection structurally impossible. This is the single bug that kills every AI assistant I've tried.
- **Individual context, not organisational.** Glean knows what the company knows. DailyOS knows what *you* know. Different layer, complementary.
- **Open by default, safe by design.** Content stays on your laptop. BYO-key LLM. Metadata-only at the server boundary. Markdown output, consumable by any AI tool in the ecosystem. The archive isn't the moat — the self-maintaining system is.

*v1.3.x shipping today. I'm the daily driver. v1.4.0 substrate design just landed — persona-reviewed, red-team hardened, founder-signed.*

## The month

Get DailyOS to a place where folks are willing to give it a try — stable enough to deliver real value, safely leveraging Automattic's internal knowledge, and see what happens.

Concretely: ship the v1.4.0 substrate end-to-end on two real abilities (`get_entity_context`, `prepare_meeting`) so trust and provenance are load-bearing on real work. Harden the install + onboarding path so a curious Automattician can pick it up without me hand-holding. Then invite them in.

## Metrics

- **Do people try it, and do they keep using it?** The honest first signal.
- **Trust calibration.** On sampled claims, does the band correlate with "this is right"? Strong correlation in `likely_current` is the bar — no silent errors in the high-trust zone.
- **Ghost-resurrection count.** Zero target. The substrate is designed to make it structural.
- **Safe leverage of internal knowledge.** Zero content-boundary surprises — content stays local, Glean stays the user's own relationship, no hidden exfil paths. This is table stakes at Automattic.

## Who I'm looking for

Comfort with Rust/Tauri (substrate lanes are explicitly parallelizable). A designer who wants to work on trust-and-provenance UX. Someone willing to be one of the first curious users and help shape what good looks like. Any one of those makes the month materially better.
