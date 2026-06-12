export type TrustBand = "likely_current" | "use_with_caution" | "needs_verification" | "unscored";

export type CompositionFeedbackEntityType = "account" | "project" | "person" | "action" | "briefing" | "meeting";

export type SectionLayout = "stacked" | "grid" | "inline";

export interface Salience {
  weight: number;
  band: "critical" | "important" | "contextual" | "background";
  reason: string;
}

export interface ProjectedSection {
  section_id: string;
  section_index: number;
  label?: string | null;
  layout: SectionLayout;
  salience: Salience;
  block_ids: string[];
  block_indexes: number[];
}

export interface ClaimRef {
  claim_id: string;
  claim_version: number;
  field_path?: string | null;
}

export interface ProvenanceRef {
  invocation_id: string;
  field_path: string;
}

export interface EditRoute {
  field_path: string;
  role: "source" | "computed_from" | "display_only" | "feedback_target";
  claim_refs: ClaimRef[];
  feedback_allowed: boolean;
  refusal_reason?: string | null;
}

export interface ProjectionDiagnostic {
  diagnostic_kind: string;
  reason: string;
  dropped_pointer_count: number;
  block_id?: string | null;
  original_type_id?: string | null;
  selected_known_type_id?: string | null;
}

export interface ProjectedBlock {
  block_id: string;
  block_index: number;
  original_type_id: string;
  selected_known_type_id: KnownCompositionBlockType | string;
  payload: Record<string, unknown>;
  banner?: string | null;
  trust_band: TrustBand;
  claim_refs: ClaimRef[];
  provenance: ProvenanceRef[];
  edit_routes: EditRoute[];
  diagnostics: ProjectionDiagnostic[];
}

export interface ProjectedComposition {
  composition_id: string;
  composition_version?: number | null;
  fallback_policy_version: number;
  sections: ProjectedSection[];
  blocks: ProjectedBlock[];
  diagnostics: ProjectionDiagnostic[];
  unknown_block_count: number;
  unknown_block_cap: number;
  dropped_unknown_block_count: number;
}

export interface RenderedProvenance {
  surface: string;
  value: unknown;
}

export interface ProjectedCompositionCommandResponse {
  ok: boolean;
  request_id: string;
  projection: ProjectedComposition;
  cache_hint_token: string;
  served_from_cache: boolean;
  rendered_provenance?: RenderedProvenance | null;
}

export const KNOWN_COMPOSITION_BLOCK_TYPES = [
  "account_overview",
  "claim_summary",
  "evidence_list",
  "health_snapshot",
  "relationship_map",
  "risk_callout",
  "action_list",
  "markdown_document",
  "dailyos/pill",
  "dailyos/status-dot",
  "dailyos/provenance-tag",
  "dailyos/health-badge",
  "dailyos/avatar",
  "dailyos/freshness-indicator",
  "dailyos/trust-band-badge",
  "dailyos/intelligence-quality-badge",
  "dailyos/entity-chip",
  "dailyos/type-badge",
  "dailyos/score-band",
] as const;

export type KnownCompositionBlockType = (typeof KNOWN_COMPOSITION_BLOCK_TYPES)[number];

export function normalizeTrustBand(value: TrustBand | string | null | undefined): Exclude<TrustBand, "unscored"> {
  if (value === "likely_current" || value === "use_with_caution" || value === "needs_verification") {
    return value;
  }
  return "needs_verification";
}
