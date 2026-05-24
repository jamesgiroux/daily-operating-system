import type {
  EntityIntelligence,
  IntelRisk,
  IntelWin,
  ItemSource,
  RenderPolicy,
  SourceManifestEntry,
} from "@/types";
import type {
  EntityFact,
  EntityIntelligenceEnvelope,
  EnvelopeProvenanceSource,
  NormalizedSubject,
  OpenLoopWithReceipt,
  Paginated,
  SubjectRef,
} from "./contracts";
import type { EntityIntelligenceAbilityResponse } from "./invoke";

const REDACTED_SOURCE_LABEL = "Redacted source";
const SUMMARY_CLAIM_TYPES = new Set(["entity_summary"]);
const RISK_CLAIM_TYPES = new Set(["entity_risk", "risk"]);
const WIN_CLAIM_TYPES = new Set(["entity_win", "win"]);
const CURRENT_STATE_CLAIM_TYPES = new Set(["entity_current_state"]);
const VALUE_CLAIM_TYPES = new Set(["value_delivered"]);

export function mergeEntityDetailIntelligence(
  _legacy: EntityIntelligence | null | undefined,
  response: EntityIntelligenceAbilityResponse | null | undefined,
): EntityIntelligence | null {
  const envelope = response?.data;
  if (!envelope) return null;

  const facts = envelope.facts.items.filter((fact) =>
    subjectRefMatchesEnvelopeSubject(fact.subjectRef, envelope.subject),
  );
  const openLoops = envelope.openLoops.items.filter((item) =>
    openLoopMatchesEnvelopeSubject(item, envelope.subject),
  );
  const hasClaimBackedContent = facts.length > 0 || openLoops.length > 0;
  if (!hasClaimBackedContent) return null;

  const sourceIndex = buildSourceIndex(envelope);
  const usedSourceIds = usedProvenanceSourceIds(facts, openLoops);
  const sources = envelope.provenance.sources.filter((source) =>
    usedSourceIds.has(source.id),
  );
  const factsArePartial = pageHasMore(envelope.facts);
  const openLoopsArePartial = pageHasMore(envelope.openLoops);
  const latestSourceAt = newestTimestamp([
    ...facts.map((fact) => fact.sourceAsof),
    ...openLoops.map((item) => item.openLoop.source_asof ?? null),
    ...sources.map((source) => source.asOf),
  ]);
  const producedAt = response?.rendered_provenance?.value?.produced_at
    ?? response?.rendered_provenance?.value?.producedAt;
  const enrichedAt = latestSourceAt ?? producedAt ?? "";

  const merged: EntityIntelligence = {
    ...emptyIntelligence(envelope, enrichedAt),
  };

  merged.sourceFileCount = sources.length;
  merged.sourceManifest = sources.map((source) =>
    sourceManifestEntry(source, enrichedAt),
  );

  const summary = firstFact(facts, SUMMARY_CLAIM_TYPES);
  if (summary) {
    merged.executiveAssessment = summary.renderedText.text;
    merged.executiveAssessmentRenderPolicy = summary.renderedText.policy as RenderPolicy;
    delete merged.pullQuote;
  }

  const risks = facts
    .filter((fact) => RISK_CLAIM_TYPES.has(fact.claimType))
    .map((fact): IntelRisk => ({
      text: fact.renderedText.text,
      renderPolicy: fact.renderedText.policy as RenderPolicy,
      claimId: fact.claimId,
      urgency: riskUrgency(fact.trustBand),
      itemSource: itemSourceForFact(fact, sourceIndex),
    }));
  if (risks.length > 0 && !factsArePartial) {
    merged.risks = risks;
  }

  const wins = facts
    .filter((fact) => WIN_CLAIM_TYPES.has(fact.claimType))
    .map((fact): IntelWin => ({
      text: fact.renderedText.text,
      renderPolicy: fact.renderedText.policy as RenderPolicy,
      claimId: fact.claimId,
      itemSource: itemSourceForFact(fact, sourceIndex),
    }));
  if (wins.length > 0 && !factsArePartial) {
    merged.recentWins = wins;
  }

  const currentState = facts.filter((fact) =>
    CURRENT_STATE_CLAIM_TYPES.has(fact.claimType),
  );
  if (currentState.length > 0 && !factsArePartial) {
    merged.currentState = {
      working: currentState
        .filter((fact) => currentStateBucket(fact) === "working")
        .map((fact) => fact.renderedText.text),
      notWorking: currentState
        .filter((fact) => currentStateBucket(fact) === "notWorking")
        .map((fact) => fact.renderedText.text),
      unknowns: currentState
        .filter((fact) => currentStateBucket(fact) === "unknowns")
        .map((fact) => fact.renderedText.text),
    };
  }

  const valueDelivered = facts
    .filter((fact) => VALUE_CLAIM_TYPES.has(fact.claimType))
    .map((fact) => ({
      statement: fact.renderedText.text,
      renderPolicy: fact.renderedText.policy as RenderPolicy,
      claimId: fact.claimId,
      itemSource: itemSourceForFact(fact, sourceIndex),
    }));
  if (
    valueDelivered.length > 0
    && !factsArePartial
  ) {
    merged.valueDelivered = valueDelivered;
  }

  if (
    openLoops.length > 0
    && !openLoopsArePartial
  ) {
    merged.openCommitments = openLoops.map((item) => ({
      commitmentId: item.receiptTarget.claimId,
      description: item.openLoop.description,
      owner: item.openLoop.owner ?? undefined,
      dueDate: item.openLoop.due_date ?? undefined,
      status: item.openLoop.status ?? undefined,
      source: item.openLoop.loop_kind,
      itemSource: itemSourceForOpenLoop(item, sourceIndex),
    }));
  }

  return merged;
}

function emptyIntelligence(
  envelope: EntityIntelligenceEnvelope,
  enrichedAt: string,
): EntityIntelligence {
  return {
    version: envelope.schemaVersion,
    entityId: envelope.subject.id,
    entityType: envelope.subject.kind,
    enrichedAt,
    sourceFileCount: 0,
    sourceManifest: [],
    risks: [],
    recentWins: [],
    stakeholderInsights: [],
  };
}

function firstFact(facts: EntityFact[], claimTypes: Set<string>): EntityFact | null {
  return facts.find((fact) => claimTypes.has(fact.claimType)) ?? null;
}

function buildSourceIndex(
  envelope: EntityIntelligenceEnvelope,
): Map<string, EnvelopeProvenanceSource> {
  return new Map(envelope.provenance.sources.map((source) => [source.id, source]));
}

function itemSourceForFact(
  fact: EntityFact,
  sourceIndex: Map<string, EnvelopeProvenanceSource>,
): ItemSource | undefined {
  const source = firstSource(fact.provenance.sourceIds, sourceIndex);
  const sourcedAt = fact.sourceAsof ?? source?.asOf;
  if (!source && !sourcedAt) return undefined;
  return {
    source: source ? sourceLabel(source) : "Claim substrate",
    confidence: trustConfidence(fact.trustBand),
    sourcedAt: sourcedAt ?? "",
    reference: sourceReference(source),
  };
}

function itemSourceForOpenLoop(
  item: OpenLoopWithReceipt,
  sourceIndex: Map<string, EnvelopeProvenanceSource>,
): ItemSource | undefined {
  const source = firstSource(item.provenance.sourceIds, sourceIndex);
  const sourcedAt = item.openLoop.source_asof ?? source?.asOf;
  if (!source && !sourcedAt) return undefined;
  return {
    source: source ? sourceLabel(source) : item.openLoop.loop_kind,
    confidence: trustConfidence(item.trustBand),
    sourcedAt: sourcedAt ?? "",
    reference: sourceReference(source),
  };
}

function firstSource(
  ids: string[],
  sourceIndex: Map<string, EnvelopeProvenanceSource>,
): EnvelopeProvenanceSource | undefined {
  for (const id of ids) {
    const source = sourceIndex.get(id);
    if (source) return source;
  }
  return undefined;
}

function sourceManifestEntry(
  source: EnvelopeProvenanceSource,
  fallbackAt: string,
): SourceManifestEntry {
  return {
    filename: sourceLabel(source),
    modifiedAt: source.asOf ?? fallbackAt,
    format: source.redacted ? undefined : source.sourceType ?? undefined,
  };
}

function sourceLabel(source: EnvelopeProvenanceSource): string {
  return source.redacted ? REDACTED_SOURCE_LABEL : source.label;
}

function sourceReference(
  source: EnvelopeProvenanceSource | undefined,
): string | undefined {
  if (!source || source.redacted) return undefined;
  return source.sourceType ?? undefined;
}

function newestTimestamp(values: Array<string | null | undefined>): string | null {
  let newest: string | null = null;
  let newestTime = Number.NEGATIVE_INFINITY;
  for (const value of values) {
    if (!value) continue;
    const time = new Date(value).getTime();
    if (!Number.isFinite(time)) continue;
    if (time > newestTime) {
      newestTime = time;
      newest = value;
    }
  }
  return newest;
}

function trustConfidence(trustBand: string): number {
  switch (trustBand) {
    case "likely_current":
      return 0.9;
    case "use_with_caution":
      return 0.65;
    case "needs_verification":
      return 0.35;
    default:
      return 0.5;
  }
}

function riskUrgency(_trustBand: string): string {
  return "medium";
}

function currentStateBucket(fact: EntityFact): "working" | "notWorking" | "unknowns" {
  const field = (fact.fieldPath ?? "").toLowerCase();
  if (field.includes("unknown")) return "unknowns";
  if (field.includes("not_working") || field.includes("notworking")) return "notWorking";
  return "working";
}

function usedProvenanceSourceIds(
  facts: EntityFact[],
  openLoops: OpenLoopWithReceipt[],
): Set<string> {
  const ids = new Set<string>();
  for (const fact of facts) {
    for (const id of fact.provenance.sourceIds) ids.add(id);
  }
  for (const item of openLoops) {
    for (const id of item.provenance.sourceIds) ids.add(id);
  }
  return ids;
}

function pageHasMore<T>(page: Paginated<T>): boolean {
  return Boolean(page.nextCursor)
    || (typeof page.totalHint === "number" && page.totalHint > page.items.length);
}

function openLoopMatchesEnvelopeSubject(
  item: OpenLoopWithReceipt,
  subject: NormalizedSubject,
): boolean {
  return subjectRefMatchesEnvelopeSubject(item.receiptTarget.subjectRef, subject)
    || (
      item.openLoop.subject.entity_type === subject.kind
      && item.openLoop.subject.entity_id === subject.id
    );
}

function subjectRefMatchesEnvelopeSubject(
  ref: SubjectRef,
  subject: NormalizedSubject,
): boolean {
  if (typeof ref === "string") return false;
  switch (subject.kind) {
    case "account":
      return "account" in ref && ref.account === subject.id;
    case "project":
      return "project" in ref && ref.project === subject.id;
    case "person":
      return "person" in ref && ref.person === subject.id;
    case "meeting":
      return "meeting" in ref && ref.meeting === subject.id;
  }
}
