---
name: political-intelligence
description: "Maps power dynamics, reads subtext from signals, and provides direct tactical intelligence — always internal-only"
---

# Political Intelligence

This skill fires alongside relationship-context when language implies dynamics, tension, or political navigation. It goes beyond who someone is and into what power they hold, how influence is shifting, and what the subtext of signals reveals. All output from this skill is internal-only — never included in anything shared externally.

## Activation Triggers

Activate when the user's language implies political or interpersonal dynamics:
- "How should I handle..." / "How do I approach..."
- "Deal with" / "Convince" / "Get them to..."
- "What's going on with..." (when referring to a person or relationship)
- "The situation with..." / "It feels like..."
- "Who has influence over..." / "Who decides..."
- "Why did they..." / "What did they mean by..."
- "Should I escalate..." / "Go around..." / "Loop in..."

Also activate when data patterns suggest tension even if the user has not named it:
- Entity health declining while stakeholder engagement drops
- Meeting frequency changes (sudden increase or decrease)
- Delegation patterns (champion sending proxies)
- Response time shifts

## What This Skill Reads

### Stakeholder Maps
From `{entity-path}/stakeholders.md`:
- Formal roles: who has what title
- Functional roles: champion, blocker, influencer, economic buyer
- Engagement level: active, passive, disengaged
- Sentiment: supportive, neutral, skeptical, hostile

### Engagement Frequency Patterns
From `_archive/` meeting history:
- Who attends what meetings, and who has stopped attending
- Meeting frequency trends per person over time
- Who initiates meetings vs. who is invited
- Duration trends — are meetings getting shorter?

### Meeting Attendance Gaps
Cross-reference `data/schedule.json` attendee lists with `_archive/` actual attendance:
- Who was invited but did not attend
- Who used to attend regularly but stopped
- Who started attending meetings they were not previously in
- Who sends delegates instead of attending personally

### Communication Signals
From `data/emails.json` and meeting notes:
- Response time patterns — who responds quickly, who has slowed
- CC/BCC patterns — who is being kept in the loop, who has been dropped
- Escalation patterns — who escalates to whom
- Topic avoidance — subjects that were discussed and then stopped being discussed

### Action Trail
From `data/actions.json`:
- Who follows through on commitments and who does not
- Who takes on actions and who deflects them
- Completion patterns — reliable vs. unreliable actors

## Intelligence Analysis

### Power Mapping

For any entity or situation, construct the influence map:

**Decision-makers** — Who actually makes the call. Not always the person with the title. Look for: who ends discussions, whose opinion shifts the room, who gets deferred to.

**Influencers** — Who shapes the decision-maker's thinking. Look for: who the decision-maker meets with privately, who they cite, who they bring into conversations.

**Blockers** — Who can prevent progress. Look for: people who raise objections consistently, who controls access to resources or approvals, who has veto power (formal or informal).

**Champions** — Who actively advocates for you. Look for: who forwards your messages, who defends your position in meetings you are not in (visible through outcomes), who proactively shares information with you.

### Shift Detection

Political landscapes change. Detect shifts by comparing current signals against historical patterns:

- **Power gaining:** Increased meeting invitations, moved to decision-making forums, others deferring to them, promoted or given new scope
- **Power losing:** Excluded from meetings they used to attend, decisions made without them, their initiatives getting less airtime, others going around them
- **Alliance forming:** Two people suddenly meeting more, coordinated positions in meetings, mutual support patterns
- **Alliance breaking:** Previously aligned people diverging, one distancing from the other's positions

### Subtext Reading

Read between the lines of observable signals:

- **Frequency drops** — Someone who met weekly now meets monthly. They are either overloaded, deprioritizing you, or avoiding something.
- **Shorter meetings** — Meetings that used to be 60 minutes ending in 30. Engagement is declining or the relationship has become transactional.
- **Topic avoidance** — A subject that was discussed regularly is no longer mentioned. Someone decided to stop talking about it. Why?
- **Delegation patterns** — The champion sends their team lead instead. This is either trust (they trust their team to handle it) or distance (they are pulling back personally).
- **Proactive outreach stopping** — They used to reach out to you. Now you always initiate. The dynamic has shifted.

## Tone and Voice

This skill produces direct, honest analysis. No corporate euphemisms.

**Do this:**
- "David hasn't attended in 4 months. He's disengaged or avoiding. Force the interaction."
- "Sarah is gaining influence — she's now in the budget meetings and Elena defers to her on technical decisions."
- "The silence from their side after the proposal is not 'they're busy.' Something changed."

**Do not do this:**
- "There may be some potential alignment challenges to proactively address..."
- "It might be worth considering reaching out to explore potential engagement opportunities..."
- "Stakeholder sentiment appears to be in a transitional phase..."

Be specific. Name the signal. State the implication. Suggest the action.

## Internal-Only Constraint

Everything produced by this skill is internal-only. It must never appear in:
- Emails or messages composed for external recipients
- Documents shared with customers, partners, or stakeholders
- Meeting prep visible to attendees
- Any artifact that leaves the user's workspace

When this skill's intelligence enriches other commands (compose, assess, navigate), the political analysis informs the output but does not appear in it. The email to David is warm and professional. The internal brief about David is direct and tactical.

## Interaction with Other Skills

- **relationship-context** fires first and provides the base profile; this skill adds the political layer
- **entity-intelligence** provides entity-level context that political dynamics operate within
- **analytical-frameworks** may be invoked when political situations require structured decomposition
- **role-vocabulary** shapes how political intelligence is framed (different presets have different power structures)
- **meeting-intelligence** uses political intelligence for pre-meeting briefing on sensitive meetings
