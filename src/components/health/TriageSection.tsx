/**
 * TriageSection — "Needs attention" chapter for the Health tab (DOS-203).
 *
 * Source priority:
 *   1. Glean leading signals (when `gleanSignals` is not null) emit dedicated
 *      card types per the Wave-0c spec:
 *        - championRisk → champion-at-risk (spine: urgent/soon)
 *        - commercialSignals.arrDirection → commercial signal (spine: soon)
 *        - channelSentiment.divergenceDetected → deferred to DivergenceSection
 *        - transcriptExtraction.competitorBenchmarks (decision_relevant) → competitive (urgent)
 *        - transcriptExtraction.budgetCycleSignals (locked) → budget (soon)
 *   2. Local intelligence fallback — `intelligence.risks` (urgency-driven)
 *      + `intelligence.recentWins` (always rendered as gap/meta positive tone)
 *
 * Returns `null` when no cards exist so the caller can branch into the fine state.
 */
import type { ReactNode } from "react";
import type { EntityIntelligence, HealthOutlookSignals } from "@/types";
import { ChapterFreshness } from "@/components/editorial/ChapterFreshness";
import { TriageCard, type TriageTone, type TriageSource } from "./TriageCard";
import styles from "./health.module.css";

interface BuiltCard {
  key: string;
  tone: TriageTone;
  kind: string;
  headline: string;
  evidence?: ReactNode;
  sources: TriageSource[];
}

function toneFromUrgency(urgency?: string): TriageTone {
  const u = (urgency ?? "").toLowerCase();
  if (u === "high" || u === "urgent" || u === "critical") return "urgent";
  if (u === "medium" || u === "soon") return "soon";
  return "gap";
}

function kindFromUrgency(urgency?: string): string {
  const tone = toneFromUrgency(urgency);
  if (tone === "urgent") return "Active friction · unresolved";
  if (tone === "soon") return "Watchpoint · soon";
  return "Gap · note";
}

/** Build cards from Glean leading signals when present. */
function buildGleanCards(glean: HealthOutlookSignals): BuiltCard[] {
  const cards: BuiltCard[] = [];

  // DOS-232 Codex fix: product-usage trend — declining/unknown should surface
  // as a triage card instead of silently falling into the fine state.
  const usage = glean.productUsageTrend;
  const usageTrend = usage?.overallTrend30d;
  if (usageTrend === "declining" || usageTrend === "unknown") {
    const tone: TriageTone = usageTrend === "declining" ? "urgent" : "gap";
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
      key: "glean-usage-trend",
      tone,
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
    });
  }

  // DOS-232 Codex fix: churn-adjacent transcript questions — each question is
  // direct evidence of elevated risk; do not require another signal family to
  // be present.
  const churnQs = glean.transcriptExtraction?.churnAdjacentQuestions ?? [];
  for (const [i, q] of churnQs.entries()) {
    const evidenceParts: string[] = [];
    if (q.speaker) evidenceParts.push(`${q.speaker} asked.`);
    if (q.riskSignal) evidenceParts.push(q.riskSignal);
    cards.push({
      key: `glean-churn-q-${i}`,
      tone: "urgent",
      kind: "Transcript · churn-adjacent question",
      headline: q.question,
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: q.source ?? q.date ?? "Transcript" }],
    });
  }

  // DOS-232 Codex fix: decision-maker shifts — changes at the buying table
  // materially change renewal posture.
  const shifts = glean.transcriptExtraction?.decisionMakerShifts ?? [];
  for (const [i, s] of shifts.entries()) {
    const evidenceParts: string[] = [];
    if (s.who) evidenceParts.push(s.who);
    if (s.implication) evidenceParts.push(s.implication);
    cards.push({
      key: `glean-dm-shift-${i}`,
      tone: "soon",
      kind: "Decision-maker shift",
      headline: s.shift,
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: s.source ?? s.date ?? "Transcript" }],
    });
  }

  // DOS-232 Codex fix: advocacy trend cooling — advocacy is a leading loyalty
  // indicator; cooling advocacy belongs in triage.
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
      key: "glean-advocacy-cooling",
      tone: "soon",
      kind: "Advocacy · cooling",
      headline: "Advocacy is cooling — reference posture is weakening.",
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: "Advocacy track" }],
    });
  } else if (advTrend === "strengthening") {
    // Strengthening advocacy is a capture-the-moment opportunity, not an
    // urgent risk — render as a low-tone gap so the fine state doesn't
    // masquerade over a usable expansion window.
    cards.push({
      key: "glean-advocacy-strengthening",
      tone: "gap",
      kind: "Advocacy · strengthening",
      headline: "Advocacy is strengthening — capture the reference window.",
      evidence: undefined,
      sources: [{ origin: "glean", label: "Advocacy track" }],
    });
  }

  // Champion at risk
  if (glean.championRisk?.atRisk) {
    const cr = glean.championRisk;
    const level = (cr.riskLevel ?? "moderate").toLowerCase();
    const tone: TriageTone = level === "high" ? "urgent" : level === "low" ? "gap" : "soon";
    const evidenceParts: string[] = [];
    if (cr.riskEvidence?.length) evidenceParts.push(cr.riskEvidence.slice(0, 2).join(" "));
    if (cr.recentRoleChange) evidenceParts.push(cr.recentRoleChange);
    if (cr.emailSentimentTrend30d) {
      evidenceParts.push(`Email sentiment ${cr.emailSentimentTrend30d} over 30d.`);
    }
    cards.push({
      key: "glean-champion",
      tone,
      kind: `Champion risk · ${cr.championName ?? "primary contact"}`,
      headline: `${cr.championName ?? "Champion"} shows ${level}-risk signals.`,
      evidence: evidenceParts.length ? evidenceParts.join(" ") : undefined,
      sources: [{ origin: "glean", label: "Champion analysis" }],
    });
  }

  // Commercial (ARR direction)
  const arrDir = glean.commercialSignals?.arrDirection;
  if (arrDir && arrDir !== "flat") {
    const tone: TriageTone = arrDir === "shrinking" ? "urgent" : "soon";
    const headline =
      arrDir === "shrinking"
        ? "ARR trajectory is shrinking."
        : "ARR trajectory is growing — capture the expansion window.";
    const evidence = glean.commercialSignals?.paymentEvidence ?? glean.commercialSignals?.paymentBehavior;
    cards.push({
      key: "glean-commercial",
      tone,
      kind: "Commercial signal · ARR trajectory",
      headline,
      evidence: evidence ?? undefined,
      sources: [{ origin: "glean", label: "Commercial signals" }],
    });
  }

  // Competitive benchmarks — decision_relevant is urgent; actively_comparing
  // is a soon-watchpoint. (DOS-232 Codex fix: surface actively_comparing too.)
  const competitors = (glean.transcriptExtraction?.competitorBenchmarks ?? []).filter(
    (c) => c.threatLevel === "decision_relevant" || c.threatLevel === "actively_comparing",
  );
  for (const [i, c] of competitors.entries()) {
    const tone: TriageTone = c.threatLevel === "decision_relevant" ? "urgent" : "soon";
    const kindSuffix = c.threatLevel === "decision_relevant" ? "decision-relevant" : "actively comparing";
    const headline =
      c.threatLevel === "decision_relevant"
        ? `${c.competitor} surfaced in a decision-relevant context.`
        : `${c.competitor} is being actively compared.`;
    cards.push({
      key: `glean-competitor-${i}`,
      tone,
      kind: `Competitive pressure · ${kindSuffix}`,
      headline,
      evidence: c.context ?? undefined,
      sources: [{ origin: "glean", label: c.source ?? c.date ?? "Transcript" }],
    });
  }

  // Budget cycle (locked)
  const budgets = (glean.transcriptExtraction?.budgetCycleSignals ?? []).filter((b) => b.locked);
  for (const [i, b] of budgets.entries()) {
    cards.push({
      key: `glean-budget-${i}`,
      tone: "soon",
      kind: "Budget cycle · locked",
      headline: b.signal,
      evidence: b.implication ?? undefined,
      sources: [{ origin: "glean", label: b.source ?? b.date ?? "Transcript" }],
    });
  }

  return cards;
}

/** Fallback: build cards from local `intelligence.risks` + `recentWins`. */
function buildLocalCards(intel: EntityIntelligence): BuiltCard[] {
  const cards: BuiltCard[] = [];

  for (const [i, risk] of (intel.risks ?? []).entries()) {
    const label = risk.source ?? (risk.itemSource?.source ?? null);
    cards.push({
      key: `local-risk-${i}`,
      tone: toneFromUrgency(risk.urgency),
      kind: kindFromUrgency(risk.urgency),
      headline: risk.text,
      evidence: undefined,
      sources: [{ origin: "local", label: label ?? undefined }],
    });
  }

  for (const [i, win] of (intel.recentWins ?? []).entries()) {
    cards.push({
      key: `local-win-${i}`,
      tone: "gap",
      kind: "Recent win · momentum",
      headline: win.text,
      evidence: win.impact ?? undefined,
      sources: [{ origin: "local", label: win.source ?? win.itemSource?.source ?? undefined }],
    });
  }

  return cards;
}

interface TriageSectionProps {
  intelligence: EntityIntelligence | null;
  gleanSignals: HealthOutlookSignals | null;
}

export function TriageSection({ intelligence, gleanSignals }: TriageSectionProps) {
  const gleanCards = gleanSignals ? buildGleanCards(gleanSignals) : [];
  const localCards = intelligence ? buildLocalCards(intelligence) : [];
  // Glean-first when available; always union with local fallback so nothing
  // is silently dropped when Glean is unavailable.
  const cards = [...gleanCards, ...localCards];

  if (cards.length === 0) return null;

  return (
    <>
      <section className={styles.triageHeader}>
        <hr className={styles.triageRule} />
        <div className={styles.triageTitleRow}>
          <h2 className={styles.triageTitle}>Needs attention</h2>
          <span className={styles.triageCount}>
            {cards.length} item{cards.length === 1 ? "" : "s"}
          </span>
        </div>
        <ChapterFreshness
          enrichedAt={intelligence?.enrichedAt}
          fragments={[gleanSignals ? "Glean + local signals" : "Local signals"]}
        />
      </section>
      <div>
        {cards.map((c) => (
          <TriageCard
            key={c.key}
            tone={c.tone}
            kind={c.kind}
            headline={c.headline}
            evidence={c.evidence}
            sources={c.sources}
          />
        ))}
      </div>
    </>
  );
}

/** Exposed for callers that want to know whether triage will render. */
export function hasTriageContent(
  intelligence: EntityIntelligence | null,
  gleanSignals: HealthOutlookSignals | null,
): boolean {
  if (intelligence && ((intelligence.risks?.length ?? 0) > 0 || (intelligence.recentWins?.length ?? 0) > 0)) {
    return true;
  }
  if (gleanSignals && buildGleanCards(gleanSignals).length > 0) return true;
  return false;
}
