/**
 * Dashboard Types
 * Core type definitions for the DailyOS dashboard
 */

export type ProfileType = "customer-success" | "general";

export type EntityMode = "account" | "project" | "both";
export type SettingsTabId =
  | "you"
  | "connectors"
  | "system"
  | "diagnostics"
  // Legacy tab IDs — kept for deep-link backwards compatibility
  | "profile"
  | "role"
  | "integrations"
  | "workflows"
  | "intelligence"
  | "hygiene";

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

/** I634: Meeting page temporal lifecycle stage. */
export type MeetingStage = "upcoming" | "in-progress" | "just-ended" | "processed";

/** I637: Meeting-to-meeting continuity thread. */
export interface ThreadAction { title: string; date?: string; isOverdue: boolean; }
export interface HealthDelta { previous: number; current: number; }
export interface ContinuityThread {
  previousMeetingDate?: string;
  previousMeetingTitle?: string;
  entityName?: string;
  actionsCompleted: ThreadAction[];
  actionsOpen: ThreadAction[];
  healthDelta?: HealthDelta;
  newAttendees: string[];
  isFirstMeeting: boolean;
}

/** I635: Prediction scorecard. */
export type PredictionCategory = "confirmed" | "notRaised" | "surprise";
export interface PredictionResult {
  text: string;
  category: PredictionCategory;
  source?: string;
  matchText?: string;
}
export interface PredictionScorecard {
  riskPredictions: PredictionResult[];
  winPredictions: PredictionResult[];
  hasData: boolean;
}

/** Feature flags for gating incomplete features (I537). */
export interface FeatureFlags {
  role_presets_enabled: boolean;
  book_of_business_enabled: boolean;
  glean_discovery_enabled: boolean;
}

export interface DatabaseRecoveryStatus {
  required: boolean;
  reason: string;
  detail: string;
  dbPath: string;
}

export interface BackupInfo {
  path: string;
  createdAt: string;
  sizeBytes: number;
  kind: string;
  filename: string;
  schemaVersion: number | null;
}

export interface DatabaseInfo {
  path: string;
  sizeBytes: number;
  schemaVersion: number;
  lastBackup: string | null;
}

export type Priority = 0 | 1 | 2 | 3 | 4;

export type ActionStatus = "backlog" | "unstarted" | "started" | "completed" | "cancelled" | "archived";

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
  relationship?: PersonRelationship;
}

export interface SourceReference {
  label: string;
  path?: string;
  lastUpdated?: string;
}

export type OverlayStatus = "enriched" | "cancelled" | "new" | "briefing_only";

export interface CalendarAttendee {
  email: string;
  name: string;
  rsvp: string;
  domain: string;
}

export interface LinkedEntity {
  id: string;
  name: string;
  entityType: "account" | "project" | "person";
  /** DOS-74: per-junction confidence (0.0 – 1.0). Higher = stronger match. */
  confidence?: number;
  /** DOS-74: true if this is the primary entity for the meeting. */
  isPrimary?: boolean;
  /** DOS-74: true for low-confidence siblings rendered as muted suggestions. */
  suggested?: boolean;
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
  /** Calendar event description from Google Calendar */
  calendarDescription?: string;
  /** Raw calendar attendees (not AI-enriched) with RSVP status */
  calendarAttendees?: CalendarAttendee[];
  /** Structured intelligence quality assessment for schedule meetings */
  intelligenceQuality?: {
    level: "sparse" | "developing" | "ready" | "fresh";
    signalCount: number;
    lastEnriched?: string;
    hasEntityContext: boolean;
    hasAttendeeHistory: boolean;
    hasRecentSignals: boolean;
    staleness: "current" | "aging" | "stale";
    hasNewSignals: boolean;
  };
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
  priority: number;
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
  /** Next upcoming meeting title for the action's account (I342) */
  nextMeetingTitle?: string;
  /** Next upcoming meeting start time for the action's account (I342) */
  nextMeetingStart?: string;
  /** Whether this action requires a decision (DOS-17) */
  needsDecision?: boolean;
  /** Who owns the decision (DOS-17) */
  decisionOwner?: string;
  /** What's at stake if the decision is delayed (DOS-17) */
  decisionStakes?: string;
  /** Linear issue identifier when pushed to Linear (DOS-52) */
  linearIdentifier?: string;
  /** Linear issue URL when pushed to Linear (DOS-52) */
  linearUrl?: string;
}

/** Result of pushing an action to Linear (DOS-52). */
export interface LinearPushResult {
  identifier: string;
  url: string;
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

/** Email sync stats from DB enrichment state counts (I373 / DOS-31). */
export interface EmailSyncStats {
  lastFetchAt: string | null;
  /**
   * DOS-31: Last time the Gmail fetch itself completed successfully,
   * independent of enrichment success. When `lastFetchAt` is stale but
   * `lastSuccessfulFetchAt` is recent, the inbox is healthy and only the
   * enrichment pipeline is stuck — different message than "can't reach Gmail".
   */
  lastSuccessfulFetchAt: string | null;
  total: number;
  enriched: number;
  pending: number;
  failed: number;
  /**
   * DOS-29: Subset of `failed` that has exhausted automatic retries. Rows
   * still under the auto-retry cap will be silently re-attempted by the
   * next refresh (DOS-31) and shouldn't bother the user. The failure UX
   * shows `permanentlyFailed`, not `failed`.
   */
  permanentlyFailed: number;
}

/** DOS-29: Lightweight preview of a permanently-failed email for the
 *  "View details" expansion on the EmailsPage failure UX. */
export interface FailedEmailPreview {
  emailId: string;
  subject: string | null;
  senderEmail: string | null;
  senderName: string | null;
  lastEnrichmentAt: string | null;
  autoRetryCount: number;
}

export interface Email {
  id: string;
  sender: string;
  senderEmail: string;
  subject: string;
  snippet?: string;
  priority: EmailPriority;
  /** Whether this email is unread in Gmail */
  isUnread?: boolean;
  avatarUrl?: string;
  /** AI-generated one-line summary of the email */
  summary?: string;
  /** Suggested next action (e.g. "Reply with counter-proposal") */
  recommendedAction?: string;
  /** Thread history arc (e.g. "Initial outreach → follow-up → this response") */
  conversationArc?: string;
  /** Email category from AI classification */
  emailType?: string;
  /** Commitments extracted from the email (I354) */
  commitments?: string[];
  /** Questions requiring a response (I354) */
  questions?: string[];
  /** Overall sentiment: positive, neutral, negative, urgent (I354) */
  sentiment?: string;
  /** Urgency from AI enrichment (I369) */
  urgency?: string;
  /** Resolved entity ID from enrichment (I368) */
  entityId?: string;
  /** Resolved entity type (account, person, project) */
  entityType?: string;
  /** Human-readable entity name */
  entityName?: string;
  /** Relevance score from scoring pipeline (I395) — 0.0 to 1.0 */
  relevanceScore?: number;
  /** Human-readable score reason (I395) */
  scoreReason?: string;
  /** When this email was pinned for triage sort boost (I579) */
  pinnedAt?: string;
  /** Actions created from commitments extracted from this email (I580) */
  trackedCommitments?: TrackedEmailCommitment[];
  /** Meeting this email's sender is attending (upcoming only) (I582) */
  meetingLinked?: LinkedMeeting;
}

/** An upcoming meeting linked to an email via sender-attendee match (I582). */
export interface LinkedMeeting {
  meetingId: string;
  title: string;
  startTime: string;
}

export interface TrackedEmailCommitment {
  actionId: string;
  commitmentText: string;
  actionTitle: string;
  dueDate?: string;
  owner?: string;
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
  suggestedEntityName?: string;
}

export type DataFreshness =
  | { freshness: "fresh"; generatedAt: string }
  | { freshness: "stale"; dataDate: string; generatedAt: string }
  | { freshness: "unknown" };

/** A thread awaiting the user's reply (I318 — "ball in your court"). */
export interface ReplyNeeded {
  threadId: string;
  subject: string;
  from: string;
  date?: string;
  waitDuration?: string;
}

/** I577: Reply debt item — an email where the ball is in the user's court. */
export interface ReplyDebtItem {
  emailId: string;
  senderName: string;
  senderEmail: string;
  subject: string;
  /** AI-generated contextual summary */
  summary?: string;
  entityId?: string;
  entityName?: string;
  entityType?: string;
  ageHours: number;
  /** "today", "1-2 days", "3-5 days", "overdue" */
  ageBucket: string;
  urgency?: string;
  sentiment?: string;
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
  /** User org domains for internal/external attendee grouping */
  userDomains?: string[];
  emails?: Email[];
  emailSync?: EmailSyncStatus;
  focus?: DailyFocus;
  lifecycleUpdates?: DashboardLifecycleUpdate[];
  /** AI-synthesized email narrative (I322/I355) */
  emailNarrative?: string;
  /** Threads awaiting user reply (I318/I355) */
  repliesNeeded?: ReplyNeeded[];
  /** I502: Health data keyed by entity ID for accounts linked to today's meetings. */
  entityHealthMap?: Record<string, IntelligenceAccountHealth>;
  /** Briefing callouts from signal propagation (I623 AC4). */
  briefingCallouts?: BriefingCallout[];
  /** DOS-53: Count of actions approaching the 30-day auto-archive threshold. */
  agingActionCount?: number;
}

/** A briefing callout surfaced to the daily briefing (I623). */
export interface BriefingCallout {
  id: string;
  entityId: string;
  entityType: string;
  entityName?: string | null;
  calloutType: string;
  headline: string;
  detail?: string | null;
  severity: string;
  createdAt: string;
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
  meetingId?: string;
  type: MeetingType;
  prepStatus: PrepStatus;
  /** Entities linked via M2M junction table or entity resolution (I339) */
  linkedEntities?: LinkedEntity[];
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
  /** Per-day prioritized actions from live DB enrichment (I279) */
  prioritizedActions?: PrioritizedAction[];
  focusImplications?: {
    achievableCount: number;
    totalCount: number;
    atRiskCount: number;
    summary: string;
  };
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
  actionId?: string;
  meetingId?: string;
}

/** AI-identified top priority for the week (I94) */
export interface TopPriority {
  title: string;
  reason: string;
  meetingId?: string;
  actionId?: string;
}

export interface LiveProactiveSuggestion {
  day: string;
  date: string;
  start: string;
  end: string;
  durationMinutes: number;
  title: string;
  reason: string;
  source: "live" | string;
  actionId?: string;
  meetingId?: string;
  capacityFit: number;
  urgencyImpact: number;
  confidence: number;
  totalScore: number;
}

// =============================================================================
// Focus Data Types
// =============================================================================

/** Capacity-aware daily focus: ranked actions against today's available time. */
export interface DailyFocus {
  availableMinutes: number;
  deepWorkMinutes: number;
  meetingMinutes: number;
  meetingCount: number;
  prioritizedActions: PrioritizedAction[];
  topThree: string[];
  implications: FocusImplications;
  availableBlocks: TimeBlock[];
}

/** A single action ranked by urgency/effort/feasibility against capacity. */
export interface PrioritizedAction {
  action: DbAction;
  score: number;
  effortMinutes: number;
  feasible: boolean;
  atRisk: boolean;
  reason: string;
}

/** High-level summary of achievable vs at-risk actions. */
export interface FocusImplications {
  achievableCount: number;
  totalCount: number;
  atRiskCount: number;
  summary: string;
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
  emailSignals?: EmailSignal[];
}

export interface EmailSignal {
  id?: number;
  emailId?: string;
  senderEmail?: string;
  personId?: string;
  entityId?: string;
  entityType?: string;
  signalType: string;
  signalText: string;
  confidence?: number;
  sentiment?: string;
  urgency?: string;
  detectedAt?: string;
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
// Enriched Email Briefing Types (Phase 4 — Email Intelligence)
// =============================================================================

export interface EnrichedEmail extends Email {
  signals: EmailSignal[];
}

export interface EntityEmailThread {
  entityId: string;
  entityName: string;
  entityType: string;
  emailCount: number;
  signalSummary: string;
  signals: EmailSignal[];
}

export interface EmailBriefingStats {
  total: number;
  highCount: number;
  mediumCount: number;
  lowCount: number;
  needsAction: number;
}

export interface EmailBriefingData {
  highPriority: EnrichedEmail[];
  mediumPriority: EnrichedEmail[];
  lowPriority: EnrichedEmail[];
  entityThreads: EntityEmailThread[];
  stats: EmailBriefingStats;
  hasEnrichment: boolean;
  /** AI-synthesized narrative headline for the dispatch (I355) */
  emailNarrative?: string;
  /** Threads awaiting user reply — "ball in your court" (I355) */
  repliesNeeded?: ReplyNeeded[];
  /** I577: Reply debt — entity-linked emails awaiting user reply. */
  replyDebt?: ReplyDebtItem[];
  /** Accounts whose email cadence has dropped significantly (I581) */
  goneQuiet?: GoneQuietAccount[];
}

/** An account whose email cadence has gone quiet (I581). */
export interface GoneQuietAccount {
  entityId: string;
  entityName: string;
  entityType: string;
  normalIntervalDays: number;
  daysSinceLastEmail: number;
  lastEmailDate?: string;
  lastEmailSender?: string;
  historicalEmailCount: number;
}

// =============================================================================
// Post-Meeting Intelligence (I558)
// =============================================================================

export interface SpeakerSentiment {
  name: string;
  sentiment: string;
  evidence: string;
}

export interface CompetitorMention {
  competitor: string;
  context: string;
}

export interface EscalationQuote {
  quote: string;
  speaker: string;
}

export interface InteractionDynamics {
  meetingId: string;
  talkBalanceCustomerPct?: number;
  talkBalanceInternalPct?: number;
  speakerSentiments: SpeakerSentiment[];
  questionDensity?: string;
  decisionMakerActive?: string;
  forwardLooking?: string;
  monologueRisk: boolean;
  competitorMentions: CompetitorMention[];
  escalationLanguage: EscalationQuote[];
}

export interface ChampionHealthAssessment {
  meetingId: string;
  championName?: string;
  championStatus: string;
  championEvidence?: string;
  championRisk?: string;
}

export interface RoleChange {
  id: string;
  meetingId: string;
  personName: string;
  oldStatus?: string;
  newStatus?: string;
  evidenceQuote?: string;
}

export interface EnrichedCapture {
  id: string;
  meetingId: string;
  meetingTitle: string;
  accountId?: string;
  captureType: string;
  content: string;
  subType?: string;
  urgency?: string;
  impact?: string;
  evidenceQuote?: string;
  speaker?: string;
  capturedAt: string;
}

export interface MeetingPostIntelligence {
  interactionDynamics?: InteractionDynamics;
  championHealth?: ChampionHealthAssessment;
  roleChanges: RoleChange[];
  enrichedCaptures: EnrichedCapture[];
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

export type GleanAuthStatus =
  | { status: "notconfigured" }
  | { status: "authenticated"; email: string; name?: string };

// I561 — Onboarding: Three Connectors
export interface OnboardingImportResult {
  created: number;
  failed: string[];
}

export interface UserProfileSuggestion {
  name: string | null;
  title: string | null;
  department: string | null;
  company: string | null;
}

export interface EnrichmentProgress {
  entityId: string;
  name: string;
  status: "queued" | "analyzing" | "complete" | "failed";
  completed: number;
  total: number;
  stakeholderCount: number;
  riskCount: number;
}

export interface GleanTokenHealth {
  connected: boolean;
  status: "healthy" | "expiring" | "expired" | "not_connected";
  expiresAt: string | null;
  expiresInHours: number | null;
}

export interface HygieneFixView {
  key: string;
  label: string;
  count: number;
}

export interface HygieneFixDetail {
  fixType: string;
  entityName?: string;
  description: string;
}

export interface HygieneGapActionView {
  kind: "navigate" | "run_scan_now";
  label: string;
  route?: string;
}

export interface HygieneGapView {
  key: string;
  label: string;
  count: number;
  impact: "critical" | "medium" | "low";
  description: string;
  action: HygieneGapActionView;
}

export interface HygieneBudgetView {
  usedToday: number;
  dailyLimit: number;
  queuedForNextBudget: number;
}

export interface HygieneStatusView {
  status: "running" | "healthy" | "needs_attention";
  statusLabel: string;
  lastScanTime?: string;
  nextScanTime?: string;
  totalGaps: number;
  totalFixes: number;
  isRunning: boolean;
  fixes: HygieneFixView[];
  fixDetails: HygieneFixDetail[];
  gaps: HygieneGapView[];
  budget: HygieneBudgetView;
  scanDurationMs?: number;
}

export interface HygieneNarrativeView {
  narrative: string;
  remainingGaps: HygieneGapSummary[];
  lastScanTime?: string;
  totalFixes: number;
  totalRemainingGaps: number;
}

export interface HygieneGapSummary {
  label: string;
  count: number;
  severity: "critical" | "medium" | "low";
}

export interface CalendarEvent {
  id: string;
  title: string;
  start: string;
  end: string;
  type: MeetingType;
  account?: string;
  linkedEntities?: LinkedEntity[];
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
  transcriptWaitMinutes?: number;
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

/** Sentiment analysis from transcript processing (I509) */
export interface TranscriptSentiment {
  overall?: string;
  customer?: string;
  engagement?: string;
  forwardLooking: boolean;
  competitorMentions: string[];
  championPresent?: boolean;
  championEngaged?: boolean;
}

/** Engagement quality signals from a transcript (I509) */
export interface EngagementSignals {
  questionDensity?: string;
  decisionMakerActive?: string;
  forwardLooking?: string;
  monologueRisk?: boolean;
}

/** An escalation signal detected in meeting language (I509) */
export interface EscalationSignal {
  quote: string;
  speaker?: string;
}

/** Result of transcript processing */
export interface TranscriptResult {
  status: "success" | "error";
  summary?: string;
  destination?: string;
  wins: string[];
  risks: string[];
  decisions: string[];
  actions: CapturedAction[];
  discussion: string[];
  analysis?: string;
  message?: string;
  sentiment?: TranscriptSentiment;
  interactionDynamics?: InteractionDynamics;
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

export interface DbMeeting {
  id: string;
  title: string;
  meetingType: string;
  startTime: string;
  endTime?: string;
  accountId?: string;
  attendees?: string;
  notesPath?: string;
  summary?: string;
  createdAt: string;
  calendarEventId?: string;
  description?: string;
  prepContextJson?: string;
  userAgendaJson?: string;
  userNotes?: string;
  prepFrozenJson?: string;
  prepFrozenAt?: string;
  prepSnapshotPath?: string;
  prepSnapshotHash?: string;
  transcriptPath?: string;
  transcriptProcessedAt?: string;
}

export interface MeetingIntelligence {
  meeting: DbMeeting;
  prep?: FullMeetingPrep;
  isPast: boolean;
  isCurrent: boolean;
  isFrozen: boolean;
  canEditUserLayer: boolean;
  userAgenda?: string[];
  userNotes?: string;
  dismissedTopics?: string[];
  hiddenAttendees?: string[];
  outcomes?: MeetingOutcomeData;
  captures: DbCapture[];
  actions: DbAction[];
  linkedEntities: LinkedEntity[];
  prepSnapshotPath?: string;
  prepFrozenAt?: string;
  transcriptPath?: string;
  transcriptProcessedAt?: string;
  /** Structured intelligence quality assessment (ADR-0081) */
  intelligenceQuality?: {
    level: "sparse" | "developing" | "ready" | "fresh";
    signalCount: number;
    lastEnriched?: string;
    hasEntityContext: boolean;
    hasAttendeeHistory: boolean;
    hasRecentSignals: boolean;
    staleness: "current" | "aging" | "stale";
    hasNewSignals: boolean;
  };
  /** I502: Health data keyed by entity ID for linked accounts that have intelligence health. */
  entityHealthMap?: Record<string, IntelligenceAccountHealth>;
}

export interface ApplyPrepPrefillResult {
  meetingId: string;
  addedAgendaItems: number;
  notesAppended: boolean;
  mode: string;
}

export interface AgendaDraftResult {
  meetingId: string;
  subject?: string;
  body: string;
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
  /** Intelligence summary — executive assessment from entity_assessment DB (I513). */
  intelligenceSummary?: string;
  /** Entity-level risks from intelligence.json (I135) */
  entityRisks?: IntelRisk[];
  /** Entity meeting readiness items from intelligence.json (I135) */
  entityReadiness?: string[];
  /** Stakeholder insights from intelligence.json (I135) */
  stakeholderInsights?: StakeholderInsight[];
  /** Recent email-derived signals linked to meeting entity context (I215) */
  recentEmailSignals?: EmailSignal[];
  /** Structured digest of linked recent correspondence (I582 / I317). */
  emailDigest?: MeetingEmailDigest;
  /** I527: Deterministic consistency status from intelligence checks. */
  consistencyStatus?: ConsistencyStatus;
  /** I527: Deterministic consistency findings for trust transparency. */
  consistencyFindings?: ConsistencyFinding[];
}

export interface MeetingEmailDigest {
  threadSummary: string;
  senderCount: number;
  threads: MeetingEmailDigestThread[];
}

export interface MeetingEmailDigestThread {
  from: string;
  snippet: string;
  date: string;
  source: string;
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
  // Clay enrichment fields (I228)
  linkedinUrl?: string;
  twitterHandle?: string;
  phone?: string;
  photoUrl?: string;
  bio?: string;
  titleHistory?: Array<{
    title: string;
    company: string;
    startDate?: string;
    endDate?: string;
  }>;
  companyIndustry?: string;
  companySize?: string;
  companyHq?: string;
  lastEnrichedAt?: string;
  enrichmentSources?: Record<string, { source: string; at: string }>;
}

/** Person with pre-computed signals for list pages (I106). */
export interface PersonListItem extends Person {
  temperature: string;
  trend: string;
  /** Comma-separated names of linked account entities. */
  accountNames?: string;
  daysSinceLastMeeting?: number;
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
  recentCaptures?: {
    id: string;
    captureType: string;
    content: string;
    meetingTitle: string;
    meetingId?: string;
  }[];
  recentEmailSignals?: EmailSignal[];
  intelligence?: EntityIntelligence;
  openActions: Action[];
  upcomingMeetings?: { id: string; title: string; startTime: string; meetingType: string }[];
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

/** Account classification: customer, internal org, or partner (I382). */
export type AccountType = "customer" | "internal" | "partner";

/** I494: Account discovered from Glean search. */
export interface DiscoveredAccount {
  name: string;
  myRole: string | null;
  evidence: string | null;
  source: string | null;
  domain: string | null;
  industry: string | null;
  contextPreview: string | null;
  alreadyInDailyos: boolean;
}

/** I495: A single section within an ephemeral briefing. */
export interface BriefingSection {
  title: string;
  content: string;
  source: string | null;
}

/** I495: One-shot briefing about an account from Glean. */
export interface EphemeralBriefing {
  name: string;
  summary: string;
  sections: BriefingSection[];
  sourceCount: number;
  /** Entity ID if the account already exists in DailyOS. */
  alreadyExists: string | null;
}

/** Summary item for the accounts list page. */
export interface AccountListItem {
  id: string;
  name: string;
  lifecycle?: string;
  arr?: number;
  health?: AccountHealth;
  nps?: number;
  renewalDate?: string;
  openActionCount: number;
  daysSinceLastMeeting?: number;
  /** I114: Parent-child hierarchy fields */
  parentId?: string;
  parentName?: string;
  childCount: number;
  isParent: boolean;
  accountType: AccountType;
  archived: boolean;
  /** I502: Intelligence health data when available (populated from entity_intelligence). */
  intelligenceHealth?: IntelligenceAccountHealth | null;
  /** DOS-110: User's manual health sentiment assessment. */
  userHealthSentiment?: string;
  /** DOS-110: When the sentiment was last set. */
  sentimentSetAt?: string;
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
  accountType: string;
}

export interface AccountTeamMember {
  accountId: string;
  personId: string;
  personName: string;
  personEmail: string;
  role: string;
  createdAt: string;
}

/** DB-first stakeholder with full person data and provenance. */
export interface StakeholderFull {
  personId: string;
  personName: string;
  personEmail?: string | null;
  organization?: string | null;
  personRole?: string | null;
  /** Comma-separated roles from account_stakeholder_roles. */
  stakeholderRole: string;
  /** Typed multi-role assignments with per-role provenance (I652). */
  roles: StakeholderRole[];
  dataSource: string;
  /** Engagement level (I652). */
  engagement?: string | null;
  /** Provenance for engagement (I652). */
  dataSourceEngagement?: string | null;
  /** Free-text assessment (I652). */
  assessment?: string | null;
  /** Provenance for assessment (I652). */
  dataSourceAssessment?: string | null;
  lastSeenInGlean?: string | null;
  createdAt: string;
  linkedinUrl?: string | null;
  photoUrl?: string | null;
  meetingCount?: number | null;
  lastSeen?: string | null;
}

export interface StakeholderRole {
  role: string;
  dataSource: string;
}

export interface StakeholderSuggestion {
  id: number;
  accountId: string;
  personId?: string | null;
  suggestedName?: string | null;
  suggestedEmail?: string | null;
  suggestedRole?: string | null;
  suggestedEngagement?: string | null;
  source: string;
  status: string;
  createdAt: string;
  resolvedAt?: string | null;
}

export interface AccountTeamImportNote {
  id: number;
  accountId: string;
  legacyField: string;
  legacyValue: string;
  note: string;
  createdAt: string;
}

/** Full detail for the account detail page. */
export interface AccountDetail extends AccountListItem {
  contractStart?: string;
  renewalStage?: string | null;
  /** I646 C3: Separate commercial opportunity stage. */
  commercialStage?: string | null;
  /** JSON-serialized string[] of resolution keywords (I305) */
  keywords?: string;
  /** ISO timestamp when keywords were last extracted (I305) */
  keywordsExtractedAt?: string;
  companyOverview?: CompanyOverview;
  strategicPrograms: StrategicProgram[];
  notes?: string;
  upcomingMeetings: MeetingSummary[];
  /** ADR-0063: richer type with optional prep context for preview cards. */
  recentMeetings: MeetingPreview[];
  /** DOS-233 Codex fix: total meeting count for the About-dossier chapter.
   * `recentMeetings` is capped at 10 for preview rendering; this is the
   * unbounded COUNT(*) source of truth. */
  meetingTotalCount?: number;
  /** DOS-233 Codex fix: total transcripts-on-record count, unbounded. */
  transcriptTotalCount?: number;
  openActions: Action[];
  linkedPeople: Person[];
  accountTeam: AccountTeamMember[];
  accountTeamImportNotes: AccountTeamImportNote[];
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
  recentEmailSignals?: EmailSignal[];
  /** I114: Parent-child hierarchy */
  children: AccountChildSummary[];
  parentAggregate?: ParentAggregate;
  objectives: AccountObjective[];
  lifecycleChanges?: LifecycleChange[];
  products?: AccountProduct[];
  fieldProvenance?: AccountFieldProvenance[];
  fieldConflicts?: AccountFieldConflictSuggestion[];
  /** ADR-0057: Synthesized entity intelligence */
  intelligence?: EntityIntelligence;
  /** I628 AC5: Recently auto-completed milestones for timeline display. */
  autoCompletedMilestones?: AccountMilestone[];
  /** I649: Technical footprint, adoption, and service-delivery data. */
  technicalFootprint?: AccountTechnicalFootprint;
  /** DB-first stakeholder read model: all stakeholders with provenance. */
  stakeholdersFull?: StakeholderFull[];
  /** I644: Per-field source attribution from account_source_refs. */
  sourceRefs?: AccountSourceRef[];
  /** DOS-27: Most recent journal note attached to the current sentiment value. */
  sentimentNote?: string;
  /** DOS-27: Sentiment journal entries from the last 90 days, newest-first. */
  sentimentHistory?: SentimentJournalEntry[];
  /** DOS-27: Daily computed-health sparkline points (last 90 days, chronological). */
  healthSparkline?: HealthSparklinePoint[];
  /** DOS-15: Glean leading-signal enrichment bundle (health & outlook signals). */
  gleanSignals?: HealthOutlookSignals | null;
  /**
   * DOS-228 Wave 0e Fix 4: Current risk-briefing generation job status.
   * Present when a briefing has ever been enqueued for this account. The
   * Health tab uses this to pin a "generating…" affordance at the top while
   * status === "running", surface the error + retry button when
   * status === "failed", and confirm success with the completedAt timestamp.
   */
  riskBriefingJob?: RiskBriefingJob;
}

/** DOS-228 Wave 0e Fix 4: Risk-briefing job status contract. */
export interface RiskBriefingJob {
  /** One of: `enqueued`, `running`, `complete`, `failed`. */
  status: "enqueued" | "running" | "complete" | "failed";
  enqueuedAt: string;
  completedAt?: string;
  errorMessage?: string;
}

/** DOS-27: A single sentiment journal entry. */
export type SentimentValue =
  | "strong"
  | "on_track"
  | "concerning"
  | "at_risk"
  | "critical";

export interface SentimentJournalEntry {
  sentiment: SentimentValue;
  note?: string;
  computedBand?: string;
  computedScore?: number;
  setAt: string;
}

/** DOS-27: One day of computed health score for the sparkline. */
export interface HealthSparklinePoint {
  day: string;
  score: number;
  band: string;
}

/** DOS-15: Glean leading-signal enrichment types (Health & Outlook tab). */
export interface HealthOutlookSignals {
  championRisk?: ChampionRiskSignal | null;
  productUsageTrend?: ProductUsageTrendSignal | null;
  channelSentiment?: ChannelSentimentSignal | null;
  transcriptExtraction?: TranscriptExtractionSignal | null;
  commercialSignals?: CommercialSignalsBlock | null;
  advocacyTrack?: AdvocacyTrackSignal | null;
  quoteWall: QuoteWallEntry[];
  /** Trend signals from a separate PTY pass (DOS-204). `null` until that pass runs. */
  trends?: TrendSignals | null;
}

export interface ChampionRiskSignal {
  championName?: string | null;
  atRisk: boolean;
  riskLevel?: "low" | "moderate" | "high" | null;
  riskEvidence: string[];
  tenureSignal?: string | null;
  recentRoleChange?: string | null;
  emailSentimentTrend30d?: "warming" | "stable" | "cooling" | null;
  emailResponseTimeTrend?: "faster" | "stable" | "slower" | "unknown" | null;
  backupChampionCandidates: {
    name: string;
    role?: string | null;
    why?: string | null;
    engagementLevel?: "high" | "medium" | "low" | null;
  }[];
}

export interface ProductUsageTrendSignal {
  overallTrend30d?: "growing" | "stable" | "declining" | "unknown" | null;
  overallTrend90d?: "growing" | "stable" | "declining" | "unknown" | null;
  features: {
    name: string;
    adoptionStatus?: string | null;
    activeUsersEstimate?: unknown;
    usageTrend30d?: string | null;
    evidence?: string | null;
  }[];
  underutilizedFeatures: {
    name: string;
    licensedButUnusedDays?: number | null;
    coachingOpportunity?: string | null;
  }[];
  highlyStickyFeatures: { name: string; whySticky?: string | null }[];
  summary?: string | null;
}

export interface ChannelSentimentSignal {
  email?: ChannelReading | null;
  meetings?: ChannelReading | null;
  supportTickets?: ChannelReading | null;
  slack?: ChannelReading | null;
  divergenceDetected: boolean;
  divergenceSummary?: string | null;
}

export interface ChannelReading {
  sentiment?: string | null;
  trend30d?: "warming" | "stable" | "cooling" | null;
  evidence?: string | null;
}

export interface TranscriptExtractionSignal {
  churnAdjacentQuestions: TranscriptQuestion[];
  expansionAdjacentQuestions: TranscriptQuestion[];
  competitorBenchmarks: {
    competitor: string;
    context?: string | null;
    threatLevel?:
      | "mentioned"
      | "evaluating"
      | "actively_comparing"
      | "decision_relevant"
      | null;
    date?: string | null;
    source?: string | null;
  }[];
  decisionMakerShifts: {
    shift: string;
    who?: string | null;
    date?: string | null;
    source?: string | null;
    implication?: string | null;
  }[];
  budgetCycleSignals: {
    signal: string;
    date?: string | null;
    source?: string | null;
    implication?: string | null;
    locked: boolean;
  }[];
}

export interface TranscriptQuestion {
  question: string;
  speaker?: string | null;
  date?: string | null;
  source?: string | null;
  riskSignal?: string | null;
  opportunitySignal?: string | null;
  estimatedArrUpside?: unknown;
}

export interface CommercialSignalsBlock {
  arrTrend12mo: { period: string; arr?: number | null; note?: string | null }[];
  arrDirection?: "growing" | "flat" | "shrinking" | null;
  paymentBehavior?: string | null;
  paymentEvidence?: string | null;
  discountHistory: {
    date?: string | null;
    percentOrAmount?: string | null;
    reason?: string | null;
  }[];
  discountAppetiteRemaining?: "full" | "partial" | "exhausted" | "unknown" | null;
  budgetCycleAlignment?: string | null;
  procurementComplexity?: {
    lastCycleLengthDays?: number | null;
    signersRequired?: number | null;
    legalReviewRequired?: boolean | null;
    knownGotchas?: string | null;
  } | null;
  previousRenewalOutcome?: string | null;
}

export interface AdvocacyTrackSignal {
  isReferenceCustomer?: boolean | null;
  logoPermission?: "yes" | "no" | "requested" | "unknown" | null;
  caseStudy?: {
    published?: boolean | null;
    inProgress?: boolean | null;
    topic?: string | null;
    publishDate?: string | null;
  } | null;
  speakingSlots: {
    event: string;
    date?: string | null;
    speaker?: string | null;
    topic?: string | null;
  }[];
  betaProgramsIn: {
    program: string;
    enrolledDate?: string | null;
    engagementLevel?: string | null;
  }[];
  referralsMade: {
    referredCompany: string;
    outcome?: string | null;
    date?: string | null;
  }[];
  npsHistory: {
    surveyDate?: string | null;
    score?: number | null;
    verbatim?: string | null;
    respondent?: string | null;
  }[];
  advocacyTrend?: "strengthening" | "stable" | "cooling" | null;
}

export interface QuoteWallEntry {
  quote: string;
  speaker?: string | null;
  role?: string | null;
  date?: string | null;
  source?: string | null;
  sentiment?: "positive" | "neutral" | "negative" | "mixed" | null;
  whyItMatters?: string | null;
}

export interface TrendSignals {
  usageTrajectory: unknown[];
  sentimentOverTime: unknown[];
}

/** I644: Source reference for a tracked account field. */
export interface AccountSourceRef {
  id: string;
  accountId: string;
  field: string;
  sourceSystem: string;
  sourceKind: string;
  sourceValue?: string | null;
  observedAt: string;
}

/** I649: Technical footprint data for an account. */
export interface AccountTechnicalFootprint {
  usageTier?: string | null;
  adoptionScore?: number | null;
  activeUsers?: number | null;
  supportTier?: string | null;
  csatScore?: number | null;
  openTickets?: number | null;
  servicesStage?: string | null;
  source: string;
  sourcedAt: string;
}

export interface AccountFieldProvenance {
  field: string;
  source: string;
  updatedAt?: string | null;
}

export interface LifecycleChange {
  id: number;
  accountId: string;
  previousLifecycle?: string | null;
  newLifecycle: string;
  previousStage?: string | null;
  newStage?: string | null;
  previousContractEnd?: string | null;
  newContractEnd?: string | null;
  source: string;
  confidence: number;
  evidence?: string | null;
  healthScoreBefore?: number | null;
  healthScoreAfter?: number | null;
  userResponse: string;
  responseNotes?: string | null;
  createdAt: string;
  reviewedAt?: string | null;
}

export interface AccountProduct {
  id: number;
  accountId: string;
  name: string;
  category?: string | null;
  status: string;
  arrPortion?: number | null;
  source: string;
  confidence: number;
  notes?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface AccountFieldConflictSuggestion {
  field: string;
  source: string;
  suggestedValue: string;
  signalId: string;
  confidence: number;
  detectedAt?: string | null;
}

export interface DashboardLifecycleUpdate {
  changeId: number;
  accountId: string;
  accountName: string;
  previousLifecycle?: string | null;
  newLifecycle: string;
  renewalStage?: string | null;
  source: string;
  confidence: number;
  evidence?: string | null;
  healthScoreBefore?: number | null;
  healthScoreAfter?: number | null;
  actionState: string;
  createdAt: string;
}

export interface PickerAccount {
  id: string;
  name: string;
  parentName?: string;
  accountType: AccountType;
}

export interface OnboardingPrimingCard {
  id: string;
  title: string;
  startTime?: string;
  dayLabel: string;
  suggestedEntityId?: string;
  suggestedEntityName?: string;
  suggestedAction: string;
}

export interface OnboardingPrimingContext {
  googleConnected: boolean;
  cards: OnboardingPrimingCard[];
  prompt: string;
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

/** A record of a user edit to an intelligence field (protects from AI overwrite). */
export interface UserEdit {
  fieldPath: string;
  editedAt: string;
}

export type ConsistencyStatus = "ok" | "corrected" | "flagged";
export type ConsistencySeverity = "high" | "medium" | "low";

/** Deterministic contradiction finding recorded during consistency checks (I527). */
export interface ConsistencyFinding {
  code: string;
  severity: ConsistencySeverity;
  fieldPath: string;
  claimText: string;
  evidenceText: string;
  autoFixed: boolean;
}

/** A child account flagged as a hotspot in a parent's portfolio assessment (I384). */
export interface PortfolioHotspot {
  childId: string;
  childName: string;
  reason: string;
}

/** Portfolio-level intelligence for parent accounts (I384). */
export interface PortfolioIntelligence {
  healthSummary?: string;
  hotspots: PortfolioHotspot[];
  crossBuPatterns: string[];
  portfolioNarrative?: string;
}

/** A key relationship in a person's network (I391, ADR-0088). */
export interface NetworkKeyRelationship {
  personId: string;
  name: string;
  relationshipType: string;
  confidence: number;
  signalSummary?: string;
}

/** Network intelligence for person entities (I391, ADR-0088). */
export interface NetworkIntelligence {
  health: 'strong' | 'at_risk' | 'weakened' | 'unknown';
  keyRelationships: NetworkKeyRelationship[];
  risks: string[];
  opportunities: string[];
  influenceRadius: number;
  clusterSummary?: string;
}

/** A person's stakeholder role for a specific account. */
export interface PersonAccountRole {
  accountId: string;
  accountName: string;
  role: string;
  dataSource: string;
}

/** A person-to-person relationship edge (I390, ADR-0088). */
export interface PersonRelationshipEdge {
  id: string;
  fromPersonId: string;
  toPersonId: string;
  fromPersonName?: string;
  toPersonName?: string;
  relationshipType: string;
  direction: string;
  confidence: number;
  effectiveConfidence: number;
  contextEntityId?: string;
  contextEntityType?: string;
  contextEntityName?: string;
  source: string;
  rationale?: string;
  createdAt: string;
  updatedAt: string;
  lastReinforcedAt?: string;
}

/** ADR-0097: Structured health model used in entity intelligence payloads. */
export interface IntelligenceAccountHealth {
  score: number;
  band: "green" | "yellow" | "red";
  source: "org" | "computed" | "userSet";
  confidence: number;
  /** DOS-84: true when >= 3 of 6 health dimensions have data.
   *  When false, UI should show "Insufficient Data" instead of the score. */
  sufficientData?: boolean;
  trend: IntelligenceHealthTrend;
  dimensions: RelationshipDimensions;
  divergence?: HealthDivergence | null;
  narrative?: string | null;
  recommendedActions?: string[];
}

export interface IntelligenceHealthTrend {
  direction: "improving" | "stable" | "declining" | "volatile";
  rationale?: string | null;
  timeframe?: string;
  confidence?: number;
}

export interface RelationshipDimensions {
  meetingCadence: DimensionScore;
  emailEngagement: DimensionScore;
  stakeholderCoverage: DimensionScore;
  championHealth: DimensionScore;
  financialProximity: DimensionScore;
  signalMomentum: DimensionScore;
}

export interface DimensionScore {
  score: number;
  weight: number;
  evidence?: string[];
  trend: "improving" | "stable" | "declining";
}

export interface HealthDivergence {
  severity: "minor" | "notable" | "critical";
  description: string;
  leadingIndicator: boolean;
}

export interface OrgHealthData {
  healthBand?: string;
  healthScore?: number;
  renewalLikelihood?: string;
  growthTier?: string;
  customerStage?: string;
  supportTier?: string;
  icpFit?: string;
  source: string;
  gatheredAt: string;
}

/** Intelligence-layer transcript sentiment (from io.rs TranscriptSentiment, used in entity assessment) */
export interface IntelligenceTranscriptSentiment {
  overall: string;
  customer?: string;
  engagement?: string;
  forwardLooking?: boolean;
  competitorMentions?: string[];
  championPresent?: string;
  championEngaged?: string;
}

// =============================================================================
// I508a: Intelligence Dimension Sub-Types
// =============================================================================

// -- Dimension 1: Strategic Assessment --

export interface CompetitiveInsight {
  competitor: string;
  threatLevel?: string;
  context?: string;
  source?: string;
  detectedAt?: string;
  /** I576: Structured source attribution with confidence. */
  itemSource?: ItemSource;
  /** I576: True if multiple sources disagree on this item. */
  discrepancy?: boolean;
}

export interface StrategicPriority {
  priority: string;
  status?: string;
  owner?: string;
  source?: string;
  timeline?: string;
}

// -- Dimension 2: Relationship Health --

export interface CoverageAssessment {
  roleFillRate?: number;
  gaps?: string[];
  covered?: string[];
  level?: string;
}

export interface OrgChange {
  changeType: string;
  person: string;
  from?: string;
  to?: string;
  detectedAt?: string;
  source?: string;
  /** I576: Structured source attribution with confidence. */
  itemSource?: ItemSource;
  /** I576: True if multiple sources disagree on this item. */
  discrepancy?: boolean;
}

export interface InternalTeamMember {
  personId?: string;
  name: string;
  role: string;
  source?: string;
}

// -- Dimension 3: Engagement Cadence --

export interface CadenceAssessment {
  meetingsPerMonth?: number;
  trend?: string;
  daysSinceLast?: number;
  assessment?: string;
  evidence?: string[];
}

export interface ResponsivenessAssessment {
  trend?: string;
  volumeTrend?: string;
  assessment?: string;
  evidence?: string[];
}

// -- Dimension 4: Value & Outcomes --

export interface Blocker {
  description: string;
  owner?: string;
  since?: string;
  impact?: string;
  source?: string;
}

// -- Dimension 5: Commercial Context --

export interface ContractContext {
  contractType?: string;
  autoRenew?: boolean;
  contractStart?: string;
  renewalDate?: string;
  currentArr?: number;
  multiYearRemaining?: number;
  previousRenewalOutcome?: string;
  procurementNotes?: string;
  customerFiscalYearStart?: number;
}

export interface ExpansionSignal {
  opportunity: string;
  arrImpact?: number;
  source?: string;
  stage?: string;
  /** I576: Structured source attribution with confidence. */
  itemSource?: ItemSource;
  /** I576: True if multiple sources disagree on this item. */
  discrepancy?: boolean;
}

export interface RenewalOutlook {
  confidence?: string;
  riskFactors?: string[];
  expansionPotential?: string;
  recommendedStart?: string;
  negotiationLeverage?: string[];
  negotiationRisk?: string[];
}

// -- Dimension 6: External Health Signals --

export interface SupportHealth {
  openTickets?: number;
  criticalTickets?: number;
  avgResolutionTime?: string;
  trend?: string;
  csat?: number;
  source?: string;
}

export interface AdoptionSignals {
  adoptionRate?: number;
  trend?: string;
  featureAdoption?: string[];
  lastActive?: string;
  source?: string;
}

export interface SatisfactionData {
  nps?: number;
  csat?: number;
  surveyDate?: string;
  verbatim?: string;
  source?: string;
}

/** Synthesized intelligence for an entity (account, project, or person). */
export interface EntityIntelligence {
  version: number;
  entityId: string;
  entityType: string;
  enrichedAt: string;
  sourceFileCount: number;
  sourceManifest: SourceManifestEntry[];
  executiveAssessment?: string;
  /** I576: Concise editorial pull quote — one impactful sentence. */
  pullQuote?: string;
  risks: IntelRisk[];
  recentWins: IntelWin[];
  currentState?: IntelCurrentState;
  stakeholderInsights: StakeholderInsight[];
  nextMeetingReadiness?: IntelMeetingReadiness;
  companyContext?: IntelCompanyContext;
  /** Portfolio intelligence for parent accounts (I384) */
  portfolio?: PortfolioIntelligence;
  /** Network intelligence for person entities (I391) */
  network?: NetworkIntelligence;
  userEdits?: UserEdit[];
  /** ADR-0097 structured health payload. */
  health?: IntelligenceAccountHealth | null;
  /** I500 org-health baseline payload (when available). */
  orgHealth?: OrgHealthData | null;
  /** I396: Value delivered to the account. */
  valueDelivered?: Array<{ date?: string; statement: string; source?: string; impact?: string; itemSource?: ItemSource; discrepancy?: boolean }> | null;
  /** I396: Success metrics / KPIs tracked for this entity. */
  successMetrics?: Array<{ name: string; target?: string; current?: string; status?: string; owner?: string }> | null;
  /** I396: Open commitments (promises made to/from the account). */
  openCommitments?: Array<{ description: string; owner?: string; dueDate?: string; source?: string; status?: string; itemSource?: ItemSource; discrepancy?: boolean }> | null;
  /** I396: Relationship depth assessment. */
  relationshipDepth?: { championStrength?: string; executiveAccess?: string; stakeholderCoverage?: string; coverageGaps?: string[] } | null;
  /** I527: Deterministic consistency status. */
  consistencyStatus?: ConsistencyStatus;
  /** I527: Deterministic contradiction findings. */
  consistencyFindings?: ConsistencyFinding[];
  /** I527: Timestamp of latest consistency check. */
  consistencyCheckedAt?: string;

  // I508a: Intelligence Dimension Fields

  /** Dimension 1: Competitive insights. */
  competitiveContext?: CompetitiveInsight[];
  /** Dimension 1: Strategic priorities. */
  strategicPriorities?: StrategicPriority[];

  /** Dimension 2: Stakeholder coverage assessment. */
  coverageAssessment?: CoverageAssessment | null;
  /** Dimension 2: Organizational changes detected. */
  organizationalChanges?: OrgChange[];
  /** Dimension 2: Internal team assigned to this account. */
  internalTeam?: InternalTeamMember[];

  /** Dimension 3: Meeting cadence assessment. */
  meetingCadence?: CadenceAssessment | null;
  /** Dimension 3: Email responsiveness assessment. */
  emailResponsiveness?: ResponsivenessAssessment | null;

  /** Dimension 4: Active blockers. */
  blockers?: Blocker[];

  /** Dimension 5: Contract and commercial context. */
  contractContext?: ContractContext | null;
  /** Dimension 5: Expansion signals. */
  expansionSignals?: ExpansionSignal[];
  /** Dimension 5: Renewal outlook. */
  renewalOutlook?: RenewalOutlook | null;

  /** Dimension 6: Support ticket health. */
  supportHealth?: SupportHealth | null;
  /** Dimension 6: Product adoption signals. */
  productAdoption?: AdoptionSignals | null;
  /** Dimension 6: NPS/CSAT satisfaction data. */
  npsCsat?: SatisfactionData | null;

  /** Cross-cutting: source attribution (I507). */
  sourceAttribution?: Record<string, string[]> | null;

  /** DOS-13: AI-recommended actions from intelligence enrichment. */
  recommendedActions?: RecommendedAction[];
}

/** DOS-13: A recommended action produced by intelligence enrichment. */
export interface RecommendedAction {
  title: string;
  rationale: string;
  priority: number;
  suggestedDue?: string | null;
}

export interface SourceManifestEntry {
  filename: string;
  modifiedAt: string;
  format?: string;
}

/** I576: Source attribution for individual intelligence items. */
export interface ItemSource {
  source: string;
  confidence: number;
  sourcedAt: string;
  reference?: string;
}

export interface IntelRisk {
  text: string;
  source?: string;
  urgency: string;
  /** I576: Structured source attribution with confidence. */
  itemSource?: ItemSource;
  /** I576: True if multiple sources disagree on this item. */
  discrepancy?: boolean;
}

export interface IntelWin {
  text: string;
  source?: string;
  impact?: string;
  /** I576: Structured source attribution with confidence. */
  itemSource?: ItemSource;
  /** I576: True if multiple sources disagree on this item. */
  discrepancy?: boolean;
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
  /** Deterministic link to a Person entity (I420: reconciliation). */
  personId?: string;
  /** Suggested Person link (0.6–0.85 confidence) awaiting user confirmation (I420). */
  suggestedPersonId?: string;
  /** I576: Structured source attribution with confidence. */
  itemSource?: ItemSource;
  /** I576: True if multiple sources disagree on this item. */
  discrepancy?: boolean;
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
  /** I388: Parent-child hierarchy fields */
  parentId?: string;
  parentName?: string;
  childCount: number;
  isParent: boolean;
}

export interface ProjectMilestone {
  name: string;
  status: string;
  targetDate?: string;
  notes?: string;
}

/** Compact child project summary for parent detail pages (I388). */
export interface ProjectChildSummary {
  id: string;
  name: string;
  status: string;
  milestone?: string;
  openActionCount: number;
}

/** Aggregated signals for parent project's children (I388). */
export interface ProjectParentAggregate {
  childCount: number;
  activeCount: number;
  onHoldCount: number;
  completedCount: number;
  nearestTargetDate?: string;
}

/** Full detail for the project detail page. */
export interface ProjectDetail extends ProjectListItem {
  description?: string;
  milestones: ProjectMilestone[];
  notes?: string;
  openActions: Action[];
  recentMeetings: MeetingSummary[];
  linkedPeople: Person[];
  /** JSON-serialized string[] of resolution keywords (I305) */
  keywords?: string;
  /** ISO timestamp when keywords were last extracted (I305) */
  keywordsExtractedAt?: string;
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
  recentEmailSignals?: EmailSignal[];
  /** ADR-0057: Synthesized entity intelligence */
  intelligence?: EntityIntelligence;
  /** I388: Parent-child hierarchy */
  children: ProjectChildSummary[];
  parentAggregate?: ProjectParentAggregate;
}

// =============================================================================
// AI Model Config (I174)
// =============================================================================

export interface AiModelConfig {
  synthesis: string;
  extraction: string;
  background: string;
  mechanical: string;
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
  consistencyStatus?: ConsistencyStatus;
  consistencyFindings?: ConsistencyFinding[];
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
  | "downsell"
  | "downgrade"
  | "escalation"
  | "escalation_resolved"
  | "champion_change"
  | "executive_sponsor_change"
  | "contract_signed"
  | "pilot_start"
  | "kickoff"
  | "go_live"
  | "qbr_completed"
  | "ebr_completed"
  | "onboarding_complete"
  | "health_review";

export interface AccountEvent {
  id: number;
  accountId: string;
  eventType: AccountEventType;
  eventDate: string;
  arrImpact?: number;
  notes?: string;
  createdAt: string;
}

export interface AccountMilestone {
  id: string;
  objectiveId: string;
  accountId: string;
  title: string;
  status: "pending" | "completed" | "skipped";
  targetDate?: string | null;
  completedAt?: string | null;
  autoDetectSignal?: string | null;
  completedBy?: string | null;
  completionTrigger?: string | null;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface AccountObjective {
  id: string;
  accountId: string;
  title: string;
  description?: string | null;
  status: "draft" | "active" | "completed" | "abandoned";
  targetDate?: string | null;
  completedAt?: string | null;
  createdAt: string;
  updatedAt: string;
  source: "user" | "ai_suggested" | "template";
  sortOrder: number;
  milestones: AccountMilestone[];
  linkedActions: Action[];
  linkedActionCount: number;
  completedMilestoneCount: number;
  totalMilestoneCount: number;
  /** DOS-14: Evidence from AI enrichment matching this objective */
  evidenceJson?: string | null;
  /** DOS-14: ID linking to original AI statedObjective */
  aiOriginId?: string | null;
}

export interface SuggestedMilestone {
  title: string;
  targetDate?: string | null;
  autoDetectEvent?: string | null;
}

export interface SuggestedObjective {
  title: string;
  description?: string | null;
  confidence: "high" | "medium" | "low" | string;
  sourceEvidence?: string | null;
  milestones: SuggestedMilestone[];
  sourceCommitmentIds: string[];
}

export interface SuccessPlanTemplate {
  id: string;
  name: string;
  description: string;
  lifecycleTrigger: string;
  objectives: {
    title: string;
    description: string;
    milestones: {
      title: string;
      offsetDays: number;
      autoDetectSignal?: string | null;
    }[];
  }[];
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

// =============================================================================
// Risk Briefing (6-Slide Executive Presentation)
// =============================================================================

export interface RiskBriefing {
  accountId: string;
  generatedAt: string;
  cover: RiskCover;
  bottomLine: RiskBottomLine;
  whatHappened: RiskWhatHappened;
  stakes: RiskStakes;
  thePlan: RiskThePlan;
  theAsk: RiskTheAsk;
}

export interface RiskCover {
  accountName: string;
  riskLevel?: string;
  arrAtRisk?: number;
  date: string;
  tamName?: string;
}

export interface RiskBottomLine {
  headline: string;
  riskLevel?: string;
  renewalWindow?: string;
}

export interface RiskWhatHappened {
  narrative: string;
  healthArc?: HealthSnapshot[];
  keyLosses?: string[];
}

export interface HealthSnapshot {
  period: string;
  status: string;
  detail?: string;
}

export interface RiskStakes {
  financialHeadline?: string;
  stakeholders?: RiskStakeholder[];
  decisionMaker?: string;
  worstCase?: string;
}

export interface RiskStakeholder {
  name: string;
  role?: string;
  alignment?: string;
  engagement?: string;
  decisionWeight?: string;
  assessment?: string;
}

export interface RiskThePlan {
  strategy: string;
  actions?: ActionStep[];
  timeline?: string;
  assumptions?: string[];
}

export interface ActionStep {
  step: string;
  owner?: string;
  timeline?: string;
}

export interface RiskTheAsk {
  requests?: ConcreteRequest[];
  decisions?: string[];
  escalation?: string;
}

export interface ConcreteRequest {
  request: string;
  urgency?: string;
  from?: string;
}

// =============================================================================
// Quill Integration
// =============================================================================

export interface QuillStatus {
  enabled: boolean;
  bridgeExists: boolean;
  bridgePath: string;
  pendingSyncs: number;
  failedSyncs: number;
  completedSyncs: number;
  lastSyncAt: string | null;
  lastError?: string | null;
  lastErrorAt?: string | null;
  abandonedSyncs?: number;
  pollIntervalMinutes?: number;
}

export interface QuillSyncState {
  id: string;
  meetingId: string;
  quillMeetingId: string | null;
  state: "pending" | "polling" | "fetching" | "processing" | "completed" | "failed" | "abandoned";
  attempts: number;
  maxAttempts: number;
  nextAttemptAt: string | null;
  lastAttemptAt: string | null;
  completedAt: string | null;
  errorMessage: string | null;
  matchConfidence: number | null;
  transcriptPath: string | null;
  createdAt: string;
  updatedAt: string;
  source: "quill" | "granola";
}

export interface GravatarStatus {
  enabled: boolean;
  cachedCount: number;
  apiKeySet: boolean;
}

export interface GranolaStatus {
  enabled: boolean;
  cacheExists: boolean;
  cachePath: string;
  documentCount: number;
  pendingSyncs: number;
  failedSyncs: number;
  completedSyncs: number;
  lastSyncAt: string | null;
  pollIntervalMinutes: number;
}

// =============================================================================
// Clay Integration (I228)
// =============================================================================

export interface ClayStatusData {
  enabled: boolean;
  apiKeySet: boolean;
  autoEnrichOnCreate: boolean;
  sweepIntervalHours: number;
  enrichedCount: number;
  pendingCount: number;
  lastEnrichmentAt: string | null;
}

export interface EnrichmentLogEntry {
  id: string;
  entityType: string;
  entityId: string;
  source: string;
  eventType: string;
  signalType?: string;
  fieldsUpdated?: string;
  createdAt: string;
}

// =============================================================================
// Linear Integration (I346)
// =============================================================================

export interface LinearStatusData {
  enabled: boolean;
  apiKeySet: boolean;
  pollIntervalMinutes: number;
  issueCount: number;
  projectCount: number;
  lastSyncAt: string | null;
}

// =============================================================================
// Google Drive Integration (I426)
// =============================================================================

export interface DriveStatusData {
  enabled: boolean;
  watchedCount: number;
  lastSyncAt: string | null;
}

export interface DriveWatchedSource {
  id: string;
  googleId: string;
  name: string;
  type: "document" | "spreadsheet" | "folder" | "presentation";
  googleDocUrl: string | null;
  entityId: string;
  entityType: string;
  lastSyncedAt: string | null;
}

// =============================================================================
// Meeting Timeline (±7 day intelligence timeline)
// =============================================================================

export interface TimelineMeeting {
  id: string;
  title: string;
  startTime: string;
  endTime?: string;
  meetingType: string;
  intelligenceQuality?: {
    level: "sparse" | "developing" | "ready" | "fresh";
    signalCount: number;
    lastEnriched?: string;
    hasEntityContext: boolean;
    hasAttendeeHistory: boolean;
    hasRecentSignals: boolean;
    staleness: "current" | "aging" | "stale";
    hasNewSignals: boolean;
  };
  hasOutcomes: boolean;
  outcomeSummary?: string;
  entities: LinkedEntity[];
  hasNewSignals: boolean;
  priorMeetingId?: string;
  /** Count of follow-up actions linked to this meeting (I342) */
  followUpCount?: number;
  /** Whether a meeting briefing exists (prep_frozen_json or disk file) */
  hasPrep?: boolean;
  /** I502: Health data keyed by entity ID for linked accounts with intelligence health. */
  entityHealthMap?: Record<string, IntelligenceAccountHealth>;
}

// =============================================================================
// User Entity Types (I411 — ADR-0089/0090)
// =============================================================================

export interface UserEntity {
  id: number;
  name: string | null;
  company: string | null;
  title: string | null;
  focus: string | null;
  valueProposition: string | null;
  successDefinition: string | null;
  currentPriorities: string | null;
  productContext: string | null;
  playbooks: string | null;
  companyBio: string | null;
  roleDescription: string | null;
  howImMeasured: string | null;
  pricingModel: string | null;
  /** JSON array of strings */
  differentiators: string | null;
  /** JSON array of strings */
  objections: string | null;
  competitiveContext: string | null;
  /** JSON array of AnnualPriority objects */
  annualPriorities: string | null;
  /** JSON array of QuarterlyPriority objects */
  quarterlyPriorities: string | null;
  userRelevanceWeight: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface UserContextEntry {
  id: string;
  title: string;
  content: string;
  embeddingId: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface EntityContextEntry {
  id: string;
  entityType: string;
  entityId: string;
  title: string;
  content: string;
  createdAt: string;
  updatedAt: string;
}

export interface AnnualPriority {
  id: string;
  text: string;
  linkedEntityId: string | null;
  linkedEntityType: string | null;
  createdAt: string;
}

export interface QuarterlyPriority {
  id: string;
  text: string;
  linkedEntityId: string | null;
  linkedEntityType: string | null;
  createdAt: string;
}

// =============================================================================
// I427: Global Search
// =============================================================================

export interface GlobalSearchResult {
  entityId: string;
  entityType: "account" | "project" | "person" | "meeting" | "action" | "email";
  name: string;
  secondaryText: string;
  route: string;
  rank: number;
}

export interface CopyToInboxReport {
  copiedCount: number;
  copiedFilenames: string[];
}

// =============================================================================
// I428: Connectivity / Sync Freshness
// =============================================================================

export interface SyncFreshness {
  source: string;
  status: "green" | "amber" | "red" | "unknown";
  lastSuccessAt: string | null;
  lastAttemptAt: string | null;
  lastError: string | null;
  consecutiveFailures: number;
  ageDescription: string;
}

// =============================================================================
// I429: Data Export
// =============================================================================

export interface ExportReport {
  path: string;
  timestamp: string;
  counts: ExportCounts;
}

export interface ExportCounts {
  accounts: number;
  people: number;
  projects: number;
  meetings: number;
  actions: number;
  signals: number;
  intelligence: number;
}

// =============================================================================
// I430: Privacy Controls
// =============================================================================

export interface DataSummary {
  accounts: number;
  people: number;
  projects: number;
  meetings: number;
  actions: number;
  insights: number;
  signals: number;
  emails: number;
}

export interface ClearReport {
  assessmentsDeleted: number;
  feedbackDeleted: number;
  signalsDeleted: number;
  summariesCleared: number;
}
