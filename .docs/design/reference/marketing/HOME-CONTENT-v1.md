# Home Content — v1 Draft

This is the content rewrite for the Home page, written before any visual design.
Read it as prose and reading order. Implementation structure (sections, columns,
cards) comes later in the HTML mock.

Source-of-truth references:
- `.docs/design/product/MISSION.md`
- `.docs/design/product/PRODUCT-THESIS.md`
- `.docs/design/product/PHILOSOPHY.md`
- `.docs/design/product/PRINCIPLES.md`

The locked headline is **"Start from what matters."** Everything else is up for revision.

---

## Why this rewrite

The current Home page is well-written but tries to do too many things at once. It runs through eight sections — hero, demo card, the-enemy, outcomes grid, three-band proof rail, "not another slot" cards, ideas teaser, beta CTA — and by the end the reader has heard seven different angles on the product. None of them get to land fully.

The canon (MISSION, PRODUCT-THESIS, PHILOSOPHY) is tighter. It picks fewer points and lets them carry their own weight. This rewrite tries to do the same:

- **One positioning idea up front:** memory plus judgement, not just storage.
- **A vivid moment instead of an abstract claim:** the Killer Moment from PHILOSOPHY (the mystery 30-min meeting) — the most concrete passage in the canon, currently absent from Home.
- **Trust as the differentiator,** because that's the wedge per PRODUCT-THESIS — not feature parity, not productivity, not AI-quality. Trust.
- **Cut the outcomes grid.** It dilutes. The Killer Moment makes the same point with one image instead of four bullet rows.
- **Cut the three-band proof rail** ("A working app / A portable context surface / A personal graph") — proof points belong on Demo and Ideas, not Home. Home is the door, not the tour.
- **Keep the foundation-line teaser**, but recast as "a few statements carry the whole product" rather than a feature grid in disguise.
- **End on Beta + finis marker.** Same as today.

Net: 7-8 sections collapse to **6 sections**, each dense, each landing one idea.

---

## Voice notes

The canon's voice is **editorial, declarative, slightly literary**. Short sentences. Plain words with weight. No SaaS hype vocabulary ("seamless", "powerful", "leverage"). No marketing throat-clearing.

Examples that set the tone (lifted from canon):
- "Magic, not machinery."
- "The system operates. You leverage."
- "Your brain shouldn't have a landlord."
- "The moat is the discipline, not the data."
- "Pick up where you are, not where you left off."

Cadence: a clear thesis line, a small turn, a fact, a small turn, a closing image. Reads like a magazine column, not an SDR email.

---

## Open questions for James

1. ~~**Killer-moment placement.**~~ — Cut. James: "feels really fake and unnecessary."
2. ~~**Daily Briefing preview card.**~~ — Yes, on Home. Render it like the app, modeled on `.docs/design/reference/surfaces/briefing-d-spine.html`. (Decided.)
3. ~~**"Trust as a feature" section.**~~ — Keep. James: "great headline." (Decided.)
4. ~~**"What it is not" cards.**~~ — Cut entirely. James: "we don't need to talk about what it is not, that's typical AI contrast speak." (Decided.)
5. **Foundation-line cards.** Today there are 3 (intelligence personal / context when it matters / brain landlord). Should the Home grid show all 7 published Idea-page lines, or curate to 3? My instinct: 3 is right; 7 dilutes again.
6. **Hero CTA.** Today: "Watch the demo" + "Join the beta". Keep both? Reverse order? Just one (the stronger one)?

## Standing rule (recorded in memory)

**Marketing copy is outcomes-first, not features.** No "what it is not" / contrast cards / feature lists. Lift the canon's positions, never its prose. When uncertain, ask: "if a reader stops here, what changed for them?" If the answer is "they learned what we built," rewrite.

This rule has now been applied. v2 below replaces v1.

---

## Proposed Home content (v2 — outcomes-first)

Every section answers: *what changes for the reader?* Mechanism-language ("provenance attached", "claims persist", "structured intelligence") translated into reader-time experience ("you can act on what it tells you", "you stop second-guessing"). Where the canon's prose was load-bearing, I lifted the *position* and rewrote the words.

### 1 — Hero

> *No kicker — "DailyOS" already sits in the FolioBar pub label, repeating it as a hero eyebrow is redundant. Hero opens directly with the headline.*

**Headline (display, serif, large):** Start from what matters.

**Standfirst (serif, generous size, slight italic on the second sentence):**
DailyOS keeps up with your work so you walk in prepared for every meeting, decision, and follow-up. The thread you need is already pulled. The next move is already in front of you.

**Buttons:** [See it work] [Get on the list]

> *Changes:* third paragraph cut (was redundant with the standfirst). CTA labels reframed as reader-actions ("see it work" / "get on the list") rather than feature-objects ("watch the demo" / "join the beta").

---

### 2 — Daily Briefing slate

> *App-faithful preview, modeled on `.docs/design/reference/surfaces/briefing-d-spine.html`. Renders as an actual instance of the chrome (FolioBar accent, MeetingSpineItem rhythm, Pill colours), not a screenshot. Generic placeholder customer names (Acme, Globex, Northwind) per the d-spine reference and CLAUDE.md.*

**Eyebrow (small, uppercase, mono):** A morning, picked up

**Single line above the slate (serif, italic):**
Your 9am opens to this — a day already understood, not a blank canvas.

**Slate content (representative — exact pacing locked in HTML mock):**
- Mini-folio header: TUESDAY, OCTOBER 14 · MEMORY + JUDGEMENT · the brand mark
- Lead headline (serif): *Four meetings today, two with customers. The Acme renewal at 10:00 is the one to nail.*
- Compressed day-chart strip — 2-3 visible meeting bars
- 2 MeetingSpineItem entries: 10:00 Acme renewal (in progress, sage pill "Briefing fresh") + 2:00 Northwind partner sync (terracotta pill "No briefing yet")
- 1 "Moving" row: Acme — Renewal moved forward — Health 71 +3
- Finis-marker spacer

**Single line below the slate:**
You did not assemble any of this. Your morning starts with reading, not gathering.

> *The slate is the page's only concrete proof. Its job is to make the abstract positioning physical without resorting to marketing prose. The line above frames it as outcome ("you open to this"), the line below confirms it ("you didn't assemble it").*

---

### 3 — The flip

**Eyebrow:** The flip

**Headline:** Stop being the integration layer.

**Body:**

Every other tool wants you to maintain it before it helps you. Tag the notes. Update the CRM. Clean the task list. Then prompt the AI to reason over the mess you just spent your morning preparing.

You stop doing all that. You walk in with a thread already pulled, an account history already current, an action list that doesn't need babysitting. The work that used to happen between meetings happens for you, before them.

**Pull-out (italic, set off):** Your morning starts with reading, not gathering.

> *Outcome-led. Headline IS the reader's outcome. Body describes user-time, not system-time. "Magic, not machinery" cut from here — better placed elsewhere or held back, and "your morning starts with reading, not gathering" is the more concrete pullout.*

---

### 4 — Trust as a feature

**Eyebrow:** Trust

**Headline:** You can act on what it tells you.

**Body:**

Every other AI for work confidently hands you something plausible-but-wrong, stripped of context, forgetful of the correction you just made. You spend your day verifying its basics.

You stop doing that. You see why something surfaced, where it's certain and where it isn't, and what would change its mind. Corrections you make stay made — not just for the next prompt, but for the way it reads your work tomorrow.

**Pull-out:** Trust is a product behaviour, not a disclaimer.

> *Headline IS the outcome. Body opens on the reader's pain (verifying basics), pivots to what changes (you stop). Pull-out preserves the canon's wedge phrase, demoted from headline to coda.*
>
> *Visual hint for the mock:* a small **trust-band Pill** ("Likely current" / "Use with caution" / "Needs verification") could sit inline next to a phrase, anchoring the abstract claim in a real-product affordance.

---

### 5 — Ownership

**Eyebrow:** Ownership

**Headline:** Your brain should not have a landlord.

**Body:**

Your professional context — your relationships, your judgement patterns, the lessons you carry — is irreducibly personal. It loses value the moment it's shared.

It stays on your machine. You can take it with you. You can read it without DailyOS open. Any AI tool you trust can read it too. If you walk away one day, you walk away with everything you built.

**Pull-out:** The moat is the discipline, not the data.

> *Outcome rewrite. "Open formats every AI tool can read" → "any AI tool you trust can read it too." "Sync to whatever you trust" → "you can take it with you." Same substance, reader-time framing.*

---

### 6 — The foundation lines

**Eyebrow:** Ideas

**Headline:** A few statements carry the whole product.

**Body:**

Each is a page on its own. Read one and the rest follow.

**Cards (3 — link to existing `/ideas/...` slugs):**

— **Personal.** *DailyOS makes intelligence personal.*
The category is not more AI at work. It is intelligence that knows your world.

— **Timing.** *Your context, when it matters most.*
Preparation beats engagement. Ready beats fast.

— **The system's fault.** *The guilt is not laziness. It is the system's fault.*
Every productivity tool failed you for the same reason. Here is what changes.

> *Three cards. Each card description is reader-outcome flavour ("knows your world", "ready beats fast", "what changes"). Open question 5 still standing — happy to expand to more, but my read is 3 hits hardest.*

---

### 7 — Beta

**Eyebrow:** Public beta coming soon

**Headline:** A real demo, not a roadmap tour.

**Body:**

DailyOS is in private beta. The first invites go to colleagues who want to see a working system, not a deck. Public beta opens later this year — leave your email and we'll write to you directly when it does.

**Button:** [Get on the list →]

> *Outcome version. Lead with what the reader gets ("a real demo" + a direct-write follow-up) instead of an instruction.*

---

### 8 — End mark

Three centered asterisks (the FinisMarker pattern from the app reference).

Then, one line, centered, in the serif:
> *Start from what matters.*

---

### 9 — Footer

> *Standard corporate chrome — same shape as Studio Write's. Repo URL is `github.com/jamesgiroux/daily-operating-system` per the local repo's `public` remote. Press email stays Automattic.*

**Layout:** Two-column flex on desktop; stacks on narrow.

**Left (small, sans):**
© 2026 DailyOS · An Automattic experiment, not an official product · [Radical Speed Month 2026](https://bsky.app/profile/automattic.com/post/3mkavah2m2k2w) · Press: [press@automattic.com](mailto:press@automattic.com)

**Right (link row, small caps or small mono):**
Blog · [GitHub](https://github.com/jamesgiroux/daily-operating-system) · X (TBD if there's a handle yet) · RSS

> *Open question 7:* is there an X handle for DailyOS yet, or do we drop the X link for now? Same question for any other channel (Bluesky? LinkedIn?) — easier to add than remove.

---

## Section count + density

| # | Section              | Lift                                | Status         |
|---|----------------------|-------------------------------------|----------------|
| 1 | Hero                 | Locked headline + tightened lede    | v2 — outcome   |
| 2 | Daily Briefing slate | App-faithful preview                | New            |
| 3 | The flip             | Replaces "the enemy" + outcomes grid | v2 — outcome  |
| 4 | Trust as a feature   | Canonical wedge per PRODUCT-THESIS  | v2 — outcome   |
| 5 | Ownership            | Compress + lift "moat is discipline"| v2 — outcome   |
| 6 | The foundation lines | Re-curate to 3 cards w/ outcome subs| v2             |
| 7 | Beta                 | A real demo, not a roadmap tour     | v2 — outcome   |
| 8 | End mark             | Keep                                | —              |
| 9 | Footer               | Standard corporate chrome           | New            |

Net: 9 sections. 5 short (hero, end-mark, beta, footer, slate-frame), 4 carrying long-form prose (slate, flip, trust, ownership). Same reading rhythm as the d-spine reference: dense passages with breathing room between.

---

## What to react to first

If you only react to one thing, react to:
1. **Open questions 1-6** above.
2. **Whether the trust section earns its place on Home** or belongs elsewhere.
3. **Whether to keep the Daily Briefing preview slate** (existing) or let the prose carry the proof.

Once those three are settled, the rest is wording polish, which I'll do as a v2 of this doc.
