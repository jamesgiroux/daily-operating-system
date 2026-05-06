# MovingRow

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `MovingRow`
**`data-ds-spec`:** `patterns/MovingRow.md`
**Variants:** `kind="customer" | "person" | "project" | "internal" | "lifecycle"`
**Design system version introduced:** 0.6.0

## Job

Render one entity's "what's moving" row in the Daily Briefing Moving section. Three-column grid: entity identity (left), narrative + signal feed (center), stacked provenance stats (right). The pattern enforces restraint — ≤3 entities per Moving section, lede ≤2 sentences — so the user can scan the day's substantive movement at a glance instead of triaging an inbox.

## When to use it

- Inside the Daily Briefing Moving section (DOS-413 contract)
- When the surface needs to show "what changed for this entity, with supporting signals" as a single editorial unit
- When the entity is identifiable by short name + state pill (account, person, project, internal-engagement, lifecycle transition)

## When NOT to use it

- For an action triage row — that's `WatchRow`
- For an editorial meeting list entry — that's `BriefingMeetingCard` or `MeetingSpineItem`
- For a generic claim or finding row — that's `ClaimRow` or `EntityRow`
- When the lede would exceed 2 sentences — restructure or fold the entity into another row's signal feed

## States / variants

`kind` selects the left-column accent bar color and state pill tone:

- `customer` — turmeric accent
- `person` — larkspur accent
- `project` — olive accent
- `internal` — neutral
- `lifecycle` — saffron (lifecycle transitions render as their own row when not folded into another entity's signal feed)

Accent implemented via `::before` pseudo on the row, color from `--moving-accent` CSS variable set by `data-kind`.

States:

- **Default** — full row.
- **Hover** — border-color shift, no transform (matches editorial-row hover convention).
- **Focus** — focus ring on the row wrapper (the row is the link).
- **Loading / error / empty** — handled by parent `BriefingLoadState`; never per-row.

## Composition

Composes:

- `Pill` — the `statePill` in compact size with the entity's tone
- `SignalDot` — each `signals[]` item, kind-tinted, with optional `threadAction`
- `ProvenanceStat` — each `provenanceStats[]` item in the right column
- `EntityChip` — optional, for secondary entity references inside the lede or signal `whatSegments`

Click target: the whole row is the link. Wrapped in a `<div>` with `role="link"` + `tabindex="0"` + click handler navigating to `href`. **Not** a wrapping `<a>` — that would invalidate any nested interactive element. Inside the row, `MovingSignalViewModel.threadAction` is a separate `<button>` that stops event propagation; clicking the button performs the thread action without triggering the row navigation.

Grid: `minmax(120px, 0.2fr) minmax(0, 1fr) minmax(120px, auto)`.

## Tokens consumed

- `--color-account-turmeric`, `--color-person`, `--color-project-olive`, `--color-spice-saffron` — accent bar via `--moving-accent` (one per kind)
- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary` — name / lede / state text
- `--color-border-subtle` — row border (default)
- `--color-border-strong` — row border (hover)
- `--font-serif` — entity name
- `--font-sans` — lede + state pill
- `--space-md`, `--space-lg` — column gap and row padding

## API sketch

```tsx
<MovingRow
  kind="customer"
  entity={{ id: "acct_1", name: "Globex", href: "/accounts/acct_1" }}
  href="/accounts/acct_1"
  statePill={{ label: "Renewal ↑", tone: "turmeric" }}
  lede="Pricing memo went out Tuesday. Legal flagged 3 MSA clauses; champion still on track."
  signals={[
    { kind: "meeting", when: "10:00", whatSegments: [{ text: "Pricing alignment — in progress" }], urgency: "normal" },
    { kind: "action", when: "2d", whatSegments: [{ text: "Send pricing memo — overdue" }], urgency: "overdue" },
  ]}
  provenanceStats={[
    { label: "Health", value: "71 +3", trend: "up" },
    { label: "Stage", value: "Renewal" },
    { label: "Conf.", value: "82%", trend: "up" },
    { label: "Owner", value: "You" },
  ]}
/>
```

Contract type:

```ts
interface MovingEntityViewModel {
  kind: MovingEntityKind;
  entity: LinkedEntity;
  href: string;
  statePill: { label: string; tone: PillTone };
  lede: string;                      // 1-2 sentences
  signals: MovingSignalViewModel[];  // 3-5
  provenanceStats: ProvenanceStat[];
}
```

The view does not sort or filter — service emits ordered, capped lists. The view does not compose the lede or signal `whatSegments` — service emits typed segments.

## Source

- **Code:** ships W1 (DOS-423) at `src/components/dashboard/MovingRow.tsx` + `src/components/dashboard/MovingRow.module.css`
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign.html` (Moving section)

## Surfaces that consume it

- DailyBriefing (Moving section)

## Naming notes

`MovingRow` is the canonical name. Not `BriefingMovingRow` — the pattern is reusable in principle by any surface that needs the same restraint contract. Follows `NAMING.md` rule: patterns named for the pattern, not the surface. The "Moving" register comes from the contract section name (DOS-413), which is the user-visible job — what's moving on this entity today.

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W1 under DOS-423. Reference HTML uses provisional `.dspine-moving-row` class until W1 cutover to `MovingRow_*` scoped names.
