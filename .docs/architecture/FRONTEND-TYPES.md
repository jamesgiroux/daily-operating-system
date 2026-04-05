# Frontend Types Audit

> Generated 2026-03-02. Audits TypeScript types in `src/types/` against Rust backend structs in `src-tauri/src/types.rs`, `src-tauri/src/db/types.rs`, `src-tauri/src/commands.rs`, and service modules.

---

## 1. Type Registry

### `src/types/index.ts`

| Type Name | Kind | Fields | Rust Counterpart | Used By |
|-----------|------|--------|------------------|---------|
| `ProfileType` | union | `"customer-success" \| "general"` | String (validated in `set_profile`) | SettingsPage, ProfileSelector |
| `EntityMode` | union | `"account" \| "project" \| "both"` | String (validated in `validate_entity_mode`) | SettingsPage, OnboardingFlow |
| `SettingsTabId` | union | 10 variants | Frontend-only routing | SettingsPage |
| `MeetingType` | union | 10 variants | `types::MeetingType` enum | Dashboard, Week, MeetingDetail |
| `Priority` | union | `"P1" \| "P2" \| "P3"` | `types::Priority` enum | Actions, Dashboard |
| `ActionStatus` | union | 4 variants | `types::ActionStatus` enum | Actions |
| `PrepStatus` | union | 7 variants | `types::PrepStatus` enum | Week view |
| `Stakeholder` | interface | name, role?, focus?, relationship? | `types::Stakeholder` | MeetingPrep, FullMeetingPrep |
| `SourceReference` | interface | label, path?, lastUpdated? | `types::SourceReference` | MeetingPrep, Intelligence |
| `OverlayStatus` | union | 4 variants | `types::OverlayStatus` enum | Dashboard meetings |
| `CalendarAttendee` | interface | email, name, rsvp, domain | `types::CalendarAttendeeEntry` | Meeting cards |
| `LinkedEntity` | interface | id, name, entityType | `types::LinkedEntity` | Everywhere |
| `Meeting` | interface | 17 fields | `types::Meeting` | Dashboard |
| `MeetingPrep` | interface | 10 fields | `types::MeetingPrep` | Dashboard meetings |
| `Action` | interface | 10 fields | `types::Action` | Dashboard, Focus |
| `DbAction` | interface | 17 fields | `db::types::DbAction` | Actions, Focus, MeetingDetail |
| `DayStats` | interface | 4 fields | `types::DayStats` | Dashboard |
| `EmailPriority` | union | 3 variants | `types::EmailPriority` enum | Emails |
| `EmailSyncState` | union | 3 variants | `types::EmailSyncState` enum | Dashboard |
| `EmailSyncStage` | union | 4 variants | `types::EmailSyncStage` enum | Dashboard |
| `EmailSyncStatus` | interface | 8 fields | `types::EmailSyncStatus` | Dashboard |
| `EmailSyncStats` | interface | 5 fields | `db::types::EmailSyncStats` | Settings diagnostics |
| `Email` | interface | 17 fields | `types::Email` | Dashboard, Emails |
| `InboxFileType` | union | 7 variants | `types::InboxFileType` enum | Inbox |
| `InboxFile` | interface | 8 fields | `types::InboxFile` | Inbox |
| `DataFreshness` | discriminated union | 3 variants | `json_loader::DataFreshness` | Dashboard |
| `ReplyNeeded` | interface | 5 fields | `json_loader::DirectiveReplyNeeded` | Dashboard, Emails |
| `DashboardData` | interface | 10 fields | `types::DashboardData` | Dashboard |
| `WeekOverview` | interface | 8 fields | `types::WeekOverview` | WeekPage |
| `WeekDay` | interface | 3 fields | `types::WeekDay` | WeekPage |
| `WeekMeeting` | interface | 5 fields | `types::WeekMeeting` | WeekPage |
| `WeekActionSummary` | interface | 5 fields | `types::WeekActionSummary` | WeekPage |
| `WeekAction` | interface | 7 fields | `types::WeekAction` | WeekPage |
| `ReadinessCheck` | interface | 5 fields | `types::ReadinessCheck` | WeekPage |
| `DayShape` | interface | 8 fields | `types::DayShape` | WeekPage |
| `AlertSeverity` | union | 3 variants | `types::AlertSeverity` enum | WeekPage |
| `HygieneAlert` | interface | 5 fields | `types::HygieneAlert` | WeekPage |
| `TimeBlock` | interface | 7 fields | `types::TimeBlock` | Week, Focus |
| `TopPriority` | interface | 4 fields | `types::TopPriority` | WeekPage |
| `LiveProactiveSuggestion` | interface | 14 fields | `types::LiveProactiveSuggestion` | WeekPage |
| `DailyFocus` | interface | 8 fields | `types::DailyFocus` | Dashboard |
| `PrioritizedAction` | interface | 6 fields | `types::PrioritizedFocusAction` | Focus, DayShape |
| `FocusImplications` | interface | 4 fields | `types::FocusImplications` | Focus, DayShape |
| `EmailDetail` | interface | 11 fields | `types::EmailDetail` | Emails |
| `EmailSignal` | interface | 9 fields | `types::EmailSignal` | Emails, MeetingDetail |
| `EmailSummaryData` | interface | 3 fields | `types::EmailSummaryData` | Emails |
| `EmailStats` | interface | 4 fields | `types::EmailStats` | Emails |
| `EnrichedEmail` | interface | extends Email + signals | `types::EnrichedEmail` | Emails |
| `EntityEmailThread` | interface | 6 fields | `types::EntityEmailThread` | Emails |
| `EmailBriefingStats` | interface | 5 fields | `types::EmailBriefingStats` | Emails |
| `EmailBriefingData` | interface | 8 fields | `types::EmailBriefingData` | Emails |
| `ActionWithContext` | interface | 4 fields | `types::ActionWithContext` | FullMeetingPrep |
| `AgendaItem` | interface | 3 fields | `types::AgendaItem` | FullMeetingPrep |
| `GoogleAuthStatus` | discriminated union | 3 variants | `types::GoogleAuthStatus` enum | Settings, Dashboard |
| `GleanAuthStatus` | discriminated union | 2 variants | `glean::GleanAuthStatus` | Settings |
| `HygieneStatusView` | interface | 13 fields | `hygiene::HygieneStatusView` | Settings |
| `HygieneNarrativeView` | interface | 5 fields | `hygiene::HygieneNarrativeView` | Settings |
| `CalendarEvent` | interface | 8 fields | `types::CalendarEvent` | Calendar, PostMeeting |
| `PostMeetingCaptureConfig` | interface | 4 fields | `types::PostMeetingCaptureConfig` | Settings |
| `CapturedOutcome` | interface | 7 fields | `types::CapturedOutcome` | PostMeeting |
| `CapturedAction` | interface | 3 fields | `types::CapturedAction` | PostMeeting, Transcript |
| `TranscriptResult` | interface | 9 fields | `types::TranscriptResult` | MeetingDetail |
| `MeetingOutcomeData` | interface | 8 fields | `types::MeetingOutcomeData` | MeetingDetail |
| `DbMeeting` | interface | 22 fields | `db::types::DbMeeting` | MeetingDetail |
| `MeetingIntelligence` | interface | 17 fields | `types::MeetingIntelligence` | MeetingDetail |
| `ApplyPrepPrefillResult` | interface | 4 fields | `commands (inline)` | MeetingDetail |
| `AgendaDraftResult` | interface | 3 fields | `commands (inline)` | MeetingDetail |
| `DecisionSignal` | interface | 5 fields | No Rust counterpart found | Executive intelligence |
| `DelegationSignal` | interface | 6 fields | No Rust counterpart found | Executive intelligence |
| `PortfolioAlert` | interface | 4 fields | No Rust counterpart found | Executive intelligence |
| `CancelableSignal` | interface | 4 fields | No Rust counterpart found | Executive intelligence |
| `SkipSignal` | interface | 2 fields | No Rust counterpart found | Executive intelligence |
| `SignalCounts` | interface | 5 fields | No Rust counterpart found | Executive intelligence |
| `ExecutiveIntelligence` | interface | 6 fields | No Rust counterpart found | Executive intelligence |
| `FullMeetingPrep` | interface | 28 fields | `types::FullMeetingPrep` | MeetingDetail |
| `AccountSnapshotItem` | interface | 4 fields | `types::AccountSnapshotItem` | FullMeetingPrep |
| `StakeholderSignals` | interface | 6 fields | `db::types::StakeholderSignals` | MeetingPrep, Accounts |
| `PersonRelationship` | union | 3 variants | String in Rust | People |
| `Person` | interface | 21 fields | `db::types::DbPerson` | People, Accounts, Projects |
| `PersonListItem` | interface | extends Person + 4 | `db::types::PersonListItem` | People |
| `PersonSignals` | interface | 5 fields | `db::types::PersonSignals` | PersonDetail |
| `PersonDetail` | interface | extends Person + 8 | Assembled in service | PersonDetailPage |
| `AttendeeContext` | interface | 10 fields | `types::AttendeeContext` | FullMeetingPrep |
| `AccountHealth` | union | 3 variants | String in Rust | Accounts |
| `AccountType` | union | 3 variants | `db::types::AccountType` enum | Accounts |
| `AccountListItem` | interface | 12 fields | Assembled in service | AccountsPage |
| `AccountDetail` | interface | extends AccountListItem + 19 | Assembled in service | AccountDetailPage |
| `AccountTeamMember` | interface | 6 fields | `db::types::DbAccountTeamMember` | AccountDetail |
| `AccountTeamImportNote` | interface | 6 fields | `db::types::DbAccountTeamImportNote` | AccountDetail |
| `ParentAggregate` | interface | 4 fields | `db::types::ParentAggregate` | AccountDetail |
| `AccountChildSummary` | interface | 5 fields | Assembled in service | AccountDetail |
| `PickerAccount` | interface | 4 fields | Assembled in query | Entity linking |
| `OnboardingPrimingCard` | interface | 7 fields | `commands::OnboardingPrimingCard` | Onboarding |
| `OnboardingPrimingContext` | interface | 3 fields | `commands::OnboardingPrimingContext` | Onboarding |
| `ContentFile` | interface | 13 fields | `db::types::DbContentFile` | Entity detail |
| `EntityIntelligence` | interface | 18 fields | `intelligence::io` | Entity detail |
| `PortfolioIntelligence` | interface | 4 fields | `intelligence::io` | AccountDetail |
| `NetworkIntelligence` | interface | 6 fields | `intelligence::io` | PersonDetail |
| `PersonRelationshipEdge` | interface | 14 fields | `db::person_relationships` | PersonDetail |
| `ProjectListItem` | interface | 11 fields | Assembled in service | ProjectsPage |
| `ProjectDetail` | interface | extends ProjectListItem + 12 | Assembled in service | ProjectDetailPage |
| `ProjectParentAggregate` | interface | 5 fields | `db::types::ProjectParentAggregate` | ProjectDetail |
| `AiModelConfig` | interface | 3 fields | `types::AiModelConfig` | Settings |
| `ProcessingLogEntry` | interface | 9 fields | `db::types::DbProcessingLog` | HistoryPage |
| `DbCapture` | interface | 8 fields | `db::types::DbCapture` | MeetingDetail |
| `ActionDetail` | interface | extends DbAction + 2 | `commands::ActionDetail` | ActionDetailPage |
| `MeetingHistoryDetail` | interface | 13 fields | `commands::MeetingHistoryDetail` | MeetingHistory |
| `PrepContext` | interface | 10 fields | `commands::PrepContext` | MeetingHistory |
| `MeetingSearchResult` | interface | 6 fields | `commands::MeetingSearchResult` | CommandMenu |
| `AccountEventType` | union | 10 variants | String in Rust | AccountDetail |
| `AccountEvent` | interface | 7 fields | `db::types::DbAccountEvent` | AccountDetail |
| `DuplicateCandidate` | interface | 6 fields | Assembled in query | People hygiene |
| `RiskBriefing` | interface | 7 fields | `types::RiskBriefing` | RiskBriefingPage |
| `QuillStatus` | interface | 10 fields | Assembled in integrations service | Settings |
| `QuillSyncState` | interface | 14 fields | `db::types::DbQuillSyncState` | Settings |
| `GravatarStatus` | interface | 3 fields | Assembled in service | Settings |
| `GranolaStatus` | interface | 8 fields | Assembled in service | Settings |
| `ClayStatusData` | interface | 7 fields | Assembled in service | Settings |
| `EnrichmentLogEntry` | interface | 8 fields | Assembled in query | Settings |
| `LinearStatusData` | interface | 6 fields | Assembled in service | Settings |
| `DriveStatusData` | interface | 3 fields | Assembled in service | Settings |
| `DriveWatchedSource` | interface | 8 fields | Assembled in service | Settings |
| `TimelineMeeting` | interface | 12 fields | `types::TimelineMeeting` | MeetingDetail |
| `UserEntity` | interface | 19 fields | `types::UserEntity` | MePage |
| `UserContextEntry` | interface | 6 fields | `types::UserContextEntry` | MePage |
| `EntityContextEntry` | interface | 7 fields | `types::EntityContextEntry` | Entity detail |
| `AnnualPriority` | interface | 5 fields | `types::AnnualPriority` | MePage |
| `QuarterlyPriority` | interface | 5 fields | `types::QuarterlyPriority` | MePage |

### `src/types/callout.ts`

| Type | Fields | Rust Counterpart |
|------|--------|------------------|
| `BriefingCallout` | id, severity, headline, detail, entityName?, entityType, entityId, relevanceScore? | `signals::callouts::BriefingCallout` |

### `src/types/preset.ts`

| Type | Fields | Rust Counterpart |
|------|--------|------------------|
| `PresetVitalField` | key, label, fieldType, source, columnMapping?, options? | `presets::schema::PresetVitalField` |
| `PresetMetadataField` | key, label, fieldType, options?, required | `presets::schema::PresetMetadataField` |
| `PresetVitalsConfig` | account, project, person | `presets::schema::PresetVitalsConfig` |
| `PresetMetadataConfig` | account, project, person | `presets::schema::PresetMetadataConfig` |
| `PresetVocabulary` | 7 fields | `presets::schema::PresetVocabulary` |
| `PresetPrioritization` | 3 fields | `presets::schema::PresetPrioritization` |
| `PresetRoleDefinition` | id, label, description? | `presets::schema::PresetRoleDefinition` |
| `RolePreset` | 14 fields | `presets::schema::RolePreset` |

### `src/types/reports.ts`

| Type | Fields | Rust Counterpart |
|------|--------|------------------|
| `ReportType` | 6 variants | `reports::mod::ReportType` enum |
| `ReportRow` | 9 fields | `reports::mod::ReportRow` |
| `SwotContent` | 5 fields | `reports::swot::SwotContent` |
| `AccountHealthContent` | 12 fields | `reports::account_health::AccountHealthContent` |
| `WeeklyImpactContent` | 9 fields | `reports::weekly_impact::WeeklyImpactContent` |
| `MonthlyWrappedContent` | 8 fields | `reports::monthly_wrapped::MonthlyWrappedContent` |
| `EbrQbrContent` | 9 fields | `reports::ebr_qbr::EbrQbrContent` |

---

## 2. Alignment Matrix -- Field-Level Diffs

### CRITICAL: Type Mismatches

#### `Email.commitments` / `Email.questions`
- **Rust**: `pub commitments: Vec<String>` / `pub questions: Vec<String>` (always present, defaults to empty vec)
- **TS**: `commitments?: string[]` / `questions?: string[]` (optional)
- **Risk**: Low. Rust uses `#[serde(skip_serializing_if = "Vec::is_empty", default)]`, so empty vecs are omitted from JSON. TS treating them as optional is correct behavior.

#### `DashboardData.repliesNeeded`
- **Rust**: `pub replies_needed: Vec<DirectiveReplyNeeded>` (always present, skip if empty)
- **TS**: `repliesNeeded?: ReplyNeeded[]` (optional)
- **Risk**: Low. Same skip-if-empty serde behavior.

#### `DayStats` numeric types
- **Rust**: `total_meetings: usize`, `customer_meetings: usize`, `actions_due: usize`, `inbox_count: usize`
- **TS**: `totalMeetings: number`, etc.
- **Risk**: None. JS `number` handles all Rust integer types.

#### `IntelligenceQuality.signalCount`
- **Rust**: `signal_count: u32`
- **TS**: `signalCount: number`
- **Risk**: None. But the TS type is defined inline (not a named type) in multiple places.

#### `CalendarEvent.start` / `CalendarEvent.end`
- **Rust**: `pub start: DateTime<Utc>` / `pub end: DateTime<Utc>` (serialized as ISO 8601 string)
- **TS**: `start: string` / `end: string`
- **Risk**: None. Chrono DateTime serializes to string via serde.

#### `CapturedOutcome.capturedAt`
- **Rust**: `pub captured_at: DateTime<Utc>` (chrono DateTime)
- **TS**: `capturedAt: string`
- **Risk**: **MEDIUM**. When the frontend sends this to `capture_meeting_outcome`, it must send a valid ISO 8601 string that chrono can deserialize. If the frontend sends a non-ISO string, the Tauri deserialization will fail silently.

#### `PrioritizedAction.score` vs `PrioritizedFocusAction.score`
- **Rust**: `pub score: i32`
- **TS**: `score: number`
- **Risk**: None numerically, but the TS type is named `PrioritizedAction` while Rust is `PrioritizedFocusAction`.

#### `PrioritizedAction.effortMinutes`
- **Rust**: `pub effort_minutes: u32`
- **TS**: `effortMinutes: number`
- **Risk**: None.

#### `WeekAction.daysOverdue`
- **Rust**: `pub days_overdue: Option<i64>`
- **TS**: `daysOverdue?: number`
- **Risk**: None.

#### `Action.daysOverdue`
- **Rust**: `pub days_overdue: Option<i32>`
- **TS**: `daysOverdue?: number`
- **Risk**: None.

#### `EmailSyncStats.total`
- **Rust (db::types)**: `pub total: i32`, `pub enriched: i32`, `pub pending: i32`, `pub failed: i32`
- **TS**: `total: number`, `enriched: number`, `pending: number`, `failed: number`
- **Risk**: None.

#### `EmailSyncStats.lastFetchAt`
- **Rust**: `pub last_fetch_at: Option<String>`
- **TS**: `lastFetchAt: string | null`
- **Risk**: None. `Option<String>` serializes as `null` when None.

#### `Stakeholder.relationship`
- **Rust**: `types::Stakeholder` does NOT have a `relationship` field
- **TS**: `relationship?: PersonRelationship`
- **Risk**: **LOW -- Phantom field.** The TS `Stakeholder` has `relationship?` which is never set from the Rust `Stakeholder` struct. This field is likely populated by frontend logic when hydrating from people data.

#### `EmailSignal` field differences
- **Rust**: Has `id`, `signal_type`, `signal_text`, `confidence`, `sentiment`, `urgency`, `detected_at` (7 fields)
- **TS**: Has `id?`, `emailId?`, `senderEmail?`, `personId?`, `entityId?`, `entityType?`, `signalType`, `signalText`, `confidence?`, `sentiment?`, `urgency?`, `detectedAt?` (12 fields)
- **Risk**: **MEDIUM -- TS has 5 extra fields** (`emailId`, `senderEmail`, `personId`, `entityId`, `entityType`). These exist on `DbEmailSignal` in Rust but NOT on `types::EmailSignal`. The frontend EmailSignal type merges concepts from two distinct Rust types. Fields will be `undefined` when sourced from `types::EmailSignal` contexts.

#### `EmailSyncStatus` field differences
- **Rust**: Has extra fields `enrichment_pending`, `enrichment_enriched`, `enrichment_failed`, `total_active`
- **TS**: Missing these 4 fields
- **Risk**: **LOW.** The extra Rust fields are `skip_serializing_if = "Option::is_none"` and only present in some contexts. TS doesn't use them directly.

### NOTABLE: Naming Differences

#### `AccountSnapshotItem.type` vs `.item_type`
- **Rust**: `pub item_type: String` with `#[serde(rename = "type")]`
- **TS**: `type: "status" | "currency" | "text" | "date" | "intelligence" | "risk" | "win"`
- **Risk**: None. Serde `rename` handles the wire format. But TS constrains to 7 string literals while Rust accepts any string.

#### `Meeting.type` vs `.meeting_type`
- **Rust**: `pub meeting_type: MeetingType` with `#[serde(rename = "type")]`
- **TS**: `type: MeetingType`
- **Risk**: None. Correct use of serde rename.

### NOTABLE: Missing Fields (Rust has, TS lacks)

#### `WeekOverview`
- **Rust**: Has `week_narrative: Option<String>`, `top_priority: Option<TopPriority>`
- **TS**: Missing `weekNarrative` and `topPriority`
- **Risk**: **MEDIUM.** If the backend populates these fields, the frontend silently ignores them. These are AI enrichment features that may be in use.

#### `DbMeeting` (TS) vs `db::types::DbMeeting` (Rust)
- **Rust**: Has `intelligence_state`, `intelligence_quality`, `last_enriched_at`, `signal_count`, `has_new_signals`, `last_viewed_at`
- **TS**: Missing all 6 intelligence lifecycle fields
- **Risk**: **LOW.** The TS `DbMeeting` type is only used within `MeetingIntelligence` which has its own `intelligenceQuality` field. The raw DB fields don't need to be in TS.

#### `FullMeetingPrep`
- **Rust**: Has `raw_markdown: Option<String>`
- **TS**: Missing `rawMarkdown`
- **Risk**: **LOW.** Only used for debugging/fallback display.

#### `DashboardData`
- **Rust**: `replies_needed` is a `Vec<DirectiveReplyNeeded>` (non-optional, skip if empty)
- **TS**: `repliesNeeded?: ReplyNeeded[]` (optional)
- **Risk**: None. Functionally equivalent.

### NOTABLE: Extra Fields (TS has, Rust lacks)

#### `DbMeeting` (TS) has `accountId`
- **TS**: `accountId?: string`
- **Rust**: `db::types::DbMeeting` does NOT have `account_id`
- **Risk**: **MEDIUM -- Phantom field.** This field will always be `undefined` from backend. Any frontend code relying on `meeting.accountId` gets nothing from the backend.

---

## 3. Ghost Types

Types defined in `src/types/` but **never imported** elsewhere in the codebase:

| Type | File | Assessment |
|------|------|------------|
| `CompanyOverview` | index.ts:1026 | Used only in `AccountDetail.companyOverview` -- indirectly imported via `AccountDetail` |
| `StrategicProgram` | index.ts:1034 | Used only in `AccountDetail.strategicPrograms` |
| `MeetingPreview` | index.ts:1048 | Used in `AccountDetail.recentMeetings` |
| `ProjectMilestone` | index.ts:1349 | Used in `ProjectDetail.milestones` |
| `ProjectChildSummary` | index.ts:1357 | Used in `ProjectDetail.children` |
| `EmailSummaryData` | index.ts:488 | **Likely dead.** Was for old email summary format, now replaced by `EmailBriefingData`. |
| `EmailStats` | index.ts:494 | **Likely dead.** Part of old `EmailSummaryData`. |
| `FocusImplications` | index.ts:446 | Used inline in `DailyFocus` and `DayShape` |
| `DecisionSignal` | index.ts:774 | Part of `ExecutiveIntelligence` |
| `DelegationSignal` | index.ts:782 | Part of `ExecutiveIntelligence` |
| `PortfolioAlert` | index.ts:793 | Part of `ExecutiveIntelligence` |
| `CancelableSignal` | index.ts:800 | Part of `ExecutiveIntelligence` |
| `SkipSignal` | index.ts:807 | Part of `ExecutiveIntelligence` |
| `SignalCounts` | index.ts:812 | Part of `ExecutiveIntelligence` |
| `ExecutiveIntelligence` | index.ts:820 | **No Rust counterpart found.** Used only in `useExecutiveIntelligence` hook but the backend command that would return this is not identifiable in `commands.rs`. May be assembled client-side. |

**Confirmed dead types** (no import, no usage):
- `EmailSummaryData` -- superseded by `EmailBriefingData`
- `EmailStats` -- part of dead `EmailSummaryData`

---

## 4. Inline Type Violations

Components and hooks that define types inline instead of importing from `src/types/`:

| File | Inline Type | Should Use |
|------|-------------|------------|
| `src/pages/MeetingDetailPage.tsx` | `intelligenceQuality` object literal type | Should reference a shared `IntelligenceQuality` interface |
| `src/types/index.ts:103-112` | `intelligenceQuality` inline on `Meeting` | Duplicated at lines 745-754 on `MeetingIntelligence` and lines 1748-1757 on `TimelineMeeting`. Should be a named `IntelligenceQuality` interface. |
| `src/types/index.ts:1110-1117` | `signals` object on `AccountDetail` | Duplicates `StakeholderSignals` but with inline definition |
| `src/types/index.ts:1386-1394` | `signals` object on `ProjectDetail` | Nearly identical to `StakeholderSignals` but includes `daysUntilTarget` and `openActionCount` -- should be `ProjectSignals` |
| `src/types/index.ts:1118-1124` | `recentCaptures` inline on `AccountDetail` | Has different fields than `DbCapture` (includes `meetingId` but lacks `accountId`, `projectId`) |
| `src/types/index.ts:964-969` | `recentMeetings` inline on `PersonDetail` | Duplicates `MeetingSummary` |
| `src/types/index.ts:1487-1491` | `stakeholderInsights` inline on `PrepContext` | Subset of `StakeholderInsight` (missing `engagement`, `source`, `personId`, `suggestedPersonId`) |

**The `IntelligenceQuality` inline type is defined in 3 places** with identical shape. It should be extracted to a named interface.

---

## 5. Type Safety Issues

### `any` Casts

| File | Line | Code | Risk |
|------|------|------|------|
| `src/pages/AccountDetailEditorial.tsx` | 228 | `navigate({ to: "...", params: ... } as any)` | LOW -- Router type workaround |
| `src/pages/AccountDetailEditorial.tsx` | 230 | `navigate({ to: "...", params: ... } as any)` | LOW -- Router type workaround |
| `src/components/ui/meeting-entity-chips.tsx` | 172 | `params={linkParams as any}` | LOW -- Router type workaround |

### `unknown` Usage

| File | Usage | Assessment |
|------|-------|------------|
| `src/pages/AccountHealthPage.tsx` | `toArr<T>(v: unknown): T[]` | OK -- defensive array parsing |
| `src/pages/EbrQbrPage.tsx` | `toArr<T>(v: unknown): T[]` | OK -- same pattern |
| `src/pages/MonthlyWrappedPage.tsx` | `toArr<T>(v: unknown): T[]` | OK -- same pattern |
| `src/pages/SwotPage.tsx` | `toArr<T>(v: unknown): T[]` | OK -- same pattern |
| `src/pages/WeeklyImpactPage.tsx` | `toArr<T>(v: unknown): T[]` | OK -- same pattern |
| `src/lib/preset-vitals.ts` | Returns `unknown` from resolve functions | OK -- generic value resolution |
| `src/components/settings/ConnectorsGrid.tsx` | `resolveStatus(id: string, result: unknown)` | OK -- generic status resolution |
| `src/hooks/useRevealObserver.ts` | `revision?: unknown` | OK -- used only for dependency tracking |

### Missing Type Annotations on `invoke()` Calls

Tauri `invoke()` calls should be checked for correct parameter types. The main risk areas are:
- `invoke("capture_meeting_outcome", { outcome })` -- the `outcome` object must match `CapturedOutcome` including `capturedAt` as ISO 8601
- `invoke("update_email_entity", { emailId, entityId, entityType })` -- parameter names must be snake_case in the Rust command but camelCase in the invoke call (Tauri auto-converts)

---

## 6. Enum Alignment

| TS Type | TS Variants | Rust Enum | Rust Variants | Aligned? |
|---------|-------------|-----------|---------------|----------|
| `MeetingType` | customer, qbr, training, internal, team_sync, one_on_one, partnership, all_hands, external, personal | `types::MeetingType` | Customer, Qbr, Training, Internal, TeamSync, OneOnOne, Partnership, AllHands, External, Personal | YES (serde rename_all = "snake_case") |
| `Priority` | P1, P2, P3 | `types::Priority` | P1, P2, P3 | YES |
| `ActionStatus` | pending, completed, proposed, archived | `types::ActionStatus` | Pending, Completed, Proposed, Archived | YES (serde rename_all = "lowercase") |
| `PrepStatus` | prep_needed, agenda_needed, bring_updates, context_needed, prep_ready, draft_ready, done | `types::PrepStatus` | PrepNeeded, AgendaNeeded, BringUpdates, ContextNeeded, PrepReady, DraftReady, Done | YES (serde rename_all = "snake_case") |
| `OverlayStatus` | enriched, cancelled, new, briefing_only | `types::OverlayStatus` | Enriched, Cancelled, New, BriefingOnly | YES (serde rename_all = "snake_case") |
| `EmailPriority` | high, medium, low | `types::EmailPriority` | High, Medium, Low | YES (serde rename_all = "lowercase") |
| `AlertSeverity` | critical, warning, info | `types::AlertSeverity` | Critical, Warning, Info | YES |
| `AccountType` | customer, internal, partner | `db::types::AccountType` | Customer, Internal, Partner | YES |
| `ReportType` | swot, account_health, ebr_qbr, weekly_impact, monthly_wrapped, risk_briefing | `reports::ReportType` | Swot, AccountHealth, EbrQbr, WeeklyImpact, MonthlyWrapped, RiskBriefing | YES (serde rename_all = "snake_case") |
| `GoogleAuthStatus` (TS: discriminated by `.status`) | notconfigured, authenticated, tokenexpired | `types::GoogleAuthStatus` | NotConfigured, Authenticated, TokenExpired | YES (serde tag = "status", rename_all = "lowercase") |
| `InboxFileType` | markdown, image, spreadsheet, document, data, text, other | `types::InboxFileType` | Markdown, Image, Spreadsheet, Document, Data, Text, Other | YES (serde rename_all = "snake_case") |
| `WorkflowId` (not in TS types, used as string) | today, archive, inbox_batch, week | `types::WorkflowId` | Today, Archive, InboxBatch, Week | YES |
| `WorkflowStatus` (not in TS types, used as raw JSON) | idle, running, completed, failed | `types::WorkflowStatus` | Idle, Running, Completed, Failed | YES (serde tag = "status") |
| `PersonRelationship` | internal, external, unknown | String in Rust | N/A | YES (no enum in Rust, just validated strings) |
| `AccountEventType` | 10 string variants | String in Rust | N/A | Risk: **Rust stores as unconstrained string.** New event types added in DB won't be caught by TS type. |
| `ProjectStatus` | active, on_hold, completed, archived | String in Rust | N/A | Same risk as AccountEventType |
| `AccountHealth` | green, yellow, red | String in Rust | N/A | Same risk |
| `QuillSyncState.state` | 7 variants | String in Rust | N/A | Same risk |

---

## 7. Summary and Risk Assessment

### Overall Alignment: GOOD

The TypeScript types are well-maintained and closely mirror the Rust backend structs. The `#[serde(rename_all = "camelCase")]` convention is consistently applied in Rust, matching TypeScript's camelCase field names. All major enums are aligned.

### High-Priority Issues

1. **`IntelligenceQuality` inline duplication (3 locations)**
   - Defined inline on `Meeting`, `MeetingIntelligence`, and `TimelineMeeting` with identical 8-field shape
   - Should be extracted to a named `IntelligenceQuality` interface and referenced
   - Risk: Field drift between the three copies

2. **`DbMeeting.accountId` phantom field**
   - TS declares `accountId?: string` but the Rust `DbMeeting` struct has no `account_id` field
   - Any frontend code reading `meeting.accountId` from a backend response gets `undefined`
   - Audit: Check if any component depends on this field

3. **`EmailSignal` type conflation**
   - TS `EmailSignal` has 12 fields merging `types::EmailSignal` (7 fields) and `db::types::DbEmailSignal` (12 fields)
   - Fields like `emailId`, `senderEmail`, `personId`, `entityId`, `entityType` will be `undefined` when the source is `types::EmailSignal` (used in FullMeetingPrep)

### Medium-Priority Issues

4. **`WeekOverview` missing `weekNarrative` and `topPriority`**
   - Backend can send these; frontend type doesn't declare them
   - May cause data loss if frontend destructures with rest spreading

5. **`ExecutiveIntelligence` has no Rust counterpart**
   - 7 TS types (`DecisionSignal`, `DelegationSignal`, etc.) with no matching backend structs
   - Either assembled client-side from raw data, or this is a planned feature with types pre-defined

6. **Dead types: `EmailSummaryData`, `EmailStats`**
   - Superseded by `EmailBriefingData` / `EmailBriefingStats`
   - Should be removed to avoid confusion

### Low-Priority Issues

7. **`AccountDetail.signals` inline type** -- should reference `StakeholderSignals`
8. **`ProjectDetail.signals` inline type** -- should be a named `ProjectSignals` type
9. **`Stakeholder.relationship` phantom field** -- exists in TS, not in Rust struct
10. **`AccountEventType` and similar string-typed enums** -- Rust uses unconstrained strings while TS narrows to specific values; new values added in Rust won't type-error in TS
11. **3 `as any` casts** -- all for router type workarounds, low risk
12. **`CapturedOutcome.capturedAt` type difference** -- TS `string` vs Rust `DateTime<Utc>`; must send valid ISO 8601

### Drift Risk: LOW-MEDIUM

The codebase has good type discipline. The main risk vectors are:
- Adding fields to Rust structs without updating TS (already happened with `WeekOverview`)
- Inline type definitions diverging from their Rust sources (the `IntelligenceQuality` triple)
- String-typed enums in Rust allowing values not in the TS union

### Recommendations

1. Extract `IntelligenceQuality` to a named interface and use it in all 3 locations
2. Remove `DbMeeting.accountId` phantom field or add it to Rust
3. Split `EmailSignal` into `EmailSignal` (light, from types.rs) and `DbEmailSignal` (full, from db/types.rs) to match Rust
4. Add `weekNarrative` and `topPriority` to `WeekOverview` TS type
5. Delete dead `EmailSummaryData` and `EmailStats` types
6. Replace inline `signals` types on `AccountDetail` and `ProjectDetail` with named interfaces
