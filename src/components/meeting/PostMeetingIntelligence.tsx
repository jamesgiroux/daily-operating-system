import type {
  MeetingPostIntelligence as PostIntelData,
  EnrichedCapture,
  SpeakerSentiment,
} from "@/types";
import {
  ArrowRight,
  Check,
  X,
} from "lucide-react";
import { TalkBalanceBar } from "@/components/shared/TalkBalanceBar";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import styles from "./PostMeetingIntelligence.module.css";

// =============================================================================
// Props
// =============================================================================

interface PostMeetingIntelligenceProps {
  data: PostIntelData;
  /** Flat outcomes summary (executive summary text) */
  summary?: string;
  /** Per-item feedback value getter. Field path like "captures[0].content". */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
  /** Callback when a proposed action is accepted */
  onAcceptAction?: (captureId: string) => void;
  /** Callback when a proposed action is dismissed */
  onDismissAction?: (captureId: string) => void;
}

// =============================================================================
// Component
// =============================================================================

export function PostMeetingIntelligence({
  data,
  summary,
  getItemFeedback,
  onItemFeedback,
  onAcceptAction,
  onDismissAction,
}: PostMeetingIntelligenceProps) {
  const { interactionDynamics, championHealth, roleChanges, enrichedCaptures } = data;

  // Pair each capture with its original index for stable feedback field paths
  const indexed = enrichedCaptures.map((c, i) => ({ capture: c, originalIndex: i }));
  const wins = indexed.filter((c) => c.capture.captureType === "win");
  const risks = indexed
    .filter((c) => c.capture.captureType === "risk")
    .sort((a, b) => urgencyOrder(a.capture.urgency) - urgencyOrder(b.capture.urgency));
  const decisions = indexed.filter((c) => c.capture.captureType === "decision");
  const commitments = indexed.filter((c) => c.capture.captureType === "commitment");

  const hasEngagement = !!interactionDynamics;
  const hasChampion = !!championHealth && championHealth.championStatus !== "none";
  const hasWins = wins.length > 0;
  const hasRisks = risks.length > 0;
  const hasDecisions = decisions.length > 0;
  const hasCommitments = commitments.length > 0;
  const hasFindings = hasWins || hasRisks || hasDecisions;
  const hasRoleChanges = roleChanges.length > 0;
  const hasCompetitorMentions = (interactionDynamics?.competitorMentions?.length ?? 0) > 0;
  const hasEscalation = (interactionDynamics?.escalationLanguage?.length ?? 0) > 0;

  return (
    <div className={styles.container}>
      {/* ═══════ EXECUTIVE SUMMARY ═══════ */}
      {summary && (
        <div className={styles.summaryBlock}>
          <p className={styles.summaryText}>{summary}</p>
        </div>
      )}

      {/* ═══════ ENGAGEMENT ═══════ */}
      {hasEngagement && interactionDynamics && (
        <section className={styles.chapter}>
          <ChapterHeading title="Engagement" />

          {/* Talk balance bar */}
          {interactionDynamics.talkBalanceCustomerPct != null &&
            interactionDynamics.talkBalanceInternalPct != null && (
            <div className={styles.talkBalance}>
              <TalkBalanceBar
                customerPct={interactionDynamics.talkBalanceCustomerPct}
                internalPct={interactionDynamics.talkBalanceInternalPct}
              />
            </div>
          )}

          {/* Speaker sentiments */}
          {interactionDynamics.speakerSentiments.length > 0 && (
            <div className={styles.speakerSentiments}>
              {interactionDynamics.speakerSentiments.map((speaker, i) => (
                <SpeakerSentimentBlock key={i} speaker={speaker} />
              ))}
            </div>
          )}

          {/* Signal grid */}
          {(interactionDynamics.questionDensity ||
            interactionDynamics.decisionMakerActive ||
            interactionDynamics.forwardLooking ||
            interactionDynamics.monologueRisk) && (
            <div className={styles.signalGrid}>
              {interactionDynamics.questionDensity && (
                <>
                  <span className={styles.signalKey}>Question Density</span>
                  <span className={styles.signalValue}>{interactionDynamics.questionDensity}</span>
                </>
              )}
              {interactionDynamics.decisionMakerActive && (
                <>
                  <span className={styles.signalKey}>Decision Maker</span>
                  <span className={styles.signalValue}>{interactionDynamics.decisionMakerActive}</span>
                </>
              )}
              {interactionDynamics.forwardLooking && (
                <>
                  <span className={styles.signalKey}>Forward Looking</span>
                  <span className={styles.signalValue}>{interactionDynamics.forwardLooking}</span>
                </>
              )}
              {interactionDynamics.monologueRisk && (
                <>
                  <span className={styles.signalKey}>Monologue Risk</span>
                  <span className={styles.signalValue}>Yes</span>
                </>
              )}
            </div>
          )}

          {/* Escalation language */}
          {hasEscalation && interactionDynamics.escalationLanguage.map((e, i) => (
            <div key={i} className={styles.escalationBlock}>
              <p className={styles.escalationQuote}>&ldquo;{e.quote}&rdquo;</p>
              <p className={styles.escalationAttribution}>&mdash; {e.speaker}</p>
            </div>
          ))}

          {/* Competitor mentions */}
          {hasCompetitorMentions && interactionDynamics.competitorMentions.map((m, i) => (
            <p key={i} className={styles.competitorMention}>
              <span className={styles.competitorName}>{m.competitor}</span> &mdash; {m.context}
            </p>
          ))}
        </section>
      )}

      {/* ═══════ CHAMPION HEALTH ═══════ */}
      {hasChampion && championHealth && (
        <section className={styles.chapter}>
          <ChapterHeading title="Champion Health" />
          <div className={styles.championHeader}>
            {championHealth.championName && (
              <span className={styles.championName}>{championHealth.championName}</span>
            )}
            <span className={championStatusBadgeClass(championHealth.championStatus)}>
              {championHealth.championStatus}
            </span>
          </div>
          {championHealth.championEvidence && (
            <p className={styles.championEvidence}>{championHealth.championEvidence}</p>
          )}
          {championHealth.championRisk && (
            <p className={styles.championRisk}>{championHealth.championRisk}</p>
          )}
        </section>
      )}

      {/* ═══════ KEY FINDINGS ═══════ */}
      {hasFindings && (
        <section className={styles.chapter}>
          <ChapterHeading title="Key Findings" />

          {/* Wins */}
          {hasWins && (
            <div className={styles.findingsGroup}>
              <p className={styles.monoLabelSage}>Wins</p>
              {wins.map((c) => (
                <FindingItem
                  key={c.capture.id}
                  capture={c.capture}
                  dotClass={styles.findingDotSage}
                  fieldPath={`captures[${c.originalIndex}].content`}
                  getItemFeedback={getItemFeedback}
                  onItemFeedback={onItemFeedback}
                />
              ))}
            </div>
          )}

          {/* Risks */}
          {hasRisks && (
            <div className={styles.findingsGroup}>
              <p className={styles.monoLabelTerracotta}>Risks</p>
              {risks.map((c) => (
                <FindingItem
                  key={c.capture.id}
                  capture={c.capture}
                  dotClass={riskDotClass(c.capture.urgency)}
                  fieldPath={`captures[${c.originalIndex}].content`}
                  getItemFeedback={getItemFeedback}
                  onItemFeedback={onItemFeedback}
                />
              ))}
            </div>
          )}

          {/* Decisions */}
          {hasDecisions && (
            <div className={styles.findingsGroup}>
              <p className={styles.monoLabel}>Decisions</p>
              {decisions.map((c) => (
                <FindingItem
                  key={c.capture.id}
                  capture={c.capture}
                  dotClass={styles.findingDotCharcoal}
                  fieldPath={`captures[${c.originalIndex}].content`}
                  getItemFeedback={getItemFeedback}
                  onItemFeedback={onItemFeedback}
                />
              ))}
            </div>
          )}
        </section>
      )}

      {/* ═══════ COMMITMENTS & ACTIONS ═══════ */}
      {hasCommitments && (
        <section className={styles.chapter}>
          <ChapterHeading title="Commitments & Actions" />

          {/* Commitment items (explicit commitments from transcript) */}
          <div>
            {commitments.map((c) => (
              <div key={c.capture.id} className={styles.commitmentItem}>
                <ArrowRight size={14} className={styles.commitmentIcon} />
                <span>
                  {c.capture.content}
                  {c.capture.subType && (
                    <span className={styles.commitmentTag}>{c.capture.subType}</span>
                  )}
                </span>
              </div>
            ))}
          </div>

          {/* Proposed actions — accept/dismiss pattern */}
          {commitments.some((c) => c.capture.impact) && (
            <>
              <p className={styles.actionsSublabel}>Suggested Actions</p>
              <div>
                {commitments
                  .filter((c) => c.capture.impact)
                  .map((c) => (
                    <div key={`action-${c.capture.id}`} className={styles.actionItemProposed}>
                      <span className={styles.proposedPill}>Proposed</span>
                      <div className={styles.actionText}>
                        {c.capture.content}
                        {c.capture.urgency && (
                          <span className={`${styles.actionMeta} ${priorityClass(c.capture.urgency)}`}>
                            {c.capture.urgency}
                          </span>
                        )}
                        {c.capture.evidenceQuote && (
                          <span className={styles.actionContext}>
                            {c.capture.evidenceQuote}
                          </span>
                        )}
                      </div>
                      <div className={styles.actionControls}>
                        <button
                          className={styles.btnAccept}
                          onClick={() => onAcceptAction?.(c.capture.id)}
                        >
                          <Check size={12} /> Accept
                        </button>
                        <button
                          className={styles.btnDismiss}
                          onClick={() => onDismissAction?.(c.capture.id)}
                        >
                          <X size={12} /> Dismiss
                        </button>
                      </div>
                    </div>
                  ))}
              </div>
            </>
          )}
        </section>
      )}

      {/* ═══════ ROLE CHANGES ═══════ */}
      {hasRoleChanges && (
        <section className={styles.chapter}>
          <ChapterHeading title="Role Changes" />
          {roleChanges.map((rc) => (
            <div key={rc.id} className={styles.roleChange}>
              <p className={styles.roleHeader}>
                <span className={styles.roleName}>{rc.personName}</span>
                {rc.newStatus && (
                  <>
                    <ArrowRight size={14} className={styles.roleArrow} />
                    {rc.newStatus}
                  </>
                )}
              </p>
              {rc.evidenceQuote && (
                <p className={styles.roleEvidence}>
                  &ldquo;{rc.evidenceQuote}&rdquo;
                </p>
              )}
            </div>
          ))}
        </section>
      )}
    </div>
  );
}

// =============================================================================
// FindingItem — a single finding with dot, title, badge, evidence
// =============================================================================

function FindingItem({
  capture,
  dotClass,
  fieldPath,
  getItemFeedback,
  onItemFeedback,
}: {
  capture: EnrichedCapture;
  dotClass: string;
  fieldPath: string;
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}) {
  const urgencyLower = (capture.urgency ?? "").toLowerCase();
  const badgeClass = captureBadgeClass(capture);
  const evidenceBlockClass =
    urgencyLower === "red"
      ? styles.evidenceBlockRed
      : urgencyLower === "yellow"
      ? styles.evidenceBlockYellow
      : styles.evidenceBlock;

  return (
    <div className={styles.findingItem}>
      <div className={dotClass} />
      <div className={styles.findingContent}>
        <p className={styles.findingTitle}>
          {capture.content}
          {badgeClass && (
            <span className={badgeClass.className}>{badgeClass.label}</span>
          )}
        </p>
        {capture.impact && (
          <p className={styles.findingImpact}>{capture.impact}</p>
        )}
        {capture.evidenceQuote && (
          <div className={evidenceBlockClass}>
            <p className={styles.evidenceText}>&ldquo;{capture.evidenceQuote}&rdquo;</p>
          </div>
        )}
        {capture.speaker && (
          <p className={styles.attribution}>&mdash; {capture.speaker}</p>
        )}
      </div>
      {onItemFeedback && (
        <span className={styles.feedbackSlot}>
          <IntelligenceFeedback
            value={getItemFeedback?.(fieldPath) ?? null}
            onFeedback={(t) => onItemFeedback(fieldPath, t)}
          />
        </span>
      )}
    </div>
  );
}

// =============================================================================
// SpeakerSentimentBlock
// =============================================================================

function SpeakerSentimentBlock({ speaker }: { speaker: SpeakerSentiment }) {
  return (
    <div>
      <div className={styles.speakerHeader}>
        <span className={styles.speakerName}>{speaker.name}</span>
        <span className={sentimentClass(speaker.sentiment)}>
          {speaker.sentiment}
        </span>
      </div>
      {speaker.evidence && (
        <p className={styles.speakerEvidence}>{speaker.evidence}</p>
      )}
    </div>
  );
}

// =============================================================================
// Helpers
// =============================================================================

function sentimentClass(sentiment: string): string {
  const s = sentiment.toLowerCase();
  if (s === "positive" || s === "supportive" || s === "enthusiastic") {
    return styles.sentimentPositive;
  }
  if (s === "cautious" || s === "reserved" || s === "hesitant") {
    return styles.sentimentCautious;
  }
  if (s === "negative" || s === "frustrated" || s === "concerned") {
    return styles.sentimentNegative;
  }
  return styles.sentimentNeutral;
}

function championStatusBadgeClass(status: string): string {
  const s = status.toLowerCase();
  if (s === "strong") return styles.championBadgeStrong;
  if (s === "weak") return styles.championBadgeWeak;
  if (s === "lost") return styles.championBadgeLost;
  return styles.championBadgeNone;
}

function urgencyOrder(urgency?: string): number {
  const u = (urgency ?? "").toLowerCase();
  if (u === "red") return 0;
  if (u === "yellow") return 1;
  if (u === "green_watch") return 2;
  return 3;
}

function riskDotClass(urgency?: string): string {
  const u = (urgency ?? "").toLowerCase();
  if (u === "red") return styles.findingDotTerracotta;
  if (u === "yellow") return styles.findingDotTurmeric;
  return styles.findingDotCharcoal;
}

function captureBadgeClass(capture: EnrichedCapture): { className: string; label: string } | null {
  const type = capture.captureType;
  const urgency = (capture.urgency ?? "").toLowerCase();
  const subType = (capture.subType ?? "").toLowerCase();

  if (type === "win") {
    if (subType.includes("expansion")) return { className: styles.badgeExpansion, label: "Expansion" };
    if (subType.includes("value")) return { className: styles.badgeValue, label: "Value Realized" };
    if (subType) {
      return { className: styles.badgeExpansion, label: formatSubType(subType) };
    }
    return null;
  }
  if (type === "risk") {
    if (urgency === "red") return { className: styles.badgeRed, label: "Red" };
    if (urgency === "yellow") return { className: styles.badgeYellow, label: "Yellow" };
    return null;
  }
  return null;
}

function priorityClass(urgency: string): string {
  const u = urgency.toLowerCase();
  if (u === "red" || u === "p1") return styles.priorityP1;
  return styles.priorityP2;
}

function formatSubType(subType: string): string {
  return subType.replace(/_/g, " ").toLowerCase().replace(/^\w/, (c) => c.toUpperCase());
}
