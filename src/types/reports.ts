// Report types for v0.15.0 — mirrors Rust ReportRow and report-specific content schemas

export type ReportType =
  | 'swot'
  | 'account_health'
  | 'ebr_qbr'
  | 'weekly_impact'
  | 'monthly_wrapped'
  | 'risk_briefing';

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
export interface PriorityMove {
  priorityText: string;
  whatHappened: string;
  source: string;
}

export interface WeeklyImpactContent {
  weekLabel: string;
  headlineStat: string;
  prioritiesMoved: PriorityMove[];
  wins: string[];
  activitySummary: string;
  watch: string[];
  carryForward: string[];
}

// Monthly Wrapped Report content schema (mirrors Rust MonthlyWrappedContent)
export interface MonthlyWin {
  headline: string;
  detail?: string | null;
  source: string;
}

export interface PriorityProgress {
  priorityText: string;
  progress: 'strong' | 'some' | 'none';
  evidence?: string | null;
}

export interface MonthlyWrappedContent {
  monthLabel: string;
  headlineStat: string;
  openingReflection: string;
  topWins: MonthlyWin[];
  priorityProgress: PriorityProgress[];
  honestMiss?: string | null;
  momentumBuilder: string;
  byTheNumbers: string[];
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

// Human-readable labels per report type
export const REPORT_TYPE_LABELS: Record<ReportType, string> = {
  swot: 'SWOT Analysis',
  account_health: 'Account Health Review',
  ebr_qbr: 'EBR / QBR',
  weekly_impact: 'Weekly Impact',
  monthly_wrapped: 'Monthly Wrapped',
  risk_briefing: 'Risk Briefing',
};
