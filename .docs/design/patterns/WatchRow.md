# WatchRow

**Tier:** pattern
**Status:** integrated
**Owner:** James
**Last updated:** 2026-05-06
**`data-ds-name`:** `WatchRow`
**`data-ds-spec`:** `patterns/WatchRow.md`
**Variants:** `kind="suggestedAction" | "openAction" | "parked" | "aging"`
**Design system version introduced:** 0.6.0

## Job

Render one Watch-section row in the Daily Briefing. Adaptive across four kinds: suggested actions (with `InferredActionSelector` dropdown), open actions (with completion checkbox), parked items (passive label), aging items (with restore / archive choices). The pattern is the action triage register — three columns, who / what / affordance — and the kind discriminator picks the right-column affordance per row.

## When to use it

- Inside the Daily Briefing Watch section (DOS-413 contract)
- When the surface needs to triage many small items adaptively (different rows have different right-column affordances) without making each row a separate pattern
- When the items are claim-bearing (suggested actions carry `LifecycleMixin`)

## When NOT to use it

- For "what's moving on this entity" — that's `MovingRow`
- For a generic action row outside the briefing's restraint contract — that's `ActionRow` or a list pattern that doesn't enforce kind discrimination
- For an editorial meeting row — that's `BriefingMeetingCard`
- For a parked item that has no human-relevant context (then drop it from the contract — service decides)

## States / variants

`kind` is the discriminator. Four variants:

- **`suggestedAction`** — affordance is `InferredActionSelector` (trigger button + dropdown of options). Carries `LifecycleMixin` (correctionState) — these rows are claim-bearing.
- **`openAction`** — affordance is a circular check button. Click triggers `actions::mark_complete(actionId)`.
- **`parked`** — affordance is a muted "Parked" / "Snoozed until Q3" label. Non-interactive.
- **`aging`** — affordance is a pair of small choice buttons (restore / archive). Click triggers `actions::restore(actionId)` or `actions::archive(actionId)`.

States across all variants:

- **Default**, **hover** (border-color shift), **focus** (focus ring on row).
- **Loading / error / empty** — handled by parent `BriefingLoadState`; never per-row.

## Composition

Three columns: `minmax(110px, 0.22fr) minmax(0, 1fr) auto;`

```
[WHO]   [What text — service-rendered editorial]   [Affordance]
```

Per-variant right-column anatomy:

**`suggestedAction`:**
```
[Globex Inc]   Pushing intro to Q3; not dead.   [Snooze to Q3 ▾]
                                                 └─ menu opens inline
```

**`openAction`:**
```
[Acme Corp]    Send revised pricing appendix.    [○]
                                                  └─ click marks complete
```

**`parked`:**
```
[Internal]     New tier 3 deck circulating.      [Parked]
```

**`aging`:**
```
[Stark]        Old support thread, no movement.  [Restore] [Archive]
```

Mobile collapse (≤720px) — single-column stack of `[WHO] / [What] / [Affordance]` for all four variants.

Composes:

- `InferredActionSelector` — trigger button + dropdown for `suggestedAction` variant
- `Pill` — optional secondary tags inside `who` (e.g. account-type pill)

## Tokens consumed

- `--color-text-primary`, `--color-text-secondary`, `--color-text-tertiary` — who / what / parked-label text
- `--color-border-subtle`, `--color-border-strong` — row border default / hover
- `--color-account-turmeric` — open-action check button hover fill
- `--color-spice-saffron` — suggested-action selector accent
- `--font-serif` — `who` text
- `--font-sans` — `what` text + affordance labels
- `--font-mono` — parked / age labels
- `--space-md`, `--space-lg` — column gap and row padding

## API sketch

```tsx
<WatchRow
  kind="suggestedAction"
  who="Globex Inc"
  what="Pushing intro to Q3; not dead."
  selector={{ trigger: "Snooze to Q3", options: [...] }}
/>

<WatchRow
  kind="openAction"
  who="Acme Corp"
  what="Send revised pricing appendix."
  actionId="act_abc"
  checkButtonLabel="Mark complete"
/>

<WatchRow kind="parked" who="Internal" what="New tier 3 deck circulating." parkedLabel="Parked" />

<WatchRow
  kind="aging"
  who="Stark"
  what="Old support thread, no movement."
  ageLabel="2w"
  since="2026-04-22"
  options={[
    { label: "Restore", actionId: "act_def" },
    { label: "Archive", actionId: "act_def" },
  ]}
/>
```

Contract type:

```ts
type WatchRowViewModel =
  | WatchSuggestedActionRow
  | WatchOpenActionRow
  | WatchParkedRow
  | WatchAgingRow;

interface WatchRowBase extends TrustMixin {
  who: string;
  what: string;
}

interface WatchSuggestedActionRow extends WatchRowBase, LifecycleMixin {
  kind: "suggestedAction";
  selector: InferredActionSelectorViewModel;
}

interface WatchOpenActionRow extends WatchRowBase {
  kind: "openAction";
  actionId: string;
  checkButtonLabel: string;
}

interface WatchParkedRow extends WatchRowBase {
  kind: "parked";
  parkedLabel: string;
}

interface WatchAgingRow extends WatchRowBase {
  kind: "aging";
  actionId: string;
  ageLabel: string;
  since: string;
  options: WatchAgingOption[];
}
```

The view does not decide which kind to render — `WatchService` (DOS-415) picks the kind from TODAY-relevance + claim-bearing rules. The view does not mutate — it emits the corresponding `actions::*` or `claims::*` call.

## Source

- **Code:** ships W1 (DOS-424) at `src/components/dashboard/WatchRow.tsx` + `src/components/dashboard/WatchRow.module.css`
- **Reference render:** `.docs/design/reference/surfaces/briefing-redesign.html` (Watch section)

## Surfaces that consume it

- DailyBriefing (Watch section)

## Naming notes

`WatchRow` is the canonical name. Not `BriefingWatchRow` — patterns are named for the pattern, not the surface (`NAMING.md`). The "Watch" register comes from the contract section name (DOS-413). Distinct from `ActionRow` (generic action list row) and `EntityRow` (entity directory row).

## History

- 2026-05-06 — Promoted to canonical from Daily Briefing redesign exploration. TSX ships W1 under DOS-424. Reference HTML uses provisional `.dspine-watch-row` class until W1 cutover to `WatchRow_*` scoped names.
