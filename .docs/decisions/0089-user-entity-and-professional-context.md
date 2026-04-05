# ADR-0089: User Entity and Professional Context Layer

**Date:** 2026-02-22
**Status:** Accepted
**Target:** v0.14.0
**Complements:** ADR-0079 (Role Presets) — role presets govern vocabulary and field structure; this ADR governs professional context and intelligence framing. They are not alternatives.
**Complements:** ADR-0086 (Intelligence as Shared Service) — user context becomes a shared input to every entity enrichment prompt, alongside entity signals and meeting history.

---

## Context

DailyOS has deep intelligence about the world the user operates in: accounts, people, projects, signals, meetings. It knows almost nothing about the user's own professional context — what they sell or deliver, what their organisation's value proposition is, what they consider success for their customers, what plays they rely on, what they're trying to accomplish this quarter.

This asymmetry creates a systematic framing failure. An account signalling "we need to reduce operational costs" is surfaced as a risk (account under pressure). But if DailyOS knew that the user's platform has a compelling automation cost-reduction narrative, the correct framing is an opportunity (account expressing a pain point that aligns directly with what the user delivers). The risk framing is not wrong — it's incomplete. The missing ingredient is the user's own professional lens.

The same problem appears in every report type introduced in v0.15.0:

- **EBR/QBR**: The "Value Delivered" section requires knowing what the user considers value — not just what the account experienced, but what the user's organisation delivered and why it matters. Without this, the section produces generic outcome summaries that any vendor could have written.
- **Success Plan**: A success plan is the intersection of *what the account wants* and *what the user can deliver*. DailyOS has the first side from entity intelligence. The second side does not exist.
- **Account Health Review**: Stakeholder framing ("the champion is engaged") is generic without knowing what the user needs from that champion. Context changes what "engaged" means.

**The role preset (ADR-0079) does not solve this.** Role presets shape vocabulary — "churn risk" vs "deal stalled" vs "scope creep." They do not capture what makes the user's product or service valuable, what their customers are trying to achieve, or what the user themselves is trying to accomplish. A CSM using the Customer Success preset and a CSM at a different company using the same preset have entirely different professional contexts that the vocabulary cannot express.

**The existing `user_focus` field does not solve this.** It is a single freeform string ("What's important right now") — useful for the daily briefing's focus directive, but not structured enough to inform intelligence framing, signal prioritisation, or report generation.

---

## Decisions

### 1. The user is a first-class entity, not just configuration

The user exists in DailyOS as an entity — not merely as a config struct with name, domain, and personality. This entity has:

- **Professional context** (static): what they sell or deliver, their organisation's value proposition, what success looks like for their customers, their current priorities, their product/service category
- **Dynamic context** (attachable): documents they add (product decks, playbooks, case studies, internal training notes), meeting transcripts about their own work and methodology
- **Generated intelligence** (computed): AI-synthesised understanding of the user's professional strengths, typical go-to plays, context that should inform how entity intelligence is framed

The user entity lives in a dedicated location in the workspace (e.g., `_user/`) and in the DB, following the same three-file pattern as other entities. It has a settings surface in YouCard that functions more like an entity detail page than a config form.

### 2. Two layers of user context with different update cadences

**Layer 1 — Declared context (user-maintained)**

Structured fields the user explicitly maintains. These are not inferred — the user tells DailyOS what is true:

| Field | What it captures | Example |
|-------|-----------------|---------|
| `value_proposition` | What your product/service does for customers, in plain language | "We help media companies publish faster and scale their digital audience without infrastructure headaches." |
| `success_definition` | What winning looks like for your customers — the outcomes they measure | "Customers succeed when time-to-publish drops, uptime is above 99.9%, and their editorial team can work independently." |
| `current_priorities` | What you're focused on this quarter or period | "Expand the Crestview Media account to two new business units. Close Jefferson Advisors before Q2." |
| `product_context` | The category, key differentiators, common objections | "Enterprise WordPress hosting. Differentiators: managed infrastructure, editorial workflow tools, compliance. Common objection: cost vs. open-source self-hosting." |
| `playbooks` | How you approach recurring situations | "At-risk renewals: exec sponsor call + cost reduction data + expansion carrot." |

**Layer 2 — Attached context (documents and transcripts)**

Files the user can attach that DailyOS ingests and indexes for context:
- Product overview decks (PDF/Markdown)
- Internal playbooks and methodology documents
- Case studies and reference customer narratives
- Transcripts of training sessions or internal product calls

Attached documents are processed by the existing file processor (processor/) and embedded via nomic-embed-text — the same pipeline already used for inbox files. They become searchable context that enriches intelligence prompts.

### 3. User context feeds into every entity intelligence prompt

When the intel_queue enriches an entity (account, project, person), the enrichment prompt includes a user context block assembled from Layer 1 declared context:

```
## User Context
You are generating intelligence for a user with the following professional context.
Frame your analysis through this lens — surface opportunities where the account's
situation aligns with what the user delivers, not just risks.

Value proposition: {user.value_proposition}
Success definition: {user.success_definition}
Current priorities: {user.current_priorities}
```

This is a mechanical addition to every enrichment prompt — no new AI call, no new pipeline. It changes what the existing entity intelligence says, not how it's generated.

**The risk → opportunity reframe:** With user context, a cost-pressure signal at an account is no longer only a risk. The enrichment prompt, knowing the user has a cost-reduction story, will frame it as: "Account is expressing cost pressure — this aligns with your automation efficiency narrative. Consider surfacing the operational savings data from the Acme case study."

### 4. User-context-weighted signal scoring

Signals are currently scored by confidence and temporal decay. User context adds a third dimension: **relevance to the user's stated priorities**.

When `current_priorities` contains "expand Crestview Media account", signals from Crestview Media-domain entities get a relevance multiplier. When `value_proposition` mentions security compliance, security-related signals from accounts rank higher in the briefing's attention section.

This is an extension of the email relevance scoring mechanism (I395) applied to entity signals. The user context produces a priority weight vector — a set of topics or entities that are currently high-salience — and that vector is dotted against each signal's topic embedding to produce a user-relevance score.

### 5. The success plan as the formalization of the intersection

A success plan is the document-level expression of: *what does this account want* ∩ *what can the user deliver* ∩ *how do we measure it*. With the user entity:

- *What the account wants* — from entity intelligence (`success_metrics`, `open_commitments`, `health_trend`)
- *What the user delivers* — from `user.value_proposition` and `user.success_definition`
- *How we measure it* — from `entity_intel.success_metrics` + `user.success_definition`

The success plan report type (v0.15.0) becomes genuinely differentiating because both sides of the intersection are machine-readable.

### 6. Relationship to role preset (ADR-0079)

Role preset and user entity serve different purposes and are additive:

| Dimension | Role Preset (ADR-0079) | User Entity (this ADR) |
|-----------|----------------------|----------------------|
| What it governs | Vocabulary, field structure, entity mode | Professional context, framing, value narrative |
| Changes with | Switching roles | Changing jobs, product focus, quarterly priorities |
| Example | "I'm a CSM — show me renewal fields, use churn-risk vocabulary" | "I sell enterprise WordPress hosting to media companies — frame cost signals as opportunities" |
| Stability | Semi-permanent (role doesn't change often) | Dynamic (priorities change quarterly; product evolves) |
| Affects | UI fields, AI prompt vocabulary | Intelligence framing, signal prioritisation, report content |

A user can switch from the Customer Success preset to the Partnerships preset while their declared value proposition, product context, and current priorities remain unchanged. The preset shapes how fields are labelled and how the AI uses certain words. The user entity shapes what the AI knows about the user and their work.

### 7. User entity is owned entirely by the user

The user entity is personal professional context. It is never shared, never exposed via the MCP server to external tools, and never included in any outbound integration. The MCP sidecar's four read-only tools (`get_briefing`, `query_entity`, `list_entities`, `search_meetings`) do not expose user entity fields — they are for workspace intelligence, not the user's own professional context.

---

## Consequences

- The YouCard settings surface evolves from a 4-field config form into a richer professional context card. Users who don't fill it in get the current behavior (no user context in prompts). Users who do fill it in get meaningfully different intelligence framing.
- Every entity enrichment prompt grows by ~150–300 tokens for the user context block. At entity enrichment frequency (not per-view), this is a marginal cost increase.
- The file processor (processor/) already handles document ingestion and embedding. Attaching documents to the user entity reuses this pipeline without modification.
- Signal scoring changes to incorporate user-relevance. The existing scoring function (email_scoring.rs, relevance.rs) is extended — not replaced.
- The success plan report type (v0.15.0) is only possible because of this ADR. Without the user entity, a success plan produced by DailyOS is structurally correct but professionally hollow.
- Users who update `current_priorities` should see their briefing attention section shift within one signal scoring cycle. This creates observable, personal responsiveness — the app demonstrably knows what matters to the user right now.
