<!--
  ═══════════════════════════════════════════════════════════════════════════
  PRIMARY EXTERNAL REFERENCE — keep this link permanent.
  Wikipedia: "Signs of AI writing"
  https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing
  The best-maintained public catalog of AI writing tells. It is community-
  curated and updated as models evolve, so re-read it periodically and graduate
  any new prose-relevant tell into the classes below.
  Note: ~2/3 of that page is Wikipedia-specific (DOIs, category tags,
  maintenance templates, wikitext bugs) and does NOT apply to prose — only the
  language / structure / rhetoric / formatting tells transfer here.
  Caveat the page itself makes: automated AI-detectors are unreliable and humans
  detect at near chance. The goal is NOT to beat a detector. It's to not read as
  AI to a suspicious human. That's a higher and more durable bar.
  ═══════════════════════════════════════════════════════════════════════════
-->

# AI-Tells — The Current-Generation Detection Layer

The canonical taxonomy of patterns that make writing read as machine-generated **to a 2026 reader**. This is the reference for the Authenticity pass and is partially enforced by `scripts/detect_patterns.py`.

**Primary external reference:** [Wikipedia — Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing). Community-maintained, kept current as models change. Many classes below are adapted from it (marked *[Wikipedia]*); Class 1, 4, 5, 7 and the aphorism/narrator frame are this skill's own additions.

---

## The one idea behind every tell: aphorism mode vs. narrator mode

There are two registers a piece can be written in.

**Aphorism mode** is the machine's default. It writes in crisp, confident, balanced declaratives. It loves the metaphor-equation ("X is the gate"), the antithesis ("not X, but Y"), the rule of three, the drumroll reveal, and the tidy summary line. Each sentence *sounds* like an insight. Stacked together, they read as a confident voice that belongs to no one. **That confidence-without-a-person is the tell.** A human writing fast doesn't reach for the perfect aphorism; the machine reaches for it every time, because "sounds insightful" is what it was trained to produce.

**Narrator mode** is the target. It's a specific person thinking out loud. It's allowed to be uncertain ("I think," "what I keep noticing," "I'm not sure this is right but"). It's grounded in a real moment rather than a general truth. It lets a sentence be plain when the content is plain. It doesn't perform insight — it reports what someone actually noticed.

**The single highest-leverage edit in this whole skill: when a sentence sounds like it would look good on a slide, it's probably aphorism mode. Rewrite it as something a person would say to a colleague.**

Every class below is a specific shape of aphorism mode.

---

## Class 1 — Abstract-noun equations (the "X is the gate" family)

**This is the priority class.** It's the one that slips into executive briefings and makes them read as machine-written.

**What it is:** a concrete situation gets equated to an abstract strategic noun. The sentence asserts that some thing *is* the [gate / unlock / wedge / lever / forcing function / moment / through-line / north star / tell / crux / linchpin / fulcrum / inflection point / litmus test / tip of the spear].

**Why readers clock it:** nobody talks like this. It's consultant-deck language compressed into a declarative. It signals "an LLM tried to sound strategic." It's also almost always content-free — "this meeting is the gate" tells the reader nothing they couldn't get from "this meeting matters."

| Aphorism (cut it) | Narrator (what a person says) |
|---|---|
| "This meeting is the gate." | "A lot rides on this meeting — if it goes well, the rest opens up." |
| "This is the unlock." | "This is what's been blocking us." |
| "The pilot is the forcing function." | "The pilot is what finally makes us decide." |
| "That's the wedge into the account." | "That's how we get in the door." |
| "Trust is the through-line here." | "Most of these come back to trust." |
| "This is the moment that defines the quarter." | "How this quarter goes probably hinges on the next few weeks." |

**Fix:** state the plain thing the metaphor is standing in for, then (if it earns it) say *why* it matters concretely. Delete the noun-equation.

---

## Class 2 — Antithesis / the "not X, but Y" reflex *[Wikipedia: "negative parallelisms"]*

**What it is:** defining something by first negating a thing nobody claimed. "It's not about speed. It's about trust." "This isn't a feature. It's a philosophy." "They weren't asking if we could help — they were asking how fast."

**Why readers clock it:** it manufactures false tension and it's the single most overused LLM rhetorical move of this era. One instance is fine. Two in a piece is a pattern. Three is a signature.

**Fix:** state Y directly. ("This is about trust." / "We move fast.") Keep the antithesis only when the X is a real position a real reader actually holds.

See also `ANTI-PATTERNS.md` → Contrast Framing for the line-level examples.

---

## Class 3 — The rule of three *[Wikipedia]*

**What it is:** every list resolves to exactly three items; every important sentence has three clauses. "Faster, cheaper, better." "It's clear, it's specific, it's actionable." The cadence of three regardless of how many things are actually true.

**Why readers clock it:** real arguments have one, two, four, seven things. The machine pads or trims to three because three scans as rhetorically complete.

**Fix:** say the number of things that are true. If there are two, say two. If there's one, say one and let it stand.

---

## Class 4 — The drumroll reveal & false-intimacy windup

**What it is:** a sentence whose only job is to promise a payoff before delivering it. "Here's the thing." "Here's what nobody tells you." "The part I keep coming back to is this." "And that's where it gets interesting." Also the *oracle reveal*: "there's a third axis nobody has named," "what most people miss is."

**Why readers clock it:** it's a confidence trick — performing intimacy or revelation to borrow authority the content hasn't earned. The author should read as someone noticing something, not an oracle unveiling it.

**Fix:** delete the windup and start the real sentence. Write as a colleague-from-inside-the-space, not an analyst revealing a category.

See also `ANTI-PATTERNS.md` → Windup & Pointer Connectives.

---

## Class 5 — The tidy summary close

**What it is:** the aphoristic last line that ties a bow. "At the end of the day, it comes down to people." "And that's the real story." The closing one-liner engineered to be quotable.

**Why readers clock it:** real endings trail off, ask something, or just stop. The manufactured-mic-drop is a structural tell.

**Fix:** end on the last real thing you had to say. A genuine question to the reader often beats a manufactured aphorism.

---

## Class 6 — Participial run-ups & connective scaffolding

**What it is:** sentences that open with a participial throat-clear that summarizes the previous sentence before adding anything. "Building on this," "Having established that," "With that in mind," "Taken together,". Also transition-word stuffing: "Furthermore," "Moreover," "Additionally,".

**Why readers clock it:** it's the seams of an outline showing through. A person continues the thought; the machine announces that it's continuing.

**Fix:** cut the run-up; start with the substance. The logical connection is usually obvious without the signpost.

---

## Class 7 — The rhetorical one-word question

**What it is:** "The result? A 40% lift." "Why does this matter? Because…" "The catch? It only works once." Self-asked, self-answered, in fragments.

**Why readers clock it:** it's a marketing-copy cadence. Used once for genuine emphasis it's fine; used as a structural habit it screams generated.

**Fix:** make it a normal sentence. "The result was a 40% lift."

---

## Class 8 — Inflated diction *[Wikipedia]*

**What it is:** the words LLMs reach for far more than humans do. The Wikipedia catalog tracks these by era, which is useful — the vocabulary drifts as models update:
- **Persistent:** crucial, pivotal, underscore, enhance, emphasizing, showcasing, highlight, leverage (verb), robust, seamless
- **2023–mid-2024:** delve, boasts, bolstered, enduring, garner, intricate, interplay, landscape, meticulous, tapestry, testament, valuable, vibrant, additionally
- **mid-2024–mid-2025:** align with, fostering, featuring
- **Metaphor crutches:** navigate the landscape, in the realm of, a testament to, ever-evolving, fast-paced, multifaceted, harness, unlock (verb), elevate, embark, journey (metaphorical)

**Why readers clock it:** this vocabulary is now so associated with AI marketing output that a single "delve" or "tapestry" flips the reader's suspicion on.

**Fix:** use the plain word. *Delve into → look at. Leverage → use. Robust → solid / works. A testament to → shows. Underscore → show.*

---

## Class 11 — Copula avoidance *[Wikipedia]*

**What it is:** replacing plain "is/are/has/was" with a wordier stand-in. "X *serves as* a..." / "X *stands as* a..." / "X *boasts* three..." / "X *represents* a..." / "She *began her career as*..." instead of "is / has / was."

**Why readers clock it:** humans reach for the short verb. The machine inflates it because the longer verb scans as more formal. Stacked, these make everything sound like a press release.

**Fix:** "is," "has," "was." "The platform has three tiers," not "the platform boasts three tiers."

---

## Class 12 — Significance & legacy inflation *[Wikipedia]*

**What it is:** grafting grand importance onto an ordinary thing. "This is a testament to..." "underscores the significance of..." "marks a pivotal moment..." "reflects broader trends in..." "leaves an indelible mark." "cements its place as..."

**Why readers clock it:** it's the machine padding thin material with borrowed gravity. The reader feels the reach exceed the grip.

**Fix:** state what actually happened and let the reader judge the significance. If it's genuinely significant, the facts carry it; if it isn't, no amount of "indelible mark" will fix that.

---

## Class 13 — Vague attribution *[Wikipedia]*

**What it is:** sourcing a claim to a fog. "Experts argue..." "Industry reports suggest..." "Observers have noted..." "Some critics say..." "It is widely regarded as..." Also: "such as" in front of a list that's actually meant to be exhaustive.

**Why readers clock it:** real arguments name a source or own the claim. The fog-source is how the machine asserts authority it doesn't have. It's also a close cousin of unverified-absence claims ("no competitor offers X") — both are unsupported confidence.

**Fix:** name the source, or own the claim in first person ("my read is"), or cut it. "Three of the five people I talked to said…" beats "observers have noted."

---

## Class 14 — Superficial-analysis participles *[Wikipedia]*

**What it is:** tacking a vague present-participle clause onto the end of a sentence to manufacture insight. "...ensuring long-term success." "...reflecting broader industry shifts." "...contributing to its lasting appeal." "...highlighting the importance of collaboration."

**Why readers clock it:** the clause adds no information; it's the shape of analysis without the substance. (Related to Class 6's connective scaffolding, but this one trails rather than leads.)

**Fix:** delete the trailing clause, or replace it with the actual specific consequence if there is one.

---

## Class 15 — Elegant variation *[Wikipedia]*

**What it is:** compulsively swapping in synonyms to avoid repeating a word, even when repetition would be clearer. "The company… the firm… the organization… the enterprise…" all for the same subject in four sentences.

**Why readers clock it:** it comes from the model's repetition penalty, not from style. Humans happily repeat a plain noun; the thesaurus-churn reads as machine.

**Fix:** repeat the plain word. Use a pronoun. Variation should serve meaning, not avoid a repeat.

---

## Formatting tells *[Wikipedia]*

These are surface tells that survive even good prose. They matter most for articles, posts, and docs.

| Tell | Fix |
|------|-----|
| **Title Case In Every Heading** | Sentence case headings ("What this means", not "What This Means") |
| **Mechanical boldface** — bolding a phrase in every bullet, or every instance of a term | Bold sparingly, for genuine emphasis only |
| **Emoji as section markers** (✅ 🚀 💡 as structure) | Use real headers; emoji only where the voice profile genuinely allows |
| **Em-dash clause-stuffing** | (Already enforced — see `ANTI-PATTERNS.md` / lint_typography) |
| **Inline-header bullet lists** — every bullet is "**Header:** description" | Mix in prose; not every list needs bolded lead-ins |
| **Thematic breaks (`---`) before every heading** | Let headings stand on their own |

---

## Colon-as-setup

**What it is:** a short phrase, a colon, then the real sentence. "The reality: we're behind." "My take: this won't scale." "Bottom line: ship it."

**Why readers clock it:** the colon is doing throat-clearing work — a setup-colon in flowing prose is a common AI-tell.

**Fix:** delete the setup and the colon. "We're behind." "I don't think this scales." (Colons are fine in their literal jobs — lists, ratios, time. The tell is the rhetorical setup-colon in flowing prose.)

---

## How to use this file (for the Authenticity pass)

1. Read the draft once for **register**: is this aphorism mode or narrator mode? If a paragraph would look good on a slide, flag it.
2. Sweep for each class above. Count instances. One instance of a class is usually fine; **two of the same class is a pattern; three is a signature that must be broken.**
3. Cross-check `voices/LEARNINGS.md` for any corrections logged in this project that aren't yet a formal class here — those carry the same weight as the classes above.
4. Do not just flag. **Rewrite.** Produce the narrator-mode version of every flagged line so the author edits prose, not a checklist.
5. The bar is not "no tells exist." The bar is: **would a sharp reader who is actively suspicious of AI writing be unable to point to a line and say 'a bot wrote that'?**

## On detection (the honest caveat)

Per the Wikipedia source: automated AI-detectors have non-trivial error rates, and humans detect AI writing at close to chance (heavy LLM users hit ~90%, everyone else barely above random). Two consequences:
- **Don't trust a detector, and don't write to beat one.** The detector regex in `detect_patterns.py` is a coarse first pass, not the standard.
- **The real bar is a suspicious human.** Your readers are increasingly the heavy-user 90%. Narrator mode isn't about evading detection; it's about the writing actually being a person's, which is the only thing that survives a careful read.
- These tells drift as models change. Re-read the source periodically and graduate new ones via `voices/LEARNINGS.md`.
