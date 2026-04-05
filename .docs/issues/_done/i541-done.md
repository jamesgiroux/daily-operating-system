# I541 — Settings Page UX Rebuild

**Priority:** P1
**Area:** Frontend / Settings / UX
**Version:** v1.0.0 (Phase 3d, Wave 4)
**Depends on:** I447, I521

## Problem

The Settings page is functionally complete but structurally incoherent. YouCard conflates 5 unrelated settings (email domains, role presets, workspace path, briefing time, personality) into a single section. The System section mixes version info, AI models, hygiene, capture, and data management into one disclosure. Every settings component uses inline `style={{}}` — the design system explicitly forbids this. The audit log loads 200 records max with no pagination.

Users cannot predict where settings live. Cognitive load is high. The page does not reflect the editorial design language that the rest of the app achieves.

## Scope

### Information Architecture

Reorganize Settings from 4 sections into 5 coherent groups:

| Current | New | Contents |
|---------|-----|----------|
| You (everything) | **Identity** | Name, company, title, role preset |
| You (cont.) | **Workspace** | Workspace path, internal email domains |
| You (cont.) | **Preferences** | Briefing start time, personality/tone |
| Connectors (unchanged) | **Connectors** | Google, Claude Code, Glean, Clay, etc. |
| Data (unchanged) | **Data** | Activity log (with pagination), database recovery |
| System (everything) | **System** | Version/updates, health summary, security, AI models, hygiene, capture, data management |

YouCard splits into 3 components. System section gets clearer sub-grouping within the Advanced disclosure (or promoted sub-sections for frequently-accessed items).

### Style Migration

Migrate all inline `style={{}}` to CSS modules across all 8 settings components:
- SettingsPage.tsx (container, banners, hero, ClaudeCodeSection)
- YouCard.tsx (domains, role presets, workspace, day start, personality)
- ConnectorsGrid.tsx (connector rows, status indicators)
- ConnectorDetail.tsx (close button, detail panel)
- ContextSourceSection.tsx (Glean config panel, strategy radio buttons)
- SystemStatus.tsx (all sub-sections, disclosure, health, hygiene dots)
- DatabaseRecoveryCard.tsx (cards, buttons, status displays)
- DiagnosticsSection.tsx (developer toggle, entity mode, schedule rows)

ActivityLogSection.tsx already uses CSS modules — use as reference pattern.

### Audit Log Pagination

Add cursor-based pagination to ActivityLogSection:
- Initial load: 50 records (not 200)
- "Load more" button at bottom
- Total count indicator
- Preserve filter state across pagination

### Component Consolidation

- Extract `StatusDot` to single shared component (currently defined 3+ times)
- Standardize button usage through styles.ts variants or CSS module classes

### Micro-Interaction Polish

- **YouCard loading state**: Single 40px skeleton bar is too subtle for 5 subsections — add per-section skeleton structure
- **HygieneSection loading states**: 3 redundant states (loading / no-scan / scan-complete) — clarify transitions, remove redundancy
- **Hygiene scan feedback**: Long operation with no progress indicator — add loading toast or inline progress
- **Mono labels overused**: Design system reserves mono for timestamps/metadata — several settings labels use mono as primary text; migrate to DM Sans
- **ConnectorDetail close button**: No hover feedback — add visible hover state
- **Advanced disclosure affordance**: Mono uppercase label identical to static labels — users may not recognize it's interactive; add hover state or visual distinction

### Vocabulary Fixes

- "Select your role to tailor vitals, vocabulary, and AI emphasis" → "Select your role to personalize what matters most"
- "daily narrative" → "daily summary"
- "Healed" → "Resolved"
- "tamper-evident" → "immutable record"
- Raw category names (`data_access`, `anomaly`) → friendly labels ("Data Access", "Unusual Activity")
- Glean strategy descriptions: remove "local signals", "enrichment"

## Acceptance Criteria

1. Zero `style={{}}` in any settings component. All styling via CSS modules or Tailwind.
2. YouCard split into 3 coherent sub-sections (Identity, Workspace, Preferences) with ChapterHeading-like visual separation.
3. Activity log paginates with "Load more" button. Initial load ≤ 50 records.
4. StatusDot defined once, used everywhere.
5. Zero ADR-0083 vocabulary violations in settings surface.
6. ContextSourceSection inset panel uses design tokens (not ad-hoc border/radius/background).
7. All interactive elements have visible hover/focus feedback.
8. FinisMarker present (already is — verify preserved).
9. YouCard loading skeleton matches section structure (not a single bar).
10. HygieneSection has clear, non-redundant loading/empty/loaded states.
11. Mono font used only for timestamps and metadata, not primary labels.

## Out of Scope

- Settings restructuring into separate routes/pages (post-GA consideration)
- Responsive/mobile layout
- Dark mode theming
