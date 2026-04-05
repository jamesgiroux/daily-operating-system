# I455 — 1:1 Meeting Prep Focuses on Person Entity Intelligence

**Priority:** P1
**Area:** Backend / Intelligence
**Version:** v0.14.1

## Problem

Meeting prep currently assembles context from the meeting's linked account entities regardless of meeting type. For a 1:1 meeting (`one_on_one` type), the prep should focus on the **person** you're meeting with — their relationship history, recent interactions, signals, and professional context — not the account's health, renewal status, or portfolio metrics.

A 1:1 with your direct report about their career development shouldn't open with "Account health: Green. Renewal in 45 days." It should open with the person's engagement trend, recent topics, relationship signals, and context from prior 1:1s.

## Current Behavior

`prepare/meeting_context.rs` → `gather_meeting_context()` resolves the meeting to an entity (usually an account) and assembles context from:
- Account intelligence (executive assessment, stakeholder map, risks, wins)
- Account signals and email cadence
- Linear issues linked to the account's project
- Historical meeting context for the entity

The person is only referenced as an attendee — their individual intelligence, relationship signals, and interaction history are not surfaced as primary context.

## Desired Behavior

When `meeting.type == "one_on_one"`:

1. **Primary context source: the person entity.** Resolve the non-user attendee (there should be exactly one after filtering out the authenticated user). Load their person intelligence, relationship signals, and meeting history.

2. **Context assembly for 1:1:**
   - Person's executive assessment (from entity intelligence)
   - Person's bio and enrichment data (Clay/Gravatar)
   - Relationship temperature and trend
   - Last N meetings with this person (topics, outcomes, action items)
   - Open actions involving this person
   - Recent email signals involving this person
   - Person's role and organizational context (which accounts/projects they're linked to — for background, not as the primary frame)

3. **What changes in the prompt:** The prep prompt should frame around the relationship ("Your last 1:1 with {name} was {N days} ago. You discussed {topics}. Open items: {actions}.") rather than the account frame ("Account health is {status}. Key risks: {risks}.").

4. **Account context as secondary:** If the person is linked to an account, account context can be included as background ("For context: {name} works at {account}, which is in {lifecycle_stage}") but should not dominate the prep.

## Scope

- `prepare/meeting_context.rs` — branch on `meeting_type == "one_on_one"` to assemble person-focused context
- `workflow/deliver.rs` — may need to adjust prep summary for 1:1 framing
- No schema changes needed — all data already exists in the DB (person signals, person meeting history, person actions)

## Acceptance Criteria

1. Open a meeting detail page for a `one_on_one` type meeting. The prep context leads with the person's name, relationship context, and recent interaction history — not account health or renewal status.
2. A 1:1 with an internal team member surfaces their recent meeting topics and open actions with you — not a customer account frame.
3. A 1:1 with an external contact includes their account as background context, not the primary frame.
4. Non-1:1 meeting types are unchanged — `customer`, `team_sync`, `qbr` etc. continue to use account-focused context.
5. `cargo test` passes. No regressions on existing meeting prep.

## Related

- I440 (meeting prep preset persona) — the prompt persona should also adapt for 1:1 framing
- ADR-0088 (people relationship network) — person signals and relationship data are the primary input
- v0.13.9 attendee name resolution — the user is now filtered from "The Room", making the 1:1 counterpart identifiable
