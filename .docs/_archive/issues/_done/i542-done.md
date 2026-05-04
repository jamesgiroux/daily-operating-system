# I542 — MeetingDetailPage Style Migration + Vocabulary

**Priority:** P1
**Area:** Frontend / Meeting Detail / UX
**Version:** v1.0.0 (Phase 3d, Wave 4)
**Depends on:** I447, I521

## Problem

MeetingDetailPage has the worst design system compliance in the app. 51 inline `style={{}}` props scattered throughout the component. The CSS module (`meeting-intel.module.css`) exists and has infrastructure but isn't fully used. Hardcoded hex colors in the CSS module (`#d4a853`, `rgba(245, 240, 230, 0.3)`). Three vocabulary violations visible to users.

This is the most-visited page after the daily briefing — every meeting click lands here. The craft gap between this page and RiskBriefingPage (A+) is unacceptable for GA.

## Scope

### Style Migration

Migrate all 51 inline `style={{}}` usages to `meeting-intel.module.css`. Categories:
- Layout containers (flex, gap, padding)
- Typography overrides (font-family, font-size, color)
- Decorative elements (borders, backgrounds, opacity)
- Interactive states (hover, active, disabled)

### Token Compliance

Replace hardcoded colors in meeting-intel.module.css:
- `#d4a853` → `var(--color-spice-turmeric)`
- `rgba(245, 240, 230, 0.3)` → turmeric tint token (or `var(--color-paper-cream)` at opacity)
- Audit for any other hardcoded hex/rgba values

### Vocabulary Fixes

- "Meeting Intelligence Report" kicker text → "Meeting Briefing"
- "Prep not ready yet" → "Not ready yet"
- Folio bar transcript button for past meetings → move to body CTA only

## Acceptance Criteria

1. Zero `style={{}}` in MeetingDetailPage.tsx.
2. Zero hardcoded hex or rgba values in meeting-intel.module.css (all via design tokens).
3. Zero ADR-0083 vocabulary violations on the meeting detail surface.
4. Folio bar: no transcript button for past meetings (body CTA only).
5. Visual parity: page looks identical before and after migration (no regressions).
6. Parity gate passes for meeting detail surface.

## Out of Scope

- Meeting detail content/logic changes
- New meeting detail features
- The Room section redesign
