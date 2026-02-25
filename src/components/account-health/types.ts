/**
 * AccountHealthContent — typed schema for the account_health report.
 * Mirrors the Rust AccountHealthContent struct for v0.15.0.
 */

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
  /** Optional CSM name stored in the report for the cover slide */
  csmName?: string | null;
}
