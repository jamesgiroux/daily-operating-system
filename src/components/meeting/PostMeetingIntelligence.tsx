import type {
  MeetingPostIntelligence as PostIntelData,
  ContinuityThread,
  DbAction,
  EnrichedCapture,
  PredictionResult,
  PredictionScorecard,
  SpeakerSentiment,
} from "@/types";
import {
  ArrowRight,
  Check,
  Circle,
  CircleDot,
  UserPlus,
  X,
  Zap,
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
  continuityThread?: ContinuityThread | null;
  predictionScorecard?: PredictionScorecard | null;
  /** Flat outcomes summary (executive summary text) */
  summary?: string;
  /** Extracted actions from transcript processing */
  actions?: DbAction[];
  /** Per-item feedback value getter. Field path like "captures[0].content". */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
  /** Accept a suggested action (moves to pending) */
  onAcceptAction?: (actionId: string) => void;
  /** Dismiss a suggested action */
  onDismissAction?: (actionId: string) => void;
  /** Complete/reopen an accepted action */
  onToggleAction?: (actionId: string) => void;
  /** Cycle action priority P1→P2→P3→P1 */
  onCyclePriority?: (actionId: string) => void;
}

// =============================================================================
// Component
// =============================================================================

export function PostMeetingIntelligence({
  data,
  continuityThread,
  predictionScorecard,
  summary,
  actions = [],
  getItemFeedback,
  onItemFeedback,
  onAcceptAction,
  onDismissAction,
  onToggleAction: _onToggleAction,
  onCyclePriority: _onCyclePriority,
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
  const hasThread = !!continuityThread;
  const hasPredictions = !!predictionScorecard?.hasData;

  return (
    <div className={styles.container}>
      {/* ═══════ EXECUTIVE SUMMARY ═══════ */}
      {summary && (
        <div className={styles.summaryBlock}>
          <p className={styles.summaryText}>{summary}</p>
        </div>
      )}

      {/* ═══════ THE THREAD ═══════ */}
      {hasThread && continuityThread && (
        <section className={styles.chapter}>
          <ChapterHeading title="The Thread" />
          <p className={styles.threadIntro}>
            {buildThreadIntro(continuityThread)}
          </p>
          {!continuityThread.isFirstMeeting && (
            <ul className={styles.threadList}>
              {continuityThread.actionsCompleted.map((action, index) => (
                <li key={`completed-${action.title}-${index}`} className={styles.threadItem}>
                  <Check size={14} className={styles.threadIconConfirmed} />
                  <span>
                    {action.title}
                    <span className={styles.threadDetail}>
                      closed since the last meeting
                    </span>
                  </span>
                </li>
              ))}

              {continuityThread.actionsOpen.map((action, index) => (
                <li key={`open-${action.title}-${index}`} className={styles.threadItem}>
                  <Circle size={14} className={styles.threadIconOpen} />
                  <span>
                    {action.title}
                    {action.date && (
                      <span className={styles.threadDetail}>
                        due {formatShortDate(action.date)}
                        {action.isOverdue ? " · overdue" : ""}
                      </span>
                    )}
                    {!action.date && action.isOverdue && (
                      <span className={styles.threadDetail}>overdue</span>
                    )}
                  </span>
                </li>
              ))}

              {continuityThread.healthDelta && (
                <li className={styles.threadItem}>
                  <span className={styles.threadIconNeutral}>
                    <CircleDot size={12} />
                  </span>
                  <span>
                    Health moved from {continuityThread.healthDelta.previous} to{" "}
                    <span
                      className={
                        continuityThread.healthDelta.current > continuityThread.healthDelta.previous
                          ? styles.threadDeltaUp
                          : undefined
                      }
                    >
                      {continuityThread.healthDelta.current}
                    </span>
                  </span>
                </li>
              )}

              {continuityThread.newAttendees.map((attendee, index) => (
                <li key={`attendee-${attendee}-${index}`} className={styles.threadItem}>
                  <UserPlus size={14} className={styles.threadIconNewFace} />
                  <span>
                    {attendee}
                    <span className={styles.threadDetail}>new attendee</span>
                  </span>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}

      {/* ═══════ WHAT WE PREDICTED VS WHAT HAPPENED ═══════ */}
      {hasPredictions && predictionScorecard && (
        <section className={styles.chapter}>
          <ChapterHeading title="What We Predicted vs What Happened" />

          {predictionScorecard.riskPredictions.length > 0 && (
            <div className={styles.predictionGroup}>
              <p className={styles.monoLabelTerracotta}>Risks</p>
              {predictionScorecard.riskPredictions.map((prediction, index) => (
                <PredictionItem
                  key={`risk-${prediction.text}-${index}`}
                  prediction={prediction}
                />
              ))}
            </div>
          )}

          {predictionScorecard.winPredictions.length > 0 && (
            <div className={styles.predictionGroup}>
              <p className={styles.monoLabelSage}>Wins</p>
              {predictionScorecard.winPredictions.map((prediction, index) => (
                <PredictionItem
                  key={`win-${prediction.text}-${index}`}
                  prediction={prediction}
                />
              ))}
            </div>
          )}
        </section>
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

      {/* ═══════ COMMITMENTS & ACTIONS ═══════ */}
      {(hasCommitments || actions.length > 0) && (
        <section className={styles.chapter}>
          <ChapterHeading title="Commitments & Actions" />

          {/* Commitments — only shown when no extracted actions exist (avoids duplication) */}
          {hasCommitments && actions.length === 0 && (
            <div className={styles.commitmentsList}>
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
          )}

          {/* Actions — each shows its lifecycle status. Archived/dismissed are hidden. */}
          {actions.filter(a => a.status !== "archived" && a.status !== "cancelled").length > 0 && (
            <div className={styles.actionsList}>
              {actions.filter(a => a.status !== "archived" && a.status !== "cancelled").map((action) => (
                <div
                  key={action.id}
                  className={
                    action.status === "suggested"
                      ? styles.actionItemSuggested
                      : action.status === "completed"
                        ? styles.actionItemCompleted
                        : styles.actionItemPending
                  }
                >
                  {/* Status indicator */}
                  {action.status === "suggested" && (
                    <span className={styles.suggestedPill}>Suggested</span>
                  )}
                  {action.status === "completed" && (
                    <Check size={14} className={styles.completedIcon} />
                  )}
                  {action.status !== "suggested" && action.status !== "completed" && (
                    <span className={styles.pendingPill}>Pending</span>
                  )}

                  {/* Action content */}
                  <div className={styles.actionText}>
                    <span className={action.status === "completed" ? styles.actionTitleCompleted : undefined}>
                      {action.title}
                    </span>
                    <span className={`${styles.actionMeta} ${action.priority === "P1" ? styles.priorityP1 : styles.priorityP2}`}>
                      {action.priority}
                      {action.dueDate && <> &middot; due {action.dueDate}</>}
                    </span>
                    {action.context && (
                      <span className={styles.actionContext}>{action.context}</span>
                    )}
                  </div>

                  {/* Accept/Dismiss for suggested actions */}
                  {action.status === "suggested" && (
                    <div className={styles.actionControls}>
                      <button
                        className={styles.btnAccept}
                        onClick={() => onAcceptAction?.(action.id)}
                      >
                        <Check size={12} /> Accept
                      </button>
                      <button
                        className={styles.btnDismiss}
                        onClick={() => onDismissAction?.(action.id)}
                      >
                        <X size={12} /> Dismiss
                      </button>
                    </div>
                  )}

                  {/* Complete toggle for pending/waiting/active actions */}
                  {action.status !== "suggested" && action.status !== "completed" && (
                    <div className={styles.actionControls}>
                      <button
                        className={styles.btnComplete}
                        onClick={() => _onToggleAction?.(action.id)}
                      >
                        <Check size={12} /> Done
                      </button>
                    </div>
                  )}

                  {/* Reopen for completed actions */}
                  {action.status === "completed" && (
                    <div className={styles.actionControls}>
                      <button
                        className={styles.btnReopen}
                        onClick={() => _onToggleAction?.(action.id)}
                      >
                        Reopen
                      </button>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </section>
      )}

      {/* ═══════ ROLE CHANGES ═══════ */}
      {hasRoleChanges && (
        <section className={styles.chapter}>
          <ChapterHeading title="Role Changes" />
          {roleChanges.map((rc) => (
            <div key={rc.id} className={styles.roleChange}>
              <div className={styles.roleHeader}>
                <span className={styles.roleName}>{rc.personName}</span>
                {(rc.oldStatus || rc.newStatus) && (
                  <span className={styles.roleTransition}>
                    {rc.oldStatus && (
                      <span className={styles.roleStatus}>{rc.oldStatus}</span>
                    )}
                    <ArrowRight size={12} className={styles.roleArrow} />
                    {rc.newStatus && (
                      <span className={styles.roleStatus}>{rc.newStatus}</span>
                    )}
                  </span>
                )}
              </div>
              {rc.evidenceQuote && (
                <div className={styles.roleEvidenceBlock}>
                  <p className={styles.roleEvidence}>
                    &ldquo;{rc.evidenceQuote}&rdquo;
                  </p>
                </div>
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

function PredictionItem({ prediction }: { prediction: PredictionResult }) {
  const itemClass =
    prediction.category === "confirmed"
      ? styles.predictionItemConfirmed
      : prediction.category === "surprise"
      ? styles.predictionItemSurprise
      : styles.predictionItemNotRaised;
  const iconClass =
    prediction.category === "confirmed"
      ? styles.predictionIconConfirmed
      : prediction.category === "surprise"
      ? styles.predictionIconSurprise
      : styles.predictionIconNotRaised;
  const IconComponent =
    prediction.category === "confirmed"
      ? Check
      : prediction.category === "surprise"
      ? Zap
      : X;

  return (
    <div className={itemClass}>
      <span className={iconClass}><IconComponent size={14} /></span>
      <div>
        <p>{prediction.text}</p>
        {(prediction.matchText || prediction.source) && (
          <p className={styles.predictionMatch}>
            {prediction.matchText ?? prediction.source}
          </p>
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Helpers
// =============================================================================

function buildThreadIntro(thread: ContinuityThread): string {
  if (thread.isFirstMeeting) {
    return thread.entityName
      ? `This is the first recorded meeting with ${thread.entityName}.`
      : "This is the first recorded meeting in the thread.";
  }

  const meetingLabel = thread.previousMeetingTitle ?? "the previous meeting";
  if (thread.previousMeetingDate) {
    return `Since ${meetingLabel} on ${formatShortDate(thread.previousMeetingDate)}, here’s what changed.`;
  }
  return `Since ${meetingLabel}, here’s what changed.`;
}

function formatShortDate(date: string): string {
  const parsed = new Date(date);
  if (Number.isNaN(parsed.getTime())) {
    return date;
  }
  return parsed.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

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
  if (type === "decision") {
    if (subType.includes("joint_agreement")) return { className: styles.badgeDecisionJoint, label: "Joint Agreement" };
    if (subType.includes("customer_commitment")) return { className: styles.badgeDecisionCustomer, label: "Customer Commitment" };
    if (subType.includes("internal_decision")) return { className: styles.badgeDecisionInternal, label: "Internal" };
    return null;
  }
  return null;
}

function formatSubType(subType: string): string {
  return subType.replace(/_/g, " ").toLowerCase().replace(/^\w/, (c) => c.toUpperCase());
}
