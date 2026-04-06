# Linear Migration Prep — Future Plans and Missing Issues

_Date: 2026-04-05_

This prep file identifies future version briefs that should migrate into Linear projects and highlights issue IDs referenced in those briefs that do not yet appear in current Linear issue titles.


## v1.1.2 — Transcript Routing Fix

- Source: `.docs/plans/v1.1.2.md`
- Thesis: Since mid-February, every transcript and meeting record has been silently dumped into `_archive/{date}/` instead of routing to account directories. The Intelligence Loop is broken for transcript data: 57+ files are invisible to MCP search, meeting prep, and intelligence context. Three linked root causes: empty account_domains table kills entity resolution, transcript processor has no fallback routing, and stranded files need recovery. v1.1.2 fixes the pipeline, adds defensive fallbacks, and recovers the backlog.
- Existing Linear project: none found
- Issue refs found in brief: I660, I661, I662
- Already present in Linear by title mapping: none
- Missing from Linear by title mapping: I660, I661, I662

## v1.1.3 — Design Hardening

- Source: `.docs/plans/v1.1.3.md`
- Thesis: v1.1.1 hardened the intelligence pipeline, security posture, and code quality. v1.1.3 addresses the two remaining UX architecture issues that need their own design sessions before implementation: the navigation model and the account detail page information architecture.
- Existing Linear project: none found
- Issue refs found in brief: none
- Already present in Linear by title mapping: none
- Missing from Linear by title mapping: none

## v1.3.0 — Report Engine Rebuild: Intelligence-First, Display-Only Reports

- Source: `.docs/plans/v1.3.0.md`
- Thesis: Every report in DailyOS re-derives intelligence from raw tables via expensive PTY calls, ignoring the pre-computed intelligence that the Intelligence Loop already stores in `entity_assessment`. v1.3.0 inverts this: reports become *assembly surfaces* that render pre-computed intelligence, not independent AI targets. With 3 new columns on `entity_assessment` and 3 new tables (`portfolio_assessment`, `user_weekly_summary`, `user_monthly_summary`), most reports become display-only — no generation step, no loading spinner, no "Generating..." wait. The few genuinely editorial AI fields are pre-computed during enrichment or at week/month rollover, not on demand. The Book of Business is rebuilt on this foundation and re-enabled. Weekly Impact and Monthly Wrapped get the delight treatment (animations, archetypes, achievements) on top of the new instant-render foundation.
- Existing Linear project: none found
- Issue refs found in brief: I601, I603, I604, I605, I606, I607, I608
- Already present in Linear by title mapping: none
- Missing from Linear by title mapping: I601, I603, I604, I605, I606, I607, I608

## v1.4.0 — Publication + Portfolio + Intelligence Quality

- Source: `.docs/plans/v1.4.0.md`
- Thesis: v1.0.0 makes DailyOS the operating system for an individual account manager (including Glean-first intelligence in Phase 5). v1.1.0 adds lifecycle intelligence and briefing depth. v1.2.0 closes action loops. v1.3.0 rebuilds the report engine. v1.4.0 makes it the operating system for leadership. Publication gives ICs a way to share curated narrative with VPs. The portfolio page gives VPs an aggregate surface. The surfacing model teaches the app when to interrupt vs when to stay quiet.
- Existing Linear project: none found
- Issue refs found in brief: I535, I531, I494, I495, I533, I534, I492, I532, I491, I529, I530, I499, I508, I489, I496, I498, I432, I434, I483, I456
- Already present in Linear by title mapping: none
- Missing from Linear by title mapping: I535, I531, I494, I495, I533, I534, I492, I532, I491, I529, I530, I499, I508, I489, I496, I498, I432, I434, I483, I456
