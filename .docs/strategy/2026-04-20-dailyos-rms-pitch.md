# DailyOS: a trusted daily briefing for the mess of a CS day

**Date:** 2026-04-20. **Author:** James Giroux. **For:** Automattic Radical Speed Month.

DailyOS is a project I've been building that I believe aligns with the RMS — a native macOS app that gives a Customer Success Manager a single daily briefing they actually trust. Looking for one or two partners for the month.

## The Problem

Every CSM I know rebuilds the same mental model every morning. Who did we last talk to at Acme. What did we commit to. What's trending. What might I be missing before this 10am call. That model gets reconstructed by hand daily, stored nowhere, evaporates on context switch.

Salesforce shows you fields. Gmail shows you threads. Glean finds documents. Otter has the transcript. None of them sit together as one thing you reach for before a call. The AI layer everyone's bolting on top tends to hallucinate confidently — opaque outputs, no provenance, user corrections that silently revert.

## Goal

Turn a CSM's calendars, emails, transcripts, CRM, and Glean search into a **believable daily briefing**. Every claim links back to the source it came from. Every claim carries a trust score. User corrections are durable — when you say "Alice isn't the champion anymore," the next enrichment cycle cannot quietly put it back.

*v1.3.x shipping, I'm the daily driver, v1.4.0 substrate design just landed — persona-reviewed, red-team hardened, founder-signed.*

## Where I think it gets interesting

- **Provenance.** Click any claim, see the transcript or email it came from and when. No opaque model output.
- **Trust as a visible band.** Six factors compile into `likely_current` / `use_with_caution` / `needs_verification`. The user sees what to rely on, not just what was generated.
- **Corrections stick.** Tombstones + append-only claims + pre-gate on re-enrichment make ghost-resurrection structurally impossible. This is the single bug that kills every AI assistant I've tried.
- **Role-adjacent substrate.** Tuned for CS (renewals, health, stakeholder coverage), but the same pattern works for any role where mental-model reconstruction is the bottleneck.
- **Privacy posture is the moat.** Content stays on the laptop. BYO-key LLM. Metadata-only server boundary. What makes the tool enterprise-thinkable.
- **MCP-ready.** Curated slices of trusted intelligence served to other agents without leaking raw content.

## The month

Ship the v1.4.0 substrate end-to-end on two real abilities (`get_entity_context`, `prepare_meeting`). Use it as the daily driver. Measure whether it holds up. Publish what I learn.

## Metrics

- **Daily-driver stickiness.** Do I reach for it every morning? Do I notice when it's broken?
- **Trust calibration.** On sampled claims, does the band correlate with "this is right"?
- **Ghost-resurrection count.** Zero target.

## Who I'm looking for

Comfort with Rust/Tauri (substrate lanes are explicitly parallelizable), or a designer who wants to work on trust-and-provenance UX, or a CSM willing to be the second daily driver. Any one of those makes the month materially better.
