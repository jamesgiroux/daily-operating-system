// Report types for v0.15.0 — mirrors Rust ReportRow and report-specific content schemas

export type ReportType =
  | 'swot'
  | 'account_health'
  | 'ebr_qbr'
  | 'weekly_impact'
  | 'monthly_wrapped'
  | 'risk_briefing'
  | 'book_of_business';

export interface ReportRow {
  id: string;
  entityId: string;
  entityType: string;
  reportType: ReportType;
  contentJson: string;  // JSON string — parse to get typed content
  generatedAt: string;
  intelHash: string;
  isStale: boolean;
  createdAt: string;
  updatedAt: string;
}

// SWOT content schema (mirrors Rust SwotContent)
export interface SwotItem {
  text: string;
  source: string | null;
}

export interface SwotContent {
  strengths: SwotItem[];
  weaknesses: SwotItem[];
  opportunities: SwotItem[];
  threats: SwotItem[];
  summary: string | null;
}

// Account Health Review content schema (mirrors Rust AccountHealthContent)
export interface AccountHealthSignal {
  text: string;
  source: string | null;
}

export interface AccountHealthRisk {
  risk: string;
  status: string; // "open" | "mitigated" | "resolved"
}

export interface AccountHealthContent {
  overallAssessment: string;
  healthScoreNarrative: string | null;
  relationshipSummary: string;
  engagementCadence: string;
  customerQuote: string | null;
  whatIsWorking: string[];
  whatIsStruggling: string[];
  expansionSignals: string[];
  valueDelivered: AccountHealthSignal[];
  risks: AccountHealthRisk[];
  renewalContext: string | null;
  recommendedActions: string[];
}

// Weekly Impact Report content schema (mirrors Rust WeeklyImpactContent)
export interface WeeklyImpactMove {
  priorityText: string;
  whatHappened: string;
  source: string;
}

export interface WeeklyImpactItem {
  text: string;
  source?: string | null;
}

export interface WeeklyImpactContent {
  weekLabel: string;
  totalMeetings: number;
  totalActionsClosed: number;
  headline: string;
  prioritiesMoved: WeeklyImpactMove[];
  wins: WeeklyImpactItem[];
  whatYouDid: string;
  watch: WeeklyImpactItem[];
  intoNextWeek: string[];
}

// Monthly Wrapped Report content schema (mirrors Rust MonthlyWrappedContent)
export interface WrappedPersonality {
  trait: string;
  evidence: string;
}

export interface WrappedMoment {
  headline: string;
  detail?: string | null;
  source?: string | null;
}

export interface MonthlyWrappedContent {
  monthLabel: string;
  openingReflection: string;
  topMoments: WrappedMoment[];
  byTheNumbers: string[];
  personalityRead: WrappedPersonality[];
  honestMiss?: string | null;
  momentumBuilder: string;
  intoNextMonth: string[];
}

// EBR/QBR Report content schema (mirrors Rust EbrQbrContent)
export interface EbrQbrMetric {
  metric: string;
  baseline?: string | null;
  current: string;
  trend?: string | null;
}

export interface EbrQbrValueItem {
  outcome: string;
  source: string;
  impact?: string | null;
}

export interface EbrQbrRisk {
  risk: string;
  resolution?: string | null;
  status: string;
}

export interface EbrQbrAction {
  action: string;
  owner: string;
  timeline: string;
}

export interface EbrQbrContent {
  quarterLabel: string;
  executiveSummary: string;
  storyBullets: string[];
  customerQuote: string | null;
  valueDelivered: EbrQbrValueItem[];
  successMetrics: EbrQbrMetric[];
  challengesAndResolutions: EbrQbrRisk[];
  strategicRoadmap: string;
  nextSteps: EbrQbrAction[];
}

// Book of Business content schema (mirrors Rust BookOfBusinessContent)
export interface BookRiskItem {
  accountName: string;
  risk: string;
  arr: number | null;
}

export interface BookOpportunityItem {
  accountName: string;
  opportunity: string;
  estimatedValue: string | null;
}

export interface AccountSnapshotRow {
  accountId: string;
  accountName: string;
  arr: number | null;
  healthBand: string | null;
  healthTrend: string | null;
  healthScore: number | null;
  lifecycle: string | null;
  renewalDate: string | null;
  meetingCount90d: number;
  keyContact: string | null;
  isParent?: boolean;
  buCount?: number;
  parentId?: string;
}

export interface AccountDeepDive {
  accountName: string;
  accountId: string;
  arr: number | null;
  renewalDate: string | null;
  statusNarrative: string;
  activeWorkstreams: string[];
  renewalOrGrowthImpact: string;
  risksAndGaps: string[];
}

export interface ValueDeliveredRow {
  accountName: string;
  headlineOutcome: string;
  whyItMatters: string;
  source: string | null;
}

export interface BookTheme {
  title: string;
  narrative: string;
  citedAccounts: string[];
}

export interface LeadershipAsk {
  ask: string;
  context: string;
  impactedAccounts: string[];
  status: string | null;
}

export interface BookOfBusinessContent {
  periodLabel: string;
  executiveSummary: string;
  totalAccounts: number;
  totalArr: number | null;
  atRiskArr: number | null;
  upcomingRenewals: number;
  upcomingRenewalsArr: number | null;
  hasLeadershipAsks: boolean;
  topRisks: BookRiskItem[];
  topOpportunities: BookOpportunityItem[];
  accountSnapshot: AccountSnapshotRow[];
  deepDives: AccountDeepDive[];
  valueDelivered: ValueDeliveredRow[];
  keyThemes: BookTheme[];
  leadershipAsks: LeadershipAsk[];
}

// Human-readable labels per report type
export const REPORT_TYPE_LABELS: Record<ReportType, string> = {
  swot: 'SWOT Analysis',
  account_health: 'Account Health Review',
  ebr_qbr: 'EBR / QBR',
  weekly_impact: 'Weekly Impact',
  monthly_wrapped: 'Monthly Wrapped',
  risk_briefing: 'Risk Briefing',
  book_of_business: 'Book of Business',
};
