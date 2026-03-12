/**
 * Type definitions for the Monthly Wrapped page.
 */

export interface WrappedPersonality {
  typeName: string;
  description: string;
  keySignal: string;
  rarityLabel: string;
}

export interface WrappedMoment {
  label: string;
  headline: string;
  subtext?: string | null;
}

export interface MonthlyWrappedContent {
  monthLabel: string;
  totalConversations: number;
  totalEntitiesTouched: number;
  totalPeopleMet: number;
  signalsCaptured: number;
  topEntityName: string;
  topEntityTouches: number;
  moments: WrappedMoment[];
  hiddenPattern: string;
  personality: WrappedPersonality;
  priorityAlignmentPct: number | null;
  priorityAlignmentLabel: string | null;
  topWin: string;
  carryForward: string;
  wordOne: string;
  wordTwo: string;
  wordThree: string;
}
