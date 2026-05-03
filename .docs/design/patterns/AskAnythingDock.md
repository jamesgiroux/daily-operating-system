# AskAnythingDock

**Tier:** pattern
**Status:** proposed
**Owner:** James
**Last updated:** 2026-05-02
**`data-ds-name`:** `AskAnythingDock`
**`data-ds-spec`:** `patterns/AskAnythingDock.md`
**Variants:** `placement="inline" | "fixed-bottom"`; suggestion chips configurable
**Design system version introduced:** 0.1.0

## Job

Multi-line "front door" to the conversational layer of DailyOS — an editorial input that doesn't read like a search box. Three rows: italic placeholder input + suggestion chips + a scope footer that names sources and write-back behavior. Receives context from `ThreadMark` clicks elsewhere on the surface.

## When to use it

- DailyBriefing (canonical; placed at the bottom of the briefing as a "what now?" affordance)
- Any future surface where the user benefits from being able to ask the assistant something with context
- Cross-version foundational pattern for v1.4.6 (Proactive Intelligence)

## When NOT to use it

- For a global command-K palette — that's a different pattern (use `command.tsx` from shadcn)
- For a quick search box — too heavy
- Inside dialogs (compete with primary action)

## Composition

Three-row vertical stack inside a card with subtle border and `--frosted-glass-nav` background:

**Row 1 — input**:
- Search-glyph icon (left, 20px, color tertiary)
- Input (serif 19px, weight 400, italic placeholder rotates every 4.5s through example questions)
- ⌘K kbd badge (right, mono small)

**Row 2 — suggestion chips**:
- Horizontal flex of `AskChip` instances (sans 12.5px, rounded pill, charcoal-4 background)
- On click: seeds the input

**Row 3 — scope footer**:
- Left: scope dot (sage) + "Mail · Calendar · Notes · CRM · Slack" + "Since Jan 1, 2024"
- Right: "Writes back to your briefing" (turmeric, weight 600)
- Background: subtle charcoal tint, border-top rule-light

Focus state: border-color shifts to text-secondary, soft shadow appears.

## Variants

- **inline** (default) — sits in the page flow at section width
- **fixed-bottom** (potential) — pinned to bottom of viewport when surface is long

## Tokens consumed

- `--frosted-glass-nav` (background), `--color-paper-warm-white`
- `--color-rule-medium` (border)
- `--radius-md`
- `--font-serif italic` (input + placeholder), `--font-sans` (chips), `--font-mono` (kbd, scope, suggestion-trigger labels)
- `--color-text-tertiary` (placeholder, default), `--color-text-primary` (input text on focus)
- `--color-spice-turmeric-7`, `--color-spice-turmeric` (chip hover, scope-write emphasis)
- `--color-garden-sage` (scope status dot)
- `--color-desk-charcoal-4`, `--color-desk-charcoal-15` (scope footer background, kbd background)

## API sketch

```tsx
<AskAnythingDock
  placeholderRotation={[
    "What did Sara say about tier 3 last quarter?",
    "Remind me to follow up with James on Monday…",
    "What's slipping on Acme this week?",
    "Show me Northwind's last 5 emails",
  ]}
  suggestionChips={[
    { label: "What's slipping on Acme this week?" },
    { label: "Show me Northwind's last 5 emails" },
    { label: "Remind me about James Monday" },
    { label: "Who's quiet that I should check on?" },
  ]}
  scopeSources={["Mail", "Calendar", "Notes", "CRM", "Slack"]}
  scopeSince="Jan 1, 2024"
  writesBack
  onSubmit={(query, context) => /* dispatch to assistant */}
/>
```

Receives context seeded by `ThreadMark` clicks (via global event or context provider).

## Source

- **Mockup substrate:** `.docs/mockups/claude-design-project/mockups/briefing/variations/D-spine.html` (`.ask`, `.ask-bar`, `.ask-bar-input`, `.ask-bar-suggestions`, `.ask-bar-scope`, `.ask-chip`)
- **Code:** to be implemented in `src/components/dashboard/AskAnythingDock.tsx` (Wave 1 follow-on)

## Surfaces that consume it

DailyBriefing (canonical). Cross-version: foundational for v1.4.6 (Proactive Intelligence) — should propagate as a global / persistent affordance once that version's scope is settled.

## Naming notes

`AskAnythingDock` — refers to its role as a "dock" (anchored input area, not a floating bar) and the explicit "ask anything" framing. Don't rename to `SearchBar` (it's not search), `CommandPalette` (different pattern), or `AssistantInput` (too generic).

## History

- 2026-05-02 — Proposed pattern for v1.4.3 from D-spine mockup. Cross-version foundational pattern (extends in v1.4.6).
