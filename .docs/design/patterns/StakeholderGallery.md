# StakeholderGallery

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-10
**`data-ds-name`:** `StakeholderGallery`
**`data-ds-spec`:** `patterns/StakeholderGallery.md`
**Variants:** card / chip / depth-strip; engagement state via `Pill`-shaped badges; multi-role badges via `data-source` taxonomy
**Design system version introduced:** 0.4.0

## Job

Render the cast of people associated with an entity (account, project) — the people the user is working with, what each person does, how engaged they are, and what's known about the working relationship. The gallery answers "who's in the room?" for a given dossier.

Three composable shapes:
1. **card grid** — 2-column expanded cards with avatar, name, title, multi-role badges, assessment, engagement state, last-seen
2. **team chips** — compact rounded chips for "your team on this account"
3. **relationship-depth strip** — full-width grid of relationship-quality cells (used at the bottom of the gallery for at-a-glance relationship health)

## When to use it

- On Account Detail's "The room" / stakeholders chapter
- On Project Detail's "The room" chapter
- On Person Detail's network / relationships sub-section (chips form)
- Anywhere a curated cast of stakeholders needs editorial treatment

## When NOT to use it

- For raw contact lists or directories — use entity list shells
- For pending stakeholder suggestions awaiting review — use `PendingStakeholderQueue`
- For a single featured stakeholder — use the entity's hero or `EntityChip` inline

## States / variants

- **card** — full editorial card with avatar, name, title line, multi-role badges, assessment, engagement state, last-seen
- **chip (team)** — compact rounded pill for sidebar / sub-section listing
- **depth strip** — full-width relationship-depth visualization (cells with mono labels + serif values + tone-coded depth badges)
- **suggested** — overlay variant for stakeholders proposed by intelligence (accept / dismiss affordances on hover)
- **pending assessment** — saffron-tinted blockquote when an assessment hasn't been verified
- **engagement state** — rendered via the existing `Pill`-shaped engagement badge variants:
  - `engagementActive` (sage tone — strong)
  - `engagementWarm` (turmeric tone — engaged)
  - `engagementCooling` (terracotta tone — chasing)
  - `engagementNew` (larkspur tone — sparse signal)
  - `engagementStrongAdvocate`, `engagementEngaged`, `engagementNeutral`, `engagementDisengaged`, `engagementUnknown` (semantic relationship variants)

## Composition

Composes:
- `Avatar` (or avatar ring) — primitive
- `EntityChip` — for organization affiliation in title line
- `EditableText` — for inline editable name / role / assessment
- `Pill` — semantically the engagement badge follows Pill's tone vocabulary; the existing `engagement*` classes ARE Pill compositions in everything but name. Engagement state is NOT rendered as a 3-dot intensity indicator; the existing badge text label is sufficient (per chrome-overlap audit 2026-05-10)
- `RemovableChip` — for multi-role badges with hover-revealed remove
- `EnrichmentTag` — small mono provenance indicator (`data-source="clay | gravatar | glean | google"`)

## Tokens consumed

- `--font-mono`, `--font-sans`, `--font-serif`
- `--color-garden-larkspur*`, `--color-spice-turmeric*`, `--color-garden-sage*`, `--color-spice-terracotta*` (per-tone engagement / relationship coloring)
- `--color-text-primary | secondary | tertiary`
- `--color-rule-light | heavy`, `--color-paper-warm-white | cream`, `--color-desk-charcoal*`
- `--space-xs | sm | md | lg | xl`, `--radius-sm`, `--radius-editorial-sm`, `--shadow-lg`

## API sketch

```html
<section class="StakeholderGallery_section" data-ds-name="StakeholderGallery">
  <div class="StakeholderGallery_grid">
    <article class="StakeholderGallery_card">
      <div class="StakeholderGallery_cardHeader">
        <span class="StakeholderGallery_avatarRing">…avatar…</span>
        <a href="…" class="StakeholderGallery_nameLink">James Park</a>
        <span class="StakeholderGallery_engagementBadge StakeholderGallery_engagementActive">Strong</span>
      </div>
      <p class="StakeholderGallery_titleLine">TPM · Internal · Lead</p>
      <div class="StakeholderGallery_roleBadges">
        <span class="StakeholderGallery_roleBadge" data-source="user">Project lead</span>
        <span class="StakeholderGallery_roleBadge" data-source="user">Decision maker</span>
      </div>
      <p class="StakeholderGallery_assessment">
        Accountable end-to-end. Calm, allergic to status theater.
      </p>
      <div class="StakeholderGallery_lastSeen">Last seen: Apr 23, project standup</div>
    </article>
    <!-- … more cards … -->
  </div>
</section>
```

React form:

```tsx
<StakeholderGallery
  stakeholders={stakeholders}
  layout="card-grid"
  onAccept={handleAcceptSuggestion}
  onDismiss={handleDismissSuggestion}
/>
```

## Source

- **Code:** `src/components/entity/StakeholderGallery.tsx`
- **Reference CSS:** `.docs/design/reference/_shared/styles/StakeholderGallery.module.css`
- **Mockup origin:** Account/project editorial mockups (multiple iterations); `.docs/design/figma/mockups/project-detail/variations/D-composite.html` (`.stake-card`, `.stake-roles`, `.stake-assessment`, `.stake-engagement`, `.stake-last-seen`)

## Surfaces that consume it

- AccountDetail (`/accounts/$accountId`)
- ProjectDetail (`/projects/$projectId`)
- PersonDetail (`/people/$personId`) — chips/network variants

## Naming notes

`StakeholderGallery` — "gallery" because the card grid is editorial and curated, not a directory. Engagement state is rendered through the existing badge variants (which are `Pill`-shaped tone-coded labels); the variation D mockup proposed a new 3-dot intensity indicator next to the text label, but the chrome-overlap audit 2026-05-10 rejected it as redundant signaling. The text label alone carries the meaning; adding dots duplicates the information without adding signal.

## History

- 2026-05-10 — Promoted to canonical pattern spec. Engagement-dots-vs-text decision: text wins (Pill-style badge variants are sufficient). Suggestion flow + pending-assessment + multi-role badges + relationship-depth strip + champion designation are all accepted as part of the integrated pattern.
- pre-2026-05-10 — Component shipped at `src/components/entity/StakeholderGallery.tsx` with no canonical spec. Reference CSS at `.docs/design/reference/_shared/styles/StakeholderGallery.module.css` covered the visual contract.
