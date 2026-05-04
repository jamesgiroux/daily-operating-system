# MeetingSpineItem

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `MeetingSpineItem`
**`data-ds-spec`:** `patterns/MeetingSpineItem.md`
**Variants:** state (`past | in_progress | upcoming | cancelled`); type (`customer | internal | one_on_one`); modifier (`warn`)
**Design system version introduced:** 0.1.0

## Job

Render a single meeting as a magazine-article entry in DailyBriefing's Today section: time on the left, body on the right with entity eyebrow, serif title, italic context line, and a footer with attendees + prep state + briefing action. State-aware (past / now / upcoming / cancelled).

## When to use it

- DailyBriefing's Today section (canonical home)
- Potential extension: any surface that lists meetings with full editorial context (Week view, search results)

## When NOT to use it

- Compact meeting rows (use a `MeetingRow` primitive — `src/components/shared/MeetingRow.tsx` already exists for this)
- Calendar tile rendering (use `DayChart` bars)
- Meeting recap surface (use `MeetingDetail` patterns)

## Composition

Two-column grid: 88px time column | 1fr body.

**Time column** (right-aligned):
- Time (mono 18px)
- Duration (mono 10px uppercase, e.g., "45M" or "30M · ENDED")
- Optional state tag (mono 9.5px uppercase): `Now` (terracotta), `Up next` (rosemary)

**Body**:
- Entity eyebrow (mono uppercase 10px) — colored glyph dot + entity name + horizontal rule. Color per type: customer=turmeric (or terracotta if `warn`), internal=tertiary, one_on_one=larkspur
- Title (serif 26px, weight 400) — links to meeting detail; on hover: turmeric
- Context (serif italic 16px) — the "what this is actually about" line, optional
- Footer (mono 11px) — attendees · prep state pill · briefing link OR create button (per state)

## Variants

- **`past`** — opacity 0.5; smaller title (22px)
- **`in_progress`** — terracotta-tinted background gradient; time in terracotta
- **`upcoming`** — default
- **`cancelled`** — opacity 0.4; title strikethrough
- **`customer.warn`** — entity eyebrow shifts to terracotta (signals risk on this account)

Footer changes per prep state:
- `ready` (sage) — "Notes captured" / "Briefing fresh" + briefing link
- `building` (saffron) — "Briefing building" + briefing link
- `needs` (terracotta) — "No briefing yet" + Create button

## Tokens consumed

- `--font-mono` (time, duration, eyebrow, footer)
- `--font-serif` (title), `--font-serif italic` (context)
- `--color-spice-turmeric` (customer eyebrow), `--color-spice-terracotta` (warn, in-progress, NOW), `--color-garden-larkspur` (one_on_one), `--color-text-tertiary` (internal)
- `--color-garden-rosemary` (Up next, ready prep state)
- `--color-spice-saffron` (building prep state)
- `--color-rule-light` (item bottom border)
- `--space-md`, `--space-xl` (gaps, padding)

## API sketch

```tsx
<MeetingSpineItem
  time="10:00"
  duration="45m"
  state="in_progress"
  type="customer"
  entityName="Acme Corp · Renewal"
  title="Acme renewal — pricing & tier 3"
  context="Decisions on tier 3 pricing. Jen wants the adjusted memo before legal goes deep on MSA — it still hasn't gone, and that's the blocker for everything else today."
  attendees="Jen Park, Dan Mitchell, +2"
  prepState="ready"
  prepLabel="Briefing fresh"
  briefingUrl="/meeting/acme-renewal"
/>
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` (`.meeting`, `.meeting-time`, `.meeting-eyebrow`, `.meeting-title`, `.meeting-context`, `.meeting-foot`)
- **Code:** to be implemented in `src/components/dashboard/MeetingSpineItem.tsx`

## Surfaces that consume it

DailyBriefing (canonical, Today section).

## Naming notes

`MeetingSpineItem` — refers to its role as the "spine" of the briefing (D-spine = "schedule as spine" mockup direction). The existing `MeetingRow` primitive (`src/components/shared/MeetingRow.tsx`) is for compact rows; this is the editorial-density variant.

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from D-spine mockup.
