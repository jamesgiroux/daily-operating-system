# User Research: Claude Code + Quill as Discovery Call Support

**Source:** Lucas Radke, P2 (VIP AI Initiative), 2026-02-06
**Type:** Internal workflow observation (unprompted)
**Relevance:** Validates DailyOS prep workflow, confirms integration thesis

---

## Context

Lucas (PM at Automattic) shared a workflow where Claude Code acts as a "second PM" during customer discovery calls. He pairs it with Quill (live transcript MCP) and preloads context from multiple tools each morning. This is a manual, CLI-native version of what DailyOS automates.

## The Problem He Names

> "I found myself either over-preparing (spending 60+ minutes pulling context from multiple tabs) or under-preparing (scrambling to remember details mid-call)."

This is the exact job DailyOS is hired for: eliminate the prep/no-prep tradeoff by making context loading automatic.

## Key Insights for DailyOS

### 1. Context Loading as Daily Routine (Validates Morning Briefing)

Lucas makes context loading part of his daily workflow, not a per-meeting task. Each morning, Claude pulls from Google Calendar, Linear, P2, Quill history, and Obsidian.

**DailyOS parallel:** This is our Phase 1 + Phase 2 briefing. The difference is he does it manually in Claude Code; we automate it with the three-phase pattern.

### 2. Integration Stack (Validates MCP Client Architecture)

His tool chain:

| His Tool | DailyOS Equivalent | Architecture Ref |
|----------|-------------------|------------------|
| Google Calendar | Core (Phase 1 prepare script) | Already integrated |
| Linear | Extension candidate | DEC26 extension arch |
| P2/MGS | Extension candidate (Automattic-specific) | DEC26 |
| Quill | Extension candidate (transcript source) | Maps to post-meeting capture |
| Obsidian | Local files (our markdown workspace) | Core — already how we work |
| Slack (accounts team) | Extension candidate | DEC27 MCP client mode |

This confirms the MCP dual-mode decision (DEC27) and the extension architecture (DEC26). Linear and Quill are strong candidates for early integrations that would make DailyOS immediately useful to PMs at Automattic.

### 3. People-First Prep (Validates Enrichment Philosophy)

> "Tools only get you so far. The most important prep is thinking about the people: Who am I talking to? What's their role, their perspective, their likely concerns?"

He distinguishes between data gathering (automatable) and strategic thinking (human). DailyOS should surface context so users can spend their prep time on the human layer, not the data layer. This aligns with Principle 6 (AI-Native) and Principle 7 (Consumption Over Production).

### 4. Post-Call Workflow (Validates Post-Meeting Capture)

After calls, he uses Claude to:
- Summarize the Quill transcript
- Update his Obsidian project file with decisions
- Draft follow-up emails

This maps directly to our post-meeting capture feature (background archive + enrichment). The Quill transcript is the input; the account dashboard update and follow-up draft are the outputs.

### 5. Accumulated Context Compounds

> "The more customer context you have loaded into Claude, the more useful this becomes."

His project files in Obsidian accumulate meeting notes, decisions, and links over time. This is exactly how our Accounts directory works — each account folder is a growing context store that makes every subsequent briefing richer.

## What's Out of Scope

His real-time "dot commands" during calls (`.objection`, `.probe`, `.bridge`) are a different product surface — live call support, not daily operating system. Interesting but not DailyOS's job.

## Implications for Launch

1. **Internal adoption path:** PMs at Automattic are already doing this manually. DailyOS can automate the daily prep portion and give them back that 60 minutes.
2. **Integration priorities:** Linear and Quill MCP servers would make DailyOS immediately compelling for the PM persona, not just CSMs.
3. **Positioning proof point:** Lucas's post is evidence that the workflow works and people want it. The gap is that it requires CLI fluency and manual setup — DailyOS closes that gap.

## Raw Quote Bank

> "Discovery calls are hard. You're trying to: Stay present in the conversation, Remember customer context, Handle objections on the fly, Probe deeper when you hit a pain point, Take notes so nothing gets lost"

> "I don't need to take live notes anymore because Quill handles the transcription."

> "By the time I join a call, the base is already there: who I'm talking to, what issues are open, what we discussed last time. My prep time goes into thinking about the people and my goals, not gathering data."

> "The key is making this part of your daily routine, not a separate prep step for each call."
