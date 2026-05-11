# Outline — Personal Intelligence: What Comes After the Second Brain

**Status:** Outline for category-defining blog post (target: jamesgiroux.ca, coordinated with WP.com follow-up)
**Date:** 2026-05-10
**Target length:** 1,500–2,200 words
**Hook timing:** WordPress 7.0 ships 2026-05-20 (Abilities API client-side merges to Gutenberg). Publish a few days before WP 7.0 launches to set the frame, OR coordinated same-week.

This is structure + section purposes + anchor references. Prose is yours to write. Each section has: what it does, key references, what NOT to include.

---

## Section 0 — Title

Primary: **Personal Intelligence: What Comes After the Second Brain**

Alternates if the primary feels heavy:
- **What Comes After the Second Brain**
- **Personal Intelligence: Beyond Memory**
- **The Second Brain Was Never the Goal**

Subtitle (optional, jamesgiroux.ca pattern doesn't usually use subtitles, so probably skip).

---

## Section 1 — Open with the specific moment / observation

**Purpose:** Set the scene without preamble. Drop the reader into the gap between what we have and what comes next. Not a manifesto opening — a noticed thing.

**Possible opening angles (pick one):**

- A specific moment where your current second-brain setup failed you. Not generic — a particular Tuesday, a particular meeting, a particular thing the system *should* have known and didn't.
- The observation that you now own Notion + Obsidian + ChatGPT Memory + Claude Projects + Gemini Gems, and somehow it still feels like a filing cabinet with search.
- The realization that "memory is no longer a differentiator but a baseline expectation" (this is the dominant 2026 framing in the personal-AI press). Everyone has memory now. What changed?

**What NOT to include here:**
- Don't define "personal intelligence" yet. That's later.
- Don't name DailyOS yet. That's much later or not at all in this piece.
- Don't reference Garry / Karpathy / Andy / Maggie yet — that's the acknowledgments later.

**Anchor sentence candidate:** "The second brain era is closing. Not because it failed — because it succeeded enough that the differentiation moved."

---

## Section 2 — What "second brain" was, fairly

**Purpose:** Acknowledge the era and its accomplishments. Don't dunk on it. The reader has invested time in second-brain tooling and shouldn't be made to feel stupid.

**Key references to name (briefly, with respect):**
- Tiago Forte's *Building a Second Brain* — the capture/organize/distill/express framework that defined the consumer category.
- Notion / Obsidian / Roam / Tana / Logseq — the tooling that operationalized it.
- Karpathy's LLM Wiki (2024) — the conceptual move from notes to AI-readable knowledge.
- Garry Tan's GBrain (2026) — the parallel attempt at integration; namecheck him as a peer-collaborator, not a foil.

**What this era taught us:**
- Capture is cheap if you make it cheap.
- Retrieval works when the substrate is structured.
- Compounding through use is real.
- Format matters (markdown won the portability argument).

**What it was always going to hit:**
- The maintenance trap (you already wrote about this in *Zero-Guilt Design*).
- The reading problem (filing cabinets aren't reading surfaces).
- The trust problem (you already wrote about this in *The Next Big AEO Problem Is Trust*).

**Anchor sentence candidate:** "The second brain was a real category. It taught us how to capture, how to link, and how to retrieve. The work it didn't do was decide what was true, what was current, and what to surface when."

---

## Section 3 — Why memory alone is not enough anymore

**Purpose:** The pivot. The reader nodded through Section 2; now make them lean in. The category is closing because memory has been commoditized.

**The argument:**
- 2026: every major AI tool has memory. ChatGPT Memory shipped. Claude Memory shipped. Gemini Memory. Apple Intelligence has on-device memory. Memory is table stakes.
- That means: differentiation in personal AI has moved off the memory axis.
- The new axis is judgment — what to trust, what's stale, what's salient, what to surface, when to stay quiet.
- "Memory plus judgment" is your sharper articulation. Memory remembers; judgment decides.

**Cite the field framing:** Vellum's 2026 take that "memory is no longer a differentiator but a baseline expectation" — this is the public consensus moving in your direction. ([Vellum, 2026](https://www.vellum.ai/blog/best-personal-ai-assistants-with-memory))

**What NOT to include:**
- Don't get into substrate architecture yet. This is the meta-argument: memory is solved; what's next?
- Don't name DailyOS as the solution. Keep the argument abstract.

**Anchor sentence candidate:** "Memory was the differentiator from 2022 through 2025. From 2026 on, it's the floor."

---

## Section 4 — Name the category: Personal Intelligence

**Purpose:** Plant the flag. This is the section where you introduce the category name and define it.

**The definition (this is the load-bearing paragraph of the whole piece):**

Personal Intelligence is a system that **maintains your working understanding of your professional world over time**. It does five things memory alone doesn't:

1. **Models claims, not just notes.** The unit of intelligence isn't a paragraph; it's a claim with subject attribution, temporal scope, sensitivity, and lifecycle state.
2. **Tracks trust as a property.** Every piece of information carries provenance, freshness, and a confidence band — not as a footnote, as part of its meaning.
3. **Revises beliefs as evidence changes.** Claims supersede each other, contradictions surface as questions instead of getting silently picked, dismissals teach the system about source reliability.
4. **Decides what's salient now.** Not just retrieval — relevance ranking that knows what's about to matter.
5. **Composes intelligence into reading surfaces.** Not chat replies. Not search results. Composed pages with structure, hierarchy, finite endings, and the kind of editorial discipline that lets you actually read what the system knows.

**Frame the category against existing terms:**
- Not "personal AI assistant" — that's chat with memory.
- Not "second brain" — that's storage with search.
- Not "personal superintelligence" (Meta's term) — that's AGI rebranded for consumers.
- Personal Intelligence: memory plus judgment, with trust as a property and composition as the surface.

**What NOT to include:**
- Don't claim Automattic invented the term. Personal.ai uses it. Acknowledge that lightly.
- Don't trademark-claim. Generic descriptor, not a brand.

**Anchor sentence candidate:** "Personal Intelligence is what comes after second brains: a system that doesn't just remember, but knows what it knows and what it doesn't."

---

## Section 5 — The substrate primitives, in plain language

**Purpose:** Make the abstract definition concrete. Show, briefly, what's underneath. This is where the architecture work translates to readable prose.

**Five primitives, one paragraph each, NO engineering jargon:**

1. **Typed claims.** Replace "the system has notes about X" with "the system has claims about X, each with who said it, when, against what evidence, how confident, what would change it."
2. **Trust bands.** Replace "AI generated this" with "this is `likely_current` / `use_with_caution` / `needs_verification` — and the system knows which is which without you asking."
3. **Belief revision.** Replace "the system remembers" with "the system updates. When something changes, claims based on the old fact retire; corrections you make stick; contradictions surface as questions, not silent overwrites."
4. **Salience-driven surfacing.** Replace "the system retrieves" with "the system decides what to show you now — and what to keep quiet about — based on what's actually moving in your world."
5. **Composition.** Replace "the system gives you an answer" with "the system composes a page — sections, blocks, hierarchy — that's meant to be *read*, not consumed as chat."

**What NOT to include:**
- No code examples. This is for human readers, not engineers.
- No ADR references. They're internal.
- No mention of `SurfaceClient`, `AbilityPolicy`, `Composition` types. Translate; don't expose the substrate.

**Anchor sentence candidate:** "If memory is the floor, these five primitives are the building above it."

---

## Section 6 — The open-source argument

**Purpose:** The strategic claim. Personal Intelligence is being built right now; the question is who's building it and how. Make the case for the open-web version.

**The contrast (be specific without being combative):**

- **Meta is building "personal superintelligence."** Closed model. $115–135B in 2026 capex. Pivot away from open-source. Their version of personal intelligence is something you rent from Menlo Park. ([Meta announcement](https://about.fb.com/news/2025/07/personal-superintelligence-for-everyone/))
- **Apple Intelligence is hardware-locked.** Closed model. Tied to specific devices. Beautiful, but not portable.
- **Personal.ai, Notion AI, Rewind, Mem** — all proprietary, hosted, single-vendor.
- **None of these give you your substrate.** The data lives in their stores, the trust mechanics are their secret sauce, the rendering is their app.

**The argument:**

Personal Intelligence is, by definition, about *you*. The substrate is your working memory of your professional life. The trust scores are calibrated against *your* corrections. The compositions are rendered against *your* claims. That's not the right thing to rent.

**Three things personal intelligence needs that closed models can't offer:**

1. **Sovereignty.** Your substrate lives on your machine, in formats you own.
2. **Portability.** When a better model ships, your substrate moves to it; when a better surface ships, your substrate renders in it.
3. **Trust transparency.** You can see why the system trusts what it trusts. Closed boxes can't show their work.

**Why this fits the WordPress / Automattic story:**

WordPress is 43% of the web because it's open. The same argument applies one layer up: when personal intelligence becomes the next major software category, the open version matters more than the centralized one.

**What NOT to include:**
- Don't bash Meta or Personal.ai by name in confrontational language. Cite, contrast, move on.
- Don't make this section longer than necessary. The argument is "open matters here for the same reasons it mattered for the web." Make it; don't belabor it.

**Anchor sentence candidate:** "When AI gets personal, the question stops being which model is smartest and starts being whose servers your memory lives on."

---

## Section 7 — The WordPress angle (light, doesn't dominate)

**Purpose:** Connect to the WP 7.0 launch hook without turning the piece into a product pitch.

**The specific thing to name:**

- WordPress 7.0 (shipping 2026-05-20) ships the Abilities API in core, with the client-side API merging into Gutenberg.
- The Abilities API is a typed capability framework that any plugin can register against.
- The WordPress MCP Adapter bridges those abilities to Claude Desktop, Cursor, VS Code — making WordPress an MCP server out of the box.
- This is the substrate the open version of personal intelligence can stand on.

**What this enables (in plain terms):**

Any developer can register a personal-intelligence ability into WordPress. Any user can install a plugin that exposes intelligence on their local WordPress site. Any agent can consume those abilities. The infrastructure for the open version of personal intelligence is, as of next week, shipped.

**What NOT to include:**
- Don't introduce DailyOS in this section. If you mention it at all, save it for Section 8.
- Don't pitch the WP Abilities API as the *only* path. It's one path; a strong one because of distribution.

**Anchor sentence candidate:** "WordPress 7.0 doesn't ship a Personal Intelligence Engine. It ships the infrastructure that makes one buildable on the open web."

---

## Section 8 — Vision close: the Personal Intelligence Engine

**Purpose:** Name what comes next without making it a product launch.

**Introduce the term:**

The next noun in this category isn't "assistant" (you ask, it answers) or "second brain" (you store, it retrieves). It's **engine** — the thing that runs continuously, knows what it knows, decides what to surface, and shows up where you compose.

**Light future-state sketch:**

- You open your local WordPress. The engine has prepared your day.
- You're in Claude Desktop. The engine answers from your substrate.
- You're in Obsidian. The engine renders the same claims as blocks you can compose.
- You're nowhere — and the engine is still maintaining your substrate, ready for whichever surface you walk up to next.

**The point of "engine":**

You don't open the engine. You don't talk to the engine. The engine runs. Surfaces are how you interact with it; they're interchangeable.

**Optional final paragraph naming DailyOS:**

If you want to name it here: a one-paragraph "this is what I'm building" closer. Frame as: "I'm building one version of this engine, called DailyOS, in the open, for the WordPress ecosystem. It's not the only version that will exist, and it shouldn't be. The category is bigger than any one project."

If you'd rather keep this piece category-defining and let DailyOS land in a follow-up post — skip this paragraph and close on the engine vision.

**Anchor sentence candidate:** "Personal Intelligence is the category. The Personal Intelligence Engine is what builds it."

---

## Section 9 — Acknowledgments (short)

**Purpose:** Standing on shoulders. Honest credit.

**Name (briefly):**
- **Andrej Karpathy** — LLM Wiki sketched the original direction.
- **Garry Tan** — GBrain is parallel work in the integrated space; cite as peer-collaborator.
- **Andy Matuschak** — evergreen notes / tools for thought.
- **Maggie Appleton** — ambient AI / digital gardens framing.
- **Ink & Switch (Geoff Litt, et al.)** — local-first software, malleable software, the closest research community.
- **The WordPress AI Building Blocks team (Felix Arntz, et al.)** — Abilities API + MCP Adapter shipped the infrastructure.

**Anchor sentence candidate:** "None of this was invented in one place. The category is forming because a lot of people noticed the same thing at the same time."

---

## Connection to existing jamesgiroux.ca lineage

The piece sits as the third in a thematic trilogy:

1. *Zero-Guilt Design: Releasing My Daily Operating System* (existing) — established the design philosophy.
2. *The Next Big AEO Problem Is Trust* (existing) — established trust as the wedge.
3. **This piece** — synthesizes both into the category name.

Reference your existing posts naturally where relevant. Section 2 ("what second brain was, fairly") is the natural place to nod at the zero-guilt post. Section 4 / 5 (defining personal intelligence) is the natural place to reference the AEO trust post.

---

## What to do next (concrete)

1. **USPTO check** on "Personal Intelligence" trademark filings before publish. 30 minutes.
2. **Email the two Forrester analysts** within the week with a one-pager preview of this framework.
3. **Coordinate with the Automattic PR team** on timing (within 2-3 weeks of WP 7.0 launch).
4. **Decide:** does WP.com publish a coordinated piece, or does jamesgiroux.ca carry it alone? (Matt's call.)
5. **Decide:** does this piece name DailyOS at the end, or stay category-defining with DailyOS as a follow-up post?

---

## What this outline does NOT include (deliberately)

- The actual prose (per the agreement that voice is yours).
- Specific architectural details (substrate primitives, ADR references, SurfaceClient, etc.). Translate; don't expose.
- A product launch tone. This is category-defining, not product-pitching.
- A long acknowledgment / academic-citation section. Keep it short, generous, honest.
- Anything that sounds like "I'm the founder of personal intelligence." You're a contributor / leader-among-equals.
