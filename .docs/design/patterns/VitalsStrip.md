# VitalsStrip

**Tier:** pattern
**Status:** canonical/shipped
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `VitalsStrip`
**`data-ds-spec`:** `patterns/VitalsStrip.md`
**Variants:** read-only; editable; highlight `turmeric | saffron | olive | larkspur`; optional source attribution
**Design system version introduced:** 0.5.0

## Job

Render the inline horizontal strip of key entity vitals that sits directly beneath an entity masthead or hero. It gives account, project, and person surfaces a compact "what matters now" row without turning the hero into a dashboard.

`VitalsStrip` is the read-only presentation contract. `EditableVitalsStrip` is its editing companion and uses the same visual rhythm, separators, field stack, highlight vocabulary, and source attribution treatment.

## When to use it

- Under an entity hero when the user needs 3-8 high-signal facts before reading the rest of the dossier
- For compact entity facts that benefit from side-by-side scanning, such as ARR, health, renewal timing, lifecycle, relationship, owner, target date, and meeting frequency
- When field-level source attribution should stay attached to the value without becoming a full trust band
- In edit mode when the same strip needs inline text, number, select, or date editing controls

## When NOT to use it

- For surface-level health summaries with explanations; use report or health sections
- For settings or operational stats; use `GlanceRow` inside `SurfaceMasthead`
- For claim-level provenance or freshness; use `TrustBand` and `ClaimRow`
- For long metadata lists; use a reference grid or details section

## Composition

```
[top rule]
[vital value stack] · [vital value stack] · [vital value stack] · [vital value stack]
[bottom rule]
```

Each vital value stack contains:

- mono uppercase value text
- optional source attribution beneath the value
- optional finite highlight color

Editable fields preserve the same stack shape and replace the value text with an inline input, select, or date picker only while actively editing.

## Variants

- **read-only** - `VitalsStrip`, accepts a caller-built `VitalDisplay[]`
- **editable** - `EditableVitalsStrip`, renders configured preset fields plus optional metadata and extra read-only vitals
- **attributed** - shows the source system below individual values
- **empty editable fields** - dashed tertiary placeholder label, never rendered by the read-only strip

## Tokens consumed

- `--font-mono` for value and attribution text
- `--color-rule-heavy` for the top and bottom rules
- `--color-text-secondary` and `--color-text-tertiary`
- finite entity highlight colors: `--color-spice-turmeric`, `--color-spice-saffron`, `--color-garden-olive`, `--color-garden-larkspur`
- `--space-lg` for strip and separator rhythm

## API sketch

```tsx
<VitalsStrip
  vitals={[
    { text: "$240k ARR", highlight: "turmeric" },
    { text: "Healthy", highlight: "saffron" },
    { text: "Renewal in 44d", highlight: "olive" },
  ]}
  sourceRefs={sourceRefs}
/>
```

```tsx
<EditableVitalsStrip
  fields={preset.vitals}
  metadataFields={preset.metadata}
  entityData={account}
  metadata={metadata}
  sourceRefs={sourceRefs}
  onFieldChange={handleFieldChange}
/>
```

## Source

- **Read-only code:** `src/components/entity/VitalsStrip.tsx`
- **Read-only CSS:** `src/components/entity/VitalsStrip.module.css`
- **Editable code:** `src/components/entity/EditableVitalsStrip.tsx`
- **Editable CSS:** `src/components/entity/EditableVitalsStrip.module.css`
- **Reference CSS:** `.docs/design/reference/_shared/styles/VitalsStrip.module.css`, `.docs/design/reference/_shared/styles/EditableVitalsStrip.module.css`
- **Consolidation audit:** `.docs/design/_audits/vitals-strip-consolidation.md`

## Surfaces that consume it

Account detail, account editorial, person editorial, and project editorial consume the pattern today. The reference mirrors appear in `reference/surfaces/account.html`, `reference/surfaces/person.html`, and `reference/surfaces/project.html`.

## Naming notes

`VitalsStrip` is named for the entity-level "vitals" job, not for a specific entity type. Keep the editable companion under this pattern unless its interaction model diverges enough to need a separate contract.

## History

- 2026-05-05 - Promoted from shipped entity component into canonical pattern reference after the design-system audit found it was mirrored and reused but undocumented.
