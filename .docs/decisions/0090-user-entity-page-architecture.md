# ADR-0090: User Entity Page Architecture — Professional Context Surface, Context Entries, Two-Layer Priorities

**Date:** 2026-02-22
**Status:** Accepted
**Target:** v0.14.0
**Extends:** ADR-0089 (User Entity and Professional Context Layer)
**Complements:** ADR-0079 (Role Presets) — role presets govern vocabulary; this ADR governs page structure, input model, and role-shaped section prominence.

---

## Context

The user entity established in ADR-0089 needs a dedicated page surface — not inside Settings, but its own first-class area with the same editorial treatment as account and person detail pages. ADR-0089 defined what the user entity contains (professional context, declared priorities, attached documents). This ADR defines how that content is organised on a page, what interaction patterns it uses, how role presets shape what's prominent, and what new input types are introduced.

### The fundamental interaction asymmetry

The user entity has a fundamentally different interaction model from every other entity in DailyOS. Accounts and people are primarily READ — intelligence is pulled from signals, the AI generates it, the user corrects it occasionally. The user entity is primarily WRITTEN — the user authors the context, they decide what to tell the system. The AI consumes it.

This inversion has architectural consequences. Account detail pages are intelligence surfaces: the design emphasises generated content, with user editing as a secondary action. The user entity page is an authoring surface: the design must support comfortable, unhurried writing. The page should feel like a professional workspace, not a configuration form.

A secondary consequence: unlike accounts and people, the user entity does not have an intel_queue entry. DailyOS does not enrich the user entity. The user entity is the enrichment input for everything else. This means there are no loading states for generated content, no re-enrich affordances, no generated intelligence chapters. The page is entirely authored.

### Why Settings is the wrong home

Settings contains technical integration configuration: Google OAuth credentials, workspace path, Claude Code installation, notification preferences, appearance settings. These are infrastructure choices the user makes once and rarely revisits. Professional context — what the user delivers, who their customers are, what winning looks like — changes on a quarterly cadence. It deserves a primary navigation position alongside the entities it informs, not a secondary location inside a configuration panel.

Placing identity and professional context inside Settings signals to the user that these fields are administrative overhead. Giving the user entity a dedicated nav item signals that telling DailyOS about your professional world is as important as reviewing your accounts.

### What ADR-0089 left underspecified

ADR-0089 established the data model and the intelligence integration. It did not specify:
- The precise page layout and section structure
- The two-layer priority model (annual vs. quarterly, both persist until the user changes them — no automatic expiration)
- The context entries concept (embedded professional knowledge distinct from entity notes)
- How role presets shape section prominence on this specific page
- The document attachment UI and `_user/docs/` integration

This ADR fills those gaps.

---

## Decisions

### 1. Dedicated navigation item, not a Settings section

The user entity lives in its own nav item in the primary sidebar — not inside Settings, not inside a sub-menu. It appears alongside Accounts, People, and Projects. It routes to `/me`.

Settings retains all technical configuration: Google OAuth and integrations, workspace path, Claude Code path, role preset selection, appearance (personality tokens, theme), notifications, system status, and developer tooling. It loses the YouCard identity section (name, title, company, focus) — those fields move to the user entity page, which is the correct home for identity and professional context.

The `/me` route renders a page with the same editorial skeleton as `/accounts/:id` and `/people/:id`: a page-level heading, section rules, editorial body typography, and inline editing throughout.

### 2. Six-section page structure

All six sections exist for all role presets. The preset determines which sections are featured (expanded by default) vs. collapsed (header visible, content requires an expand gesture). Section existence is unconditional; prominence is preset-driven.

**§ About Me** — Identity layer. Fields: Name, Title, Company (migrated from Settings), Company Bio (a single paragraph — the 30-second explanation of what the company does and who it serves, not marketing copy), Role Description (what the user actually does day-to-day, what they're responsible for, how they're measured), How I'm Measured (the KPIs or outcomes the user tracks), Team Context (who they work with; can link to people entities in the DB). All fields are EditableText with role-specific placeholder text.

This section is Featured for all presets. It is the minimum useful input and the foundation for all subsequent sections.

**§ What I Deliver** — Product and service layer. Fields: Value Proposition (EditableText — customer benefit frame, not feature list), Success Definition (EditableText — outcomes customers measure), Product Context (EditableText — category, positioning), Pricing Model (EditableText — category-level description only: "annual contracts, enterprise segment" — not exact figures), Differentiators (EditableList — items the user considers their key advantages), Common Objections (EditableList — Q&A format, one objection per item with a user-maintained response), Competitive Landscape (EditableText — named competitors with one-line context per competitor).

Section prominence varies by role preset. CS and Sales presets feature this section. Leadership preset shows it but does not expand it by default. See prominence table below.

**§ My Priorities** — Two-layer priority model. This section contains two structurally distinct sub-sections: Annual Priorities (year-level bets) and Quarterly Priorities (current quarter focus). Both persist until the user changes them — no automatic expiration. See Decision 3 for the full model.

This section is Featured for all presets.

**§ My Playbooks** — Methodology layer. Named sections determined by role preset. For CS: "At-Risk Accounts," "Renewal Approach," "EBR/QBR Preparation." For Sales: "Deal Review," "Territory Planning," "Objection Handling." For other presets: one generic "My Methodology" section, with additional named sections added in v0.14.1. Each named section is an EditableText field — extended prose authored by the user and consumed by the AI semantically. These are professional knowledge bases, not instructions to the AI. The embedding is the primary interface.

**§ Context Entries** — Professional knowledge layer. User-authored intelligence inputs that are not tied to any specific entity. Each entry has a title and a body. Entries are embedded immediately on save via the existing nomic-embed-text pipeline and retrieved by semantic similarity during entity enrichment. See Decision 4 for the distinction from entity notes and the full data model.

This section is shown for all presets. It is not featured by default — it requires the user to discover it and understand the concept. Once entries exist, the section expands to show them.

**§ Attachments** — Document dropbox. A drag-and-drop zone and file list for documents the user wants DailyOS to index as professional context: product decks, case studies, playbooks, competitive battlecards, internal training materials. Files are written to `_user/docs/` in the workspace and processed via the existing file processor pipeline, indexed under a `user_context` collection. A list of attached files shows filename, date added, and a delete affordance.

This section is shown for all presets.

### 3. The two-layer priority model

Annual and quarterly priorities are structurally different, not just labelled differently. They have different scopes, different entity linking targets, and different downstream effects. **Neither layer expires automatically.** Both persist until the user removes or replaces items. This is a deliberate design choice rooted in the zero-guilt principle: requiring users to refresh priorities on a schedule creates exactly the maintenance burden that makes productivity tools feel like jobs. Stale priorities are still useful context; the system uses whatever is there and makes no demands about when it was last updated.

**Annual priorities — year-level bets**

- Scope: annual. Three to five items representing what the user is trying to accomplish this year. Examples: "Expand to enterprise tier," "Hit 95% net retention," "Land three new strategic logos."
- Structure: named items with optional entity links. Each item can be linked to an account, project, or person. The entity chip is displayed next to the priority text when a link exists.
- Storage: `user_entity.annual_priorities` as a JSON array of objects: `{id, text, linked_entity_id?, linked_entity_type?, created_at}`.
- Downstream effect: annual priorities inform signal scoring weights globally. When a priority is linked to an account entity, signals from that account domain receive a user-relevance multiplier in the scoring cycle (I414).

**Quarterly priorities — current quarter focus**

- Scope: quarterly. More specific than annual. These are the things the user is actively working on right now — the quarter-level execution against annual goals. Examples: "Expand Cox to two new BUs before Q2," "Close Jefferies before April 30."
- Structure: named items with optional links to actions, meetings, or people.
- Storage: `user_entity.quarterly_priorities` as a JSON array of objects: `{id, text, linked_to_id?, linked_to_type?, created_at}`.
- Downstream effect: quarterly priorities appear directly in the daily briefing's focus directive. They represent the briefing's understanding of what the user is actively moving forward.

The UI renders both layers in the same section, clearly visually distinct. Annual is framed as "This Year" with an entity chip affordance. Quarterly is framed as "This Quarter." No reset action, no expiration indicator, no maintenance prompts of any kind.

### 4. Context entries as a first-class input type

Context entries address a gap in the current entity model: there is no place for the user's own professional knowledge and judgment — knowledge that informs multiple entities but belongs to none of them specifically.

**The distinction that matters.** Account notes document interactions: "Met with Jane on Tuesday — she raised concerns about the migration timeline." That belongs on the Cox account entity. Context entries document professional knowledge: "How I think about migration risk conversations with technical evaluators — the key is separating implementation risk (our responsibility) from adoption risk (theirs)." That applies to every account where technical evaluation is in play, not just Cox. Putting it on Cox buries it. Putting it on the user entity makes it available everywhere semantically relevant.

**Data model.** A `user_context_entries` table: `id` (TEXT PRIMARY KEY), `title` (TEXT NOT NULL), `content` (TEXT NOT NULL), `embedding_id` (TEXT, FK to content_embeddings), `created_at`, `updated_at`.

**Embedding pipeline.** When a context entry is created or updated, the entry body is submitted to the background embedding processor (task #9) using the `user_context` collection label. The same mechanism used to embed inbox files. The UI does not wait for the embedding to complete — save is immediate, embedding is background.

**Retrieval.** During entity enrichment, the enrichment prompt builder calls `search_user_context(entity_name, domain, limit: 2)`. The top-2 entries above a similarity threshold are included in the user context block of the enrichment prompt as prose. If no entries meet the threshold, the user context block simply omits the context entries section — no empty headers.

**What context entries are not.** They are not entity notes. They are not playbook sections (those are structured by role preset and named). They are not attachments (those are documents). Context entries are the user's own distilled professional knowledge: how they think, what they've learned, what they believe works. They are more granular than playbooks and more durable than notes.

### 5. Role presets shape section prominence, not section existence

All six sections exist for all role presets. The preset determines:
- Which sections are featured (expanded by default vs. requiring an expand gesture)
- Vocabulary within sections (field labels and placeholder text that are role-specific)
- Named playbook sections (CS has three named sections; Sales has three different ones; other presets have one generic section in v0.14.0)

Prominence table for v0.14.0 (CS-full implementation; other presets use generic structure until v0.14.1):

| Section | CS | Sales | Partnerships | Product | Marketing | Leadership |
|---|---|---|---|---|---|---|
| About Me | Featured | Featured | Featured | Featured | Featured | Featured |
| What I Deliver | Featured | Featured | Featured | Featured | Featured | Shown |
| Pricing Model (within What I Deliver) | Shown | Primary | Partner tiers | Hidden | Hidden | Optional |
| Common Objections (within What I Deliver) | Featured | Featured | Shown | Hidden | Hidden | Hidden |
| My Priorities (both layers) | Featured | Featured | Featured | Featured | Featured | Featured |
| CS-specific playbooks | Featured | Hidden | Hidden | Hidden | Hidden | Hidden |
| Sales-specific playbooks | Hidden | Featured | Hidden | Hidden | Hidden | Hidden |
| Methodology (generic) | All | All | All | Featured | All | All |
| Context Entries | Shown | Shown | Shown | Shown | Shown | Shown |
| Attachments | Shown | Shown | Shown | Shown | Shown | Shown |

"Featured" = expanded by default, first field immediately visible.
"Shown" = section header visible, content collapsed, expand affordance present.
"Hidden" = section exists in the data model but is not rendered in the UI for this preset.
"Primary" = featured and visually emphasised above other sub-fields in the section.

### 6. v0.14.0 is CS-first; v0.14.1 expands to all presets holistically

The data model and page component are preset-agnostic from the start. No hardcoded CS assumptions in the DB schema or the Rust commands. Preset-specific behaviour is injected via the preset configuration.

In v0.14.0, the CS-specific vocabulary, placeholder text, and playbook labels are the only preset implemented fully. Users on other presets see the generic structure: all sections available, generic vocabulary, one "My Methodology" playbook section.

v0.14.1 is exclusively expansion work: writing role-specific vocabulary, placeholder text, and named playbook labels for all nine presets defined in ADR-0079. This is deliberate sequencing — get the architecture and interaction model right on one role before expanding. The expansion is vocabulary authoring work, not architecture work.

---

## Consequences

**Settings becomes leaner.** The Settings page loses the identity and professional context sections, retaining only technical integration configuration. This is the correct separation: Settings is infrastructure, `/me` is professional context.

**Navigation gains one item.** The primary sidebar gains a "Me" nav item. Existing nav items are unaffected in position and behaviour.

**No intel_queue entry for the user entity.** The user entity page does not generate AI content. There is no enrichment cycle, no loading state for generated intelligence, no re-enrich action. The page is entirely authored content. This is a structural characteristic, not a limitation.

**Context entries create a new indexing path.** The `user_context` collection label is a new content type in the embedding pipeline. No new infrastructure — the pipeline already handles multiple collection labels. The retrieval function (`search_user_context`) is a thin wrapper around the existing cosine similarity search.

**The playbooks section has an unusual consumption model.** Playbook content is authored as natural language prose but consumed by the AI via semantic embedding — not by reading the structured fields. The embedding is the primary interface. This means poorly-written or vague playbook text produces poor retrieval results. Placeholder text and section descriptions must guide users toward writing that embeds well: specific, scenario-grounded, first-person.

**The two-layer priority model requires no housekeeping.** Annual and quarterly priorities persist until explicitly removed — there is no automatic expiration, no reset action, no `week_of` field. This is consistent with the zero-guilt design principle: the system works with whatever context the user has provided and makes no demands about when it was last updated.

**The success plan report (v0.15.0) depends on this ADR being fully implemented.** The user entity page is where the user populates the second side of the success plan intersection (what I deliver + what the account wants = success plan). Without a well-populated user entity, success plans are structurally correct but professionally hollow.
