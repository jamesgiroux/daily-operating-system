import type {
  RenderedFieldAttribution,
  RenderedProvenanceSummary,
  TrustAnnotated,
  TrustBandWire as TrustBandWireType,
} from "@/types";

export type TrustBandWire = TrustBandWireType;

const TRUST_BANDS: TrustBandWire[] = [
  "likely_current",
  "use_with_caution",
  "needs_verification",
  "unscored",
];

const CAUTION_ORDER: Record<TrustBandWire, number> = {
  likely_current: 1,
  unscored: 1,
  use_with_caution: 2,
  needs_verification: 3,
};

export interface TrustEvidencePartition<T> {
  current: Array<TrustAnnotated<T>>;
  caution: Array<TrustAnnotated<T>>;
  needsVerification: Array<TrustAnnotated<T>>;
  revealedNeedsVerification: Array<TrustAnnotated<T>>;
  hiddenNeedsVerificationCount: number;
  totalCount: number;
}

export interface PartitionTrustEvidenceOptions<T> {
  renderedProvenance?: unknown;
  showAllEvidence?: boolean;
  getBand?: (item: T, index: number) => unknown;
  getFieldPaths?: (item: T, index: number) => string | string[] | null | undefined;
  getSourceDate?: (item: T, index: number) => string | null | undefined;
}

export function normalizeTrustBand(value: unknown): TrustBandWire {
  if (typeof value !== "string") {
    return "unscored";
  }
  return (TRUST_BANDS as string[]).includes(value) ? (value as TrustBandWire) : "unscored";
}

export function extractTrustBand(renderedProvenance: unknown, fieldPath: string): TrustBandWire {
  const fields = getFieldAttributions(renderedProvenance);
  if (!fields) {
    return "unscored";
  }
  const attribution = fields[fieldPath];
  return normalizeTrustBand(attribution?.trust_band ?? attribution?.trustBand);
}

export function extractMostCautiousTrustBand(
  renderedProvenance: unknown,
  fieldPaths: string | string[] | null | undefined,
): TrustBandWire {
  const paths = Array.isArray(fieldPaths) ? fieldPaths : fieldPaths ? [fieldPaths] : [];
  const bands = paths.map((path) => extractTrustBand(renderedProvenance, path));
  return mostCautiousTrustBand(bands);
}

export function partitionTrustEvidence<T>(
  items: readonly T[],
  options: PartitionTrustEvidenceOptions<T> = {},
): TrustEvidencePartition<T> {
  const sourceDate = getNewestRenderedProvenanceSourceDate(options.renderedProvenance);
  const annotated = items.map((item, index) => {
    const itemBand = readItemTrustBand(item);
    const explicitBand = normalizeTrustBand(options.getBand?.(item, index));
    const fieldBand = extractMostCautiousTrustBand(
      options.renderedProvenance,
      options.getFieldPaths?.(item, index),
    );
    const trustBand = mostCautiousTrustBand([itemBand, explicitBand, fieldBand]);
    const fieldPaths = options.getFieldPaths?.(item, index);
    const trustFieldPath = Array.isArray(fieldPaths) ? fieldPaths[0] : fieldPaths ?? undefined;
    return {
      ...(item as T & object),
      trustBand,
      trustFieldPath,
      trustSourceDate: options.getSourceDate?.(item, index) ?? sourceDate,
    } as TrustAnnotated<T>;
  });

  const current = annotated.filter(
    (item) => item.trustBand === "likely_current" || item.trustBand === "unscored",
  );
  const caution = annotated.filter((item) => item.trustBand === "use_with_caution");
  const needsVerification = annotated.filter((item) => item.trustBand === "needs_verification");
  const showAllEvidence = options.showAllEvidence ?? false;

  return {
    current,
    caution,
    needsVerification,
    revealedNeedsVerification: showAllEvidence ? needsVerification : [],
    hiddenNeedsVerificationCount: showAllEvidence ? 0 : needsVerification.length,
    totalCount: annotated.length,
  };
}

export function annotateTrust<T extends object>(
  items: readonly T[],
  renderedProvenance: unknown,
  getFieldPaths: (item: T, index: number) => string | string[] | null | undefined,
): Array<TrustAnnotated<T>> {
  const sourceDate = getNewestRenderedProvenanceSourceDate(renderedProvenance);
  return items.map((item, index) => {
    const fieldPaths = getFieldPaths(item, index);
    const trustBand = extractMostCautiousTrustBand(renderedProvenance, fieldPaths);
    const trustFieldPath = Array.isArray(fieldPaths) ? fieldPaths[0] : fieldPaths ?? undefined;
    return {
      ...item,
      trustBand,
      trustFieldPath,
      trustSourceDate: sourceDate,
      renderedProvenance: renderedProvenanceFrom(renderedProvenance),
    };
  });
}

export function fieldPathToJsonPointer(path: string): string {
  if (path.startsWith("/")) {
    return path;
  }
  const normalized = path
    .replace(/\[(\d+)\]/g, ".$1")
    .split(".")
    .filter(Boolean)
    .map(escapeJsonPointerSegment)
    .join("/");
  return `/${normalized}`;
}

export function fieldPathCandidates(path: string): string[] {
  const pointer = fieldPathToJsonPointer(path);
  const snakePointer = pointer
    .split("/")
    .map((segment, index) => (index === 0 ? segment : camelToSnake(segment)))
    .join("/");
  return Array.from(new Set([pointer, snakePointer]));
}

export function renderedProvenanceFrom(value: unknown): RenderedProvenanceSummary | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const candidate = value as {
    rendered_provenance?: RenderedProvenanceSummary;
    renderedProvenance?: RenderedProvenanceSummary;
    value?: unknown;
  };
  if (isRenderedProvenance(candidate.rendered_provenance)) {
    return candidate.rendered_provenance;
  }
  if (isRenderedProvenance(candidate.renderedProvenance)) {
    return candidate.renderedProvenance;
  }
  if (isRenderedProvenance(candidate)) {
    return candidate as RenderedProvenanceSummary;
  }
  return null;
}

export function getNewestRenderedProvenanceSourceDate(renderedProvenance: unknown): string | null {
  const provenance = renderedProvenanceFrom(renderedProvenance);
  const value = provenance?.value;
  const sources = Array.isArray(value?.sources) ? value.sources : [];
  const newest = sources
    .map((source) => source.source_asof ?? source.sourceAsof ?? source.observed_at ?? source.observedAt)
    .filter((date): date is string => typeof date === "string" && date.trim().length > 0)
    .map((date) => ({ raw: date, time: Date.parse(date) }))
    .filter((date) => Number.isFinite(date.time))
    .sort((a, b) => b.time - a.time)[0];
  return newest?.raw ?? null;
}

const showAllEvidenceSessionState = new Map<string, boolean>();

export function readShowAllEvidenceState(surfaceId: string): boolean {
  return showAllEvidenceSessionState.get(surfaceId) ?? false;
}

export function writeShowAllEvidenceState(surfaceId: string, value: boolean): void {
  showAllEvidenceSessionState.set(surfaceId, value);
}

export function clearShowAllEvidenceStateForTests(): void {
  showAllEvidenceSessionState.clear();
}

function readItemTrustBand(item: unknown): TrustBandWire {
  if (!item || typeof item !== "object") {
    return "unscored";
  }
  const candidate = item as { trustBand?: unknown; trust_band?: unknown };
  return normalizeTrustBand(candidate.trustBand ?? candidate.trust_band);
}

function mostCautiousTrustBand(bands: readonly TrustBandWire[]): TrustBandWire {
  let result: TrustBandWire = "unscored";
  for (const band of bands) {
    if (band === "unscored" && result !== "unscored") {
      continue;
    }
    if (CAUTION_ORDER[band] > CAUTION_ORDER[result]) {
      result = band;
    }
    if (result === "unscored" && band !== "unscored") {
      result = band;
    }
  }
  return result;
}

function getFieldAttributions(renderedProvenance: unknown): Record<string, RenderedFieldAttribution> | null {
  const provenance = renderedProvenanceFrom(renderedProvenance);
  const value = provenance?.value;
  if (!value) {
    return null;
  }
  return value.field_attributions ?? value.fieldAttributions ?? null;
}

function isRenderedProvenance(value: unknown): value is RenderedProvenanceSummary {
  return Boolean(
    value
      && typeof value === "object"
      && "value" in value
      && typeof (value as { value?: unknown }).value === "object",
  );
}

function escapeJsonPointerSegment(segment: string): string {
  return segment.replace(/~/g, "~0").replace(/\//g, "~1");
}

function camelToSnake(value: string): string {
  return value.replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`);
}
