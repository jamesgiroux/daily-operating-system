# SettingsSections

**Tier:** pattern
**Status:** shipped-local/extraction-needed
**Owner:** James
**Last updated:** 2026-05-05
**`data-ds-name`:** `SettingsSections`
**`data-ds-spec`:** `patterns/SettingsSections.md`
**Variants:** You; Connectors; Data; System; dev-only Diagnostics
**Design system version introduced:** 0.5.0

## Job

Document the real shipped Settings chapter vocabulary, rather than the older speculative Settings redesign language.

The shipped chapters are:

- **You** — `YouCard`
- **Connectors** — `ContextSourceSection` and `ConnectorsGrid`
- **Data** — `DatabaseRecoveryCard`, `ActivityLogSection`, and `DataPrivacySection`
- **System** — inline Claude Code status plus `SystemStatus`, `NotificationSection`, and `TextSizeSection`
- **Diagnostics** — `DiagnosticsSection`, development mode only

## Source

- **Code:** `src/pages/SettingsPage.tsx` and `src/features/settings-ui/*`
- **Styles:** `src/pages/SettingsPage.module.css` and feature-level CSS modules.
- **Extraction note:** this is a surface section inventory, not a single exported component.

## Surfaces that consume it

Settings.
