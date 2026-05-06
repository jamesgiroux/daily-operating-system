# Settings

**Tier:** surface
**Status:** shipped source reconciled
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `Settings`
**`data-ds-spec`:** `surfaces/Settings.md`
**Canonical name:** `Settings`
**Source files:**
- `src/pages/SettingsPage.tsx` (current router target)
- `src/pages/SettingsPage.module.css`
- `src/features/settings-ui/*` (actual section implementations)
- Settings redesign mockup: `.docs/_archive/mockups/claude-design-project/mockups/surfaces/settings/` (roadmap/reference only)

**Design system version introduced:** 0.3.0

## Job

The user's configuration surface — accounts, connectors, briefing preferences, data, system, diagnostics. Auto-saves on every change (no submit button). Long single-page scroll with chapter navigation, designed to feel "everything in one place" rather than tabbed.

## Layout regions

In reading order:

1. **FolioBar / magazine shell** — Settings label and surface actions.
2. **SurfaceMasthead** — shipped title block from `SettingsPage.tsx`.
3. **You chapter** — `YouCard` identity, domains, role, workspace, day start, and personality controls.
4. **Connectors chapter** — `ConnectorsGrid`, connector detail components, and connector status dots.
5. **Data chapter** — context sources, privacy, notification/text-size controls, and data management sections.
6. **System chapter** — system status, sync/security sections, Claude Code state, and recovery controls.
7. **Diagnostics** — development-only diagnostics section.
8. **Finis** — `FinisMarker`.

`AtmosphereLayer` (turmeric tint) renders behind everything.

## Local nav approach

**Provides chapters to `FloatingNavIsland`** per D2 (synthesis):

- `settings-you`
- `settings-connectors`
- `settings-data`
- `settings-system`
- `settings-diagnostics` in development only

Local pill renders these via FloatingNavIsland's chapters contract; click smooth-scrolls. Active chapter highlights via scroll-spy (IntersectionObserver, per mockup `app.jsx`).

**No `SectionTabbar`** — Daily Briefing redesign and Settings mockups proposed a distinct numbered scroll-spy tab bar; rejected per D2. The numbered labels (`01 You / 02 Connectors / …`) live in the chapter labels, rendered by FloatingNavIsland's local pill.

## Patterns consumed

- `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer` (chrome)
- `SurfaceMasthead`
- `FormRow`
- `YouCard`
- `SettingsSections`
- `ActivityLogSection`
- `DiagnosticsSection`
- `FinisMarker`

## Primitives consumed

- `StatusDot`
- `Switch`
- `Segmented`
- Settings-local buttons/inputs and chips from `features/settings-ui`.

Not shipped in the current Settings route:

- `GlanceRow` / `GlanceCell`
- `InlineInput`
- standalone `ConnectorSurface`

## Notable interactions

- **Auto-save**: every settings change persists immediately; FolioBar's status text updates ("Saving…" → "Auto-saved · just now"). No submit button.
- **Tweaks panel** (development affordance only): mockup includes a slide-in TweaksPanel for design exploration (density, edit mode, theme tint). NOT a product feature; do not promote.
- **Edit affordances**: per `editMode` toggle, click-to-edit InlineInput / EditableText surfaces become discoverable.

## Empty / loading / error states

- **Loading** — `EditorialLoading` skeleton; FolioBar shows "Loading settings…"
- **Error per section** — section-level `EditorialError` (terracotta) with retry; doesn't break the whole surface
- **Connector failure** — connector card surfaces error inline; warn dot in chapter nav
- **Empty data sections** — friendly "Nothing here yet" copy; no auto-populate

## Naming notes

Canonical name `Settings`. Already matches the user-facing label and the route. No rename pending.

## History

- 2026-05-03 — Surface spec authored as part of Wave 3 (Settings redesign substrate prep).
- 2026-05-05 — Corrected spec to current shipped `SettingsPage.tsx` and `features/settings-ui/*`. Older 7-chapter redesign concepts are roadmap/reference only.
