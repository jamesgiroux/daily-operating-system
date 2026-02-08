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

export interface Meeting {
  id: string;
  calendarEventId?: string;
  time: string;
  endTime?: string;
  title: string;
  type: MeetingType;
  account?: string;
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
  projectId?: string;
  sourceType?: string;
  sourceId?: string;
  sourceLabel?: string;
  context?: string;
  waitingOn?: string;
  updatedAt: string;
}

export interface DayStats {
  totalMeetings: number;
  customerMeetings: number;
  actionsDue: number;
  inboxCount: number;
}

export type EmailPriority = "high" | "medium" | "low";

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

// =============================================================================
// Focus Data Types
// =============================================================================

export interface FocusData {
  priorities: FocusPriority[];
  timeBlocks?: TimeBlock[];
  quickWins?: string[];
  energyNotes?: EnergyNotes;
}

export interface FocusPriority {
  level: string;
  label: string;
  items: string[];
}

export interface EnergyNotes {
  morning?: string;
  afternoon?: string;
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
// Weekly Planning Types (Phase 3C)
// =============================================================================

export type WeekPlanningState =
  | "notready"
  | "dataready"
  | "inprogress"
  | "completed"
  | "defaultsapplied";

export interface FocusBlock {
  day: string;
  start: string;
  end: string;
  durationMinutes: number;
  suggestedActivity: string;
  selected: boolean;
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
  /** Quick Context metrics (key-value pairs like Ring, ARR, Health) */
  quickContext?: [string, string][];
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
  questions?: string[];
  keyPrinciples?: string[];
  references?: SourceReference[];
  rawMarkdown?: string;
  /** Stakeholder relationship signals (I43) — computed live from meeting history */
  stakeholderSignals?: StakeholderSignals;
  /** Per-attendee context from people database (I51) */
  attendeeContext?: AttendeeContext[];
  /** Proposed agenda synthesized from prep data (I80) */
  proposedAgenda?: AgendaItem[];
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
  recentMeetings?: { id: string; title: string; startTime: string }[];
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

/** Full detail for the account detail page. */
export interface AccountDetail extends AccountListItem {
  contractStart?: string;
  companyOverview?: CompanyOverview;
  strategicPrograms: StrategicProgram[];
  notes?: string;
  recentMeetings: { id: string; title: string; startTime: string }[];
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
    captureType: string;
    content: string;
    meetingTitle: string;
  }[];
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
  recentMeetings: { id: string; title: string; startTime: string }[];
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
