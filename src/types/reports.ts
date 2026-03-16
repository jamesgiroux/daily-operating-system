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

// Book of Business content schema — template-aligned (14 slides)

export interface BiggestItem {
  accountName: string;
  arr: number;
  description: string;
}

export interface PortfolioHealthOverview {
  healthyCount: number;
  healthyArr: number;
  mediumCount: number;
  mediumArr: number;
  highRiskCount: number;
  highRiskArr: number;
  secureArr: number;
  renewals90d: number;
  renewals90dArr: number;
  renewals180d: number;
  renewals180dArr: number;
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

export interface RiskAccountRow {
  accountName: string;
  arr: number;
  renewalTiming: string;
  riskLevel: string;
  primaryRiskDriver: string;
}

export interface RetentionRiskDeepDive {
  accountName: string;
  arr: number;
  whyAtRisk: string;
  saveConfidence: string;
  next90Days: string;
  keyTactics: string[];
  successSignals: string[];
  helpNeeded: string[];
}

export interface SaveMotion {
  accountName: string;
  risk: string;
  saveMotion: string;
  timeline: string;
  successSignals: string;
}

export interface ExpansionRow {
  accountName: string;
  arr: number;
  readiness: string;
  expansionType: string;
  estimatedValue: string;
  timing: string;
}

export interface ExpansionReadiness {
  accountName: string;
  readiness: string;
  primaryRisk: string;
  nextAction: string;
}

export interface YearEndOutlook {
  startingArr: number;
  atRiskArr: number;
  committedExpansion: number;
  expectedChurn: number;
  projectedEoyArr: number;
}

export interface LandingScenarios {
  best: ScenarioRow;
  expected: ScenarioRow;
  worst: ScenarioRow;
}

export interface ScenarioRow {
  keyAssumptions: string;
  attrition: string;
  expansion: string;
  notes: string;
}

export interface LeadershipAsk {
  supportNeeded: string;
  whyItMatters: string;
  impactedAccounts: string[];
  dollarImpact: string | null;
  timing: string;
}

export interface AccountFocus {
  rank: number;
  accountName: string;
  arr: number;
  primaryObjective: string;
  keyTactics: string[];
  successSignals: string[];
}

export interface QuarterlyFocus {
  retention: string[];
  expansion: string[];
  execution: string[];
}

export interface BookTheme {
  title: string;
  narrative: string;
  citedAccounts: string[];
}

export interface BookOfBusinessContent {
  // Slide 1: Executive Summary
  periodLabel: string;
  executiveSummary: string;
  totalAccounts: number;
  totalArr: number;
  atRiskArr: number;
  committedExpansion: number;
  projectedChurn: number;
  topRisksSummary: string[];
  topOpportunitiesSummary: string[];
  biggestRisk: BiggestItem | null;
  biggestUpside: BiggestItem | null;
  eltHelpRequired: boolean;
  // Slide 2: Portfolio Health
  healthOverview: PortfolioHealthOverview;
  // Slide 3: Risk Table
  riskAccounts: RiskAccountRow[];
  // Slide 4: Retention Risk Deep Dives
  retentionRiskDeepDives: RetentionRiskDeepDive[];
  // Slide 5: Save Motions
  saveMotions: SaveMotion[];
  // Slide 6: Expansion
  expansionAccounts: ExpansionRow[];
  // Slide 7: Expansion Readiness
  expansionReadiness: ExpansionReadiness[];
  // Slide 8: Year-End Outlook
  yearEndOutlook: YearEndOutlook;
  // Slide 9: Landing Scenarios
  landingScenarios: LandingScenarios;
  // Slide 10+14: Leadership Asks
  leadershipAsks: LeadershipAsk[];
  // Slide 11: Account Focus
  accountFocus: AccountFocus[];
  // Slide 12: Quarterly Focus
  quarterlyFocus: QuarterlyFocus;
  // Slide 13: Key Themes
  keyThemes: BookTheme[];
  // Account snapshot
  accountSnapshot: AccountSnapshotRow[];
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
