# Product Vision: Daybreak

> Open the app. Your day is ready.

---

## What Daybreak Is

Daybreak is a native desktop application that gives knowledge workers an AI-powered executive assistant—without requiring them to understand AI, write prompts, or touch a terminal.

**The experience:**
1. You open the app
2. Your day is already prepared
3. You read, review, act
4. You close the app and do your work

That's it. No configuration. No commands. No maintenance. The system operated while you weren't looking.

---

## What Daybreak Is Not

**Not a note-taking app.**
We're not competing with Notion or Obsidian on general-purpose note capture. Daybreak is opinionated about productivity workflows, not a blank canvas.

**Not a task manager.**
We're not building a better Todoist. Tasks are a byproduct of work, not the center of it. Daybreak surfaces what's relevant; it doesn't demand you manage a backlog.

**Not a calendar app.**
We integrate with your calendar; we don't replace it. Google Calendar stays. We make what's on it actionable.

**Not a collaboration tool.**
Daybreak is for the alone part of work—individual execution, preparation, synthesis. Collaboration happens in Slack, Docs, meetings. We prepare you for those.

**Not a Claude Code replacement.**
Power users who want full control can still use Claude Code directly. Daybreak is the consumption layer on top of the same primitives.

---

## Who Daybreak Is For

### The Primary User

**Knowledge workers whose only AI experience is ChatGPT or Copilot.**

They know how to ask a question and get output. That's the extent of their AI literacy. They don't know how to build systems, write sophisticated prompts, or structure workflows.

**Their day looks like this:**
- Move from app to app (Salesforce → Glean → Gong → Slack → Docs)
- Collect information from each silo manually
- Synthesize in their head
- Action toward some outcome
- Repeat

**What they wish for:**
- "I wish I could just ask about [topic] and the system would know where to look"
- "I wish I didn't have to spend 20 minutes prepping for every meeting"
- "I wish my action items didn't fall through the cracks"
- "I wish I could remember what happened in that meeting 3 months ago"

**Tools they've tried and abandoned:**
- Todoist (set up perfectly, never used)
- Notion (built a system, didn't maintain it)
- Asana (too much overhead)
- All of them (the guilt loop)

### The Secondary User

**Technical users who want the benefits without the terminal.**

They could use Claude Code directly. They might, for some things. But for daily productivity, they want to open an app like a normal person and have their day ready.

They appreciate:
- The automation that runs while they sleep
- The one-click interactions instead of typing commands
- The ability to drop into power-user mode when needed
- The same file system they could access via terminal

---

## The Core Experience

### Morning

**6:00 AM — The system runs**
- Pulls today's calendar
- Gathers context for each meeting
- Surfaces actions due today and overdue
- Summarizes relevant emails
- Compiles into a daily overview

**8:00 AM — You open Daybreak**
- Today's overview is ready
- Meeting prep cards are populated
- Actions needing attention are surfaced
- You read it with your coffee

**8:15 AM — You're prepared**
- Click "Ready" on the day
- Minimize the app
- Get to work

### During the Day

**Meeting notification (30 min before)**
- "Acme Corp in 30 minutes"
- Click → prep doc appears
- Context, history, talking points ready
- You walk in prepared

**After a meeting**
- Drop transcript in inbox folder
- System processes automatically
- Summary, actions, routing handled
- You don't think about it

**Ad-hoc lookup**
- "What did we discuss with BigCo last quarter?"
- Daybreak finds it
- Context without searching

### Evening

**5:30 PM — Wrap notification**
- "Ready to close out?"
- Click → today's summary appears
- What you accomplished
- What's carrying to tomorrow
- Any loose ends

**5:35 PM — Done**
- Review takes 2 minutes
- Click "Close Day"
- Tomorrow's system runs tonight
- You shut the laptop

---

## What You Never Do

In Daybreak, you never:

- Open a terminal
- Type a command
- Write a prompt
- See markdown syntax (unless you want to)
- Configure folder structures
- Set up integrations manually
- Process things into your system
- Feel guilty about skipped days

The system handles the infrastructure. You handle the decisions.

---

## The Outcomes

### For the User

| Before Daybreak | After Daybreak |
|-----------------|----------------|
| 30 min prepping for meetings | Already prepared |
| Actions fall through cracks | Actions surface when relevant |
| Context requires searching | Context is presented |
| "I know it's somewhere" | "It's right here" |
| Guilty about unused systems | System keeps up with you |

### Measurable Goals

- **Meeting prep time:** 30 min → 5 min
- **Action drop rate:** Reduced 80%
- **Context retrieval:** Instant vs. hunting
- **System maintenance time:** Zero

### The Emotional Outcome

> "I feel like I have an assistant who actually knows what's going on."

That's the feeling we're designing for. Not "productive" in an abstract sense. Actually supported. Actually prepared. Actually on top of things—because the system is on top of things for you.

---

## The Trust Problem

Users have been burned. Every productivity tool promised to change their life. Every one became a maintenance burden or got abandoned.

**How we earn trust:**

**1. Immediate value, no setup**
The first experience is "here's something useful" not "let's configure your workspace." Value before investment.

**2. Demonstrated forgiveness**
Early in the experience, show that skipping a day doesn't break anything. "Welcome back. Here's today." Not "You missed 3 days. Here's your backlog."

**3. Low switching cost**
Your data is markdown files. Don't like Daybreak? Take your files and go. No lock-in. No hostage data. This reduces the risk of trying it.

**4. Visible results**
When you walk into a meeting prepared and someone asks "how did you know that?"—that's the moment Daybreak sells itself. Word of mouth from visible results.

---

## Relationship to DailyOS

**DailyOS** is the proof of concept. Built on Claude Code. CLI-native. Where we test approaches to productivity, experiment with skills and agents, identify gaps.

**Daybreak** is the destination. Same primitives underneath (skills, agents, markdown, Python). But wrapped in an interface that doesn't require technical knowledge.

**The evolution:**
```
DailyOS (CLI, proof of concept)
    ↓
Daybreak (native app, product)
    ↓
Future: Only Daybreak exists
    - Regular users get the app
    - Power users can still access primitives via Claude Code
    - Same files, same skills, same agents
    - Different interface for different needs
```

Under the hood, they're the same system. Daybreak is DailyOS that runs itself.

---

## The Boundary

Daybreak is for the alone part of knowledge work. The boundary is where individual preparation meets group collaboration.

**Daybreak's job:**
- Prepare you for meetings (your prep, your context)
- Process your transcripts (your summaries, your actions)
- Track your actions (your commitments, your follow-ups)
- Capture your impact (your wins, your evidence)

**Handoff to other tools:**
- The meeting itself (Zoom, Meet)
- Shared documents (Google Docs, Notion)
- Team communication (Slack, email)
- Project tracking (Jira, Linear, Asana)

**The integration model:**
- Rendered outputs you can copy/paste/share
- Documents ready for wherever they need to go
- Your call on where content ends up

---

## Extensibility Vision

Out of the box, Daybreak is opinionated. It makes decisions so you don't have to.

But knowledge work varies. Automattic's quarterly review cycle isn't everyone's. Customer Success workflows aren't engineering workflows.

**The extension model:**

**Level 1: Configuration**
Adjust timing, working hours, what surfaces and what doesn't. No code required.

**Level 2: Plugins**
Install community-built extensions for specific workflows, integrations, or industries. Like Obsidian plugins or WordPress themes.

**Level 3: Custom Skills**
Power users can write their own skills and agents. Drop into Claude Code, create what you need, have it work in Daybreak.

**Level 4: Fork and Own**
It's open source. Take the whole thing and make it yours.

The goal: 80% of users never leave Level 1. The other 20% have full control.

---

## Success Criteria

Daybreak succeeds when:

1. **Users open it daily** — not because they have to, but because it's useful
2. **Users feel prepared** — walking into meetings with confidence
3. **Nothing falls through cracks** — actions surfaced, not buried
4. **Guilt is gone** — the system adapts to them, not vice versa
5. **They recommend it** — "you need to try this" word of mouth

Daybreak fails if:

1. **It becomes another maintenance burden** — we've recreated the problem
2. **Users feel behind** — guilt loop reappears
3. **Technical knowledge required** — excluded the primary user
4. **Data feels trapped** — violated ownership principle
5. **It's just a pretty CLI** — didn't actually remove the friction

---

## The Name

**Daybreak** — the moment the day begins.

It's when the sun comes up and everything is new. It's the fresh start before the chaos. It's preparation meeting opportunity.

You open Daybreak. Your day breaks open. You're ready.

---

*This vision will evolve as we build and learn. But the north star stays fixed: the system operates, you leverage. Open the app, your day is ready.*
