/** Briefing callout from the signal propagation engine (I308). */
export interface BriefingCallout {
  id: string;
  severity: 'critical' | 'warning' | 'info';
  headline: string;
  detail: string;
  entityName?: string;
  entityType: string;
  entityId: string;
  relevanceScore?: number;
}
