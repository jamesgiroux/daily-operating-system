/**
 * TriageSection — "Needs attention" chapter for the Health tab (DOS-203).
 *
 * Wave-0g rebuild (DOS-249): unified Local + Glean ranking with a hard cap.
 *   - Every card is a `TriageCandidate` with `bucket` ("urgent" | "soon" |
 *     "stakeholder"), `source` ("local" | "glean"), and `sourcedAt` so we
 *     can order by urgency THEN recency and cap at 5.
 *   - Glean cards carry a `Glean` source-tag pill; Local cards carry `Local`.
 *     Cards that triangulate both origins carry both.
 *
 * `hasTriageContent()` is the public gate the caller uses to decide between
 * the triage chapter and the "On track" fine state. It must return true for
 * any card that would render — Local or Glean.
 *
 * No per-card feedback widget: the canonical mockup does not include one on
 * triage cards, and the previous IntelligenceCorrection slot widened the
 * action column enough to compress the card body. Feedback for Intelligence
 * Loop training lives at the chapter-heading level (see OutlookPanel).
 */
import type { ReactNode } from "react";
import type {
  EntityIntelligence,
  HealthOutlookSignals,
  IntelRisk,
  IntelWin,
} from "@/types";
import {
  TriageCard,
  type TriageAction,
  type TriageCitation,
  type TriageSource,
  type TriageTone,
} from "./TriageCard";
import styles from "./health.module.css";

/** Ranking bucket — drives ordering and (via `toneForBucket`) spine colour. */
export type TriageBucket = "urgent" | "soon" | "stakeholder";

/** A normalised, rank-ready triage row — Local or Glean. */
export interface TriageCandidate {
  /** Stable per-card id for feedback attribution. */
  id: string;
  /** Ranking bucket. Urgent first, then soon, then stakeholder. */
  bucket: TriageBucket;
  /** Primary origin — drives tag colour when `sources` isn't overridden. */
  source: "local" | "glean";
  /** ISO timestamp used for recency tie-breaks within a bucket. */
  sourcedAt: string;
  /** Short uppercase kind label (mockup convention). */
  kind: string;
  /** Serif one-liner. */
  headline: string;
  /** Optional evidence body (accepts ReactNode for <strong> emphasis). */
  evidence?: ReactNode;
  /** Source-origin pills (Local / Glean). Populated from `source` by default. */
  sources: TriageSource[];
  /** Dated citation links. */
  citations: TriageCitation[];
}

const MAX_CARDS = 5;

function toneForBucket(bucket: TriageBucket): TriageTone {
  if (bucket === "urgent") return "urgent";
  if (bucket === "soon") return "soon";
  return "gap"; // stakeholder → larkspur spine per mockup
}

function bucketFromUrgency(urgency?: string): TriageBucket {
  const u = (urgency ?? "").toLowerCase();
  if (u === "high" || u === "urgent" || u === "critical") return "urgent";
  if (u === "medium" || u === "soon" || u === "moderate") return "soon";
  return "stakeholder";
}

function kindFromUrgency(urgency?: string): string {
  const bucket = bucketFromUrgency(urgency);
  if (bucket === "urgent") return "Active friction · unresolved";
  if (bucket === "soon") return "Watchpoint · soon";
  return "Stakeholder change";
}

/** Parse an ISO-ish string; return Number.NEGATIVE_INFINITY when unusable so
 *  dateless items sort last within a bucket (but still render). */
function parseTime(iso?: string | null): number {
  if (!iso) return Number.NEGATIVE_INFINITY;
  const t = Date.parse(iso);
  return Number.isFinite(t) ? t : Number.NEGATIVE_INFINITY;
}

const BUCKET_ORDER: Record<TriageBucket, number> = {
  urgent: 0,
  soon: 1,
  stakeholder: 2,
};

// ─────────────────────────────────────────────────────────────────────────────
// Local card builders
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Split a free-text paragraph into a short headline (first sentence) and
 * evidence (remainder). The AI enrichment emits a single `text` field for
 * each risk/win that's often a multi-sentence paragraph. The mockup's card
 * layout expects a punchy 21px serif headline on one line plus separate
 * sans-serif evidence below — stuffing the full paragraph into the headline
 * slot reads as a wall of text.
 *
 * Splits on the first sentence-ending `.`, `!`, or `?` followed by whitespace.
 * Falls back to treating the whole string as the headline when no split
 * point exists.
 */
function splitHeadlineEvidence(text: string): { headline: string; evidence?: string } {
  const trimmed = text.trim();
  if (!trimmed) return { headline: "" };
  const match = trimmed.match(/^(.+?[.!?])(\s+)(.+)$/s);
  if (!match) return { headline: trimmed };
  const headline = match[1].trim();
  const evidence = match[3].trim();
  return { headline, evidence: evidence.length > 0 ? evidence : undefined };
}

function localRiskCandidate(risk: IntelRisk, i: number): TriageCandidate {
  const label = risk.source ?? risk.itemSource?.source ?? undefined;
  const sourcedAt = risk.itemSource?.sourcedAt ?? "";
  const citations: TriageCitation[] = [];
  if (label) {
    citations.push({
      label: risk.itemSource?.reference ? `${label} · ${risk.itemSource.reference}` : label,
    });
  }
  const { headline, evidence } = splitHeadlineEvidence(risk.text);
  return {
    id: `local-risk-${i}`,
    bucket: bucketFromUrgency(risk.urgency),
    source: "local",
    sourcedAt,
    kind: kindFromUrgency(risk.urgency),
    headline,
    evidence,
    sources: [{ origin: "local", label: undefined }],
    citations,
  };
}

function localWinCandidate(win: IntelWin, i: number): TriageCandidate {
  const label = win.source ?? win.itemSource?.source ?? undefined;
  const citations: TriageCitation[] = [];
  if (label) citations.push({ label });
  const { headline, evidence: splitEvidence } = splitHeadlineEvidence(win.text);
  // Prefer the AI-emitted `impact` field when present; otherwise use the
  // split remainder. Concatenating both would produce duplicated detail.
  const evidence = win.impact ?? splitEvidence;
  return {
    id: `local-win-${i}`,
    bucket: "stakeholder",
    source: "local",
    sourcedAt: win.itemSource?.sourcedAt ?? "",
    kind: "Recent win · momentum",
    headline,
    evidence,
    sources: [{ origin: "local" }],
    citations,
  };
}

function buildLocalCandidates(intel: EntityIntelligence): TriageCandidate[] {
  const cards: TriageCandidate[] = [];
  for (const [i, risk] of (intel.risks ?? []).entries()) {
    cards.push(localRiskCandidate(risk, i));
  }
  for (const [i, win] of (intel.recentWins ?? []).entries()) {
    cards.push(localWinCandidate(win, i));
  }
  return cards;
}

// ─────────────────────────────────────────────────────────────────────────────
// Glean card builders — each returns normalised `TriageCandidate` rows.
// (Expanded from the prior `buildGleanCards` so every row carries bucket +
// sourcedAt + unified sources/citations.)
// ─────────────────────────────────────────────────────────────────────────────

function buildGleanCandidates(glean: HealthOutlookSignals): TriageCandidate[] {
  const cards: TriageCandidate[] = [];

  // Product-usage trend
  const usage = glean.productUsageTrend;
  const usageTrend = usage?.overallTrend30d;
  if (usageTrend === "declining" || usageTrend === "unknown") {
    const underutilizedNames = (usage?.underutilizedFeatures ?? [])
      .map((f) => f.name)
      .filter(Boolean)
      .slice(0, 3);
    const evidenceParts: string[] = [];
    if (usage?.summary) evidenceParts.push(usage.summary);
    if (underutilizedNames.length) {
      evidenceParts.push(`Underutilized: ${underutilizedNames.join(", ")}.`);
    }
    cards.push({
      id: "glean-usage-trend",
      bucket: usageTrend === "declining" ? "urgent" : "stakeholder",
      source: "glean",
      sourcedAt: "",
      kind:
        usageTrend === "declining"
          ? "Product usage · declining trend"
          : "Product usage · signal gap",
      headline:
        usageTrend === "declining"
          ? "Overall product usage is declining over the last 30 days."
          : "Product usage trend is unknown — no reliable signal in the last 30 days.",
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: "Product usage" }],
      citations: [],
    });
  }

  // Churn-adjacent transcript questions
  const churnQs = glean.transcriptExtraction?.churnAdjacentQuestions ?? [];
  for (const [i, q] of churnQs.entries()) {
    const evidenceParts: string[] = [];
    if (q.speaker) evidenceParts.push(`${q.speaker} asked.`);
    if (q.riskSignal) evidenceParts.push(q.riskSignal);
    const citationLabel = q.source ?? q.date ?? "Transcript";
    cards.push({
      id: `glean-churn-q-${i}`,
      bucket: "urgent",
      source: "glean",
      sourcedAt: q.date ?? "",
      kind: "Transcript · churn-adjacent question",
      headline: q.question,
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean" }],
      citations: [{ label: citationLabel }],
    });
  }

  // Decision-maker shifts — stakeholder bucket (changes at the buying table)
  const shifts = glean.transcriptExtraction?.decisionMakerShifts ?? [];
  for (const [i, s] of shifts.entries()) {
    const evidenceParts: string[] = [];
    if (s.who) evidenceParts.push(s.who);
    if (s.implication) evidenceParts.push(s.implication);
    const citationLabel = s.source ?? s.date ?? "Transcript";
    cards.push({
      id: `glean-dm-shift-${i}`,
      bucket: "stakeholder",
      source: "glean",
      sourcedAt: s.date ?? "",
      kind: "Decision-maker shift",
      headline: s.shift,
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean" }],
      citations: [{ label: citationLabel }],
    });
  }

  // Advocacy trend
  const advTrend = glean.advocacyTrack?.advocacyTrend;
  if (advTrend === "cooling") {
    const latestNps = glean.advocacyTrack?.npsHistory?.[0];
    const evidenceParts: string[] = [];
    if (latestNps?.score != null) {
      evidenceParts.push(
        `Most recent NPS ${latestNps.score}${latestNps.surveyDate ? ` (${latestNps.surveyDate})` : ""}.`,
      );
    }
    if (latestNps?.verbatim) evidenceParts.push(`"${latestNps.verbatim}"`);
    cards.push({
      id: "glean-advocacy-cooling",
      bucket: "soon",
      source: "glean",
      sourcedAt: latestNps?.surveyDate ?? "",
      kind: "Advocacy · cooling",
      headline: "Advocacy is cooling — reference posture is weakening.",
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: "Advocacy track" }],
      citations: [],
    });
  } else if (advTrend === "strengthening") {
    cards.push({
      id: "glean-advocacy-strengthening",
      bucket: "stakeholder",
      source: "glean",
      sourcedAt: "",
      kind: "Advocacy · strengthening",
      headline: "Advocacy is strengthening — capture the reference window.",
      sources: [{ origin: "glean", label: "Advocacy track" }],
      citations: [],
    });
  }

  // Champion at risk
  if (glean.championRisk?.atRisk) {
    const cr = glean.championRisk;
    const level = (cr.riskLevel ?? "moderate").toLowerCase();
    const bucket: TriageBucket =
      level === "high" ? "urgent" : level === "low" ? "stakeholder" : "soon";
    const evidenceParts: string[] = [];
    if (cr.riskEvidence?.length) evidenceParts.push(cr.riskEvidence.slice(0, 2).join(" "));
    if (cr.recentRoleChange) evidenceParts.push(cr.recentRoleChange);
    if (cr.emailSentimentTrend30d) {
      evidenceParts.push(`Email sentiment ${cr.emailSentimentTrend30d} over 30d.`);
    }
    cards.push({
      id: "glean-champion",
      bucket,
      source: "glean",
      sourcedAt: "",
      kind: `Champion risk · ${cr.championName ?? "primary contact"}`,
      headline: `${cr.championName ?? "Champion"} shows ${level}-risk signals.`,
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: "Champion analysis" }],
      citations: [],
    });
  }

  // Commercial (ARR direction)
  const arrDir = glean.commercialSignals?.arrDirection;
  if (arrDir && arrDir !== "flat") {
    const bucket: TriageBucket = arrDir === "shrinking" ? "urgent" : "soon";
    const headline =
      arrDir === "shrinking"
        ? "ARR trajectory is shrinking."
        : "ARR trajectory is growing — capture the expansion window.";
    const evidence =
      glean.commercialSignals?.paymentEvidence ?? glean.commercialSignals?.paymentBehavior ?? undefined;
    cards.push({
      id: "glean-commercial",
      bucket,
      source: "glean",
      sourcedAt: "",
      kind: "Commercial signal · ARR trajectory",
      headline,
      evidence,
      sources: [{ origin: "glean", label: "Commercial signals" }],
      citations: [],
    });
  }

  // Competitive benchmarks
  const competitors = (glean.transcriptExtraction?.competitorBenchmarks ?? []).filter(
    (c) => c.threatLevel === "decision_relevant" || c.threatLevel === "actively_comparing",
  );
  for (const [i, c] of competitors.entries()) {
    const bucket: TriageBucket = c.threatLevel === "decision_relevant" ? "urgent" : "soon";
    const kindSuffix =
      c.threatLevel === "decision_relevant" ? "decision-relevant" : "actively comparing";
    const headline =
      c.threatLevel === "decision_relevant"
        ? `${c.competitor} surfaced in a decision-relevant context.`
        : `${c.competitor} is being actively compared.`;
    const citationLabel = c.source ?? c.date ?? "Transcript";
    cards.push({
      id: `glean-competitor-${i}`,
      bucket,
      source: "glean",
      sourcedAt: c.date ?? "",
      kind: `Competitive pressure · ${kindSuffix}`,
      headline,
      evidence: c.context ?? undefined,
      sources: [{ origin: "glean" }],
      citations: [{ label: citationLabel }],
    });
  }

  // Quote wall — sentiment branching (DOS-203 Wave-0f preserved)
  const quoteWall = glean.quoteWall ?? [];
  if (quoteWall.length > 0) {
    const negativeCount = quoteWall.filter((q) => q.sentiment === "negative").length;
    let negativesSeen = 0;
    for (const [i, q] of quoteWall.entries()) {
      const sentiment = q.sentiment ?? "neutral";
      let bucket: TriageBucket;
      let kind: string;
      let headline: string;
      if (sentiment === "negative") {
        negativesSeen += 1;
        bucket = negativeCount >= 2 && negativesSeen === 1 ? "urgent" : "soon";
        kind = "Quote wall · negative customer voice";
        headline = q.speaker
          ? `${q.speaker}${q.role ? ` (${q.role})` : ""}: "${q.quote}"`
          : `"${q.quote}"`;
      } else if (sentiment === "mixed") {
        bucket = "stakeholder";
        kind = "Quote wall · mixed sentiment";
        headline = q.speaker
          ? `${q.speaker}${q.role ? ` (${q.role})` : ""}: "${q.quote}"`
          : `"${q.quote}"`;
      } else if (sentiment === "positive") {
        bucket = "stakeholder";
        kind = "Quote wall · capture opportunity";
        headline = q.speaker
          ? `Promote to case study / references — ${q.speaker}${q.role ? ` (${q.role})` : ""}: "${q.quote}"`
          : `Promote to case study / references — "${q.quote}"`;
      } else {
        bucket = "stakeholder";
        kind = "Quote wall · customer voice";
        headline = q.speaker
          ? `${q.speaker}${q.role ? ` (${q.role})` : ""}: "${q.quote}"`
          : `"${q.quote}"`;
      }
      const citationLabel = q.source ?? q.date ?? "Quote wall";
      cards.push({
        id: `glean-quote-${i}`,
        bucket,
        source: "glean",
        sourcedAt: q.date ?? "",
        kind,
        headline,
        evidence: q.whyItMatters ?? undefined,
        sources: [{ origin: "glean" }],
        citations: [{ label: citationLabel }],
      });
    }
  }

  // Budget cycle (locked)
  const budgets = (glean.transcriptExtraction?.budgetCycleSignals ?? []).filter((b) => b.locked);
  for (const [i, b] of budgets.entries()) {
    const citationLabel = b.source ?? b.date ?? "Transcript";
    cards.push({
      id: `glean-budget-${i}`,
      bucket: "soon",
      source: "glean",
      sourcedAt: b.date ?? "",
      kind: "Budget cycle · locked",
      headline: b.signal,
      evidence: b.implication ?? undefined,
      sources: [{ origin: "glean" }],
      citations: [{ label: citationLabel }],
    });
  }

  return cards;
}

// ─────────────────────────────────────────────────────────────────────────────
// Ranking + section render
// ─────────────────────────────────────────────────────────────────────────────

/** Ranks candidates by bucket THEN recency (newest first). */
export function rankTriageCandidates(candidates: TriageCandidate[]): TriageCandidate[] {
  return [...candidates].sort((a, b) => {
    const byBucket = BUCKET_ORDER[a.bucket] - BUCKET_ORDER[b.bucket];
    if (byBucket !== 0) return byBucket;
    return parseTime(b.sourcedAt) - parseTime(a.sourcedAt);
  });
}

function buildAllCandidates(
  intelligence: EntityIntelligence | null,
  gleanSignals: HealthOutlookSignals | null,
): TriageCandidate[] {
  const local = intelligence ? buildLocalCandidates(intelligence) : [];
  const glean = gleanSignals ? buildGleanCandidates(gleanSignals) : [];
  return [...glean, ...local];
}

/**
 * Default actions per bucket — mockup-faithful pill pair on every card.
 *
 * TODO(DOS-250): wire to real mutation backends (snooze / resolve / reassign /
 * map-stakeholder / defer). Today the buttons are visually present with
 * no-op handlers so the editorial weight matches the mockup. A visibly
 * dead card (no actions at all) was the prior state — this closes that gap
 * without fabricating a backend that doesn't exist yet.
 */
function defaultActionsFor(candidate: TriageCandidate): TriageAction[] {
  const noop = () => {
    /* TODO(DOS-250): wire triage action handlers. */
  };
  const kindLower = candidate.kind.toLowerCase();

  if (candidate.bucket === "urgent") {
    return [
      { label: "Snooze", onClick: noop },
      { label: "Confirm resolved", primary: true, onClick: noop },
    ];
  }

  if (candidate.bucket === "soon") {
    // Glean expansion or transcript-question → "Send pricing"; otherwise a
    // generic "Follow up" primary for risk-leaning soon cards.
    let primaryLabel = "Address";
    if (
      kindLower.includes("expansion") ||
      kindLower.includes("transcript") ||
      kindLower.includes("question")
    ) {
      primaryLabel = "Send pricing";
    } else if (
      kindLower.includes("budget") ||
      kindLower.includes("competitive")
    ) {
      primaryLabel = "Follow up";
    }
    return [
      { label: "Reassign", onClick: noop },
      { label: primaryLabel, primary: true, onClick: noop },
    ];
  }

  // stakeholder
  return [
    { label: "Defer", onClick: noop },
    { label: "Map stakeholder", primary: true, onClick: noop },
  ];
}

interface TriageSectionProps {
  intelligence: EntityIntelligence | null;
  gleanSignals: HealthOutlookSignals | null;
  /** Hard cap on rendered cards. Defaults to 5 per the DOS-249 spec. */
  maxCards?: number;
}

export function TriageSection({
  intelligence,
  gleanSignals,
  maxCards = MAX_CARDS,
}: TriageSectionProps) {
  const allCandidates = buildAllCandidates(intelligence, gleanSignals);
  if (allCandidates.length === 0) return null;

  const ranked = rankTriageCandidates(allCandidates).slice(0, maxCards);

  return (
    <>
      {/* Compact header: the chapter title "Needs attention" lives in the
          MarginSection gutter label, so this strip carries only the count
          chip + horizontal rule. No inline 28px h2 duplicating the gutter. */}
      <div className={styles.triageCompactHeader}>
        <span className={styles.triageCompactCount}>
          {ranked.length} item{ranked.length === 1 ? "" : "s"}
          {allCandidates.length > ranked.length
            ? ` · showing top ${ranked.length} of ${allCandidates.length}`
            : ""}
          {" · scan 60s"}
        </span>
      </div>
      <div>
        {ranked.map((c) => (
          <TriageCard
            key={c.id}
            tone={toneForBucket(c.bucket)}
            kind={c.kind}
            headline={c.headline}
            evidence={c.evidence}
            sources={c.sources}
            citations={c.citations}
            actions={defaultActionsFor(c)}
          />
        ))}
      </div>
    </>
  );
}

/** Exposed for callers that want to know whether triage will render.
 *
 *  Symmetric with `TriageSection`: returns true when ANY local or Glean card
 *  would render. This is the gate for the Health-tab fine state — if it
 *  returns false, the caller renders the "On track" chapter instead.
 */
export function hasTriageContent(
  intelligence: EntityIntelligence | null,
  gleanSignals: HealthOutlookSignals | null,
): boolean {
  return buildAllCandidates(intelligence, gleanSignals).length > 0;
}
