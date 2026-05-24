# DailyOS — Product Principles

> Internal decision-making guide. For the founding manifesto, see `/design/PHILOSOPHY.md`. For GTM narrative, see `POSITIONING.md`.
>
> These principles translate the founding philosophy into specific product decisions. Positioning changes when markets shift. These principles change when the product model changes.


---

## 1. The most valuable context is irreducibly personal

In 2024, Harvard Business Review published a study of two B2B technology firms with identical CRM processes and materially different execution outcomes. The finding: systems of record capture outcomes, not how execution unfolded. The decisive context — which signals triggered escalation, how risk was weighed, when to defer — lived in emails, chat threads, side conversations, and individual judgment. It disappeared once the deal moved forward.

The article recommended building organisational context libraries to capture these patterns. It's a sound idea. But it misses something: the most actionable part of that context belongs to individuals, not organisations.

The sales director who reads "internal budget alignment" in a customer email and knows it means something different from "legal review" has that knowledge because of *her* history — her prior deals, her particular relationships, her accumulated pattern recognition in her specific book of business. You can try to index that into a shared library, but you lose the specificity that makes it signal rather than noise. And once she knows the system is capturing her private observations for colleagues to see, she stops writing them honestly.

Individual context cannot be fully captured without degrading it. It cannot be shared without transforming it. It lives in one person's experience and judgment, and it belongs to them.

DailyOS is built around this belief. The brief is personal not as a limitation but as a design principle. The intelligence is yours because its value depends on it being yours.

---

## 2. The three-layer knowledge model

There are three distinct layers of work knowledge, and they require different architectures:

| Layer | What it holds | Who owns it | Appropriate tool |
|-------|--------------|-------------|------------------|
| **Systems of record** | Outcomes and transactions — what happened | The organisation | Salesforce, Zendesk, Jira |
| **Organisational context** | How the org makes decisions — patterns across roles, teams, and deals | The organisation | Glean, enterprise knowledge graphs |
| **Individual context** | How this person works — their specific relationships, judgment patterns, signals, and accumulated professional knowledge | The individual | DailyOS |

Most enterprise AI investment is going into layers 1 and 2. Layer 3 — the individual context layer — is largely unaddressed. It's also the layer that most directly shapes daily execution: the briefing you walk into a meeting with, the relationship intelligence that tells you how to frame a conversation, the pattern recognition that comes from 18 months of history with a specific account.

DailyOS is the layer 3 tool. Not a replacement for systems of record or organisational knowledge graphs — a complement that adds the individual's professional context to the stack.

---

## 3. Sharing happens at the output layer, never the signal layer

The brief draws on private signals: your email tone patterns, relationship temperature readings, personal coaching observations, the things you've noticed about a stakeholder that never made it into Salesforce. These signals have value precisely because they're unfiltered and honest. The moment you introduce an audience — colleagues, managers, AI systems that might relay content — the honesty disappears.

Reports are different. An EBR/QBR, an Account Health Review, a Success Plan — these are documents you've reviewed, edited, and decided represent your considered view. They're authored, not automatically generated. They're appropriate to share, export, or publish back to an organisational knowledge layer.

The architecture follows from this:
- Signals → intelligence → brief: private, local, never leaves your machine
- Intelligence → report: curated by you, shareable at your discretion
- Report → org knowledge layer (e.g., Glean): intentional, user-initiated, never automatic

This is not a technical constraint. It's a values statement. DailyOS is not a surveillance tool. The value of personal intelligence depends on its remaining personal.

---

## 4. AI produces; users curate

The greatest waste in knowledge work is not lack of intelligence — it's the time spent gathering and assembling context that could be gathered and assembled automatically. The daily preparation ritual: digging through emails, reading old notes, checking the CRM, building a mental map of where things stand. This is production work disguised as preparation. It consumes hours that should be spent on the actual work.

DailyOS inverts this. The AI produces the brief, the intelligence, the synthesis. The user's job is to receive it, evaluate it, correct what's wrong, and act. Not to build it.

This changes what AI means in a product. Not AI-assisted (AI helps you do tasks faster). Not AI-enhanced (AI adds a feature to an existing tool). AI-native: the AI is the primary producer, not a helper. The product doesn't exist without the AI. The user doesn't use the AI — the user benefits from its outputs.

Corrections are intelligence. When the user edits the brief, dismisses a signal, or overrides an intelligence assessment, they're not fixing a bug — they're teaching the system. Every correction makes the next brief more accurate. The system is designed to reduce how often corrections are needed, not to make corrections feel like maintenance.

---

## 5. The personal computing reclamation

DOS was the last era when computing felt truly *yours*: one user, one machine, your files on your disk, a direct relationship between you and your machine. That relationship eroded through the cloud era. Your data moved to their servers. Your tools became tenancies. Your cognitive work became someone else's training data.

DailyOS reclaims what was lost without abandoning what was gained. Local-first: all data on your machine, all processing on your hardware, no cloud dependency for core function. AI-native: the intelligence that makes the machine truly useful now exists, and it runs locally. The result is the original personal computer promise, finally fulfilled: a machine that knows you, that works for you, that is genuinely yours.

This isn't nostalgia. It's a principled architectural choice with increasing market relevance. As AI capabilities become commoditised, data sovereignty becomes the differentiator. The tools that trust users with their own data will outlast the ones that don't.

*"Your brain shouldn't have a landlord."*

---

## 6. Zero guilt is a design requirement, not a feature

Productivity tools have a structural failure mode: they create obligations. You have to maintain them, update them, log things in them, keep them current. When life happens and you stop, the guilt compounds. The backlog grows. The system that was supposed to help you starts to feel like another demand on your time.

DailyOS has one answer to this: don't create obligations. The system catches up — you don't have to. If you don't open the app for a week, the intelligence continues accumulating. If you don't update your account notes, the signal bus does it for you. If you skip your weekly review, nothing breaks.

This is not just an emotional design choice. It's a constraint on every feature decision: if a feature creates a recurring obligation that the user must maintain for the system to work correctly, the feature is wrong. The maintenance burden belongs to the machine.

Applied to sharing and collaboration: shared systems create the strongest obligations. "My colleagues are depending on me to keep this up to date." That pressure, once introduced, never leaves. Keeping DailyOS personal is partly about privacy and partly about keeping the obligation model intact.

---

## 7. Context compounds; access does not

When everyone has access to the same AI models, the same platforms, and the same vendor ecosystem, model quality is not a differentiator. Access is not a moat.

Context is. The individual who has 18 months of synthesised relationship history, whose signal bus has learned their priorities and attention patterns, whose professional context is encoded in their user entity — that person's AI is genuinely more useful than someone who just started. Not because they have better tools. Because their context compounds.

This is the long-term value proposition: the longer you use DailyOS, the better it knows you. The briefs get more accurate. The signal weightings get more precise. The intelligence reflects your actual relationship with your accounts, not a generic synthesis of similar accounts.

This compounding only works if the context stays with the user. It cannot be shared with the organisation (it loses the personal specificity), it cannot live in a cloud (it becomes the vendor's asset), and it cannot be rebuilt from scratch each session (it loses the accumulated learning). The local-first, privacy-preserving architecture is not a constraint on this model — it's what makes the model possible.

---

## What these beliefs imply for product decisions

**On adding collaboration features:** Don't. The brief is personal for the same reason a private notebook is more honest than a shared document. If users want to share intelligence, they do it through curated reports — documents they've deliberately authored. Not through automatic pipelines.

**On cloud architecture:** Keep what the user values on the user's machine. Connectivity to external services (Google, Glean, Clay) is acceptable — those are pull relationships where DailyOS requests context. Push relationships — where DailyOS sends the user's intelligence outward automatically — require explicit user consent per action.

**On organisational features (teams, shared accounts, manager views):** These are out of scope not because they're impossible but because they change the value proposition in a way that undermines it. The brief would stop being honest. The signals would be sanitised for an audience. The context layer would cease to be personal. If these features are ever built, they belong in a separate product.

**On monetisation:** Never train on user data. Never use personal intelligence to improve a shared model. The user's context is theirs. The product can learn to serve *that user* better, not to serve other users better with what it learned about this one.

**On the enterprise angle:** DailyOS's value to an enterprise is a better-prepared, more effective individual professional — not an organisational intelligence layer. The enterprise benefit is emergent and indirect. Selling to enterprises is fine. Redesigning the product to serve the enterprise directly — at the cost of serving the individual — is not.

---

*Last updated: 2026-02-24*
*Informed by: POSITIONING.md, ADR-0086, ADR-0089, ADR-0090, .docs/research/glean-integration-analysis.md, HBR "Context Is the New Competitive Advantage" (2024)*
