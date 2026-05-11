# Project Detail · Variation D → Design System Gap Map

**Date:** 2026-05-10  
**Audit scope:** D-composite.html mockup (variations/D-composite.html) + project.css  
**Target system:** DailyOS design system (0.5.0+)  
**Dropped sections:** (1) local tab nav (Overview/Plan/Work), (2) vitals strip (Phase/Health/GA target/Owner/design partners) — not included below

---

## Mapping Table

| D Section | What it is | Existing DS entry | Gap / new entry needed | Notes |
|---|---|---|---|---|
| **1. Masthead** | Title + date + tier + meeting/account counts | `SurfaceMasthead` pattern | None | Uses `.masthead`, `.masthead-title`, `.masthead-sub`, `.hero-watermark`. Matches existing pattern API; tier badge uses `color-garden-olive-12` / `color-garden-olive`. Watermark glyph pattern `*` (mark element, separate from hero). |
| **2. The Lead** | Large serif sentence with terracotta `.sharp` highlight + eyebrow | `Lead` pattern | None | Uses `.vD-lead-eyebrow`, `.vD-lead-sentence`, `.sharp` with `color-spice-terracotta-15` underline gradient. Matches D-spine spec: serif 38px, eyebrow mono 10px, inline highlight. **Token present:** `--color-spice-terracotta-15` ✓ |
| **3. Meter cluster** | 4 horizontal meter bars (Momentum / Confidence / Open risks / Engagement) with axis label, value+tone, fill, trend | **New pattern:** `MeterCluster` | **NEW** — proposed `MeterCluster` pattern | Uses `.meter-cluster` (grid 4-col), `.meter-axis-label`, `.meter-value[data-tone]`, `.meter-bar` + `.fill[data-tone]`, `.meter-trend[.up/.down]`. Tones: `sage` (rosemary), `saffron`, `terracotta`, `larkspur`. CSS is in project.css, not in design tokens. Composes primitive `Pill` (tone-colored badges). |
| **4. Phase strip** | Month ticks, phase bars, milestone dots, "now" line, legend swatches, freshness line | **New pattern:** `PhaseTimeline` | **NEW** — proposed `PhaseTimeline` pattern | Uses `.phase-strip` (relative layout), `.phase-legend` (swatches), `.phase-month-ticks`, `.phase-bars` (colored bars `discover`/`build`/`beta`/`launch`/`warn`), `.phase-milestone[.done/.next]`, `.phase-milestone-label`, `.phase-now-line`. Month grid is 8-col repeat; phase bars positioned absolute. NOT `ChapterHeading` (that's section opener). Called "Plan at a glance" in D. No existing pattern covers timeline + phase rendering. |
| **5. Where it stands grid** | 4-quadrant grid (What's working / not / unclear / need) | **New pattern:** `FindingsGrid` or compose `MarginGrid` | **NEW** — proposed `FindingsGrid` pattern | Uses `.vD-stand-grid` (CSS grid 2x2, `gap: var(--space-2xl)`). Each cell: mono label (10px uppercase) + serif body (17px, font-weight 400). Matches `MarginGrid` composition but this is a distinct "stand assessment" grid (not margin+content, but four equal quadrants). Could call it `StatusAssessmentGrid` or `FindingsQuad`. CSS in local style block. |
| **6. Chapter heading** | "The plan, at a glance" section opener | `ChapterHeading` pattern | None | Uses `.chapter`, `.chapter-rule`, `.chapter-title`. Matches existing pattern exactly. Followed by `.freshness` metadata line (mono small text with sep, alert styling). `ChapterHeading` + `FreshnessLine` (see primitives). |
| **7. Freshness line** | "Mar 4 → Jul 15 · 4 phases · 7 milestones · 1 milestone slipping" | **New primitive:** `FreshnessLine` | **NEW** — proposed `FreshnessLine` primitive | Uses `.freshness` (mono 11px, color text-tertiary, flex gap 10px, flex-wrap). Children: text, `.sep` (opacity 0.4), `.alert` (color spice-terracotta). Reused pattern. Is a simple line, not a chapter heading. Treated as primitive `FreshnessLine` or inline in `ChapterHeading` context. |
| **8. Activity ledger** | List of timestamped events with type-dot, body, meta-chips | `ActivityLogSection` pattern (from Settings) or new `ActivityLedger` pattern | **POTENTIAL REUSE** but structure differs | Uses `.vD-activity` (list, grid 3-col: 92px time / 22px dot / 1fr body). `.vD-activity .when` (mono 10.5px uppercase), `.typedot[.t-meeting/.t-decision/.t-email/.t-action/.t-event]` (7px circle, color-coded), `.body` (serif 16px), `.meta` (mono 10px, flex, ent-chips). `EntityChip` primitive reused. ActivityLogSection is Settings-audit-log-specific; this is a project-event timeline. Recommend new pattern `ActivityLedger` or `EventTimeline` for project/account narratives. |
| **9. Outcome callout** | Olive band with outcome statement + signoff | **New pattern:** `SuccessOutcome` or `OutcomeBlock` | **NEW** — proposed `SuccessOutcome` pattern | Uses `.vD-outcome` (padding, background `color-garden-olive-10`, border `color-garden-olive-12`, border-radius sm). Children: `.label` (mono 10px, color olive), `p` (serif 22px, font 400), `.signoff` (mono 10px). **Tokens present:** `--color-garden-olive-10`, `--color-garden-olive-12` ✓ Matches existing token coverage for callout styling. |
| **10. Commercial sub-card** | Dense ref grid (Type / Design partners / Committed ARR / Pilot terms / GA date / Pricing / MSA / Steering cadence) | **New pattern:** `RefGrid` or surface-internal | **NEW** — proposed `ReferenceGrid` pattern | Uses `.vD-commercial` (padding, border, border-radius sm, background cream), `.ref-heading` (mono 11px 600 weight uppercase), `.ref-grid` (grid 4-col, gap lg / 2xl), `.ref-field-label` (mono 10px uppercase), `.ref-field-value` (sans 14px). Pairs label + value in dense matrix. Appears to be a reusable reference-information pattern. Recommend `ReferenceGrid` primitive / pattern. |
| **11. Stakeholder cards** | Grid of cards with avatar, name, title, roles, assessment, engagement dots, last-seen | **New pattern:** `StakeholderCard` or `StakeholderGallery` | **Check existing** `StakeholderGallery` | Uses `.stake-grid` (grid), `.stake-card` (article). Children: `.stake-avatar`, `.stake-name` + link, `.stake-title`, `.stake-roles` (.stake-role[data-role]), `.stake-assessment`, `.stake-engagement[data-level]`, `.stake-last-seen`. `EntityChip` for account link. **Potential match:** `StakeholderGallery.module.css` exists in reference styles — check if spec covers all these sub-structures. If not, may need pattern spec reconciliation. |
| **12. Decision list** | Ordered list of dated decisions with text + source | **New pattern:** `DecisionLog` or check existing | **NEW** — proposed `DecisionLog` pattern | Uses `.decision-list` (ordered, no bullets), `.decision-list li` (3-col grid: 80px when / 1fr text / auto source), `.decision-when` (mono 10px), `.decision-text` (serif 15.5px), `.decision-text em` (italic, text-secondary), `.decision-source` (mono 10px). Ordered timeline of authoritative decisions. No existing pattern found. Recommend `DecisionLog` pattern. |
| **13. About this dossier** | Meta section explaining assembly, flagged assessments | **New pattern:** `DossierMetadata` or `AboutThisDossier` | **Check existing** `AboutThisDossier` | Uses `.meta-section` (margin top 4xl, padding, background desk-charcoal-4, border-radius sm), `.meta-label` (mono 10px uppercase), `.meta-body` (serif italic 15px, font-weight 300, text-secondary). Similar metadata pattern exists in AccountDetail. Check `DossierSourceCoveragePanel` / `AboutThisDossier` for consistency. Likely reusable. |
| **14. Finis marker** | `* * *` signoff + date + caption | `FinisMarker` pattern | None | Uses `.finis` (article), `.finis-mark` (the three marks), `.finis-date`, `.finis-caption`. Matches existing integrated pattern exactly. Applied at end of editorial surfaces. |

---

## Token Gap Analysis

**Tokens used in D-composite.html not in canonical tokens.css:**

| Token reference | Found in DS | Status | Notes |
|---|---|---|---|
| `--color-spice-terracotta-15` | ✓ Yes | Present | Used in Lead `.sharp` highlight |
| `--color-garden-olive-10` | ✓ Yes | Present | Used in outcome callout bg |
| `--color-garden-olive-12` | ✓ Yes | Present | Used in outcome callout border; masthead tier badge |
| `--color-garden-rosemary` | ✓ Yes | Present | Used for phase bar `.sw-launch`, meter tone `sage` |
| `--color-garden-sage` | ✓ Yes | Present | Used for meter fill tone |
| `--color-spice-saffron` | ✓ Yes | Present | Used for meter tone, vitals highlight |
| `--color-spice-turmeric` | ✓ Yes | Present | Used for phase bar `.sw-beta`, activity dot, entity chip |
| `--color-paper-cream` | ✓ Yes | Present | Used in phase now-line bg, commercial card bg |
| `--color-paper-linen` | ✓ Yes | Present | Used for phase bar `.sw-discover` |
| `--color-rule-heavy` | ✓ Yes | Present | Used for borders, rule lines |
| `--color-rule-light` | ✓ Yes | Present | Used for activity list borders |
| `--color-desk-charcoal-4` | ✓ Yes | Present | Used for meta section bg, meter bar bg |
| `--color-text-[primary/secondary/tertiary]` | ✓ Yes | Present | Used throughout |
| `--font-mono`, `--font-serif`, `--font-sans` | ✓ Yes | Present | Used throughout |
| `--space-[sm/md/lg/xl/2xl/4xl]` | ✓ Yes | Present | Used throughout |
| `--radius-sm` | ✓ Yes | Present | Used for border-radius |
| `--transition-[fast/normal]` | ✓ Yes | Present | Used in phase-bar hover |
| `--shadow-md` | ✓ Yes | Present | Used in phase-bar hover |

**Conclusion:** All color, type, and spacing tokens referenced in D-composite + project.css exist in the canonical tokens.css. No token gaps.

---

## Summary of Gaps

**Tier breakdown:**

| Tier | Category | Count | Proposals |
|---|---|---|---|
| **Pattern** | Existing, no gap | 4 | `SurfaceMasthead`, `Lead`, `ChapterHeading`, `FinisMarker` |
| **Pattern** | New, proposed | 6 | `MeterCluster`, `PhaseTimeline`, `FindingsGrid`, `ActivityLedger`, `SuccessOutcome`, `DecisionLog` |
| **Pattern** | Check existing | 3 | `StakeholderGallery` (exists, verify completeness), `ReferenceGrid` (exists as local pattern, promote?), `DossierMetadata` / `AboutThisDossier` (exists, verify match) |
| **Primitive** | New, proposed | 1 | `FreshnessLine` (or inline in ChapterHeading context) |
| **Token** | Gap | 0 | All tokens present ✓ |

**Next step:** Draft `project-d-spine.html` reference using existing patterns + 6 proposed pattern stubs (with CSS from project.css as source material for elevation to pattern specs).

---

## CSS salvageability

**project.css sections that are promote-ready:**

- `.masthead*` — mirrors existing pattern, minimal change needed
- `.vD-lead*` — extract to `Lead.module.css` fragment (highlight gradient technique)
- `.meter-cluster`, `.meter-*` — **salvageable** as `MeterCluster.module.css` (complete, self-contained)
- `.phase-*`, `.vD-phase-frame` — **salvageable** as `PhaseTimeline.module.css` (complete, self-contained)
- `.vD-stand-grid`, `.vD-stand-grid .label`, `.vD-stand-grid p` — **salvageable** as `FindingsGrid.module.css`
- `.vD-activity*` — **salvageable** as `ActivityLedger.module.css` (status-dot coloring already exists)
- `.vD-outcome*` — **salvageable** as `SuccessOutcome.module.css`
- `.vD-commercial*`, `.ref-*` — **salvageable** as `ReferenceGrid.module.css`
- `.stake-*` — check against existing `StakeholderGallery.module.css` (likely 80% overlap)
- `.decision-*` — **salvageable** as `DecisionLog.module.css` (ordered list layout)
- `.meta-*` — check against `DossierSourceCoveragePanel` / existing meta patterns (partial overlap)
- `.finis*` — mirrors existing pattern

**Total effort:** Medium. 6 new module CSS files, 3 verification checks. All CSS is already in project.css; promotion is extraction + naming.
