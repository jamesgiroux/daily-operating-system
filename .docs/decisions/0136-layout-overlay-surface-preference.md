# ADR-0136 — Layout Overlay as Surface-Side Presentation Preference

**Status:** Accepted
**Date:** 2026-06-02
**Authors:** James Giroux, Codex
**Relates to:** [ADR-0130](0130-surface-independent-composition-contract.md), [ADR-0105](0105-provenance-as-first-class-output.md), [ADR-0108](0108-provenance-rendering-and-privacy.md), [ADR-0123](0123-typed-claim-feedback-semantics.md), [ADR-0135](0135-revert-wordpress-primary-surface-headless-via-mcp.md)

## Context

ADR-0130 makes the substrate the author of `Composition`: abilities produce sections and blocks with provenance, claim refs, salience, trust, and fallback behavior. W1 proved that model on Account Detail.

v1.5.0 W2 adds user customization: hide, reorder, choose variants, and edit eligible text in context. Those actions are surface presentation preferences, not composition authorship. Without a small contract, W2 can accidentally blur three things that must stay separate:

- substrate-authored `Composition`
- user presentation overlay
- claim feedback/correction

## Decision

DailyOS stores layout customization as a **layout overlay**: local user preference data applied by the surface renderer after safe projection. The overlay never mutates a `Composition`, never writes claim text, and never changes trust/provenance.

The overlay may store:

- section and block ordering
- user visibility toggles
- selected renderer variant names
- presentation-only label overrides
- overlay metadata such as schema version, layout revision, and timestamps

The overlay must not store:

- raw unknown-block payloads
- provenance envelopes
- source labels or source refs beyond stable rendered ids needed for matching
- claim text replacements
- sensitivity decisions
- trust-band overrides

Claim-backed text edits use existing typed feedback/correction semantics per ADR-0123. Hiding a block is a layout preference, not a dismissal or source-quality penalty.

## Storage Shape

W2 stores one overlay row per entity type and surface key. The row is local preference data.

```text
(entity_type, surface_key) -> overlay_json + overlay_schema_version + layout_revision
```

`entity_type` is constrained to the existing supported type vocabulary for v1.5.0 (`account`, `project`, `person`). `surface_key` is stable (`entity_page` for W2). Version fields are metadata, not identity: `overlay_schema_version` describes the persisted JSON format, and `layout_revision` is service-owned presentation state for cache/mutation ordering. The JSON payload is bounded and service-validated before persistence.

## Application Order

The renderer applies overlay preferences after projection:

```text
ProjectedComposition + LayoutOverlay -> RenderableCompositionView
```

Visibility resolves in this order:

1. structural eligibility
2. data present after projection
3. suppression/dismissal already represented by the projected block set
4. user toggle from the layout overlay

The overlay cannot force an ineligible, empty, suppressed, or privacy-dropped block to render.

## Consequences

- Producer output remains canonical and auditable.
- Overlay writes can be fast, local, and user-controlled without claim-substrate side effects.
- Future Project/Person surfaces can consume the same overlay shape after W3.
- Reset to default is simple: delete or replace the overlay row for the type/surface key.
- Client surfaces must apply optimistic save/reset responses with latest-wins mutation guards so stale responses cannot rollback newer local presentation state.
- Renderer tests must prove the unknown-block fallback privacy boundary still holds in edit mode.

## Non-Goals

- A generic variant registry. Variant names are renderer-local strings until multiple consumers need shared declarations.
- User-defined entity types. v1.5.0 is limited to Account, Project, and Person.
- Layout changes over MCP/headless surfaces.
- Per-action undo. W2 recovery is Reset to default.
