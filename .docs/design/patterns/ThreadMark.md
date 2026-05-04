# ThreadMark

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `ThreadMark`
**`data-ds-spec`:** `patterns/ThreadMark.md`
**Variants:** default (hover-only); `persistent` (always visible at lower opacity)
**Design system version introduced:** 0.1.0

## Job

Universal "talk about this" hover affordance — appears on hover at the right edge of any addressable line (action item, thread entry, meeting, watch item). One pattern, applied everywhere. Click seeds the `AskAnythingDock` with context: `About: "<line text>" — `.

## When to use it

- Any line / item / row that the user might want to ask the assistant about (action items, thread entries, meetings, entities, watch items, suggestions)
- Wherever there's an addressable piece of intelligence on screen

## When NOT to use it

- Headings, titles, page chrome (no action context)
- Form inputs (different interaction model)
- Buttons / CTAs (would compete)

## Composition

```
↪ talk
```

Small mono lowercase label with leading hook arrow (↪). Text-tertiary by default; turmeric on hover. Background turmeric-7 on hover.

States:
- **hidden** — opacity 0, translateX -4px (default)
- **revealed** — opacity 1, translateX 0 (when parent line is hovered)
- **persistent** — opacity 0.45 always (e.g., on `.meeting-foot`)
- **hover-self** — opacity 1, color turmeric, background tint

Transition: opacity + transform 180ms ease.

## Variants

- **default** — hidden until parent hover
- **persistent** — always visible at 0.45 opacity (use sparingly; for items where the affordance benefits from being signposted)

## Tokens consumed

- `--font-mono` (label)
- `--color-text-quaternary` (default, when revealed)
- `--color-spice-turmeric` (hover color)
- `--color-spice-turmeric-7` (hover background)

## API sketch

```tsx
<ThreadMark onClick={(context) => askDock.seed(`About: "${context}" — `)} />
```

CSS pattern: a `.thread-mark` button inside any addressable line; parent line `:hover` reveals it via cascade. Click handler seeds the global `AskAnythingDock`.

```html
<li>
  <span class="who">Acme · Stakeholder</span>
  <span class="what">VP Eng (Sara Wu) added to renewal thread</span>
  <button class="thread-mark" data-ds-name="ThreadMark" data-ds-spec="patterns/ThreadMark.md">talk</button>
</li>
```

## Source

- **Mockup substrate:** `.docs/_archive/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` (`.thread-mark`, plus the click handler at lines 1923-1936 that wires it to the Ask input)
- **Code:** to be implemented as a small React component, likely in `src/components/ui/ThreadMark.tsx`

## Surfaces that consume it

DailyBriefing (canonical — used in `WatchListRow`, `EntityThreadList`, `MeetingSpineItem` foot, etc.). Cross-version: should propagate to AccountDetail / ProjectDetail / PersonDetail / MeetingDetail anywhere there are addressable lines.

## Naming notes

`ThreadMark` — refers to the "talk about this thread" semantic. Lowercase "talk" label is intentional (matches the editorial voice; not a CTA shouting "DISCUSS!"). Don't rename to "AskAbout" or "Discuss" — the lowercase + hook arrow design IS the affordance.

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from D-spine mockup. Cross-version pattern; foundational for the conversational layer.
