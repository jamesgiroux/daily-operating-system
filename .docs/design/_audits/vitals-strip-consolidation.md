# VitalsStrip consolidation audit

DOS-366. Per-use audit of VitalsStrip + EditableVitalsStrip across the app, with consolidation recommendation.

## Summary

- VitalsStrip: 4 consumers
- EditableVitalsStrip: 4 consumers
- Variants observed: 4 prop/render variants
- Recommendation: refactor differently

## Use sites

### VitalsStrip

| Consumer | File | Props passed | Variant rendered | Notes |
|---|---|---|---|---|
| AccountDetailEditorial | `src/pages/AccountDetailEditorial.tsx:152` | `vitals={buildAccountVitals(detail)}`; `sourceRefs={detail.sourceRefs}` | read-only account fallback + source attribution | Rendered only when `detail.accountType !== "internal"` and no active `preset` (`src/pages/AccountDetailEditorial.tsx:145`). The page is marked deprecated at `src/pages/AccountDetailEditorial.tsx:1`. |
| AccountDetailPage | `src/pages/AccountDetailPage.tsx:988` | `vitals={buildAccountVitals(detail)}`; `sourceRefs={detail.sourceRefs}` | read-only account fallback + source attribution | Active account detail route. Same conditional shape as the deprecated editorial page: non-internal account, no active `preset` (`src/pages/AccountDetailPage.tsx:981`). |
| ProjectDetailEditorial | `src/pages/ProjectDetailEditorial.tsx:309` | `vitals={buildProjectVitals(detail)}` | read-only project fallback | Rendered only when no active `preset` (`src/pages/ProjectDetailEditorial.tsx:289`). No `sourceRefs`. Local builder emits `status`, days-to-target, milestone progress, meeting frequency, and open actions (`src/pages/ProjectDetailEditorial.tsx:65`). |
| PersonDetailEditorial | `src/pages/PersonDetailEditorial.tsx:263` | `vitals={buildPersonVitals(detail)}` | read-only person fallback | Rendered only when no active `preset` (`src/pages/PersonDetailEditorial.tsx:246`). No `sourceRefs`. Local builder emits temperature, meeting cadence, last meeting, and total meetings (`src/pages/PersonDetailEditorial.tsx:66`). |

### EditableVitalsStrip

| Consumer | File | Props passed | Variant rendered | Notes |
|---|---|---|---|---|
| AccountDetailEditorial | `src/pages/AccountDetailEditorial.tsx:146` | `fields={preset.vitals.account}`; `metadataFields={preset.metadata.account}`; `entityData={detail}`; `metadata={metadataValues}`; `onFieldChange`; `conflicts={conflictsForStrip}`; `sourceRefs={detail.sourceRefs}` | editable account preset + metadata + source attribution | Rendered for non-internal accounts when `preset` exists. `onFieldChange` routes `metadata` to `handleMetadataChange` and `column` to `saveAccountField` (`src/pages/AccountDetailEditorial.tsx:148`). `conflicts` is currently accepted but unused by the component (`src/components/entity/EditableVitalsStrip.tsx:442`). |
| AccountDetailPage | `src/pages/AccountDetailPage.tsx:982` | `fields={preset.vitals.account}`; `metadataFields={preset.metadata.account}`; `entityData={detail}`; `metadata={page.metadataValues}`; `onFieldChange`; `conflicts={page.conflictsForStrip}`; `sourceRefs={detail.sourceRefs}` | editable account preset + metadata + source attribution | Active account detail route. Same account editable API as the deprecated editorial page, with page-hook-owned metadata/save handlers (`src/pages/AccountDetailPage.tsx:984`). |
| ProjectDetailEditorial | `src/pages/ProjectDetailEditorial.tsx:290` | `fields={preset.vitals.project}`; `metadataFields={preset.metadata.project}`; `entityData={detail}`; `metadata={metadataValues}`; `onFieldChange` | editable project preset + metadata | No `sourceRefs`, no `conflicts`, and no `extraVitals`. `onFieldChange` routes `metadata` to `update_entity_metadata` and `column` to `update_project_field` (`src/pages/ProjectDetailEditorial.tsx:295`). The read-only fallback's signal vitals are not appended in this mode. |
| PersonDetailEditorial | `src/pages/PersonDetailEditorial.tsx:247` | `fields={preset.vitals.person}`; `metadataFields={preset.metadata.person}`; `entityData={{ ...detail, signals: detail.signals as Record<string, unknown> \| undefined }}`; `metadata={metadataValues}`; `onFieldChange` | editable person preset, currently null for bundled presets | `onFieldChange` only handles `metadata` (`src/pages/PersonDetailEditorial.tsx:252`). Bundled role presets define empty `person` vitals/metadata (`src-tauri/presets/core.json:26`, `src-tauri/presets/customer-success.json:28`, `src-tauri/presets/product-marketing.json:26`, `src-tauri/presets/affiliates-partnerships.json:29`), so `EditableVitalsStrip` returns `null` for this path (`src/components/entity/EditableVitalsStrip.tsx:455`). |

## Variants observed

- Size: one standard strip size. Both components use 12px uppercase mono values (`src/components/entity/VitalsStrip.module.css:37`, `src/components/entity/EditableVitalsStrip.module.css:77`); no `compact` or density prop exists.
- Alignment: both render horizontal flex rows with wrapping (`src/components/entity/VitalsStrip.module.css:11`, `src/components/entity/EditableVitalsStrip.module.css:110`). Editable adds 14px left/right inner padding (`src/components/entity/EditableVitalsStrip.module.css:115`) that read-only does not have.
- Separator style: both use the same 3px dot separator (`src/components/entity/VitalsStrip.module.css:24`, `src/components/entity/EditableVitalsStrip.module.css:125`).
- Value highlighting: read-only accepts `VitalDisplay.highlight` (`src/lib/entity-types.ts:9`) and applies color at render (`src/components/entity/VitalsStrip.tsx:73`). Editable computes highlight from hardcoded field keys and health values (`src/components/entity/EditableVitalsStrip.tsx:65`, `src/components/entity/EditableVitalsStrip.tsx:144`). The same mapping is duplicated again in `buildVitalsFromPreset` (`src/lib/preset-vitals.ts:34`).
- Source line treatment: read-only source attribution is optional and text-matched from the rendered vital (`src/components/entity/VitalsStrip.tsx:26`). Editable attribution is field-key matched through `FIELD_TO_SOURCE_KEY` (`src/components/entity/EditableVitalsStrip.tsx:422`). Only account consumers pass `sourceRefs`.
- Editable vs read-only: read-only receives finished `text` values and skips empty strips (`src/components/entity/VitalsStrip.tsx:47`). Editable resolves data, renders empty placeholders for editable fields, opens inline editors by `fieldType`, and skips empty signal fields (`src/components/entity/EditableVitalsStrip.tsx:280`, `src/components/entity/EditableVitalsStrip.tsx:294`).
- Field configuration: read-only fields are per-entity local builders (`src/components/account/account-detail-utils.ts:56`, `src/pages/ProjectDetailEditorial.tsx:65`, `src/pages/PersonDetailEditorial.tsx:66`). Editable fields come from role preset arrays plus metadata arrays (`src/types/preset.ts:23`). Presets cover account, project, and person only; no WeekPage or WeeklyImpactPage VitalsStrip consumer was found in `src/components/` or `src/pages/`.

## Recommendation

Refactor differently: consolidate the visual renderer and value formatting, but do not keep two independent full components. The current APIs are not merely two shapes for the same thing: read-only callers pass prebuilt display strings, while editable callers pass preset field definitions, entity data, metadata, write handlers, optional provenance, and a currently-unused conflict channel.

The right consolidation is a shared `VitalsStripFrame`/`VitalItem` layer with thin read-only and editable adapters, or a single public `VitalsStrip` that accepts normalized item descriptors with optional edit controls. Migrate `AccountDetailPage` first, then `ProjectDetailEditorial`, then `PersonDetailEditorial`; leave deprecated `AccountDetailEditorial` last or delete it as part of the route cleanup.

## Migration sketch (if collapse recommended)

- Extract shared layout classes or a `VitalsStripFrame` so spacing, side padding, separator dots, mono typography, highlighting, and attribution are defined once.
- Extract value resolution/formatting from `EditableVitalsStrip` and `src/lib/preset-vitals.ts` into one preset-to-item builder.
- Normalize source attribution to field keys, not rendered text matching.
- Decide whether empty editable fields are first-class `VitalItem` placeholders; make that behavior explicit instead of coupling it to `EditableVitalsStrip`.
- Add `extraVitals` or computed signal append behavior to the normalized builder so project/person preset mode does not drop fallback signal vitals.
- Fix or explicitly document the person preset behavior: with current bundled presets, person vitals render in fallback mode but disappear when any preset is active.

Before:

```tsx
{preset ? (
  <EditableVitalsStrip
    fields={preset.vitals.account}
    metadataFields={preset.metadata.account}
    entityData={detail}
    metadata={metadataValues}
    onFieldChange={handleFieldChange}
    sourceRefs={detail.sourceRefs}
  />
) : (
  <VitalsStrip
    vitals={buildAccountVitals(detail)}
    sourceRefs={detail.sourceRefs}
  />
)}
```

After:

```tsx
<VitalsStrip
  mode={preset ? "editable" : "readOnly"}
  items={buildVitalItems({
    entityType: "account",
    preset,
    detail,
    metadata: metadataValues,
    sourceRefs: detail.sourceRefs,
  })}
  onChange={handleFieldChange}
/>
```

## Out of scope for this audit

- Implementation (audit only)
- Visual redesign of vitals
