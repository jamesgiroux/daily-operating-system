# EntityChip

**Tier:** primitive
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-05
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

- `entityType="account"` — account token (`--color-account`)
- `entityType="project"` — project token (`--color-project`)
- `entityType="person"` — person token (`--color-person`)
- Optional: `compact` for tight inline use; `removable` for X affordance; `editable` for click-to-edit via EntityPicker

## Composition

Composes `Pill` (visual base) + leading icon (lucide: Building2 / FolderKanban / User per entity type) + entity name label.

## Tokens consumed

- `--color-account`, `--color-project`, `--color-person` (text color per type)
- `--color-account-8`, `--color-project-8`, `--color-person-8` (background tints per type)
- `--font-mono` (label)
- `--space-xs`, `--space-sm`

## API sketch

```tsx
<EntityChip entityType="account" entityName="Acme Corp" />
<EntityChip entityType="person" entityName="Priya Raman" compact />
<EntityChip entityType="project" entityName="Q2 Launch" removable onRemove={handleRemove} />
```

## Source

- **Code:** shipped in `src/components/ui/EntityChip.tsx`.
- **Consumers:** `src/components/ui/meeting-entity-chips.tsx` and `src/components/ui/email-entity-chip.tsx` compose the shared primitive.

## Surfaces that consume it

DailyBriefing (Daily Briefing redesign `.entity-chip`), AccountDetail (related entities), MeetingDetail (attendees / linked entities), email/meeting workflows.

## Naming notes

Daily Briefing redesign mockup uses class `.entity-chip`. Production has `EntityChip.tsx`, with meeting and email wrappers composing it for workflow-specific behavior.

## History

- 2026-05-02 — Promoted to canonical. DOS-357 reconciled the entity-type color mapping.
- 2026-05-05 — Source updated to shipped React primitive and current `--color-account/project/person` tokens.
