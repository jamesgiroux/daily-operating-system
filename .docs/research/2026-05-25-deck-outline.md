# RSM Final Update — Deck Outline & Talk Track

Working surface for the deck at `2026-05-25-dailyos-work-record.html`. Refine messaging
here first, then port settled changes into the HTML. Avoids re-editing markup every iteration.

## How to read this doc

- **On the slide** — the words/visuals the audience sees. Terse, declarative, designed.
- **Talk track** — what James says out loud over the slide. Conversational, first person,
  fills in the connective logic the slide only gestures at. Marked _(not yet drafted)_ where unwritten.
- **Job** — the single move this slide makes in the argument. If a slide has no distinct
  job, it's a candidate to cut or merge.

## The spine

The whole deck is one equation, only named at the close:

> **Memory** knows what happened. **Judgment** decides what matters. **Trust** is what they
> produce together.

The hinge phrase is **"what matters"** — it's how *judgment* is carried through the middle
of the deck before the word "judgment" is ever spoken:

- Slide 5 closer — "How does smart memory know **what matters**?"
- Slide 6 footer — "No signal for **what matters**."
- Close — "Judgment decides **what matters**."

Design rule from this: the middle slides build the **Judgment** half of the equation.
Memory is established early (slide 4b) and shown as insufficient (slide 5). Everything from
6 onward is constructing judgment so the close lands as a resolution, not a new idea.

## Open messaging issues

- **4b ↔ 6 redundancy.** 4b's closer ("most systems keep the fact and forget everything
  around it") and slide 6 ("no signal for what matters") make the same flat-memory point.
  Slide 5 sits between them, so it must clearly be the *industry-wide proof* beat, not a
  third restatement. Current fix: anchor 5 on memory-built / judgment-missing.
- **"Judgment" never set up before the close.** Slide 5's reframe plants it via "decides
  what matters." Watch whether that's enough or whether an earlier slide should name it.
- **Talk track largely unwritten** for the back half (slides 7 onward).

---

## Slide-by-slide

### 1 — Cover · warm white
**On the slide:**
- eyebrow — DailyOS · RSM Final Update
- headline — Can we codify trust?
- lede — AI at work relies on solving this one thing.

**Talk track:** _(not yet drafted)_
**Job:** Pose the question the whole month chased.

### 2 — The pain · terracotta
**On the slide:**
- eyebrow — The pain
- headline — Is this a fact, or did the AI just make it up?
- lede — Every person using AI ends up asking some version of this. The answer arrives
  confident and complete, with no way to tell what's grounded, what's stale, what's a
  guess, or what got pulled from where.
- lede — So you double-check everything by hand. Which is the work the AI was supposed to remove.
- meta — Hallucinations aren't a model problem. They're an architecture problem. The model
  produces. The system around it has no idea what it just produced.

**Talk track:** _(not yet drafted)_
**Job:** Name the felt pain. Reframe hallucination as an architecture problem, not a model problem.

### 3 — The Trust Question · linen
**On the slide:**
- eyebrow — The Trust Question
- headline — Can you define trust in code?
- lede — Trust isn't really a fact. It's a feeling you arrive at with instincts you don't
  always reason your way to. I wanted to explore whether that 'gut feeling' was something
  we could build around.
- lede — Can we make AI trustworthy?

**Talk track:** _(not yet drafted)_
**Job:** Turn the pain into the month's question. Frame trust as a gut/instinct thing —
sets up the human-signals slide.

### 4a — How humans decide trust · warm white (split layout)
**On the slide:**
- eyebrow — How humans decide trust
- headline — You call it gut instinct. It's really micro-signals compounding.
- lede — When you meet someone new, you don't decide to trust them. Your eyes and ears pick
  up dozens of small signals, and they compound into an opinion before you've thought a
  single sentence.
- right column (THE SIGNALS) — the face / the hands / the voice / the posture / the clothing
- conclusion — Gut instinct. You didn't decide. You compounded.

**Talk track:** _(not yet drafted)_
**Job:** Show the mechanism of human trust — signals compounding. (Was a two-column
parallel; DailyOS column removed so slide 7 owns the pillars reveal.)

### 4b — The same logic, in code · linen
**On the slide:**
- eyebrow — The same logic, in code
- headline — Code has no senses. It has memory.
- lede — A person reads those signals through their senses, in the moment. Code can't. All
  it has is memory, the facts it can recall later. So if a system is going to do what the
  gut does, the signals can't live in a glance or a tone. They have to live inside the
  memory itself.
- lede — Every fact has to carry its own evidence. What it's about, when it was true,
  whether anyone still believes it. A fact that travels with its signals is what we call a claim.
- meta — Most systems keep the fact and forget everything around it.

**Talk track:** _(not yet drafted)_
**Job:** Bridge human → code. Introduce *memory* as AI's substrate and *claim* (fact +
signals) as the requirement. Closer tees up the field critique.

### 5 — The blind spot · eucalyptus  *(the Karpathy slide)*
**On the slide:**
- eyebrow — The blind spot
- headline — Smart isn't the same as trustworthy.
- lede — We're not the only ones betting on memory. Karpathy ships a wiki his AI maintains.
  Garry Tan ships a brain his AI reads before every reply. Kieran Klaassen's compound
  engineering loop runs in every new agent ecosystem. The whole field is racing to give AI
  a better memory.
- lede — But it's all the same move. Remember more, recall it faster. None of it decides
  what matters, or what to leave out. The memory keeps getting smarter without getting any
  more trustworthy.
- closer — How does smart memory know what matters?

**Talk track:** _(not yet drafted)_
**Job:** The industry-wide proof beat. The field built **Memory** but not **Judgment**.
Picks up 4b's "memory" thread; plants "decides what matters" (= judgment) for the payoff.

### 6 — What AI is missing · cream
**On the slide:**
- eyebrow — What today's AI is missing
- headline — Every fact, the same weight, at the same time.
- lede — Today's AI memory is flat. Ask about an account and it returns everything it has,
  at the same volume, with no internal sense of what to believe more, what to flag, or what
  to leave out.
- comparison — flat memory ("No signal for what matters.") vs weighted memory ("Each fact
  carries its own self-evidence."), Acme Corp example with trust bands + superseded line.

**Talk track:** _(not yet drafted)_
**Job:** Show the gap concretely — flat memory vs weighted memory, same query.

### 7 — The five pillars · sage
**On the slide:**
- eyebrow — The five pillars
- headline — The micro-signals every claim carries.
- lede — For a memory to become trust, it has to carry the proof of its own
  trustworthiness. Every claim DailyOS holds carries five things. Together they tell the
  system what to surface and how confidently.
- claim card — Subject / Temporal scope / Sensitivity / Lifecycle (Shipped) · Salience (v1.4.6 next)

**Talk track:** _(not yet drafted)_
**Job:** The reveal. The five signals that make a claim — the judgment inputs.

### 8 — How trust gets decided · linen
**On the slide:**
- eyebrow — How trust gets decided
- headline — The five signals compound into a band.
- lede — The system doesn't hand you a confidence percentage. Nobody's eye for trust works
  that way. It hands you a band, the way you'd describe a new acquaintance.
- three bands — likely_current / use_with_caution / needs_verification
- meta — Computed from the five pillars plus source quality, corroboration, contradictions,
  and every correction you've made.

**Talk track:** _(not yet drafted)_
**Job:** Judgment output. Signals compound into a trust band (callback to 4a "compounded").

### 9 — The whole loop · warm white
**On the slide:**
- eyebrow — The whole loop
- headline — Signals in. Surfaces out. Feedback closes the loop.
- diagram — Sources → Claim substrate → Trust compiler → Abilities runtime → Surface → Feedback

**Talk track:** _(not yet drafted)_
**Job:** Show the full architecture as one loop.

### 10 — Why an abilities runtime · linen
**On the slide:**
- eyebrow — Why an abilities runtime
- headline — One front door for everything the system can do.
- lede — An ability is a single thing DailyOS knows how to do (prepare_meeting,
  get_entity_context, detect_risk_shift). The runtime is the one registry that hosts them.
  Every call routes through it, which keeps provenance, versioning, permissions consistent.
- diagram — abilities → runtime fan-out
- meta — Without this front door, every consumer reinvents context, prompts, provenance. With it, those live once.

**Talk track:** _(not yet drafted)_
**Job:** Justify the runtime — one front door, consistency everywhere.

### 11 — Substrate, by the numbers · warm white
**On the slide:**
- eyebrow — Substrate, by the numbers
- headline — One month. A new substrate.
- bignums — 932 commits to public/dev · +613k net tracked lines · 22.4B tokens building it
- stat grid — Rust +264k / Markdown +102k / JSON +79k / CSS +64k / PHP +35k / HTML +23k / React +12k / SQL +11k
- meta — While building, the app was running. 10,349 AI calls, 92.5% success.

**Talk track:** _(not yet drafted)_
**Job:** Proof of throughput. (Numbers source: `2026-05-25-dailyos-work-record.md`.)

### 11b — What the old way would have cost · linen
**On the slide:**
- eyebrow — If a conventional team had built it
- headline — What the old way would have cost.
- bignums — 40 people · 42mo · $19M
- lede — Industry estimators say a substrate this size needs a sustained team and years of
  runway. RSM did it in one month. Me, plus agents.
- meta — COCOMO organic against the 508,455-line delta.

**Talk track:** _(not yet drafted)_
**Job:** Reframe the numbers as economic disruption — one person + agents vs a 40-person team.

### 12 — What wiser produces · cream
**On the slide:**
- eyebrow — So what?
- headline — A trustworthy workspace produces output you can act on.
- lede — When every claim carries its own evidence, the briefings get sharper, the prep
  gets shorter, the decisions get easier. The better the output, the better the work it lets you do.

**Talk track:** _(not yet drafted)_
**Job:** Payoff. Why the substrate matters — better output, better work.

### 13 — The pain we just created · warm white
**On the slide:**
- eyebrow — The pain we just created
- headline — The more we trust the output, the more we'll generate.
- lede — Markdown notes, HTML plans, prep docs, transcripts, six versions of an onboarding
  screen. A trustworthy substrate is also a productive one. All that rich output has to live somewhere.
- subhead — Sound familiar?
- lede — At Automattic we've been solving content management for over twenty years. Now we
  need to solve it locally. Not just remotely.

**Talk track:** _(not yet drafted)_
**Job:** Turn success into the next problem — content management, and pivot to the Automattic tie-in.

### 14 — One more thing · linen
**On the slide:**
- eyebrow — One more thing
- headline — What if it wasn't just an app?
- lede — The runtime works inside WordPress, where we already spend our time. It works
  inside Claude or Codex over MCP. Same claims, same trust bands, same provenance.
- diagram — runtime hub → Desktop app (Today) · Claude/Codex via MCP (What if) · Local WordPress layer (What if)

**Talk track:** _(not yet drafted)_
**Job:** Expand the vision — substrate, not app. Multiple surfaces, one runtime.

### 15 — Close · eucalyptus
**On the slide:**
- eyebrow — Why all this earth had to move
- headline — Memory. Judgment. Trust.
- lede — Memory knows what happened. Judgment decides what matters. Trust is what they
  produce together when the substrate is doing its job. That's the bet. That's what RSM was for.
- pills — Memory / Judgment / Trust

**Talk track:** _(not yet drafted)_
**Job:** Land the equation the whole deck was building. Resolve "what matters" → judgment.
