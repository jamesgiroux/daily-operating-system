/**
 * Dashboard Types
 * Core type definitions for the DailyOS dashboard
 */

export type ProfileType = "customer-success" | "general";

export type EntityMode = "account" | "project" | "both";

export type MeetingType =
  | "customer"
  | "qbr"
  | "training"
  | "internal"
  | "team_sync"
  | "one_on_one"
  | "partnership"
  | "all_hands"
  | "external"
  | "personal";

export type Priority = "P1" | "P2" | "P3";

export type ActionStatus = "pending" | "completed";

export type PrepStatus =
  | "prep_needed"
  | "agenda_needed"
  | "bring_updates"
  | "context_needed"
  | "prep_ready"
  | "draft_ready"
  | "done";

export interface Stakeholder {
  name: string;
  role?: string;
  focus?: string;
}

export interface SourceReference {
  label: string;
  path?: string;
  lastUpdated?: string;
}

export type OverlayStatus = "enriched" | "cancelled" | "new" | "briefing_only";

export interface LinkedEntity {
  id: string;
  name: string;
  entityType: "account" | "project";
}

export interface Meeting {
  id: string;
  calendarEventId?: string;
  time: string;
  endTime?: string;
  /** ISO 8601 start timestamp for reliable date parsing */
  startIso?: string;
  title: string;
  type: MeetingType;
  account?: string;
  accountId?: string;
  prep?: MeetingPrep;
  isCurrent?: boolean;
  /** Path to the prep file (e.g., "01-1630-customer-acme-prep.md") */
  prepFile?: string;
  /** Whether this meeting has a dedicated prep file */
  hasPrep: boolean;
  /** Calendar overlay status (ADR-0032) */
  overlayStatus?: OverlayStatus;
  /** Whether the user has reviewed this prep (ADR-0033) */
  prepReviewed?: boolean;
  /** Entities linked via M2M junction table (I52) */
  linkedEntities?: LinkedEntity[];
  /** Account ID suggestion when meeting matches an archived account (I161) */
  suggestedUnarchiveAccountId?: string;
}

export interface MeetingPrep {
  metrics?: string[];
  risks?: string[];
  wins?: string[];
  actions?: string[];
  context?: string;
  stakeholders?: Stakeholder[];
  questions?: string[];
  openItems?: string[];
  historicalContext?: string;
  sourceReferences?: SourceReference[];
}

export interface Action {
  id: string;
  title: string;
  account?: string;
  dueDate?: string;
  priority: Priority;
  status: ActionStatus;
  isOverdue?: boolean;
  /** Additional context for the action */
  context?: string;
  /** Source of the action (e.g., meeting, email) */
  source?: string;
  /** Days overdue (if applicable) */
  daysOverdue?: number;
}

/** Action from SQLite database (cross-day persistence). */
export interface DbAction {
  id: string;
  title: string;
  priority: string;
  status: string;
  createdAt: string;
  dueDate?: string;
  completedAt?: string;
  accountId?: string;
  accountName?: string;
  projectId?: string;
  sourceType?: string;
  sourceId?: string;
  sourceLabel?: string;
  context?: string;
  waitingOn?: string;
  updatedAt: string;
  personId?: string;
}

export interface DayStats {
  totalMeetings: number;
  customerMeetings: number;
  actionsDue: number;
  inboxCount: number;
}

export type EmailPriority = "high" | "medium" | "low";
export type EmailSyncState = "ok" | "warning" | "error";
export type EmailSyncStage = "fetch" | "deliver" | "enrich" | "refresh";

export interface EmailSyncStatus {
  state: EmailSyncState;
  stage: EmailSyncStage;
  code?: string;
  message?: string;
  usingLastKnownGood?: boolean;
  canRetry?: boolean;
  lastAttemptAt?: string;
  lastSuccessAt?: string;
}

export interface Email {
  id: string;
  sender: string;
  senderEmail: string;
  subject: string;
  snippet?: string;
  priority: EmailPriority;
  avatarUrl?: string;
  /** AI-generated one-line summary of the email */
  summary?: string;
  /** Suggested next action (e.g. "Reply with counter-proposal") */
  recommendedAction?: string;
  /** Thread history arc (e.g. "Initial outreach → follow-up → this response") */
  conversationArc?: string;
  /** Email category from AI classification */
  emailType?: string;
}

export type InboxFileType =
  | "markdown"
  | "image"
  | "spreadsheet"
  | "document"
  | "data"
  | "text"
  | "other";

export interface InboxFile {
  filename: string;
  path: string;
  sizeBytes: number;
  modified: string;
  preview?: string;
  fileType: InboxFileType;
  processingStatus?: string;
  processingError?: string;
}

export type DataFreshness =
  | { freshness: "fresh"; generatedAt: string }
  | { freshness: "stale"; dataDate: string; generatedAt: string }
  | { freshness: "unknown" };

export interface DashboardData {
  overview: {
    greeting: string;
    date: string;
    summary: string;
    focus?: string;
  };
  stats: DayStats;
  meetings: Meeting[];
  actions: Action[];
  emails?: Email[];
  emailSync?: EmailSyncStatus;
}

// =============================================================================
// Week Overview Types
// =============================================================================

export interface WeekOverview {
  weekNumber: string;
  dateRange: string;
  days: WeekDay[];
  actionSummary?: WeekActionSummary;
  hygieneAlerts?: HygieneAlert[];
  focusAreas?: string[];
  availableTimeBlocks?: TimeBlock[];
  /** AI-generated narrative overview of the week (I94) */
  weekNarrative?: string;
  /** AI-identified top priority (I94) */
  topPriority?: TopPriority;
  /** Proactive readiness checks surfacing prep gaps (I93) */
  readinessChecks?: ReadinessCheck[];
  /** Per-day density and meeting shape (I93) */
  dayShapes?: DayShape[];
}

export interface WeekDay {
  date: string;
  dayName: string;
  meetings: WeekMeeting[];
}

export interface WeekMeeting {
  time: string;
  title: string;
  account?: string;
  type: MeetingType;
  prepStatus: PrepStatus;
}

export interface WeekActionSummary {
  overdueCount: number;
  dueThisWeek: number;
  criticalItems: string[];
  /** Actual overdue action items (I93) */
  overdue?: WeekAction[];
  /** Actual due-this-week action items (I93) */
  dueThisWeekItems?: WeekAction[];
}

/** A single action item for week view (I93) */
export interface WeekAction {
  id: string;
  title: string;
  account?: string;
  dueDate?: string;
  priority: string;
  daysOverdue?: number;
  source?: string;
}

/** Proactive readiness check for the week (I93) */
export interface ReadinessCheck {
  checkType: string;
  message: string;
  severity: string;
  meetingId?: string;
  accountId?: string;
}

/** Per-day density shape for the week view (I93) */
export interface DayShape {
  dayName: string;
  date: string;
  meetingCount: number;
  meetingMinutes: number;
  density: string;
  meetings: WeekMeeting[];
  availableBlocks: TimeBlock[];
}

export type AlertSeverity = "critical" | "warning" | "info";

export interface HygieneAlert {
  account: string;
  lifecycle?: string;
  arr?: string;
  issue: string;
  severity: AlertSeverity;
}

export interface TimeBlock {
  day: string;
  start: string;
  end: string;
  durationMinutes: number;
  suggestedUse?: string;
}

/** AI-identified top priority for the week (I94) */
export interface TopPriority {
  title: string;
  reason: string;
  meetingId?: string;
  actionId?: string;
}

// =============================================================================
// Focus Data Types
// =============================================================================

export interface FocusData {
  focusStatement?: string;
  priorities: DbAction[];
  keyMeetings: FocusMeeting[];
  availableBlocks: TimeBlock[];
  totalFocusMinutes: number;
}

export interface FocusMeeting {
  id: string;
  title: string;
  time: string;
  endTime?: string;
  meetingType: string;
  hasPrep: boolean;
  account?: string;
  prepFile?: string;
}

// =============================================================================
// Extended Email Types
// =============================================================================

export interface EmailDetail {
  id: string;
  sender: string;
  senderEmail: string;
  subject: string;
  received?: string;
  priority: EmailPriority;
  emailType?: string;
  summary?: string;
  conversationArc?: string;
  recommendedAction?: string;
  actionOwner?: string;
  actionPriority?: string;
}

export interface EmailSummaryData {
  highPriority: EmailDetail[];
  mediumPriority?: EmailDetail[];
  stats: EmailStats;
}

export interface EmailStats {
  highCount: number;
  mediumCount: number;
  lowCount: number;
  needsAction?: number;
}

// =============================================================================
// Full Meeting Prep (from individual prep files)
// =============================================================================

export interface ActionWithContext {
  title: string;
  dueDate?: string;
  context?: string;
  isOverdue: boolean;
}

/** Proposed agenda item synthesized from prep data (I80) */
export interface AgendaItem {
  topic: string;
  why?: string;
  source?: string;
}

// =============================================================================
// Google & Calendar Types (Phase 3.0 / 3A)
// =============================================================================

export type GoogleAuthStatus =
  | { status: "notconfigured" }
  | { status: "authenticated"; email: string }
  | { status: "tokenexpired" };

export interface CalendarEvent {
  id: string;
  title: string;
  start: string;
  end: string;
  type: MeetingType;
  account?: string;
  attendees: string[];
  isAllDay: boolean;
}

// =============================================================================
// Post-Meeting Capture Types (Phase 3B)
// =============================================================================

export interface PostMeetingCaptureConfig {
  enabled: boolean;
  delayMinutes: number;
  autoDismissSecs: number;
}

export interface CapturedOutcome {
  meetingId: string;
  meetingTitle: string;
  account?: string;
  capturedAt: string;
  wins: string[];
  risks: string[];
  actions: CapturedAction[];
}

export interface CapturedAction {
  title: string;
  owner?: string;
  dueDate?: string;
}

// =============================================================================
// Transcript & Meeting Outcomes (I44 / I45 / ADR-0044)
// =============================================================================

/** Result of transcript processing */
export interface TranscriptResult {
  status: "success" | "error";
  summary?: string;
  destination?: string;
  wins: string[];
  risks: string[];
  decisions: string[];
  actions: CapturedAction[];
  message?: string;
}

/** Meeting outcomes (from transcript processing or manual capture) */
export interface MeetingOutcomeData {
  meetingId: string;
  summary?: string;
  wins: string[];
  risks: string[];
  decisions: string[];
  actions: DbAction[];
  transcriptPath?: string;
  processedAt?: string;
}

// =============================================================================
// Executive Intelligence (I42)
// =============================================================================

export interface DecisionSignal {
  actionId: string;
  title: string;
  dueDate?: string;
  account?: string;
  priority: string;
}

export interface DelegationSignal {
  actionId: string;
  title: string;
  waitingOn?: string;
  createdAt: string;
  account?: string;
  daysStale: number;
}

export type PortfolioSignalType = "renewal_approaching" | "stale_contact";

export interface PortfolioAlert {
  accountId: string;
  accountName: string;
  signal: PortfolioSignalType;
  detail: string;
}

export interface CancelableSignal {
  meetingId: string;
  title: string;
  time: string;
  reason: string;
}

export interface SkipSignal {
  item: string;
  reason: string;
}

export interface SignalCounts {
  decisions: number;
  delegations: number;
  portfolioAlerts: number;
  cancelable: number;
  skipToday: number;
}

export interface ExecutiveIntelligence {
  decisions: DecisionSignal[];
  delegations: DelegationSignal[];
  portfolioAlerts: PortfolioAlert[];
  cancelableMeetings: CancelableSignal[];
  skipToday: SkipSignal[];
  signalCounts: SignalCounts;
}

// =============================================================================
// Full Meeting Prep (from individual prep files)
// =============================================================================

export interface FullMeetingPrep {
  filePath: string;
  calendarEventId?: string;
  title: string;
  timeRange: string;
  meetingContext?: string;
  /** Calendar event description from Google Calendar (I185) */
  calendarNotes?: string;
  /** Quick Context metrics (key-value pairs like Ring, ARR, Health) — legacy */
  quickContext?: [string, string][];
  /** Intelligence-enriched account snapshot (I186) */
  accountSnapshot?: AccountSnapshotItem[];
  attendees?: Stakeholder[];
  /** Since Last Meeting section items */
  sinceLast?: string[];
  /** Current Strategic Programs */
  strategicPrograms?: string[];
  currentState?: string[];
  openItems?: ActionWithContext[];
  /** Current Risks to Monitor */
  risks?: string[];
  /** Suggested Talking Points */
  talkingPoints?: string[];
  /** Canonical recent wins for meeting prep (I196) */
  recentWins?: string[];
  /** Structured provenance for recent wins (I196) */
  recentWinSources?: SourceReference[];
  questions?: string[];
  keyPrinciples?: string[];
  references?: SourceReference[];
  /** Stakeholder relationship signals (I43) — computed live from meeting history */
  stakeholderSignals?: StakeholderSignals;
  /** Per-attendee context from people database (I51) */
  attendeeContext?: AttendeeContext[];
  /** Proposed agenda synthesized from prep data (I80) */
  proposedAgenda?: AgendaItem[];
  /** User-authored agenda items (I194 / ADR-0065) */
  userAgenda?: string[];
  /** User-authored notes (I194 / ADR-0065) */
  userNotes?: string;
  /** Intelligence summary — executive assessment from intelligence.json (I135) */
  intelligenceSummary?: string;
  /** Entity-level risks from intelligence.json (I135) */
  entityRisks?: IntelRisk[];
  /** Entity meeting readiness items from intelligence.json (I135) */
  entityReadiness?: string[];
  /** Stakeholder insights from intelligence.json (I135) */
  stakeholderInsights?: StakeholderInsight[];
}

/** Account snapshot item for intelligence-enriched Quick Context (I186) */
export interface AccountSnapshotItem {
  label: string;
  value: string;
  type: "status" | "currency" | "text" | "date" | "intelligence" | "risk" | "win";
  urgency?: string;
}

/** Relationship context signals computed from meeting history and account data (I43) */
export interface StakeholderSignals {
  meetingFrequency30d: number;
  meetingFrequency90d: number;
  lastMeeting?: string;
  lastContact?: string;
  /** "hot" (<7d), "warm" (7-30d), "cool" (30-60d), "cold" (>60d) */
  temperature: string;
  /** "increasing", "stable", "decreasing" */
  trend: string;
}

// =============================================================================
// People (I51)
// =============================================================================

export type PersonRelationship = "internal" | "external" | "unknown";

export interface Person {
  id: string;
  email: string;
  name: string;
  organization?: string;
  role?: string;
  relationship: PersonRelationship;
  notes?: string;
  trackerPath?: string;
  lastSeen?: string;
  firstSeen?: string;
  meetingCount: number;
  updatedAt: string;
  archived: boolean;
}

/** Person with pre-computed signals for list pages (I106). */
export interface PersonListItem extends Person {
  temperature: string;
  trend: string;
  /** Comma-separated names of linked account entities. */
  accountNames?: string;
}

export interface PersonSignals {
  meetingFrequency30d: number;
  meetingFrequency90d: number;
  lastMeeting?: string;
  temperature: string;
  trend: string;
}

export interface PersonDetail extends Person {
  signals?: PersonSignals;
  entities?: { id: string; name: string; entityType: string }[];
  recentMeetings?: {
    id: string;
    title: string;
    startTime: string;
    meetingType: string;
  }[];
  intelligence?: EntityIntelligence;
}

export interface AttendeeContext {
  name: string;
  email?: string;
  role?: string;
  organization?: string;
  relationship?: PersonRelationship;
  meetingCount?: number;
  lastSeen?: string;
  temperature?: string;
  notes?: string;
  personId?: string;
}

// =============================================================================
// Accounts (I72)
// =============================================================================

export type AccountHealth = "green" | "yellow" | "red";

/** Summary item for the accounts list page. */
export interface AccountListItem {
  id: string;
  name: string;
  lifecycle?: string;
  arr?: number;
  health?: AccountHealth;
  nps?: number;
  csm?: string;
  champion?: string;
  renewalDate?: string;
  openActionCount: number;
  daysSinceLastMeeting?: number;
  /** I114: Parent-child hierarchy fields */
  parentId?: string;
  parentName?: string;
  childCount: number;
  isParent: boolean;
  archived: boolean;
}

export interface CompanyOverview {
  description?: string;
  industry?: string;
  size?: string;
  headquarters?: string;
  enrichedAt?: string;
}

export interface StrategicProgram {
  name: string;
  status: string;
  notes?: string;
}

/** Compact meeting summary used across entity detail pages. */
export interface MeetingSummary {
  id: string;
  title: string;
  startTime: string;
  meetingType: string;
}

/** Richer meeting summary with optional prep context (ADR-0063). */
export interface MeetingPreview {
  id: string;
  title: string;
  startTime: string;
  meetingType: string;
  prepContext?: PrepContext;
}

/** Aggregated signals for parent account's children (I114). */
export interface ParentAggregate {
  buCount: number;
  totalArr?: number;
  worstHealth?: AccountHealth;
  nearestRenewal?: string;
}

/** Compact child account summary for parent detail pages (I114). */
export interface AccountChildSummary {
  id: string;
  name: string;
  health?: AccountHealth;
  arr?: number;
  openActionCount: number;
}

/** Full detail for the account detail page. */
export interface AccountDetail extends AccountListItem {
  contractStart?: string;
  companyOverview?: CompanyOverview;
  strategicPrograms: StrategicProgram[];
  notes?: string;
  upcomingMeetings: MeetingSummary[];
  /** ADR-0063: richer type with optional prep context for preview cards. */
  recentMeetings: MeetingPreview[];
  openActions: Action[];
  linkedPeople: Person[];
  signals?: {
    meetingFrequency30d: number;
    meetingFrequency90d: number;
    lastMeeting?: string;
    lastContact?: string;
    temperature: string;
    trend: string;
  };
  recentCaptures: {
    id: string;
    meetingId: string;
    captureType: string;
    content: string;
    meetingTitle: string;
  }[];
  /** I114: Parent-child hierarchy */
  children: AccountChildSummary[];
  parentAggregate?: ParentAggregate;
  /** ADR-0057: Synthesized entity intelligence */
  intelligence?: EntityIntelligence;
}

// =============================================================================
// Content Index (I124)
// =============================================================================

export interface ContentFile {
  id: string;
  entityId: string;
  entityType: string;
  filename: string;
  relativePath: string;
  absolutePath: string;
  format: string;
  fileSize: number;
  modifiedAt: string;
  indexedAt: string;
  extractedAt?: string;
  summary?: string;
  contentType: string;
  priority: number;
}

// =============================================================================
// Entity Intelligence (I130 / ADR-0057)
// =============================================================================

/** Synthesized intelligence for an entity (account, project, or person). */
export interface EntityIntelligence {
  version: number;
  entityId: string;
  entityType: string;
  enrichedAt: string;
  sourceFileCount: number;
  sourceManifest: SourceManifestEntry[];
  executiveAssessment?: string;
  risks: IntelRisk[];
  recentWins: IntelWin[];
  currentState?: IntelCurrentState;
  stakeholderInsights: StakeholderInsight[];
  valueDelivered: ValueItem[];
  nextMeetingReadiness?: IntelMeetingReadiness;
  companyContext?: IntelCompanyContext;
}

export interface SourceManifestEntry {
  filename: string;
  modifiedAt: string;
  format?: string;
}

export interface IntelRisk {
  text: string;
  source?: string;
  urgency: string;
}

export interface IntelWin {
  text: string;
  source?: string;
  impact?: string;
}

export interface IntelCurrentState {
  working: string[];
  notWorking: string[];
  unknowns: string[];
}

export interface StakeholderInsight {
  name: string;
  role?: string;
  assessment?: string;
  engagement?: string;
  source?: string;
}

export interface ValueItem {
  date?: string;
  statement: string;
  source?: string;
  impact?: string;
}

export interface IntelMeetingReadiness {
  meetingTitle?: string;
  meetingDate?: string;
  prepItems: string[];
}

export interface IntelCompanyContext {
  description?: string;
  industry?: string;
  size?: string;
  headquarters?: string;
  additionalContext?: string;
}

// =============================================================================
// Projects (I50)
// =============================================================================

export type ProjectStatus = "active" | "on_hold" | "completed" | "archived";

/** Summary item for the projects list page. */
export interface ProjectListItem {
  id: string;
  name: string;
  status: string;
  milestone?: string;
  owner?: string;
  targetDate?: string;
  openActionCount: number;
  daysSinceLastMeeting?: number;
  archived: boolean;
}

export interface ProjectMilestone {
  name: string;
  status: string;
  targetDate?: string;
  notes?: string;
}

/** Full detail for the project detail page. */
export interface ProjectDetail extends ProjectListItem {
  description?: string;
  milestones: ProjectMilestone[];
  notes?: string;
  openActions: Action[];
  recentMeetings: MeetingSummary[];
  linkedPeople: Person[];
  signals?: {
    meetingFrequency30d: number;
    meetingFrequency90d: number;
    lastMeeting?: string;
    daysUntilTarget?: number;
    openActionCount: number;
    temperature: string;
    trend: string;
  };
  recentCaptures: {
    id: string;
    captureType: string;
    content: string;
    meetingTitle: string;
  }[];
  /** ADR-0057: Synthesized entity intelligence */
  intelligence?: EntityIntelligence;
}

// =============================================================================
// AI Model Config (I174)
// =============================================================================

export interface AiModelConfig {
  synthesis: string;
  extraction: string;
  mechanical: string;
}

// =============================================================================
// Feature Toggles (I39)
// =============================================================================

export interface FeatureDefinition {
  key: string;
  label: string;
  description: string;
  enabled: boolean;
  csOnly: boolean;
}

// =============================================================================
// Processing History (I6)
// =============================================================================

/** A row from the processing_log table in SQLite. */
export interface ProcessingLogEntry {
  id: string;
  filename: string;
  sourcePath: string;
  destinationPath?: string;
  classification: string;
  status: string;
  processedAt?: string;
  errorMessage?: string;
  createdAt: string;
}

// =============================================================================
// Meeting History Detail
// =============================================================================

/** A capture (win/risk/decision) from SQLite. */
export interface DbCapture {
  id: string;
  meetingId: string;
  meetingTitle: string;
  accountId?: string;
  projectId?: string;
  captureType: string;
  content: string;
  capturedAt: string;
}

/** Enriched action with resolved relationships. */
export interface ActionDetail extends DbAction {
  accountName?: string;
  sourceMeetingTitle?: string;
}

/** Full detail for a historical meeting. */
export interface MeetingHistoryDetail {
  id: string;
  title: string;
  meetingType: string;
  startTime: string;
  endTime?: string;
  accountId?: string;
  accountName?: string;
  summary?: string;
  attendees: string[];
  captures: DbCapture[];
  actions: DbAction[];
  /** Persisted pre-meeting prep context (I181). */
  prepContext?: PrepContext;
}

/** Enriched pre-meeting prep context (I181). */
export interface PrepContext {
  intelligenceSummary?: string;
  entityRisks?: Array<{ text: string; urgency?: string; source?: string }>;
  entityReadiness?: string[];
  talkingPoints?: string[];
  recentWins?: string[];
  recentWinSources?: SourceReference[];
  proposedAgenda?: Array<{ topic: string; why?: string; source?: string }>;
  openItems?: Array<{ title: string; dueDate?: string; isOverdue?: boolean }>;
  questions?: string[];
  stakeholderInsights?: Array<{
    name: string;
    role?: string;
    assessment?: string;
  }>;
}

/** Meeting search result (I183). */
export interface MeetingSearchResult {
  id: string;
  title: string;
  meetingType: string;
  startTime: string;
  accountName?: string;
  matchSnippet?: string;
}

// =============================================================================
// Account Events (I143)
// =============================================================================

export type AccountEventType =
  | "renewal"
  | "expansion"
  | "churn"
  | "downgrade";

export interface AccountEvent {
  id: number;
  accountId: string;
  eventType: AccountEventType;
  eventDate: string;
  arrImpact?: number;
  notes?: string;
  createdAt: string;
}

// =============================================================================
// Duplicate People Detection (I172)
// =============================================================================

export interface DuplicateCandidate {
  person1Id: string;
  person1Name: string;
  person2Id: string;
  person2Name: string;
  confidence: number;
  reason: string;
}
