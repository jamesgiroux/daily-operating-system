# GA UX Audit — v1.0.0 Readiness Assessment

**Date:** 2026-03-11
**Scope:** Every surface, every component, every state, every document
**Verdict:** Not GA-ready. Fixable. ~15-20 hours of focused work.

---

## The Big Picture

DailyOS has strong bones. The editorial design language is well-defined, the best pages (Risk Briefing, Daily Briefing, Emails) prove the team can execute at A+ level. But the gap between best and worst is too wide for GA. MeetingDetailPage has 51 inline style violations. The Settings page conflates 5 unrelated concepts into one section. The audit log has no pagination. 11 of 23 pages have zero design documentation. 114 of 177 components aren't in the inventory.

The pattern is clear: features were built to work, not to be finished. That was correct for beta. It's not correct for 1.0.

---

## Severity Definitions

- **Critical** — Blocks GA. User-visible brokenness or design system violation that undermines trust.
- **High** — Should fix before GA. Craft issues that a discerning user would notice.
- **Medium** — Fix during GA stabilization. Won't block launch but adds debt.
- **Low** — Post-GA cleanup.

---

## I. Page Compliance Matrix

| Page | Structure | Tokens | Vocabulary | States | Finis | Overall | Blocking |
|------|-----------|--------|------------|--------|-------|---------|----------|
| Daily Briefing | A | A | A | A | A | **A-** | — |
| Weekly Forecast | A | A | **C** | A | A | **B+** | 1 vocab |
| Meeting Detail | A | **F** (51 inline) | **C** (3 violations) | A | A | **C+** | CRITICAL |
| Actions | B | **C** | A | B | **F** (missing) | **C** | CRITICAL |
| Account Detail | A | **B** | B | A | A | **B+** | 1 high |
| Project Detail | A | **B** | B | A | A | **B+** | 1 high |
| Person Detail | A | **B** | B | A | A | **B+** | 1 high |
| Risk Briefing | A | A | A | A | A | **A+** | — (reference) |
| Emails | A | A | A | A | A | **A** | — |
| Settings | A | **F** (all inline) | **C** (5+ violations) | C | A | **C-** | CRITICAL |
| Entity Lists | A | A | A | A | A | **A-** | — |
| Account Health | ? | ? | ? | ? | ? | **Undocumented** | — |
| Me / User Profile | ? | ? | ? | ? | ? | **Undocumented** | — |
| Reports (5 types) | ? | ? | ? | ? | ? | **Undocumented** | — |
| Inbox | ? | ? | ? | ? | ? | **Undocumented** | — |
| Action Detail | ? | ? | ? | ? | ? | **Undocumented** | — |

---

## II. Critical Issues (8 items — must fix)

### C1. MeetingDetailPage: 51 inline style violations
**File:** `src/pages/MeetingDetailPage.tsx`
**Problem:** Most inline-styled page in the app. CSS module exists (`meeting-intel.module.css`) but isn't fully used.
**Impact:** Unauditable for token compliance. Violates DESIGN-SYSTEM.md rule.
**Fix:** Migrate all `style={{}}` to CSS module. **Effort: 3-4 hours.**

### C2. Settings page: Information Architecture
**File:** `src/pages/SettingsPage.tsx`, `src/components/settings/YouCard.tsx`
**Problem:** YouCard conflates 5 unrelated settings (domains, role, workspace path, briefing time, personality). System section mixes version info, AI models, hygiene, capture, and data management into one disclosure. No coherent mental model.
**Impact:** User cannot predict where settings live. Cognitive load is high.
**Fix:** Reorganize into coherent groups:
- **Identity** — Name, company, title, role
- **Workspace** — Path, domains
- **Preferences** — Briefing time, personality
- **System Health** — Version, updates, database status
- **AI & Processing** — Models, hygiene, capture (or fold into System with clearer sub-sections)
**Effort: 3-4 hours.**

### C3. Settings page: Systemic inline style violations
**Files:** All 8 settings components
**Problem:** Every settings component uses `style={{}}`. ActivityLogSection is the sole exception (uses CSS module correctly — proves the team knows how).
**Impact:** Design system non-compliance across the entire settings surface.
**Fix:** Create `settings.module.css` and migrate. ActivityLogSection is the reference.
**Effort: 2-3 hours.**

### C4. ActionsPage: Missing FinisMarker
**File:** `src/pages/ActionsPage.tsx`
**Problem:** Uses custom "That's everything" footer instead of `<FinisMarker />`. Every editorial page must end with FinisMarker — non-negotiable per design system.
**Fix:** Replace custom footer with `<FinisMarker />`. **Effort: 5 minutes.**

### C5. Vocabulary violations (ADR-0083)
**Locations:**
- **WeekPage ~line 125:** `"${needsPrepCount} needs prep"` in folio bar → "needs context" or remove
- **MeetingDetailPage:** "Meeting Intelligence Report" → "Meeting Briefing"
- **MeetingDetailPage:** "Prep not ready yet" → "Not ready yet"
- **Settings/YouCard ~line 228:** "Select your role to tailor vitals, vocabulary, and AI emphasis" → "Select your role to personalize what matters most"
- **Settings/SystemStatus:** "daily narrative" → "daily summary"; "Healed" → "Resolved"
- **Settings/ContextSourceSection:** "local signals", "enrichment" in Glean strategy descriptions
- **Settings/ActivityLogSection ~line 208:** "tamper-evident" (security jargon) → "immutable record"
- **Settings/ActivityLogSection ~line 251:** Raw category names (`data_access`, `anomaly`) displayed as-is → needs friendly mapping
**Fix:** String replacements. **Effort: 30 minutes total.**

### C6. Activity Log: No pagination
**File:** `src/components/settings/ActivityLogSection.tsx`
**Problem:** Loads first 200 records, no "Load More" button, no cursor pagination. A user with 500+ audit events can't see older records.
**Impact:** Audit trail visibility — a GA security/trust feature — is incomplete.
**Fix:** Add cursor-based pagination with "Load More" button. **Effort: 1-2 hours.**

### C7. ActionsPage: Deceptive auto-archive tooltip
**File:** `src/pages/ActionsPage.tsx`
**Problem:** Tooltip claims "30+ days auto-archived" but no backend implements this.
**Impact:** Broken user expectation erodes trust.
**Fix:** Either implement backend auto-archive (I540 scoped this) or remove the tooltip. **Effort: 5 min to remove, 2-4 hours to implement.**

### C8. 11 pages completely undocumented
**Problem:** PAGE-ARCHITECTURE.md covers 10/23 pages. Account Health, EBR/QBR, SWOT, Weekly Impact, Monthly Wrapped, Me, Inbox, Action Detail, History, Report renderer, Meeting History redirect — all missing.
**Impact:** No stated JTBD, no compliance target, no state documentation. New developers build blind.
**Fix:** Document all pages using existing template. **Effort: 2-3 hours.**

---

## III. High-Priority Issues (9 items — should fix)

### H1. Account/Project/Person Detail: Inline style debt
Scattered inline styles + hardcoded color strings. Less severe than MeetingDetail but still violates the system. **Effort: 1-2 hours each.**

### H2. MeetingDetailPage: Hardcoded colors in CSS module
`meeting-intel.module.css` has `#d4a853` and `rgba(245, 240, 230, 0.3)` — should use token variables. **Effort: 5 minutes.**

### H3. Settings: Status dot component defined 3+ times
`StatusDot` in SettingsPage.tsx, `statusDot()` in styles.ts, inline divs in SystemStatus.tsx. Should be one component. **Effort: 30 minutes.**

### H4. Settings: Button styles fragmented
Some buttons use `styles.btn`/`styles.btnPrimary`, others define custom inline styles. Standardize through styles.ts or a CSS module. **Effort: 1 hour.**

### H5. Settings: Nested disclosures without clear affordance
"Advanced" disclosure in SystemStatus uses mono uppercase label identical to static section labels. Users may not recognize it's interactive. Add hover state or visual distinction. **Effort: 30 minutes.**

### H6. Settings: ContextSourceSection inset panel
Uses an ad-hoc container pattern (border + background + border-radius) not defined in the design system. Either define the pattern in tokens or remove the container. **Effort: 30 minutes.**

### H7. ActionsPage: Custom loading/error states
Uses inline-styled skeletons and error displays instead of `EditorialLoading`/`EditorialError`. **Effort: 30 minutes.**

### H8. Component Inventory: 114 components undocumented
177 actual components, 63 documented (36% coverage). Missing: 22 report components, 16 connector components, BriefingMeetingCard, IntelligenceQualityBadge, EditableText, many others. **Effort: 3 hours to audit and append.**

### H9. Missing design documents
These should exist before GA:
- **STATE-PATTERNS.md** — Per-page state matrices, empty/loading/error guidelines
- **INTERACTION-PATTERNS.md** — Inline editing, slide-deck nav, expansion, entity linking
- **DATA-PRESENTATION-GUIDELINES.md** — Lists vs. cards vs. rows decision tree
- **NAVIGATION-ARCHITECTURE.md** — FloatingNavIsland map, page connections, routing
**Effort: 4-6 hours total.**

---

## IV. Medium Issues (for GA stabilization)

| Issue | Location | Notes |
|-------|----------|-------|
| Drawer pattern not eliminated (I343) | Account/Project Detail | Drawers for field editing still active; inline editing model incomplete |
| Emails page not in FloatingNavIsland (I358) | Layout | No nav item to reach Emails directly |
| MeetingDetail folio bar transcript button | MeetingDetailPage | Should be body CTA for past meetings, not folio action |
| YouCard loading state too subtle | YouCard.tsx | Single 40px skeleton bar for 5 subsections |
| HygieneSection: 3 redundant loading states | SystemStatus.tsx | Loading / no-scan / scan-complete transitions unclear |
| ConnectorDetail close button lacks hover | ConnectorDetail.tsx | Button looks passive, no visual feedback |
| Role preset buttons: no click feedback | YouCard.tsx | Opacity drops but no confirmation signal |
| Hygiene scan: no loading toast | SystemStatus.tsx | Long operation with no progress indicator |
| Mono labels overused as primary text | Settings components | Design system reserves mono for timestamps/metadata |
| COMPONENT-INVENTORY.md dated 2026-02-22 | .docs/design/ | Major releases shipped since; listed as "no new components" |
| VIOLATIONS.md referenced but doesn't exist | .docs/design/ | Dead link from COMPONENT-INVENTORY.md |

---

## V. Proposed Issue Structure

Based on this audit, here's how to scope the work into shippable issues:

### New Issue: **I-UX-1 — Settings Page Rebuild**
- Reorganize IA (Identity / Workspace / Preferences / System)
- Migrate all inline styles to CSS module
- Add audit log pagination
- Fix vocabulary violations
- Fix button/status dot fragmentation
- **AC:** Zero inline `style={{}}` in settings components. Audit log paginates. Every section has clear JTBD title + epigraph.

### New Issue: **I-UX-2 — MeetingDetailPage Style Migration**
- Migrate 51 inline styles to meeting-intel.module.css
- Replace hardcoded colors with tokens
- Fix vocabulary ("Meeting Intelligence Report" → "Meeting Briefing")
- **AC:** Zero inline `style={{}}`. Zero hardcoded hex/rgba. Zero ADR-0083 violations.

### Expand I448: **ActionsPage Editorial Rebuild**
- Add FinisMarker
- Replace custom loading/error with EditorialLoading/EditorialError
- Remove or implement auto-archive tooltip
- **AC:** FinisMarker present. No deceptive tooltips. Uses editorial state components.

### Expand I454: **Vocabulary Pass**
- WeekPage "needs prep" label
- Settings vocabulary (all 8+ violations)
- ActivityLogSection category name mapping
- **AC:** `grep -rn` for forbidden terms returns zero user-visible hits.

### New Issue: **I-UX-3 — Design Documentation for GA**
- Document 11 missing pages in PAGE-ARCHITECTURE
- Update COMPONENT-INVENTORY with 114 missing components
- Create STATE-PATTERNS.md
- Create developer checklist for new pages/components
- **AC:** Every page in `src/pages/` has a corresponding entry in PAGE-ARCHITECTURE. Every component in `src/components/` is in COMPONENT-INVENTORY.

---

## VI. Effort Summary

| Category | Items | Effort |
|----------|-------|--------|
| Critical code fixes | C1-C7 | ~10-14 hours |
| Critical documentation | C8 | ~2-3 hours |
| High-priority code | H1-H7 | ~4-5 hours |
| High-priority documentation | H8-H9 | ~7-9 hours |
| **Total to GA-ready** | | **~15-20 hours** |

---

## VII. The Standard

RiskBriefingPage is A+. It proves the design system works when followed. The question isn't whether the system is right — it is. The question is whether every surface meets the standard the system sets.

For GA, the answer must be yes.
