# I446 — User Entity Page × Role Preset

**Status:** Open
**Priority:** P1
**Version:** 0.14.1
**Area:** Frontend / Entity

## Summary

The v0.14.0 `/me` page was built CS-first: with the Customer Success preset active, three named playbook sections appear and the What I Deliver section is expanded. With any other preset, the page shows generic structure and a single "My Methodology" fallback. Eight presets are unfinished. This issue completes the remaining 8 presets by implementing preset-specific prominence rules, field-level placeholder text, and 3 named playbook sections for each of the 9 presets, including the CS preset that already ships. It depends on I441's reactive preset context for live switching without page reload.

## Acceptance Criteria

1. On the `/me` page, the section prominence (expanded vs collapsed by default) matches the preset-specific rules from ADR-0090. With Customer Success: "What I Deliver" and its sub-sections (Value Proposition, Success Definition) are expanded; "Pricing Model" is shown; three named playbook sections appear ("At-Risk Accounts," "Renewal Approach," "EBR/QBR Preparation"). With Sales: Pricing Model is primary; competitive landscape is expanded; named playbooks are "Deal Review," "Territory Planning," "Competitive Response."
2. Placeholder text in each user entity field is role-specific. The Value Proposition placeholder for CS says "What does your platform do for customers? Write it as a one-sentence outcome, not a feature list." For Sales: "What do you sell and what makes it win against the competition?" These are not generic.
3. All 9 presets have complete prominence rules and placeholder text. Activate each preset; navigate to `/me` — each shows a meaningfully different layout emphasis and placeholder copy.
4. Switching presets updates the `/me` page within 1 second (relies on I441's reactive preset context). No page reload required.
5. The "My Playbooks" section shows 3 preset-specific named sections (not the generic "My Methodology" fallback) for all 9 presets. Each section has a role-specific placeholder.

## Dependencies

- I441 (useActivePreset cache) — must ship first. The reactive context is required for criterion 4.
- v0.14.0 `/me` page (I415) — this issue extends the existing page; the data model and base layout must already exist.

## Notes / Rationale

ADR-0090 Decision 5 and Decision 6 define the prominence rules and the v0.14.0 CS-first approach. This issue implements the remaining 8 presets. The implementation is a configuration table in the `/me` page component (or a dedicated `presetPageConfig.ts` file) that maps preset ID to prominence rules, placeholder text, and playbook section names. No new components are required — the existing section collapse/expand logic and `EditableText` placeholders already support this.

**Preset-by-preset specification:**

**Customer Success** (already in v0.14.0, verify correctness):
- What I Deliver: expanded. Value Proposition placeholder: "What does your platform do for customers? Write it as a one-sentence outcome, not a feature list." Success Definition placeholder: "How do your customers measure success? What does 'done' look like for them?"
- Playbooks: "At-Risk Accounts," "Renewal Approach," "EBR/QBR Preparation"

**Sales:**
- What I Deliver: expanded. Value Proposition placeholder: "What do you sell and what makes it win against the competition?" Success Definition placeholder: "What does a qualified opportunity look like? What signals tell you a deal is real?"
- Pricing Model section: primary (expanded by default)
- Competitive Landscape: expanded
- Playbooks: "Deal Review," "Territory Planning," "Competitive Response"

**Marketing:**
- What I Deliver: expanded. Value Proposition placeholder: "What does your team own and what business outcome does it drive?" Success Definition placeholder: "What are the KPIs that tell you a campaign or launch worked?"
- Playbooks: "Campaign Retrospective," "Launch Playbook," "Channel Strategy"

**Partnerships:**
- What I Deliver: expanded. Value Proposition placeholder: "What does your partner ecosystem do for the business? Write the mutual value, not the operational steps." Success Definition placeholder: "What does a healthy, productive partner look like?"
- Playbooks: "Partner QBR," "Co-Sell Motion," "Partner Onboarding"

**Agency:**
- What I Deliver: expanded. Value Proposition placeholder: "What does your agency deliver and what do clients hire you for? Write it from the client's perspective." Success Definition placeholder: "What does a client say when they are thrilled with your work?"
- Playbooks: "Scope Change," "Client Escalation," "Retainer Review"

**Consulting:**
- What I Deliver: expanded. Value Proposition placeholder: "What problem do you solve and what outcome do clients expect from an engagement?" Success Definition placeholder: "What does the client organisation look like when your engagement is complete?"
- Playbooks: "Engagement Kickoff," "Stakeholder Alignment," "Findings Presentation"

**Product:**
- What I Deliver: expanded. Value Proposition placeholder: "What does your product area do for users? Write it as a user outcome, not a feature list." Success Definition placeholder: "What does adoption or retention look like when the product is working?"
- Roadmap section: expanded by default (above playbooks)
- Playbooks: "Discovery Sprint," "Launch Checklist," "Feature Retrospective"

**Leadership:**
- What I Deliver: collapsed (less operational, more strategic framing needed). About Me section: expanded.
- Value Proposition placeholder: "What is your mandate in this organisation? Write it as the outcome your function delivers." Success Definition placeholder: "What does the team or function look like when it is running at full capacity?"
- Playbooks: "Team Operating Cadence," "Board Prep," "Strategic Review"

**The Desk:**
- All sections neutral prominence (none explicitly expanded beyond the default).
- Value Proposition placeholder: "What do you do and why does it matter?" Success Definition placeholder: "What does a great outcome look like for the work you do?"
- Playbooks: "Weekly Review," "Project Retrospective," "Deep Work Planning"

The placeholder text is the most visible differentiator per preset on an empty page. It should be written in the voice of the preset's user — speaking to their actual concerns — not in generic product copy. The playbook section names should be recognisable terms from the role's professional vocabulary, not invented DailyOS names.
