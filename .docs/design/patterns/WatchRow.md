# WatchRow

**Tier:** pattern
**Status:** proposed
**Owner:** DOS-424 (W1)
**Last updated:** 2026-05-06
**`data-ds-name`:** `WatchRow`
**`data-ds-spec`:** `patterns/WatchRow.md`
**Composes:** `InferredActionSelector`

## Job

Render one Watch-section row in the Daily Briefing. Adaptive across four kinds: suggested actions (with InferredActionSelector dropdown), open actions (with completion checkbox), parked items (passive label), aging items (with restore/archive choices).

## Anatomy

Three columns, kind-dependent affordance in the right column:

```
[WHO]   [What text — service-rendered editorial]   [Affordance]
```

`grid-template-columns: minmax(110px, 0.22fr) minmax(0, 1fr) auto;`

### Per-variant affordance anatomy

**`suggestedAction`:**
```
[Globex Inc]   Pushing intro to Q3; not dead.   [Snooze to Q3 ▾]
                                                 └─ menu opens inline
```
The affordance is the InferredActionSelector trigger button + dropdown. Click triggers `actions::snooze(actionId, until)` or `actions::dismiss(actionId)` based on selected option.

**`openAction`:**
```
[Acme Corp]    Send revised pricing appendix.    [○]
                                                  └─ click marks complete
```
The affordance is a circular check button (28px, 1px border, hover-fill turmeric). Click triggers `actions::mark_complete(actionId)`.

**`parked`:**
```
[Internal]     New tier 3 deck circulating.      [Parked]
```
The affordance is a muted text label (mono 11px, tertiary color). Non-interactive.

**`aging`:**
```
[Stark]        Old support thread, no movement.  [Restore] [Archive]
```
The affordance is a pair of small buttons (32px h, secondary tone). Click triggers `actions::restore(actionId)` or `actions::archive(actionId)`.

### Mobile collapse (≤720px)

Single-column stack:
```
[WHO]
[What text]
[Affordance]
```
All four variants follow the same collapse rule.

## Variants

`kind` is the discriminator. Four variants:

- **`suggestedAction`** — affordance is `InferredActionSelector` (trigger button + dropdown menu of options).
  - Carries `LifecycleMixin` (correctionState) — these rows are claim-bearing.
- **`openAction`** — affordance is a circular check button. Click marks complete via `actions::mark_complete`.
- **`parked`** — affordance is a muted "Parked" label (or "Snoozed until Q3"); informational only.
- **`aging`** — affordance is a pair of small choice buttons: restore / archive. Click triggers `actions::restore` or `actions::archive`.

## Contract type

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
  ageLabel: string;
  since: string;
  options: WatchAgingOption[];
}
```

## What it doesn't do

- Decide which kind to render — `WatchService` (DOS-415) selects the kind based on TODAY-relevance + claim-bearing rules.
- Compose the `what` text — service-rendered.
- Mutate state — clicking emits the corresponding mutation call (`actions::*` or `claims::*`); this pattern is read + emit, not write itself.

## Open questions

- Mobile layout: collapse to single column? Current d-spine HTML uses `1fr` at <720px.
- Aging variant's two-button pair: should they fit inline with the row, or stack? TBD by W1 design pass.

## Spec status

**proposed** — TSX ships in W1 (DOS-424). Reference HTML uses `.dspine-watch-row` today; W1 cuts over to `WatchRow_*` scoped classes.
