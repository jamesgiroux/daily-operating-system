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
import { useCallback, useEffect, useState, type ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type {
  EntityIntelligence,
  HealthOutlookSignals,
  IntelRisk,
  IntelWin,
  SentimentValue,
} from "@/types";
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
import { useEntitySuppressions } from "@/hooks/useEntitySuppressions";
import {
  TriageCard,
  type TriageAction,
  type TriageCitation,
  type TriageSource,
  type TriageTone,
} from "./TriageCard";
import styles from "./health.module.css";

/** DOS-269: snooze/resolve persistence row returned by list_triage_snoozes. */
interface TriageSnoozeRow {
  triageKey: string;
  snoozedUntil: string | null;
  resolvedAt: string | null;
}

/** Ranking bucket — drives ordering and (via `toneForBucket`) spine colour. */
export type TriageBucket = "urgent" | "soon" | "stakeholder";

/** A normalised, rank-ready triage row — Local or Glean. */
export interface TriageCandidate {
  /** Stable per-card id for feedback attribution. */
  id: string;
  /** Stable textual claim for suppression + feedback attribution. */
  itemKey: string;
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
    itemKey: risk.text,
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
    itemKey: win.text,
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
      itemKey:
        usageTrend === "declining"
          ? "Overall product usage is declining over the last 30 days."
          : "Product usage trend is unknown — no reliable signal in the last 30 days.",
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
      itemKey: q.question,
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
      itemKey: s.shift,
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
      itemKey: "Advocacy is cooling — reference posture is weakening.",
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
      itemKey: "Advocacy is strengthening — capture the reference window.",
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
      itemKey: `${cr.championName ?? "Champion"} shows ${level}-risk signals.`,
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
      itemKey: headline,
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
      itemKey: headline,
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
        itemKey: q.quote,
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
      itemKey: b.signal,
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

function passesSentimentThreshold(
  candidate: TriageCandidate,
  sentiment: SentimentValue | null | undefined,
): boolean {
  if (!sentiment || sentiment === "concerning" || sentiment === "at_risk" || sentiment === "critical") {
    return true;
  }
  if (sentiment === "on_track") {
    return candidate.bucket !== "stakeholder";
  }
  // "strong" should only surface high-signal concerns.
  if (candidate.bucket === "urgent") return true;
  return candidate.bucket === "soon" && candidate.source === "local";
}

/**
 * DOS-269: Per-card action handlers. Snooze → persisted to `triage_snoozes`
 * (card removed on next render; reappears after the snooze window).
 * Confirm resolved → persisted resolution tombstone + DOS-41 "corrected"
 * correction event so the Intelligence Loop sees the user acted.
 *
 * The action copy still reflects the mockup's bucket-specific secondary
 * label (Reassign / Defer). Those remain no-ops in this wave; only the
 * editorial-critical primaries and the shared Snooze are wired.
 */
interface ActionHandlers {
  onSnooze: (candidate: TriageCandidate) => Promise<void>;
  onResolve: (candidate: TriageCandidate) => Promise<void>;
}

function actionsFor(candidate: TriageCandidate, handlers: ActionHandlers): TriageAction[] {
  const kindLower = candidate.kind.toLowerCase();

  if (candidate.bucket === "urgent") {
    return [
      { label: "Snooze", onClick: () => void handlers.onSnooze(candidate) },
      {
        label: "Confirm resolved",
        primary: true,
        onClick: () => void handlers.onResolve(candidate),
      },
    ];
  }

  if (candidate.bucket === "soon") {
    let primaryLabel = "Address";
    if (
      kindLower.includes("expansion") ||
      kindLower.includes("transcript") ||
      kindLower.includes("question")
    ) {
      primaryLabel = "Send pricing";
    } else if (kindLower.includes("budget") || kindLower.includes("competitive")) {
      primaryLabel = "Follow up";
    }
    return [
      { label: "Snooze", onClick: () => void handlers.onSnooze(candidate) },
      {
        label: primaryLabel,
        primary: true,
        onClick: () => void handlers.onResolve(candidate),
      },
    ];
  }

  // stakeholder
  return [
    { label: "Snooze", onClick: () => void handlers.onSnooze(candidate) },
    {
      label: "Map stakeholder",
      primary: true,
      onClick: () => void handlers.onResolve(candidate),
    },
  ];
}

interface TriageSectionProps {
  intelligence: EntityIntelligence | null;
  gleanSignals: HealthOutlookSignals | null;
  sentiment?: SentimentValue | null;
  /** DOS-269: Account id is required to persist snooze/resolve state. When
   *  absent (tests, previews), actions render but are no-ops. */
  accountId?: string;
  /** Hard cap on rendered cards. Defaults to 5 per the DOS-249 spec. */
  maxCards?: number;
}

/** DOS-269: Is a snooze row still active (suppresses the card)? */
function isSuppressed(row: TriageSnoozeRow, now: number): boolean {
  if (row.resolvedAt) return true;
  if (!row.snoozedUntil) return false;
  const until = Date.parse(row.snoozedUntil);
  return Number.isFinite(until) && until > now;
}

export function TriageSection({
  intelligence,
  gleanSignals,
  sentiment,
  accountId,
  maxCards = MAX_CARDS,
}: TriageSectionProps) {
  const suppressions = useEntitySuppressions(accountId);
  const [snoozes, setSnoozes] = useState<TriageSnoozeRow[]>([]);
  const [optimisticallyHidden, setOptimisticallyHidden] = useState<Set<string>>(
    () => new Set(),
  );

  const refreshSnoozes = useCallback(async () => {
    if (!accountId) return;
    try {
      const rows = await invoke<TriageSnoozeRow[]>("list_triage_snoozes", {
        entityType: "account",
        entityId: accountId,
      });
      setSnoozes(rows);
    } catch (e) {
      console.warn("list_triage_snoozes failed:", e);
    }
  }, [accountId]);

  useEffect(() => {
    void refreshSnoozes();
  }, [refreshSnoozes]);

  const handleSnooze = useCallback(
    async (candidate: TriageCandidate) => {
      if (!accountId) return;
      setOptimisticallyHidden((prev) => new Set(prev).add(candidate.id));
      try {
        await invoke("snooze_triage_item", {
          entityType: "account",
          entityId: accountId,
          triageKey: candidate.id,
          days: 14,
        });
        await refreshSnoozes();
        toast.success("Snoozed for 14 days");
      } catch (e) {
        setOptimisticallyHidden((prev) => {
          const next = new Set(prev);
          next.delete(candidate.id);
          return next;
        });
        toast.error("Could not snooze");
        console.error("snooze_triage_item failed:", e);
      }
    },
    [accountId, refreshSnoozes],
  );

  const handleResolve = useCallback(
    async (candidate: TriageCandidate) => {
      if (!accountId) return;
      setOptimisticallyHidden((prev) => new Set(prev).add(candidate.id));
      try {
        await invoke("resolve_triage_item", {
          entityType: "account",
          entityId: accountId,
          triageKey: candidate.id,
        });
        await refreshSnoozes();
      } catch (e) {
        setOptimisticallyHidden((prev) => {
          const next = new Set(prev);
          next.delete(candidate.id);
          return next;
        });
        toast.error("Could not mark resolved");
        console.error("resolve_triage_item failed:", e);
      }
    },
    [accountId, refreshSnoozes],
  );

  const allCandidates = buildAllCandidates(intelligence, gleanSignals);
  if (allCandidates.length === 0) return null;

  // DOS-269: hide snoozed/resolved cards. `optimisticallyHidden` covers the
  // gap between click and the backend round-trip for `list_triage_snoozes`.
  const now = Date.now();
  const suppressedKeys = new Set<string>(optimisticallyHidden);
  for (const row of snoozes) {
    if (isSuppressed(row, now)) suppressedKeys.add(row.triageKey);
  }
  const visible = allCandidates.filter(
    (c) =>
      !suppressedKeys.has(c.id) &&
      !suppressions.isSuppressed(`triage:${c.id}`, c.itemKey) &&
      passesSentimentThreshold(c, sentiment),
  );
  if (visible.length === 0) return null;

  const ranked = rankTriageCandidates(visible).slice(0, maxCards);
  const handlers: ActionHandlers = { onSnooze: handleSnooze, onResolve: handleResolve };

  return (
    <>
      {/* Block header: 28px serif h2 + rule + terracotta count chip.
          Visually distinct from the MarginSection gutter label (small
          mono uppercase). Gutter = orientation marker; this h2 = chapter
          title, per mockup lines 637-644. */}
      <section className={styles.blockHeader}>
        <hr className={styles.blockHeaderRule} />
        <div className={styles.blockHeaderTitleRow}>
          <h2 className={styles.blockHeaderTitle}>Needs attention</h2>
          <span className={`${styles.blockHeaderCount} ${styles.blockHeaderCountTerracotta}`}>
            {ranked.length} item{ranked.length === 1 ? "" : "s"}
            {visible.length > ranked.length
              ? ` · showing top ${ranked.length} of ${visible.length}`
              : ""}
            {" · scan 60s"}
          </span>
        </div>
      </section>
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
            actions={actionsFor(c, handlers)}
            feedbackSlot={
              accountId ? (
                <IntelligenceCorrection
                  entityId={accountId}
                  entityType="account"
                  field={`triage:${c.id}`}
                  itemKey={c.itemKey}
                  onDismissed={async () => {
                    suppressions.markSuppressed(`triage:${c.id}`, c.itemKey);
                  }}
                />
              ) : undefined
            }
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
  sentiment?: SentimentValue | null,
): boolean {
  return buildAllCandidates(intelligence, gleanSignals).some((candidate) =>
    passesSentimentThreshold(candidate, sentiment),
  );
}
