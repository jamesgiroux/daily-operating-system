# Design System Violations

**Last audited:** 2026-02-20
**Total violations:** 47
**Ship-blocking:** 12

This is the punch list. Fix these before they metastasize.

---

## Severity Scale

| Level | Meaning | Action |
|-------|---------|--------|
| **CRITICAL** | Breaks product promise or misleads user | Fix before any other UI work |
| **HIGH** | Violates design system in visible, compounding way | Fix in next sprint |
| **MEDIUM** | Inconsistency that erodes quality over time | Fix when touching the file |
| **LOW** | Nitpick or edge case | Fix opportunistically |

---

## CRITICAL (5 violations)

### V-001: Deceptive auto-archive tooltip
**File:** `src/pages/ActionsPage.tsx:474`
**What:** Tooltip says "Actions pending for 30+ days are automatically archived"
**Problem:** No backend expiry logic exists. This is a lie in the UI.
**Fix:** Remove the tooltip text immediately. Restore only when the backend ships.

### V-002: "Generate Briefing" button text
**File:** `src/components/dashboard/DashboardEmpty.tsx:126`
**What:** Button says "Generate Briefing"
**Fix:** "Prepare my day" (per ADR-0083)

### V-003: "Meeting Intelligence Report" kicker
**File:** `src/pages/MeetingDetailPage.tsx:~575`
**What:** Page header says "Meeting Intelligence Report"
**Fix:** "Meeting Briefing" (per ADR-0083)

### V-004: "Prep not ready yet" text
**File:** `src/pages/MeetingDetailPage.tsx:~411`
**What:** Loading state says "Prep not ready yet"
**Fix:** "Not ready yet" or "Context building" (per ADR-0083)

### V-005: AccountFieldsDrawer still active
**File:** `src/pages/AccountDetailEditorial.tsx:50`
**What:** AccountFieldsDrawer is imported and fully rendered (~lines 449-486)
**Problem:** ADR-0084 section 5 says "eliminate field-editing drawers." I343 core deliverable.
**Fix:** Implement inline editing on hero/vitals fields. Delete drawer.

---

## HIGH (12 violations)

### V-006: ProjectFieldsDrawer still active
**File:** `src/pages/ProjectDetailEditorial.tsx:33`
**What:** Same as V-005 but for projects.
**Fix:** Inline editing. Delete drawer.

### V-007: status-badge.tsx hardcoded colors (24 instances)
**File:** `src/components/ui/status-badge.tsx`
**What:** `healthStyles`, `projectStatusStyles`, `progressStyles` objects contain hardcoded hex values and rgba strings.
**Example:** `"bg-[rgba(126,170,123,0.12)] text-[#4a6741] border-[rgba(126,170,123,0.3)]"`
**Fix:** Create CSS custom properties for opacity variants:
```css
--color-garden-sage-12: rgba(126, 170, 123, 0.12);
--color-garden-sage-30: rgba(126, 170, 123, 0.30);
```
Then reference them in Tailwind: `bg-[var(--color-garden-sage-12)]`

### V-008: "Read full intelligence" link text (2 locations)
**Files:**
- `src/components/dashboard/DailyBriefing.tsx:357`
- `src/components/dashboard/BriefingMeetingCard.tsx:~508`
**What:** Link says "Read full intelligence"
**Fix:** "Read full briefing" (per ADR-0083)

### V-009: "proposed" tab label
**File:** `src/pages/ActionsPage.tsx:64`
**What:** Tab displays "proposed"
**Fix:** "Suggested" (per ADR-0083). Internal type stays `"proposed"`.

### V-010: "AI Suggested" label
**File:** `src/pages/ActionsPage.tsx:~535`
**What:** Label says "AI Suggested"
**Fix:** "Suggested" (per ADR-0083)

### V-011: Folio bar transcript button for past meetings
**File:** `src/pages/MeetingDetailPage.tsx:347-356`
**What:** Sync button in folio bar for `isPastMeeting`
**Decision:** Body-level CTA only (ADR-0084 / I342)
**Fix:** Remove `{isPastMeeting && <button onClick={handleSyncTranscript}>}` from folio actions.

### V-012: IntelligenceQualityBadge wrong vocabulary
**File:** `src/components/ui/IntelligenceQualityBadge.tsx` (if using old format)
**What:** Uses Fresh/Recent/Stale instead of New/Building/Ready/Updated
**Fix:** Map to ADR-0083 product vocabulary. Check if 0.13.0 work already fixed this.

### V-013: hasPrep boolean dot on schedule rows
**File:** `src/components/dashboard/BriefingMeetingCard.tsx:431`
**What:** Binary `hasPrep` dot with title "Prep available"
**Fix:** Replace with IntelligenceQualityBadge component. Check if 0.13.0 work addressed this.

### V-014: Resolution Keywords visible in UI
**File:** `src/components/entity/EntityKeywords.tsx:85`
**What:** Renders label "Resolution Keywords"
**Fix:** Remove from UI entirely. Entity matching works silently.

### V-015: MeetingDetailPage inline style debt
**File:** `src/pages/MeetingDetailPage.tsx` (throughout)
**What:** Hundreds of lines of inline `style={{}}` with hardcoded px values, rgba colors, and font sizes
**Fix:** Extract to CSS module following `editorial-briefing.module.css` pattern.

### V-016: ActionsPage missing FinisMarker
**File:** `src/pages/ActionsPage.tsx:413`
**What:** Uses custom "That's everything" footer instead of `<FinisMarker />`
**Fix:** Replace with `<FinisMarker />` for consistency.

### V-017: Entity hero "Build Intelligence" button text
**Files:** AccountHero, ProjectHero, PersonHero (various lines)
**What:** Button says "Build Intelligence"
**Fix:** "Check for updates" (per ADR-0083)

---

## MEDIUM (18 violations)

### V-018 through V-023: Hardcoded rgba colors in page files

Scattered rgba values that should be design tokens:

| File | Example | Should be |
|------|---------|-----------|
| AccountDetailEditorial.tsx:373 | `rgba(222, 184, 65, 0.08)` | Saffron 8% opacity token |
| PersonDetailEditorial.tsx:344 | `rgba(143, 163, 196, 0.15)` | Larkspur 15% opacity token |
| MeetingDetailPage.tsx:819 | `rgba(106, 135, 171, 0.08)` | Larkspur-adjacent, wrong value |
| WeekPage.tsx (various) | Hardcoded rgba backgrounds | Should use tokens |

**Fix:** Create opacity variant tokens in `design-tokens.css`:
```css
/* Opacity variants — 8%, 12%, 15%, 30% */
--color-spice-turmeric-8: rgba(201, 162, 39, 0.08);
--color-spice-turmeric-12: rgba(201, 162, 39, 0.12);
--color-garden-larkspur-8: rgba(143, 163, 196, 0.08);
--color-garden-larkspur-15: rgba(143, 163, 196, 0.15);
--color-garden-sage-12: rgba(126, 170, 123, 0.12);
--color-garden-sage-30: rgba(126, 170, 123, 0.30);
--color-spice-terracotta-8: rgba(196, 101, 74, 0.08);
--color-spice-terracotta-12: rgba(196, 101, 74, 0.12);
```

### V-024 through V-029: Magic number spacing values

Inline spacing values that don't match the `--space-*` scale:

| File | Value | Nearest token |
|------|-------|---------------|
| AccountDetailEditorial.tsx:139 | `gap: 8` | `--space-sm` (8px) — OK but should use token |
| ActionsPage.tsx:240 | `gap: 20` | Use `--space-lg` (24px) or `--space-md` (16px) |
| ActionsPage.tsx:172 | `marginBottom: 12` | Use `--space-sm` (8px) or `--space-md` (16px) |
| PersonDetailEditorial.tsx:328 | `gap: 12` | Use `--space-sm` (8px) or `--space-md` (16px) |
| MeetingDetailPage.tsx (various) | Multiple arbitrary px | Full audit needed during CSS module migration |

### V-030: No opacity variant tokens defined
**File:** `src/styles/design-tokens.css`
**What:** The design tokens file defines base colors but no opacity variants.
**Impact:** Every file that needs a tinted background reinvents the rgba() call.
**Fix:** Add opacity variant section to design-tokens.css (see V-018 fix above).

### V-031: Margin grid not formalized
**What:** DailyBriefing and EmailsPage use CSS module classes (`s.marginGrid`, `s.marginLabel`, `s.marginContent`). Other pages use ad-hoc layouts.
**Fix:** Extract margin grid pattern into a shared CSS module or document as the standard.

### V-032: "Account/Project/Person Intelligence" labels in heroes
**Files:** All three hero components
**What:** Intelligence timestamp labels. "Account Intelligence" etc.
**Fix:** Remove labels or show timestamp only (per ADR-0083).

### V-033: Deep work block count in Day Frame
**File:** `src/components/dashboard/DailyBriefing.tsx` (capacity line)
**What:** References `.availableBlocks.filter(b => b.durationMinutes >= 60).length`
**Problem:** Open Time was cut from Weekly Forecast. Deep work blocks may not make sense in Day Frame.
**Fix:** Decide: keep as useful capacity context or remove.

---

## LOW (12 violations)

### V-034: tooltip.tsx hardcoded 2px radius
**File:** `src/components/ui/tooltip.tsx:253`
**What:** `rounded-[2px]` for arrow styling
**Impact:** Negligible — arrow visual effect

### V-035: sidebar.tsx still in codebase
**File:** `src/components/ui/sidebar.tsx`
**What:** Legacy sidebar component. AppSidebar was removed but the primitive remains.
**Impact:** Dead weight. Not imported.

### V-036 through V-047: Minor inline style instances
Various files with small inline style uses that should eventually migrate to CSS modules or Tailwind classes. Non-urgent but should be cleaned up when touching these files.

---

## Remediation Priority

### Immediate (before next ship)

1. V-001: Remove deceptive tooltip
2. V-002 through V-004: Vocabulary fixes (string changes only)
3. V-008 through V-010: Vocabulary fixes (string changes only)
4. V-014: Remove Resolution Keywords from UI
5. V-017: "Build Intelligence" → "Check for updates"
6. V-032: Remove intelligence labels from heroes

### Next Sprint

7. V-005, V-006: Replace field drawers with inline editing (I343)
8. V-007: Refactor status-badge.tsx colors
9. V-011: Remove folio transcript button
10. V-016: Add FinisMarker to ActionsPage

### Ongoing (fix when touching)

11. V-015: MeetingDetailPage CSS module migration
12. V-018 through V-029: Hardcoded colors and spacing
13. V-030: Create opacity variant tokens
14. V-031: Formalize margin grid

---

## Tracking

When a violation is fixed, add a line here:

| ID | Fixed in | By | Date |
|----|----------|----|------|
| — | — | — | — |
