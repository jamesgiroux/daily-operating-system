# MeetingSpineItem

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `MeetingSpineItem`
**`data-ds-spec`:** `patterns/MeetingSpineItem.md`
**Variants:** state (`past | in-progress | upcoming | cancelled`); type (`customer | partner | project | internal | one_on_one`)
**Design system version introduced:** 0.1.0

## Job

Render a single meeting as a magazine-article entry in DailyBriefing's Today section: time on the left, body on the right with account/person/partner/project/internal identity, serif title, italic context line, and a footer with attendees + prep state + briefing action. State-aware (past / now / upcoming / cancelled).

## When to use it

- DailyBriefing Daily Briefing redesign reference candidate (`reference/surfaces/briefing-redesign.html`)
- DailyBriefing's Today section, if the Daily Briefing redesign redesign is approved for the routed surface
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
- Optional state tag under duration (mono 9.5px uppercase): `Now` (turmeric), `Up next` (rosemary)

**Body**:
- Identity eyebrow (mono uppercase 10px) — colored glyph dot + name/type + horizontal rule. Color per type: customer=turmeric, one_on_one=larkspur, partner/project=olive, internal=tertiary
- Title (serif 26px, weight 400) — links to meeting detail; on hover: turmeric
- Context (serif italic 16px) — the "what this is actually about" line, optional
- Footer (mono 11px) — attendees · prep state pill · briefing link OR create button (per state)

## Variants

- **`past`** — opacity 0.5; smaller title (22px)
- **`in-progress`** — turmeric-tinted background gradient to transparent; time and `Now` tag in turmeric
- **`upcoming`** — default
- **`cancelled`** — opacity 0.4; title strikethrough
- **`partner`** — olive identity for partner meetings.

Footer changes per prep state:
- `ready` (sage) — "Notes captured" / "Briefing fresh" + briefing link
- `building` (saffron) — "Briefing building" + briefing link
- `needs` (terracotta) — "No briefing yet" + Create button

## Tokens consumed

- `--font-mono` (time, duration, state tag, eyebrow, footer)
- `--font-serif` (title), `--font-serif italic` (context)
- `--color-spice-turmeric` (customer eyebrow, in-progress, NOW), `--color-garden-larkspur` (one_on_one), `--color-garden-olive` (partner/project), `--color-text-tertiary` (internal)
- `--color-spice-terracotta` (needs prep)
- `--color-garden-rosemary` (Up next, ready prep state)
- `--color-spice-saffron` (building prep state)
- `--color-rule-light` (item bottom border)
- `--space-md`, `--space-xl` (gaps, padding)

## API sketch

```tsx
<MeetingSpineItem
  time="10:00"
  duration="45m"
  state="in-progress"
  type="customer"
  entityName="Acme Corp - Renewal"
  title="Acme renewal - pricing and tier 3"
  context="Decisions on tier 3 pricing. Legal needs final terms language before the MSA review."
  attendees="Jen Park, Dan Mitchell, +2"
  prepState="ready"
  prepLabel="Briefing fresh"
  briefingUrl="/meeting/acme-renewal"
/>
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/Daily Briefing redesign.html` (`.meeting`, `.meeting-time`, `.meeting-eyebrow`, `.meeting-title`, `.meeting-context`, `.meeting-foot`)
- **Code:** `src/components/dashboard/MeetingSpineItem.tsx` (available for the proposed cutover; not yet consumed by routed DailyBriefing)
- **Styles:** `src/components/dashboard/MeetingSpineItem.module.css`
- **Reference mirror:** `.docs/design/reference/_shared/styles/MeetingSpineItem.module.css`

## Surfaces that consume it

- `DailyBriefingRedesign` proposed reference surface (`.docs/design/reference/surfaces/briefing-redesign.html`)
- No shipped routed consumer yet. Routed DailyBriefing still uses `BriefingMeetingCard` + `MeetingCard`.

## Naming notes

`MeetingSpineItem` — refers to its role as the "spine" of the briefing (Daily Briefing redesign = "schedule as spine" mockup direction). The existing `MeetingRow` primitive (`src/components/shared/MeetingRow.tsx`) is for compact rows; this is the editorial-density variant.

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from Daily Briefing redesign mockup.
- 2026-05-06 — Source component, CSS module, reference mirror, and Daily Briefing redesign reference surface added. Remains proposed until the routed DailyBriefing rollout lands; production requires a release tag.
- 2026-05-06 — State tag moved into the time column, ThreadMark removed, and identity colors aligned to account/person/partner/project/internal semantics.
