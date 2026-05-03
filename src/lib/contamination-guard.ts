/**
 * Cross-entity contamination guard utilities.
 *
 * Provides helpers to check if intelligence fields are flagged as
 * cross-entity bleed by the backend consistency checker.
 */
import type { ConsistencyFinding } from "@/types";

/** Bleed codes emitted by the backend consistency checker. */
const BLEED_CODES = new Set([
  "CROSS_ENTITY_BLEED_SUSPECT",
  "CROSS_ENTITY_CONTENT_BLEED",
]);

/**
 * Check if any consistency finding flags a specific field path as cross-entity bleed.
 * Supports exact match and prefix match (e.g., "stakeholderInsights[" matches all stakeholder fields).
 */
export function hasBleedFlag(
  findings: ConsistencyFinding[] | undefined,
  fieldPath: string,
): boolean {
  if (!findings?.length) return false;
  return findings.some(
    (f) =>
      BLEED_CODES.has(f.code) &&
      !f.autoFixed &&
      (f.fieldPath === fieldPath || isFieldPathPrefix(f.fieldPath, fieldPath)),
  );
}

/**
 * Check if `candidate` starts with `prefix` and the next character (if any)
 * is a boundary: `.` or `[`. This prevents "health.narrative" from matching
 * "health.narrativeSummary".
 */
function isFieldPathPrefix(candidate: string, prefix: string): boolean {
  if (!candidate.startsWith(prefix)) return false;
  if (candidate.length === prefix.length) return true;
  const next = candidate[prefix.length];
  return next === "." || next === "[";
}

/**
 * Check if ANY text-level bleed finding exists (for fields like executiveAssessment,
 * health.narrative, companyContext.description, successMetrics).
 */
export function hasAnyContentBleed(
  findings: ConsistencyFinding[] | undefined,
): boolean {
  if (!findings?.length) return false;
  return findings.some(
    (f) => f.code === "CROSS_ENTITY_CONTENT_BLEED" && !f.autoFixed,
  );
}

/**
 * Get all bleed findings for display in a warning.
 */
export function getBleedFindings(
  findings: ConsistencyFinding[] | undefined,
): ConsistencyFinding[] {
  if (!findings?.length) return [];
  return findings.filter((f) => BLEED_CODES.has(f.code) && !f.autoFixed);
}
