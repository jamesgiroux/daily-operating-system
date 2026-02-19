# ADR-0083: Product Vocabulary — User-Facing Language Standard

**Date:** 2026-02-18
**Status:** Proposed
**Participants:** James Giroux, Claude Code

**Related:** ADR-0073 (Editorial design language), ADR-0076 (Brand identity), ADR-0081 (Event-driven meeting intelligence), I329 (Intelligence quality indicators)

---

## Context

DailyOS has been built at speed. The codebase, the ADRs, the UI copy, and the backlog all use language that was convenient for the builders — "entity," "enrichment," "signal," "intelligence," "prep." These terms are precise for the system's internals but meaningless or confusing for the person using the app.

As the product matures, the vocabulary needs to split cleanly into two layers:

1. **System vocabulary** — what developers and ADRs use to describe architecture. Precise, abstract, composable.
2. **Product vocabulary** — what users see in the UI. Warm, specific, self-evident.

These have been conflated. "Entity" appears in UI picker labels. "Enrichment" leaks into loading states. "Signal" is used in user-facing badge descriptions. "Intelligence" means six different things depending on context. "Prep" describes the system's output as though the user hasn't done their job yet.

The editorial design language (ADR-0073) gave DailyOS a visual identity — serif headlines, material palette, magazine layout. This ADR gives it a verbal identity.

### The Problem in Practice

A user opening DailyOS today encounters:

- "Entity Picker" — Link what to what? What's an entity?
- "Needs Prep" badge — Whose job is that? The system's or mine?
- "Intelligence Report" — Am I a spy? What does this contain?
- "Enrichment in progress" — What is being enriched? With what?
- "Proposed actions" — Proposed by whom?
- "Signal bus" concepts leaking into UI through badge labels
- "Account" used for both external customers and internal teams (ADR-0070)

The chief-of-staff metaphor is the product's north star. A chief of staff doesn't say "I've enriched your entity intelligence with new signals." They say "I found something new about Acme before your meeting."

---

## Decision

### 1. Two Vocabularies, One Rule

**System vocabulary** lives in code, ADRs, and developer documentation. It is precise and abstract. `entity`, `signal`, `enrichment`, `intelligence`, `prep`, `resolution` — all fine here.

**Product vocabulary** lives in UI strings, user-visible copy, error messages, badges, labels, and any text a user reads. It is warm, specific, and self-evident. No system term appears in user-facing text without translation.

**The rule:** If a user can see it, it uses product vocabulary. No exceptions.

### 2. The Translation Table

#### Core Concepts

| System term | Product term | Rationale |
|---|---|---|
| Entity | *(use the specific type)* — Account, Project, Team, Person | Nobody has "entities." Always use the specific type. If you must refer to the category generically: "account or project" |
| Entity resolution | *(invisible)* | The system links meetings to the right account/project. Users don't need to know this happens. When it's wrong, they correct it by changing the link. |
| Intelligence (on a meeting) | **Briefing** | "Your meeting briefing." Familiar, implies preparation on your behalf. The meeting detail page is the briefing. |
| Intelligence (on an account/project) | **Insights** | "Account insights." Lighter than intelligence, implies distilled understanding. |
| Intelligence (the system's output generally) | **Context** | "Building context for Thursday's QBR." Generic, non-threatening, accurate. |
| Intelligence quality levels | See Section 3 | |
| Enrichment | *(invisible)* or "Updating" | Users don't enrich things. The system "found new information" or is "updating context." |
| Signal | **Update** or **Change** | "2 new updates since this morning." A signal is what the system sees; an update is what the user cares about. |
| Prep / prep file | **Briefing** | The output is a briefing, not a task the user needs to do. "Needs prep" becomes a quality indicator (see Section 3). |
| Proposed (action status) | **Suggested** | "AI suggested" is already in the I334 UI. Formalize it. |
| Archived (action status) | **Dismissed** | "Archived" sounds like filing. "Dismissed" is what the user did — they rejected the suggestion. |
| Signal bus | *(never user-facing)* | |
| Bayesian fusion | *(never user-facing)* | |
| Thompson Sampling | *(never user-facing)* | |
| Correction learning | *(invisible)* | The system just "gets smarter" or "learns your preferences." |

#### Surfaces

| System term | Product term | Rationale |
|---|---|---|
| Daily Briefing | **Today** or **Daily Briefing** | Both work. "Today" is the nav label; "Daily Briefing" is the page identity. |
| Weekly Forecast | **This Week** or **Week Ahead** | "Forecast" is fine but "This Week" is warmer as a nav label. |
| Meeting Detail Page | **Meeting Briefing** | It's the briefing for that meeting. Not a "detail page." |
| Account Detail Page | **Account** (by name) | "Acme Corp" not "Account Detail: Acme Corp." |
| Actions Page | **Actions** | This one's fine. Clear, universal. |

#### Actions & States

| System term | Product term | Rationale |
|---|---|---|
| Run Briefing | **Refresh** (when intelligence exists) / **Prepare my day** (cold start) | "Run" is a developer verb. You refresh a briefing. On first open, the system prepares your day. |
| Refresh Intelligence | **Check for updates** | "Is there anything new?" not "re-enrich the meeting intelligence record." |
| Complete action | **Done** | Checkbox affordance is enough. The verb is "mark as done." |
| Reopen action | **Not done** | Undo of "done." |
| Accept (proposed action) | **Accept** | This one works. Clear intent. |
| Reject (proposed action) | **Dismiss** | Softer than reject. You're not refusing the AI; you're setting this suggestion aside. |

### 3. Intelligence Quality Labels

ADR-0081 defines four quality levels: Sparse, Developing, Ready, Fresh. These are system-accurate but user-unfriendly. "Sparse" sounds like an error. "Developing" sounds like it's the system's problem.

**Product quality labels:**

| System level | Product label | Badge style | What it communicates |
|---|---|---|---|
| Sparse | **New** | Grey, understated | "This meeting just appeared. I'm still learning about it." |
| Developing | **Building** | Turmeric, warm | "I'm putting together context. More coming." |
| Ready | **Ready** | Sage, confident | "You're prepared. I've done the work." |
| Fresh | **Updated** | Sage + dot | "Ready, and I just found something new." |
| Stale (Ready but old) | **Ready** + subtle refresh icon | Sage, muted | "Good context, but it's been a while. Tap to check for updates." |

**The "new updates" indicator** (blue dot in ADR-0081) becomes simply a dot with no label. Its presence says "something changed." Tapping reveals what.

**What replaces "Needs Prep":** Nothing. The concept disappears. Every meeting has a quality level. "New" meetings will get context as the system learns. There is no state where the user is told they need to do something the system should have done.

### 4. The Chief of Staff Voice

Beyond individual terms, the product has a voice. The editorial design language (ADR-0073) established the visual voice: restrained, confident, magazine-editorial. The verbal voice should match:

**Confident, not apologetic.** "Ready" not "We think we have enough information." The system did its job.

**Specific, not abstract.** "2 new updates about Acme since yesterday" not "New signals detected on linked entity."

**Warm, not clinical.** "Building context" not "Enrichment in progress."

**Invisible when working.** The system doesn't narrate its own processes. It doesn't say "Running entity resolution..." It just links the meeting to the right account. If it's uncertain, it asks: "Is this meeting about Acme or Globex?"

**Present when uncertain.** Confidence is silent. Uncertainty speaks. "I'm not sure which account this belongs to" is better than silently guessing wrong.

### 5. What This Doesn't Change

- **Code identifiers.** `entity_id`, `signal_events`, `enrichment_log` — all stay as-is in Rust and TypeScript. System vocabulary in code is fine and changing it would be churn.
- **ADR language.** ADRs continue using system vocabulary. They're developer documents.
- **Backlog language.** Issues use system vocabulary. They're planning documents.
- **Log messages.** `log::info!("Entity resolution: matched {} to {}", ...)` — fine. Users don't read logs.

The boundary is simple: **if a user can see it in the app, it uses product vocabulary.**

---

## Implementation

This ADR is a reference document, not an implementation plan. The implementation is a UI copy audit (see I341) that systematically finds every user-visible string using system vocabulary and translates it.

The audit should cover:
- Page titles and navigation labels
- Button labels and action text
- Badge labels and status indicators
- Loading/empty/error state messages
- Tooltip text
- Placeholder text in inputs
- Notification text (toasts, alerts)

The audit should NOT cover:
- Component names in code (`EntityPicker` stays `EntityPicker` — but its label says "Link to account or project")
- Type names (`DbAction.status` stays `"proposed"` — but the UI renders "Suggested")
- Log output, console messages, developer tooling

---

## Consequences

### Positive

- **The app feels like a product, not a prototype.** Every string the user reads was chosen for them, not for the developer.
- **The chief-of-staff metaphor becomes real.** The system speaks like a confident assistant, not a database.
- **Onboarding friction drops.** New users don't need to learn what "entity" means or why something "needs prep."
- **Design decisions get easier.** When writing new UI copy, the translation table is the reference. No more ad-hoc decisions about whether to say "intelligence" or "context."

### Negative

- **Dual vocabulary is overhead.** Every new feature requires thinking about both system and product terms. But this is a one-time cost per concept — the table grows slowly.
- **Existing documentation references system terms.** Users reading ADRs or changelogs will see different vocabulary than the app. Acceptable — these are developer documents.

### Risks

- **Over-softening.** "Dismiss" instead of "Reject" could make the action feel inconsequential. If users don't realize dismissed actions are gone, that's a problem. Mitigate with clear visual feedback (the row disappears).
- **"Building" badge anxiety.** Some users may feel the system isn't ready and delay looking at meetings until they're "Ready." Mitigate by making "Building" meetings still useful — they have attendee context, calendar details, and any available history even before AI enrichment runs.

---

## References

- ADR-0073: Editorial design language (visual voice)
- ADR-0076: Brand identity (material palette, asterisk mark)
- ADR-0081: Event-driven meeting intelligence (quality indicators, surface roles)
- I329: Intelligence quality indicators (the UI implementation of quality labels)
- I341: Product vocabulary audit (the implementation of this ADR)
