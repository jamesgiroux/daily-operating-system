# Design Principles

These principles guide every decision we make. When facing a tradeoff, these are the tiebreakers. When reviewing a feature, these are the criteria. When something feels wrong, one of these is probably being violated.

---

## The Prime Directive

> **The system operates. You leverage.**

If a feature requires the user to maintain it, it's wrong. If the user has to remember to do something for the system to work, it's wrong. If skipping a day creates debt, it's wrong.

The system does the work. The user benefits from the work.

---

## Principle 1: Zero-Guilt by Default

**Every feature must pass the guilt test:**

1. What happens if the user doesn't use this for a week?
2. Does it create accumulated backlog?
3. Does it require catching up?
4. Does it make the user feel bad about themselves?

If any answer is "yes," redesign the feature.

**Examples:**

| Guilty Design | Zero-Guilt Design |
|---------------|-------------------|
| Daily task list that grows if unreviewed | Tasks surface when relevant, archive automatically |
| Inbox count showing 47 unprocessed items | System processes automatically, shows what needs attention |
| "You haven't logged in for 5 days" | Pick up where you are, not where you left off |
| Streak counters | No streaks. Consistency isn't the goal. Outcomes are. |

**The principle:** Missing a day should be invisible, not punishing.

---

## Principle 2: Prepared, Not Empty

**The default state is "ready," not "waiting."**

When the user opens DailyOS, things should already be done:
- Today's overview: already generated
- Meeting prep: already pulled together
- Actions due: already surfaced
- Email summary: already compiled

The user's first interaction is consumption, not configuration. Reading, not prompting. Reviewing, not gathering.

**The principle:** Don't show an empty state that requires user labor to fill.

---

## Principle 3: Buttons, Not Commands

**Every CLI action must have a GUI equivalent.**

If power users can do something by typing a command, regular users must be able to do it by clicking a button. The interface is a translation layer, not a limitation.

But more importantly: **most actions shouldn't require either.**

| Level | Example |
|-------|---------|
| Automatic | Morning brief generates at 6am |
| One-click | "Refresh" button regenerates on demand |
| Command | `/today --force` for edge cases |

**The hierarchy of interaction:**
1. **Invisible** — It happens without user action (best)
2. **One-click** — User initiates with minimal effort
3. **Configuration** — User sets preferences once
4. **Command** — Power user override (acceptable but not primary)

**The principle:** The best interface is no interface. Automation beats buttons beats commands.

---

## Principle 4: Opinionated Defaults, Escapable Constraints

**The system should work beautifully out of the box for 80% of users.**

We make decisions so users don't have to. We pick the folder structure. We choose when things run. We decide what surfaces and what archives.

But we're not a prison.

**Escape hatches exist:**
- Don't like the folder structure? Change it.
- Want different timing? Configure it.
- Need something we didn't think of? Build a plugin.
- Hate a feature entirely? Turn it off.

**The balance:**
- New users get magic without configuration
- Power users get control without fighting the system
- Nobody is trapped

**The principle:** Strong opinions, loosely held. We decide, but you can override.

---

## Principle 5: Local-First, Always

**Your data lives on your machine.**

Not "synced to your machine." Not "cached locally." Actually, primarily, definitively on your machine in files you control.

**Why this matters:**
- **Speed**: No network latency for basic operations
- **Privacy**: Your thoughts don't traverse someone else's servers (beyond AI processing)
- **Ownership**: Walk away anytime with everything
- **Reliability**: Works offline (except AI features)

**What this means in practice:**
- Markdown files, not database rows
- File system is the source of truth
- Sync is user's choice (Git, Dropbox, iCloud, nothing)
- No account required for core functionality

**The principle:** You own your data. We never hold it hostage.

---

## Principle 6: AI-Native, Not AI-Assisted

**AI isn't a feature. It's the foundation.**

The difference:

| AI-Assisted | AI-Native |
|-------------|-----------|
| "Click here to generate with AI" | Generation is the default |
| AI is an add-on | AI is the engine |
| User prompts, AI responds | AI anticipates, user reviews |
| Enhancement | Architecture |

**What this means in practice:**
- Every output starts as AI-generated, user-refined
- The system runs without user prompting
- AI handles the cognitive overhead layer
- User handles decisions and judgment

**The principle:** Don't add AI to a traditional tool. Build a new kind of tool where AI is assumed.

---

## Principle 7: Consumption Over Production

**Users should spend more time reading than writing.**

The work of gathering, compiling, synthesizing, formatting—that's the system's job. The user's job is reviewing, deciding, acting.

**Time allocation goal:**
- 80% consuming (reading, reviewing, using outputs)
- 20% producing (editing, adding, deciding)

**What this means in practice:**
- Outputs are polished, not rough drafts
- Summaries are readable, not raw dumps
- The user edits rather than creates from scratch
- "Good enough to use" is the generation standard

**The principle:** The AI produces, the user consumes. Invert the traditional tool relationship.

---

## Principle 8: Forgiveness Built In

**The system recovers gracefully from neglect.**

Users will:
- Not use it for days
- Forget to close out properly
- Skip steps
- Use it inconsistently

The system must handle all of this without:
- Breaking
- Accumulating debt
- Requiring manual repair
- Making users feel guilty

**Recovery patterns:**
- Missed wrap? Next morning catches what's outstanding.
- Stale data? Surface it with a refresh option, don't block.
- Skipped a week? Start fresh from today.
- Orphaned files? Archive silently or surface gently.

**The principle:** Assume inconsistent usage. Design for the user who disappears and returns.

---

## Principle 9: Show the Work, Hide the Plumbing

**Users should see outputs, not processes.**

When the system is working:
- Show what was generated
- Show what's ready to use
- Show what needs attention

Don't show:
- API calls happening
- Files being shuffled
- Agents being invoked
- Processing steps

**The exception:** When something fails or needs input, surface just enough context to resolve it. Then hide it again.

**Power user mode:** Advanced users can opt into seeing the plumbing. But it's opt-in, not default.

**The principle:** Magic, not machinery. The system "just works" until you want to know how.

---

## Principle 10: Outcomes Over Activity

**We measure what matters.**

Wrong metrics:
- Days used in a row (streaks encourage guilt)
- Tasks completed (quantity over quality)
- Files created (activity over value)
- Time in app (engagement over outcome)

Right metrics:
- Meetings entered prepared
- Actions not dropped
- Context available when needed
- Time saved on cognitive overhead

**The principle:** The goal is a more effective user, not a more engaged user. We succeed when they need us less, not more.

---

## Decision Framework

When evaluating any feature, ask:

1. **Does it operate or wait?** (Must operate)
2. **What happens if skipped?** (Must forgive)
3. **Who does the work?** (System, not user)
4. **Is the default state ready?** (Must be prepared)
5. **Can it be overridden?** (Must be escapable)
6. **Does it respect ownership?** (Must be local-first)

If a feature fails any of these, it needs redesign or rejection.

---

## Anti-Patterns to Avoid

| Anti-Pattern | Why It's Wrong |
|--------------|----------------|
| Streak counters | Creates guilt about breaking streaks |
| Unread counts | Implies obligation to clear |
| "You haven't..." notifications | Shames non-usage |
| Empty states requiring setup | Demands labor before value |
| Required daily actions | Creates debt when skipped |
| Irreversible organization | Traps users in decisions |
| Proprietary formats | Hostage data |
| Cloud-required features | Dependency on our infrastructure |
| Engagement optimization | Wrong goal entirely |

---

## Principles in Tension

Sometimes principles conflict. Here's how to resolve:

**Zero-guilt vs. Comprehensive capture**
→ Zero-guilt wins. Better to miss something than create debt.

**Opinionated vs. Flexible**
→ Opinionated defaults, flexible overrides. Don't make users decide upfront.

**AI automation vs. User control**
→ Automate by default, but always allow override. Never take actions the user can't undo.

**Local-first vs. Convenience**
→ Local-first wins. Convenience without ownership is a trap.

**Simplicity vs. Power**
→ Simple surface, powerful depths. Progressive disclosure, not feature hiding.

---

*These principles are guardrails, not handcuffs. They exist to guide decisions, not prevent good ideas. When you find yourself wanting to violate one, ask why—sometimes the principle is wrong for the context, and we should update it.*
