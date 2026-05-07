import { stripMarkdown } from "@/lib/utils";
import {
  extractMostCautiousTrustBand,
  fieldPathCandidates,
  partitionTrustEvidence,
  type TrustBandWire,
  type TrustEvidencePartition,
} from "@/lib/trust-band";
import type { MeetingPrep } from "@/types";

export type PrepImpact = "high" | "medium" | "low";
export type PrepSectionKey = "discuss" | "watch" | "wins";

export interface ParsedPrepGridItem {
  text: string;
  impact?: PrepImpact;
  section?: PrepSectionKey;
  trustBand?: TrustBandWire;
  fieldPaths?: string[];
}

const PREP_IMPACT_TAIL_RE = /\s+[—-]\s*(high|medium|low)\s*$/i;

export function parsePrepGridItem(raw: string): ParsedPrepGridItem {
  const cleaned = stripMarkdown(raw).trim();
  const impactMatch = cleaned.match(PREP_IMPACT_TAIL_RE);
  if (!impactMatch) return { text: cleaned };
  return {
    text: cleaned.replace(PREP_IMPACT_TAIL_RE, "").trim(),
    impact: impactMatch[1].toLowerCase() as PrepImpact,
  };
}

export function normalizePrepGridText(value: string): string {
  return value.toLowerCase().replace(/\s+/g, " ").trim();
}

export function stringArrayField(source: unknown, field: string): string[] {
  if (!source || typeof source !== "object") return [];
  const value = (source as Record<string, unknown>)[field];
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

export function buildPrepGridItems(
  rawItems: string[],
  section: PrepSectionKey,
  renderedProvenance: unknown,
  getFieldPaths: (index: number) => string[],
): ParsedPrepGridItem[] {
  return rawItems
    .map((raw, index) => {
      const fieldPaths = getFieldPaths(index);
      return {
        ...parsePrepGridItem(raw),
        section,
        fieldPaths,
        trustBand: extractMostCautiousTrustBand(renderedProvenance, fieldPaths),
      };
    })
    .filter((item) => item.text);
}

export function buildLegacyPrepGridItems(
  prep: MeetingPrep | undefined,
  renderedProvenance: unknown,
): ParsedPrepGridItem[] {
  if (!prep) return [];

  const wins = buildPrepGridItems(prep.wins ?? [], "wins", renderedProvenance, (i) => [
    ...fieldPathCandidates(`wins[${i}]`),
    ...fieldPathCandidates(`recentWins[${i}]`),
  ]);
  const winKeys = new Set(wins.map((item) => normalizePrepGridText(item.text)));
  const discuss = [
    ...buildPrepGridItems(prep.actions ?? [], "discuss", renderedProvenance, (i) => [
      ...fieldPathCandidates(`actions[${i}]`),
      ...fieldPathCandidates(`talkingPoints[${i}]`),
    ]),
    ...buildPrepGridItems(prep.questions ?? [], "discuss", renderedProvenance, (i) => [
      ...fieldPathCandidates(`questions[${i}]`),
    ]),
  ].filter((item) => item.text && !winKeys.has(normalizePrepGridText(item.text)));
  const watch = buildPrepGridItems(prep.risks ?? [], "watch", renderedProvenance, (i) => [
    ...fieldPathCandidates(`risks[${i}]`),
  ]);

  return [...discuss, ...watch, ...wins];
}

export function partitionLegacyPrepGrid(
  prep: MeetingPrep | undefined,
  renderedProvenance: unknown,
  showAllEvidence: boolean,
): TrustEvidencePartition<ParsedPrepGridItem> {
  return partitionTrustEvidence(buildLegacyPrepGridItems(prep, renderedProvenance), {
    showAllEvidence,
    getBand: (item) => item.trustBand,
  });
}
