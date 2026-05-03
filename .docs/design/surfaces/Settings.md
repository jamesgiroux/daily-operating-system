# Settings

**Tier:** surface
**Status:** redesigning (separate project)
**Owner:** James
**Last updated:** 2026-05-03
**`data-ds-name`:** `Settings`
**`data-ds-spec`:** `surfaces/Settings.md`
**Canonical name:** `Settings`
**Source files:**
- `src/features/settings-ui/` (current implementation directory)
- Settings redesign mockup: `.docs/mockups/claude-design-project/mockups/surfaces/settings/`

**Design system version introduced:** 0.3.0

## Job

The user's configuration surface — accounts, connectors, briefing preferences, data, system, diagnostics. Auto-saves on every change (no submit button). Long single-page scroll with chapter navigation, designed to feel "everything in one place" rather than tabbed.

## Layout regions

In reading order:

1. **FolioBar** — surface label "Settings", crumbs "Settings", status text "Auto-saved · just now", minimal action set (search only)
2. **SurfaceMasthead** — eyebrow ("Settings · Last edited 4 minutes ago"), title ("Settings"), lede (one-line state of the system), `glance` slot with `GlanceRow` of 4 cells:
   - Connectors (count + status dot)
   - Database (size + status dot)
   - AI today (% used + warn dot if over threshold)
   - Anomalies 24h (count + warn dot)
3. **Section content** — sequence of `SectionHead` + section body for each chapter:
   - **Identity** — user profile, role, role preset
   - **Connectors** — Gmail, Calendar, Drive, Granola, Linear, Quill, Clay, Gravatar, Claude Desktop (each via `ConnectorSurface` pattern)
   - **Briefing & AI** — briefing settings, AI usage budget, system prompt customization
   - **Data** — data privacy, data retention, export
   - **Activity** — activity log
   - **System** — system status, native integrations
   - **Diagnostics** — health checks, repair tools, debug
4. **Finis** — closer marker + "Auto-saved just now" timestamp

`AtmosphereLayer` (turmeric tint) renders behind everything.

## Local nav approach

**Provides chapters to `FloatingNavIsland`** per D2 (synthesis):

- `identity` → "01 You"
- `connectors` → "02 Connectors" (warn dot if any connector unhealthy)
- `briefing` → "03 Briefing & AI"
- `data` → "04 Data"
- `activity` → "05 Activity"
- `system` → "06 System"
- `diagnostics` → "07 Diagnostics"

Local pill renders these via FloatingNavIsland's chapters contract; click smooth-scrolls. Active chapter highlights via scroll-spy (IntersectionObserver, per mockup `app.jsx`).

**No `SectionTabbar`** — D-spine and Settings mockups proposed a distinct numbered scroll-spy tab bar; rejected per D2. The numbered labels (`01 You / 02 Connectors / …`) live in the chapter labels, rendered by FloatingNavIsland's local pill.

## Patterns consumed

- `FolioBar`, `FloatingNavIsland`, `AtmosphereLayer` (chrome)
- `SurfaceMasthead` (Wave 3) — masthead with glance slot
- `GlanceRow` (Wave 3) — at-a-glance stats
- `SectionHead` (likely a Wave 3 minor pattern; needs explicit spec) — eyebrow + h2 + epi + meta + action
- `FormRow` (Wave 3) — universal label/help | ctrl | aux
- `ConnectorSurface` (per-connector pattern; lives under settings-ui)
- `FinisMarker` (existing in src; canonical from Wave 1 chrome)

## Primitives consumed

- `GlanceCell` (Wave 3) — k/v cell with status dot inside GlanceRow
- `InlineInput`, `Switch`, `Segmented` (Wave 3 form primitives)
- `RemovableChip` (Wave 3 — distinguished from Pill)
- `Btn` / Button — kind variants per section
- `Pill` (status indicators in connector rows)

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
- Settings redesign is a separate project from v1.4.x; ships when the Wave 3 substrate is in place and v1.4.3 / v1.4.4 stabilize.
