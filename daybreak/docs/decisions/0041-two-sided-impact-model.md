# ADR-0041: Two-sided impact model

**Date:** 2026-02-06
**Status:** Accepted

## Context

`/wrap` captured daily impact in two distinct dimensions:

- **Customer Outcomes** — "What value did your customers receive today?" Feeds EBRs, renewals, value stories. Observable by the customer.
- **Personal Impact** — "What did you move forward today?" Feeds performance reviews, career narrative. Observable by you and your manager.

These serve different audiences, different timescales, and different extensions:

| Dimension | Audience | Cadence | Extension |
|-----------|----------|---------|-----------|
| Customer Outcomes | Customer, leadership | Per-meeting → weekly → quarterly | CS Extension |
| Personal Impact | You, manager | Daily → weekly → monthly/quarterly | ProDev Extension |

The app's post-meeting capture (ADR-0023) currently handles per-meeting wins/risks/actions — this covers the CS Outcomes side for individual meetings. But:

1. There is no daily aggregation of per-meeting outcomes into weekly impact files.
2. As inbox enrichment matures (I30-I31), transcript processing will capture most CS outcomes automatically, making the interactive per-meeting prompt less critical for the CS dimension.
3. The Personal Impact dimension doesn't exist in the app at all. This is where the interactive capture prompt has unique value — personal reflection that can't be extracted from transcripts.

## Decision

Impact tracking is two-sided. Each side is owned by a different extension:

**CS Extension owns Customer Outcomes:**
- Primary capture path: transcript processing via inbox enrichment (I30-I31) — automatic, comprehensive
- Secondary capture path: post-meeting prompt — fills gaps when no transcript exists
- Daily rollup: aggregate per-meeting outcomes into weekly impact capture file
- Rollup cadence: weekly → quarterly for EBR and renewal narratives

**ProDev Extension owns Personal Impact:**
- Primary capture path: daily reflection prompt (end of day, not per-meeting)
- Content: what you accomplished, delivered, or influenced — career-narrative framing
- Rollup cadence: daily → weekly summary → monthly/quarterly for performance reviews
- This is the interactive capture that can't be automated — it requires personal reflection

**Post-meeting capture (ADR-0023) evolves:**
- Keeps per-meeting timing (still better than batch end-of-day for CS wins/risks)
- CS value decreases as transcript processing matures — it becomes the fallback for non-recorded meetings
- ProDev value is separate — daily reflection, not per-meeting. The ProDev extension provides its own end-of-day prompt
- No change to ADR-0023 itself — this ADR establishes the ownership model that informs how capture evolves

## Consequences

- Clear extension ownership: CS Extension doesn't need to implement personal reflection, ProDev Extension doesn't need to parse customer transcripts
- The two capture paths can evolve independently — CS moves toward automation, ProDev stays interactive
- Weekly/monthly impact files gain structure: separate sections for Customer Outcomes and Personal Impact
- ProDev extension becomes the home for career-narrative tooling that was previously undocumented
- Existing post-meeting capture continues working — this ADR adds context for future evolution, not immediate code changes
