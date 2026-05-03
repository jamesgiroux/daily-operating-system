# EntityChip

**Tier:** primitive
**Status:** canonical
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `EntityChip`
**`data-ds-spec`:** `primitives/EntityChip.md`
**Variants:** `entityType="account" | "project" | "person"`
**Design system version introduced:** 0.1.0

## Job

Render an inline reference to an entity (account / project / person), color-coded by entity type per the entity color aliases. Composes `Pill` underneath; carries entity name, optional icon, optional remove affordance.

## When to use it

- Inline references to entities within rendered content (meeting attendees, briefing items, account references in prose, email-to-entity chips, meeting-to-entity chips)
- When the user benefits from quick visual recognition of entity type via color
- The two existing implementations are `meeting-entity-chips.tsx` and `email-entity-chip.tsx`

## When NOT to use it

- For status / state labels — use `Pill` with appropriate tone
- For account-type categorical (Customer / Internal / Partner) — use `TypeBadge`
- For removable tags without entity meaning — use `Chip` (Wave 3)

## States / variants

- `entityType="account"` — turmeric (uses `--color-entity-account`)
- `entityType="project"` — olive (uses `--color-entity-project`)
- `entityType="person"` — larkspur (uses `--color-entity-person`)
- Optional: `compact` for tight inline use; `removable` for X affordance; `editable` for click-to-edit via EntityPicker

## Composition

Composes `Pill` (visual base) + leading icon (lucide: Building2 / FolderKanban / User per entity type) + entity name label.

## Tokens consumed

- `--color-entity-account`, `--color-entity-project`, `--color-entity-person` (text color per type)
- `--color-spice-turmeric-8`, `--color-garden-olive-8`, `--color-garden-larkspur-8` (background tints per type)
- `--font-mono` (label)
- `--space-xs`, `--space-sm`

## API sketch

```tsx
<EntityChip entityType="account" entityName="Acme Corp" />
<EntityChip entityType="person" entityName="Priya Raman" compact />
<EntityChip entityType="project" entityName="Q2 Launch" removable onRemove={handleRemove} />
```

## Source

- **Code:** `src/components/ui/meeting-entity-chips.tsx`, `src/components/ui/email-entity-chip.tsx` — current consumers using the `entityColor` / `entityBg` mapping; reconciled to entity tokens via DOS-357.
- **Future consolidation:** extract to a dedicated `src/components/ui/EntityChip.tsx` primitive consumed by both (Wave 1 follow-on).

## Surfaces that consume it

DailyBriefing (D-spine `.entity-chip`), AccountDetail (related entities), MeetingDetail (attendees / linked entities), email/meeting workflows.

## Naming notes

D-spine mockup uses class `.entity-chip`. Production has `meeting-entity-chips.tsx` and `email-entity-chip.tsx` — DOS-357 already consolidated their explicit type-to-color mapping to use entity tokens. Wave 1 extracts the shared primitive.

## History

- 2026-05-02 — Promoted to canonical. DOS-357 reconciled the entity-type color mapping (also fixed `--color-sky-larkspur` typo → `--color-entity-person`).
