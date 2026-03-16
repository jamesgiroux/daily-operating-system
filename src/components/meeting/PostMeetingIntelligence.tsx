import type {
  MeetingPostIntelligence as PostIntelData,
  EnrichedCapture,
} from "@/types";
import { TalkBalanceBar } from "@/components/shared/TalkBalanceBar";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import clsx from "clsx";
import styles from "./PostMeetingIntelligence.module.css";

interface PostMeetingIntelligenceProps {
  data: PostIntelData;
  /** Per-item feedback value getter. Field path like "captures[0].content". */
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  /** Per-item feedback submit. */
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}

export function PostMeetingIntelligence({ data, getItemFeedback, onItemFeedback }: PostMeetingIntelligenceProps) {
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
  const hasRoleChanges = roleChanges.length > 0;
  const hasCompetitorMentions = (interactionDynamics?.competitorMentions?.length ?? 0) > 0;
  const hasEscalation = (interactionDynamics?.escalationLanguage?.length ?? 0) > 0;

  return (
    <div className={styles.container}>
      {/* Engagement Dynamics */}
      {hasEngagement && interactionDynamics && (
        <section>
          <ChapterHeading title="Engagement Dynamics" />
          <div className={styles.dynamicsSection}>
            {/* Talk balance */}
            {interactionDynamics.talkBalanceCustomerPct != null &&
              interactionDynamics.talkBalanceInternalPct != null && (
              <div className={styles.talkBalanceWrap}>
                <TalkBalanceBar
                  customerPct={interactionDynamics.talkBalanceCustomerPct}
                  internalPct={interactionDynamics.talkBalanceInternalPct}
                />
              </div>
            )}

            {/* Speaker sentiments */}
            {interactionDynamics.speakerSentiments.length > 0 && (
              <>
                <p className={styles.sectionSubheading}>Speaker Sentiment</p>
                <div className={styles.speakerList}>
                  {interactionDynamics.speakerSentiments.map((speaker, i) => (
                    <div key={i} className={styles.speakerCard}>
                      <span className={styles.speakerName}>{speaker.name}</span>
                      <span className={sentimentBadgeClass(speaker.sentiment)}>
                        {speaker.sentiment}
                      </span>
                      {speaker.evidence && (
                        <p className={styles.speakerEvidence}>{speaker.evidence}</p>
                      )}
                    </div>
                  ))}
                </div>
              </>
            )}

            {/* Engagement signal strip */}
            <div className={styles.signalStrip}>
              {interactionDynamics.questionDensity && (
                <span className={styles.signalBadgeActive}>
                  Questions: {interactionDynamics.questionDensity}
                </span>
              )}
              {interactionDynamics.decisionMakerActive && (
                <span className={styles.signalBadgeActive}>
                  Decision-maker: {interactionDynamics.decisionMakerActive}
                </span>
              )}
              {interactionDynamics.forwardLooking && (
                <span className={styles.signalBadgeActive}>
                  Forward-looking: {interactionDynamics.forwardLooking}
                </span>
              )}
              {interactionDynamics.monologueRisk && (
                <span className={styles.signalBadgeWarning}>
                  Monologue risk
                </span>
              )}
            </div>

            {/* Competitor mentions */}
            {hasCompetitorMentions && (
              <>
                <p className={styles.sectionSubheading}>Competitor Mentions</p>
                <div className={styles.mentionList}>
                  {interactionDynamics.competitorMentions.map((m, i) => (
                    <div key={i} className={styles.mentionItem}>
                      <span className={styles.mentionLabel}>{m.competitor}</span>
                      <span>{m.context}</span>
                    </div>
                  ))}
                </div>
              </>
            )}

            {/* Escalation language */}
            {hasEscalation && (
              <>
                <p className={styles.sectionSubheading}>Escalation Language</p>
                <div className={styles.mentionList}>
                  {interactionDynamics.escalationLanguage.map((e, i) => (
                    <div key={i} className={styles.escalationItem}>
                      <span className={styles.escalationQuote}>{e.quote}</span>
                      <span className={styles.escalationSpeaker}>{e.speaker}</span>
                    </div>
                  ))}
                </div>
              </>
            )}
          </div>
        </section>
      )}

      {/* Champion Health */}
      {hasChampion && championHealth && (
        <section>
          <ChapterHeading title="Champion Health" />
          <div className={styles.championCard}>
            <div className={styles.championHeader}>
              {championHealth.championName && (
                <span className={styles.championName}>{championHealth.championName}</span>
              )}
              <span className={championStatusClass(championHealth.championStatus)}>
                {championHealth.championStatus}
              </span>
            </div>
            {championHealth.championEvidence && (
              <p className={styles.championEvidence}>{championHealth.championEvidence}</p>
            )}
            {championHealth.championRisk && (
              <p className={styles.championRisk}>{championHealth.championRisk}</p>
            )}
          </div>
        </section>
      )}

      {/* Categorized Outcomes — Wins */}
      {hasWins && (
        <section>
          <ChapterHeading title="Wins" />
          <CaptureGroup captures={wins} type="win" getItemFeedback={getItemFeedback} onItemFeedback={onItemFeedback} />
        </section>
      )}

      {/* Categorized Outcomes — Risks */}
      {hasRisks && (
        <section>
          <ChapterHeading title="Risks" />
          <CaptureGroup captures={risks} type="risk" getItemFeedback={getItemFeedback} onItemFeedback={onItemFeedback} />
        </section>
      )}

      {/* Categorized Outcomes — Decisions */}
      {hasDecisions && (
        <section>
          <ChapterHeading title="Decisions" />
          <CaptureGroup captures={decisions} type="decision" getItemFeedback={getItemFeedback} onItemFeedback={onItemFeedback} />
        </section>
      )}

      {/* Categorized Outcomes — Commitments */}
      {hasCommitments && (
        <section>
          <ChapterHeading title="Commitments" />
          <CaptureGroup captures={commitments} type="commitment" getItemFeedback={getItemFeedback} onItemFeedback={onItemFeedback} />
        </section>
      )}

      {/* Role Changes */}
      {hasRoleChanges && (
        <section>
          <ChapterHeading title="Role Changes" />
          <div className={styles.roleChangeList}>
            {roleChanges.map((rc) => (
              <div key={rc.id} className={styles.roleChangeItem}>
                <div className={styles.roleChangeHeader}>
                  <span className={styles.roleChangeName}>{rc.personName}</span>
                  {rc.oldStatus && (
                    <span className={styles.roleChangeStatus}>{rc.oldStatus}</span>
                  )}
                  {rc.oldStatus && rc.newStatus && (
                    <span className={styles.roleChangeArrow}>&rarr;</span>
                  )}
                  {rc.newStatus && (
                    <span className={styles.roleChangeStatus}>{rc.newStatus}</span>
                  )}
                </div>
                {rc.evidenceQuote && (
                  <p className={styles.roleChangeEvidence}>{rc.evidenceQuote}</p>
                )}
              </div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

// =============================================================================
// CaptureGroup — renders a list of enriched captures with badges
// =============================================================================

interface IndexedCapture {
  capture: EnrichedCapture;
  originalIndex: number;
}

function CaptureGroup({
  captures,
  type,
  getItemFeedback,
  onItemFeedback,
}: {
  captures: IndexedCapture[];
  type: "win" | "risk" | "decision" | "commitment";
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}) {
  // Group wins by sub_type
  if (type === "win") {
    const groups = new Map<string, IndexedCapture[]>();
    for (const c of captures) {
      const key = c.capture.subType || "general";
      const group = groups.get(key) ?? [];
      group.push(c);
      groups.set(key, group);
    }

    return (
      <div className={styles.captureList}>
        {Array.from(groups.entries()).map(([subType, items]) => (
          <div key={subType} className={styles.outcomeGroup}>
            {subType !== "general" && (
              <p className={styles.outcomeGroupTitle}>
                <span className={styles.subTypeBadge}>{formatSubType(subType)}</span>
                <span className={styles.outcomeGroupCount}>({items.length})</span>
              </p>
            )}
            {items.map((c) => (
              <CaptureItem
                key={c.capture.id}
                capture={c.capture}
                type={type}
                fieldPath={`captures[${c.originalIndex}].content`}
                getItemFeedback={getItemFeedback}
                onItemFeedback={onItemFeedback}
              />
            ))}
          </div>
        ))}
      </div>
    );
  }

  return (
    <div className={styles.captureList}>
      {captures.map((c) => (
        <CaptureItem
          key={c.capture.id}
          capture={c.capture}
          type={type}
          fieldPath={`captures[${c.originalIndex}].content`}
          getItemFeedback={getItemFeedback}
          onItemFeedback={onItemFeedback}
        />
      ))}
    </div>
  );
}

function CaptureItem({
  capture,
  type,
  fieldPath,
  getItemFeedback,
  onItemFeedback,
}: {
  capture: EnrichedCapture;
  type: "win" | "risk" | "decision" | "commitment";
  fieldPath: string;
  getItemFeedback?: (fieldPath: string) => "positive" | "negative" | null;
  onItemFeedback?: (fieldPath: string, type: "positive" | "negative") => void;
}) {
  const urgencyLower = (capture.urgency ?? "").toLowerCase();
  const riskAccentClass =
    type === "risk"
      ? urgencyLower === "red"
        ? styles.captureItemRiskRed
        : urgencyLower === "yellow"
        ? styles.captureItemRiskYellow
        : urgencyLower === "green_watch"
        ? styles.captureItemRiskGreen
        : undefined
      : undefined;

  const hasFeedback = !!onItemFeedback;

  return (
    <div className={clsx(styles.captureItem, riskAccentClass)}>
      <div className={styles.captureHeader}>
        {type === "risk" && capture.urgency && (
          <span className={urgencyBadgeClass(capture.urgency)}>
            {formatUrgency(capture.urgency)}
          </span>
        )}
        {type === "win" && capture.impact && (
          <span className={styles.impactBadge}>{capture.impact}</span>
        )}
        {type === "commitment" && capture.subType && (
          <span className={styles.subTypeBadge}>{capture.subType}</span>
        )}
        {capture.speaker && (
          <span className={styles.captureSpeaker}>{capture.speaker}</span>
        )}
        {hasFeedback && (
          <span className={styles.captureActions}>
            <IntelligenceFeedback
              value={getItemFeedback?.(fieldPath) ?? null}
              onFeedback={(t) => onItemFeedback(fieldPath, t)}
            />
          </span>
        )}
      </div>
      <p className={styles.captureContent}>{capture.content}</p>
      {capture.evidenceQuote && (
        <p className={styles.captureEvidence}>{capture.evidenceQuote}</p>
      )}
    </div>
  );
}

// =============================================================================
// Helpers
// =============================================================================

function sentimentBadgeClass(sentiment: string): string {
  const normalized = sentiment.toLowerCase();
  if (normalized === "positive" || normalized === "supportive" || normalized === "enthusiastic") {
    return styles.sentimentPositive;
  }
  if (normalized === "cautious" || normalized === "reserved" || normalized === "hesitant") {
    return styles.sentimentCautious;
  }
  if (normalized === "negative" || normalized === "frustrated" || normalized === "concerned") {
    return styles.sentimentNegative;
  }
  return styles.sentimentNeutral;
}

function championStatusClass(status: string): string {
  const normalized = status.toLowerCase();
  if (normalized === "strong") return styles.championStrong;
  if (normalized === "weak") return styles.championWeak;
  if (normalized === "lost") return styles.championLost;
  return styles.championNone;
}

function urgencyBadgeClass(urgency: string): string {
  const u = urgency.toLowerCase();
  if (u === "red") return styles.urgencyRed;
  if (u === "yellow") return styles.urgencyYellow;
  if (u === "green_watch") return styles.urgencyGreen;
  return styles.subTypeBadge;
}

function urgencyOrder(urgency?: string): number {
  const u = (urgency ?? "").toLowerCase();
  if (u === "red") return 0;
  if (u === "yellow") return 1;
  if (u === "green_watch") return 2;
  return 3;
}

function formatUrgency(urgency: string): string {
  const u = urgency.toLowerCase();
  if (u === "green_watch") return "Watch";
  return u.charAt(0).toUpperCase() + u.slice(1);
}

function formatSubType(subType: string): string {
  return subType.replace(/_/g, " ").toLowerCase().replace(/^\w/, (c) => c.toUpperCase());
}
