/**
 * Dashboard Types
 * Core type definitions for the DailyOS dashboard
 */

export type ProfileType = "customer-success" | "general";

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

export interface Meeting {
  id: string;
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

export type EmailPriority = "high" | "normal";

export interface Email {
  id: string;
  sender: string;
  senderEmail: string;
  subject: string;
  snippet?: string;
  priority: EmailPriority;
  avatarUrl?: string;
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
  ring?: string;
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
// Full Meeting Prep (from individual prep files)
// =============================================================================

export interface FullMeetingPrep {
  filePath: string;
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
}
