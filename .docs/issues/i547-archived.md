# I547 — Book of Business Review Report

**Version:** v1.0.0 Phase 4
**Depends on:** I499 (health scoring), I503 (health schema), I508a (intelligence schema), I502 (health surfaces)
**Type:** Feature — new report type
**Scope:** Backend report generator + frontend renderer + accounts page integration

---

## Context

CSMs and TAMs prepare Book of Business (BoB) reviews for leadership on a regular cadence (monthly or quarterly). These reviews synthesize the entire portfolio into a single document: aggregate financials, account-level snapshots, risk/opportunity analysis, cross-book themes, value delivered, and leadership asks. Today this is done manually in Google Slides — taking hours to assemble from scattered notes, meetings, and memory.

DailyOS already has per-account intelligence (health scores, assessments, stakeholder data, meeting history, email signals, actions). A BoB review is the natural aggregation of this data into a portfolio-level narrative. The intelligence is already computed — this report just needs to collect it, synthesize cross-account themes, and present it in the editorial format.

Reference materials: Three real BoB reviews (Alan Ryan/TAM, Mike Poland/TAM, Natasha Wright/RM) plus the standard template were analyzed to derive the canonical structure below.

---

## Report Structure

The Book of Business review is a **user-scoped report** (`entity_type: "user"`) that aggregates across all accounts in the user's book. It lives alongside Weekly Impact and Monthly Wrapped as a user-level report, but is triggered on-demand (not auto-scheduled).

### Sections

**1. Executive Summary**

Top-of-report vitals strip with computed metrics + AI narrative:

| Metric | Source | Notes |
|--------|--------|-------|
| Total accounts | `COUNT(entities WHERE type='account' AND archived=0)` | Active accounts only |
| Total ARR | `SUM(arr)` from entity_quality or account metadata | Null ARR accounts noted |
| At-risk ARR | `SUM(arr WHERE health_band IN ('at_risk', 'declining'))` | From health scoring (I499) |
| Upcoming renewals | Accounts with `renewal_date` within next 90 days | Count + total ARR |
| Top 3 risks | AI-synthesized from account assessments | With account name + ARR |
| Top 3 expansion opportunities | AI-synthesized from account assessments + signals | With account name + estimated value |
| Leadership asks (Y/N + count) | AI-extracted from assessments + recent actions | Binary flag + detail in section 7 |

Plus a 2-3 sentence AI narrative summarizing the state of the book.

**2. Account Snapshot**

Table of all active accounts, sorted by ARR descending:

| Column | Source |
|--------|--------|
| Account name | `entities.name` |
| ARR | `entity_quality.arr` or account metadata |
| Health band | From I499 health scoring |
| Health trend | From I499 (improving/stable/declining) |
| Renewal date | Account metadata |
| Key contact | Primary stakeholder from `account_stakeholders` |

This is a data table, not AI-generated — pulled directly from the DB.

**3. High Value Accounts — Deep Dives**

AI-generated per-account summaries for top accounts (by ARR or risk/opportunity signal). Each entry contains:
- Account name, ARR, renewal date
- Current status narrative (2-3 sentences)
- Active workstreams (bulleted)
- Renewal impact / growth opportunity
- Risks & gaps

The number of deep dives scales with portfolio size: 3-5 for small books (< 10 accounts), 5-8 for larger books. Accounts selected by: (a) highest ARR, (b) highest risk, (c) highest expansion signal.

**4. Value Delivered**

Table of headline outcomes across the book:

| Column | Source |
|--------|--------|
| Account | Entity name |
| Headline outcome | AI-synthesized from recent meeting transcripts, captures, and actions |
| Why it matters | AI-generated business impact framing |

Sources: `meeting_transcripts` (90 days), `entity_assessment.executive_assessment`, completed actions, captures marked as wins.

**5. Key Themes Across the Book**

3-4 cross-cutting themes identified by AI from the aggregate intelligence. Each theme includes:
- Theme title
- 2-3 paragraph narrative with specific account examples cited
- Pattern identification (what's happening across multiple accounts)

This is the highest-value AI section — it surfaces patterns the CSM might not see when looking at accounts individually.

**6. Decisions & Leadership Asks**

Table of items requiring leadership attention:

| Column | Source |
|--------|--------|
| Ask / Decision | AI-extracted from assessments, actions, and meeting context |
| Context | 1-2 sentence explanation |
| Impacted accounts | Account names linked |
| Status | AI-inferred: pending / awaiting customer / escalated |

**7. Appendix — Account Technical Details** (optional)

Brief per-account status cards for accounts not covered in deep dives. Lighter weight than section 3 — just current status + key contact + next action.

---

## Data Gathering

The report generator follows the established two-phase pipeline pattern:

**Phase 1 — Data Gather (brief DB lock):**
- All active accounts with health scores, ARR, renewal dates, lifecycle
- All account stakeholders (primary contacts)
- Entity assessments for all accounts (executive_assessment, enriched_at)
- Recent meeting history across all accounts (90 days)
- Open actions across all accounts
- Recent captures (wins, risks, decisions) across all accounts
- Email signal counts per account
- User context (role, title, priorities)

**Phase 2 — AI Synthesis (no DB lock):**
- PTY call with all gathered context
- Prompt instructs: synthesize portfolio view, identify cross-account themes, extract leadership asks
- Output schema enforced via JSON schema (see below)

**Phase 3 — Store (brief DB lock):**
- Upsert to `reports` table with `report_type = 'book_of_business'`, `entity_type = 'user'`
- `intel_hash` computed across all account assessments (aggregate hash)

---

## Content Schema

### Rust (backend)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookOfBusinessContent {
    /// Report period label, e.g., "March 2026" or "Q1 2026"
    pub period_label: String,

    /// Executive summary narrative (2-3 sentences)
    pub executive_summary: String,

    /// Computed portfolio metrics
    pub total_accounts: u32,
    pub total_arr: Option<f64>,
    pub at_risk_arr: Option<f64>,
    pub upcoming_renewals: u32,
    pub upcoming_renewals_arr: Option<f64>,

    /// Top risks with account attribution
    pub top_risks: Vec<BookRiskItem>,

    /// Top expansion opportunities
    pub top_opportunities: Vec<BookOpportunityItem>,

    /// Whether leadership asks exist
    pub has_leadership_asks: bool,

    /// Account snapshot table (data, not AI)
    pub account_snapshot: Vec<AccountSnapshotRow>,

    /// Deep dive accounts (AI-generated per-account summaries)
    pub deep_dives: Vec<AccountDeepDive>,

    /// Value delivered table
    pub value_delivered: Vec<ValueDeliveredRow>,

    /// Cross-book themes (AI-synthesized)
    pub key_themes: Vec<BookTheme>,

    /// Leadership asks table
    pub leadership_asks: Vec<LeadershipAsk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookRiskItem {
    pub account_name: String,
    pub risk: String,
    pub arr: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookOpportunityItem {
    pub account_name: String,
    pub opportunity: String,
    pub estimated_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSnapshotRow {
    pub account_id: String,
    pub account_name: String,
    pub arr: Option<f64>,
    pub health_band: Option<String>,
    pub health_trend: Option<String>,
    pub renewal_date: Option<String>,
    pub key_contact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDeepDive {
    pub account_id: String,
    pub account_name: String,
    pub arr: Option<f64>,
    pub renewal_date: Option<String>,
    pub status_narrative: String,
    pub active_workstreams: Vec<String>,
    pub renewal_or_growth_impact: String,
    pub risks_and_gaps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueDeliveredRow {
    pub account_name: String,
    pub headline_outcome: String,
    pub why_it_matters: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookTheme {
    pub title: String,
    pub narrative: String,
    pub cited_accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeadershipAsk {
    pub ask: String,
    pub context: String,
    pub impacted_accounts: Vec<String>,
    pub status: Option<String>,
}
```

### TypeScript (frontend mirror)

```typescript
export interface BookOfBusinessContent {
  periodLabel: string;
  executiveSummary: string;
  totalAccounts: number;
  totalArr: number | null;
  atRiskArr: number | null;
  upcomingRenewals: number;
  upcomingRenewalsArr: number | null;
  topRisks: { accountName: string; risk: string; arr: number | null }[];
  topOpportunities: { accountName: string; opportunity: string; estimatedValue: string | null }[];
  hasLeadershipAsks: boolean;
  accountSnapshot: AccountSnapshotRow[];
  deepDives: AccountDeepDive[];
  valueDelivered: ValueDeliveredRow[];
  keyThemes: { title: string; narrative: string; citedAccounts: string[] }[];
  leadershipAsks: { ask: string; context: string; impactedAccounts: string[]; status: string | null }[];
}
```

---

## Frontend

### Entry Point — Accounts Page

The BoB review button lives on the **AccountsPage** in the FolioBar actions slot, alongside the existing Archive and + New buttons. It appears as a FolioBar action button styled consistently with the existing report trigger pattern.

**Button behavior:**
- Click navigates to `/me/reports/book_of_business`
- Uses existing `ReportShell` for generation UI, staleness banner, regenerate
- Label: "Book of Business" (preset-aware — see vocabulary below)

### Report Page — `BookOfBusinessReport.tsx`

New component following the editorial report pattern (like `AccountHealthReport.tsx`). Sections rendered in order:

1. **Header** — Report title, user name/role, period label, generation date
2. **Executive Summary** — Vitals strip (metrics cards) + narrative paragraph
3. **Account Snapshot** — Editorial data table with health badges, ARR formatting, renewal dates
4. **Deep Dives** — Per-account cards with section rule separators, not boxed cards (per ADR-0073)
5. **Value Delivered** — Editorial table with account name, outcome, impact columns
6. **Key Themes** — Each theme as a ChapterHeading + narrative paragraphs with cited account names highlighted
7. **Leadership Asks** — Editorial table with ask, context, impacted accounts, status badge
8. **FinisMarker** — Required per Phase 3 standards

### Inline Editing (Live Edit)

All AI-generated text fields are editable inline using the existing `EditableText` component, following the same pattern as Account Health, EBR/QBR, SWOT, Risk Briefing, and Weekly Impact reports. The BoB review is a document the CSM will refine before presenting to leadership — editing is essential.

**Editable fields:**

| Section | Editable fields | Component |
|---------|----------------|-----------|
| Executive Summary | `executiveSummary` narrative | `EditableText` (multiline) |
| Deep Dives | `statusNarrative`, `renewalOrGrowthImpact`, each `activeWorkstreams[]` item, each `risksAndGaps[]` item | `EditableText` |
| Value Delivered | `headlineOutcome`, `whyItMatters` per row | `EditableText` |
| Key Themes | `title`, `narrative` per theme | `EditableText` (multiline for narrative) |
| Leadership Asks | `ask`, `context`, `status` per row | `EditableText` |
| Top Risks | `risk` per item | `EditableText` |
| Top Opportunities | `opportunity` per item | `EditableText` |

**Not editable:**
- Computed metrics (total accounts, total ARR, at-risk ARR, upcoming renewals) — these are DB facts
- Account snapshot table rows — data-driven, not AI-generated
- Account names and ARR values in deep dives — sourced from DB

**Persistence:** Edits call `save_report` (existing Tauri command) which writes the modified `content_json` back to the `reports` table. The `EditableText` component emits `editable-text:commit` events on blur/Enter. Each editable field receives an `onChange` callback that updates the in-memory content state, and the parent component debounces a `save_report` call to persist.

**Keyboard navigation:** Tab/Shift+Tab moves between editable fields in document order (existing `EditableText` behavior via `data-editable-text` attribute). Escape cancels. This is the same UX as all other reports.

### Routing

- Route: `/me/reports/book_of_business`
- Follows existing pattern: `ReportPage.tsx` dispatches to `BookOfBusinessReport.tsx` based on `reportType`

### Preset Vocabulary

| Preset | Report label | Account noun |
|--------|-------------|--------------|
| customer-success | Book of Business | account |
| sales | Book of Business | deal |
| agency | Client Portfolio Review | client |
| consulting | Engagement Portfolio | engagement |
| partnerships | Partner Portfolio | partner |
| leadership | Portfolio Review | account |

Add `book_of_business` to `PRESET_REPORTS` in `report-config.ts` for all presets that have account-level reports.

---

## Backend

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/src/reports/book_of_business.rs` | Data gathering + prompt construction + response parsing |

### Modified Files

| File | Change |
|------|--------|
| `src-tauri/src/reports/mod.rs` | Add `BookOfBusiness` to `ReportType` enum |
| `src-tauri/src/reports/generator.rs` | Add `book_of_business` dispatch in generation match |
| `src-tauri/src/reports/prompts.rs` | Add BoB-specific preamble |
| `src-tauri/src/commands/planning_reports.rs` | No change needed — generic `generate_report` handles it |
| `src/types/reports.ts` | Add `BookOfBusinessContent` type |
| `src/lib/report-config.ts` | Add `book_of_business` to preset report lists |
| `src/pages/ReportPage.tsx` | Add `BookOfBusinessReport` renderer |
| `src/pages/AccountsPage.tsx` | Add BoB button to FolioBar actions |

### New Frontend Files

| File | Purpose |
|------|---------|
| `src/components/reports/BookOfBusinessReport.tsx` | Report renderer |
| `src/components/reports/BookOfBusinessReport.module.css` | CSS module |

---

## Prompt Strategy

The BoB prompt differs from per-account prompts in that it receives intelligence for ALL accounts and must synthesize across them:

```
You are a senior customer success strategist preparing a Book of Business review
for {user_name} ({user_role}).

## Your Portfolio
{for each account: name, ARR, health_band, health_trend, renewal_date, key_contact}

## Account Intelligence
{for each account with assessment: executive_assessment excerpt (truncated to ~500 chars)}

## Recent Activity (90 days)
{aggregated: meeting counts per account, action counts, email signal counts}

## Open Actions
{top 20 open actions across all accounts, with account attribution}

## Recent Wins & Risks
{captures marked as wins/risks across all accounts}

## Instructions
Synthesize this portfolio into a Book of Business review. Key requirements:
- Executive summary: lead with the headline numbers, then the story
- Account snapshot: I will build this from data — you provide the deep dives
- Deep dives: select the {3-5} accounts most worth discussing (highest ARR, highest risk, or biggest opportunity)
- Value delivered: cite specific outcomes from the data, not generic statements
- Key themes: identify 3-4 patterns that span multiple accounts. This is the most valuable section.
- Leadership asks: extract specific decisions or escalations needed. Each must cite impacted accounts.
- Do NOT fabricate data. If ARR is unknown, say so. If no renewal date, omit it.
- Use human language. No jargon: no "entity", "signal", "enrichment", "intelligence".
- Period label: {current month + year}
```

Context window management: For large portfolios (20+ accounts), truncate per-account intelligence to executive_assessment first paragraph only. Prioritize accounts by ARR * health_risk_factor.

---

## Staleness

The `intel_hash` for a BoB report is computed as a hash of ALL account assessment `enriched_at` timestamps concatenated. When any account's intelligence is refreshed, the BoB report becomes stale.

---

## What This Is NOT

- **Not a scheduled report.** Unlike Weekly Impact (auto-Monday) or Monthly Wrapped (auto-1st), the BoB review is generated on-demand. Users prepare it before leadership meetings.
- **Not a replacement for per-account reports.** Account Health, EBR/QBR, and SWOT remain per-account. The BoB aggregates and synthesizes across accounts.
- **Not the Portfolio Health Summary (I491).** I491 is an exceptions-only report surfacing declining/at-risk accounts. The BoB is a full portfolio narrative for leadership review.

---

## Acceptance Criteria

1. `ReportType::BookOfBusiness` exists in backend enum. `report_type = 'book_of_business'` stored in DB.
2. AccountsPage FolioBar shows "Book of Business" action button. Clicking navigates to `/me/reports/book_of_business`.
3. Report generation gathers intelligence from ALL active accounts (not just one entity).
4. Generated report contains all 7 sections: executive summary with metrics, account snapshot table, deep dives (3-5 accounts), value delivered table, key themes (3-4), leadership asks, appendix.
5. Executive summary metrics (total accounts, total ARR, at-risk ARR, upcoming renewals) are computed from DB data, not AI-hallucinated.
6. Account snapshot table rows are data-driven (DB query), not AI-generated. Health badges render correctly.
7. Deep dive accounts are selected by relevance (ARR + risk + opportunity signal), not arbitrary order.
8. Key themes section cites specific accounts as evidence for each theme.
9. Leadership asks cite impacted accounts and have status indicators.
10. Value delivered items cite sources (meeting date or action) where available.
11. Report renders in editorial layout: Newsreader headings, DM Sans body, section rules, FinisMarker. No inline styles.
12. Staleness detection works: refreshing any account's intelligence marks the BoB report as stale.
13. ReportShell handles generation progress, staleness banner, and regenerate for BoB type.
14. Zero ADR-0083 vocabulary violations in any user-facing string.
15. Preset vocabulary applied: report label adapts per preset (Book of Business / Client Portfolio Review / etc.).
16. Portfolio with 0 accounts shows appropriate empty state, not an error.
17. Portfolio with accounts but no intelligence generates a partial report (snapshot table populated, AI sections note limited data).
18. PDF export works via existing report export mechanism.
19. All AI-generated text fields are click-to-edit via `EditableText`. Edits persist on blur via `save_report`. Reload shows saved edits.
20. Computed metrics (total accounts, ARR, renewal count) and account snapshot table rows are NOT editable — they are DB facts.
21. Tab/Shift+Tab navigates between editable fields. Escape cancels without saving. Same UX as all other reports.
