# EntityPortraitCard

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `EntityPortraitCard`
**`data-ds-spec`:** `patterns/EntityPortraitCard.md`
**Variants:** `entityType` (account | project | person); `state` per-entity (renewing, at_risk, etc.)
**Design system version introduced:** 0.1.0

## Job

A magazine-style entity portrait — a single card that renders an entity's current state at a glance: color-banded aside (state + name + key facts) on the left, main column with a giant italic "quote-mark" glyph + serif lede + threaded events on the right. Used in DailyBriefing's "What's Moving" section.

## When to use it

- DailyBriefing's "Moving" section (canonical) — one card per entity that shifted overnight
- Potential extension: weekly recap surfaces (entities that moved this week), search results that highlight entity context

## When NOT to use it

- Account / project / person detail surfaces — those have their own hero pattern (`AccountHero`, `ProjectHero`, `PersonHero`)
- Compact entity rows — use `EntityChip` or list patterns

## Composition

CSS grid 200px aside | 1fr main; min-height 220px.

**Aside (left, color-banded by entity)**:
- 6px top color band per entity tint
- State label (mono uppercase 10px): "Renewing ↑", "At Risk ↓", "Direct Report"
- Entity name (serif 30px, weight 500, letter-spacing -0.02em)
- Foot stats (mono 10px, two columns): Health / Stage / Confidence / Owner / Last touch / Tenure / Mtgs moved (per entity type)
- Stat values use direction colors: ↑ rosemary, ↓ terracotta

**Main (right)**:
- Giant italic serif glyph in top-right corner (110px, 10% opacity) — entity initial or `"` mark
- Lede (serif 18px, weight 400) — single paragraph: what's happening, why it matters
- Thread list — typed-dot timeline:
  - Dot color by event type: meeting (turmeric) / action (terracotta) / mail (larkspur) / lifecycle (rosemary)
  - Time column (mono 10px) — "10:00", "2d", "Today", "Overnight", "New", "3h ago"
  - What text (serif 14.5px) with `<em>` italic for sub-clauses
  - Optional `ThreadMark` for "talk about this"
- Background: subtle entity-tinted gradient

## Variants

- **`account`** — turmeric band/glyph (or terracotta if at-risk)
- **`project`** — olive
- **`person`** — larkspur
- Per-entity state can override aside accent color (an at-risk account uses terracotta even though account = turmeric)

## Tokens consumed

- Entity tints: `--color-spice-turmeric`, `--color-garden-olive`, `--color-garden-larkspur` (and -7, -8, -12 tints for backgrounds)
- `--color-spice-terracotta` (at-risk override)
- `--color-paper-warm-white` (card background)
- `--color-rule-medium`, `--color-rule-light` (borders)
- `--font-serif` (name, lede, glyph, thread items), `--font-mono` (state, foot stats, time)
- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary`

## API sketch

```tsx
<EntityPortraitCard
  entityType="account"
  state={{ label: "Renewing ↑", overrideColor: "turmeric" }}
  name="Acme Corp"
  glyph="“"
  asideStats={[
    { label: "Health", value: "71 ↑ 3", direction: "up" },
    { label: "Stage", value: "Renewal" },
    { label: "Confidence", value: "82%" },
    { label: "Owner", value: "You" },
  ]}
  lede="Legal review is starting. The adjusted pricing memo goes to Jen in two hours — everything else for Acme today chains to that one send."
  thread={[
    { type: "meeting", when: "10:00", what: "Pricing alignment <em>— in progress</em>", showThreadMark: true },
    { type: "action", when: "2d", what: "Send adjusted pricing memo to Jen <em>— overdue</em>", overdue: true, showThreadMark: true },
    // ...
  ]}
/>
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` (`.acc-card`, `.acc-aside`, `.acc-main`, `.acc-thread`, `.ent-acme`, `.ent-northwind`, `.ent-priya`)
- **Code:** to be implemented in `src/components/dashboard/EntityPortraitCard.tsx`

## Surfaces that consume it

DailyBriefing's Moving section (canonical).

## Naming notes

`EntityPortraitCard` — emphasizes the editorial portrait quality. Don't rename to `EntityCard` (too generic), `MovingCard` (ties to surface), or `AccountCard` (entity-specific in a multi-entity pattern).

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from D-spine mockup.
