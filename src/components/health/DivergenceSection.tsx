/**
 * DivergenceSection — saffron sub-header + divergence cards (DOS-203).
 *
 * "The story doesn't match the data — usually the highest-signal thing on the page."
 *
 * Pulls from existing `intelligence.consistencyFindings` (deterministic I527
 * contradictions) and from Glean's `channelSentiment.divergenceDetected` flag
 * when present. Returns `null` when empty so the caller can switch to fine state.
 *
 * Contract: every rendered card carries real content. Findings without
 * `claimText` are skipped. A Glean channel-sentiment card only renders when
 * we can build a concrete headline from the readings or a divergenceSummary.
 */
import { useState } from "react";
import type { ConsistencyFinding, HealthOutlookSignals, ChannelSentimentSignal, ChannelReading } from "@/types";
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
import { useEntitySuppressions } from "@/hooks/useEntitySuppressions";
import { TriageCard } from "./TriageCard";
import styles from "./health.module.css";

interface DivergenceSectionProps {
  findings: ConsistencyFinding[];
  gleanSignals: HealthOutlookSignals | null;
  /** DOS-269: account id for DOS-41 confirm-feedback attribution. */
  accountId?: string;
}

/**
 * DOS-249: Map deterministic consistency finding codes to specific human labels.
 * Specific labels match the mockup ("CRM vs reality", "Our pitch vs their authority",
 * "Channel divergence") rather than the generic "Data · major mismatch" fallback.
 */
const FINDING_KIND_LABELS: Record<string, string> = {
  ABSENCE_CONTRADICTION: "CRM vs reality · attendance gap",
  NO_PROGRESS_CONTRADICTION: "CRM vs reality · stale signals",
  AUTHORITY_UNKNOWN: "Our pitch vs their authority",
  CROSS_ENTITY_BLEED: "Data hygiene · entity bleed",
  CHANNEL_DIVERGENCE: "Channel divergence",
  SENTIMENT_MISMATCH: "Sentiment divergence",
  STAKEHOLDER_MISMATCH: "Stakeholder coverage · mismatch",
};

function kindForFinding(f: ConsistencyFinding): string {
  if (f.code && FINDING_KIND_LABELS[f.code]) {
    return FINDING_KIND_LABELS[f.code];
  }
  const severity = f.severity === "high" ? "major" : f.severity === "medium" ? "notable" : "minor";
  return `Data · ${severity} mismatch`;
}

/** A finding is renderable only if it carries real text. No claim → no card. */
function findingHasContent(f: ConsistencyFinding): boolean {
  return !!(f.claimText && f.claimText.trim().length > 0);
}

/** Render a single channel reading compactly: "Meetings say warm". */
function describeChannel(name: string, reading?: ChannelReading | null): string | null {
  if (!reading) return null;
  const sentiment = (reading.sentiment ?? "").trim();
  if (!sentiment) return null;
  return `${name} say ${sentiment.toLowerCase()}`;
}

/**
 * Assemble the Glean channel-sentiment divergence card from raw readings.
 * Prefers structured "Meetings say X, but tickets say Y" phrasing; falls back
 * to `divergenceSummary` when readings are sparse.
 */
function buildChannelCard(
  channel: ChannelSentimentSignal,
): { headline: string; evidence?: string } | null {
  const phrases = [
    describeChannel("Meetings", channel.meetings),
    describeChannel("Tickets", channel.supportTickets),
    describeChannel("Email", channel.email),
    describeChannel("Slack", channel.slack),
  ].filter((v): v is string => !!v);

  if (phrases.length >= 2) {
    const headline = phrases.slice(0, 2).join(", but ") + ".";
    const evidence = [
      channel.meetings?.evidence,
      channel.supportTickets?.evidence,
      channel.email?.evidence,
      channel.slack?.evidence,
    ]
      .map((e) => (e ?? "").trim())
      .filter((e) => e.length > 0)
      .join(" ")
      .trim();
    return { headline, evidence: evidence.length > 0 ? evidence : undefined };
  }

  const fallback = channel.divergenceSummary?.trim();
  if (fallback) return { headline: fallback };
  return null;
}

export function DivergenceSection({ findings, gleanSignals, accountId }: DivergenceSectionProps) {
  const suppressions = useEntitySuppressions(accountId);
  const channel = gleanSignals?.channelSentiment ?? null;
  const channelCard = channel?.divergenceDetected ? buildChannelCard(channel) : null;
  const realFindings = findings.filter(findingHasContent);
  const [hiddenKeys, setHiddenKeys] = useState<Set<string>>(() => new Set());

  const visibleFindings = realFindings
    .map((finding, index) => ({
      finding,
      key: `finding-${index}-${finding.code}`,
    }))
    .filter(
      ({ finding, key }) =>
        !hiddenKeys.has(key) &&
        !suppressions.isSuppressed(`triage:${key}`, finding.claimText),
    );
  const showChannelCard =
    channelCard &&
    !hiddenKeys.has("glean-channel-divergence") &&
    !suppressions.isSuppressed(
      "triage:glean-channel-divergence",
      channelCard.headline,
    );
  if (visibleFindings.length === 0 && !showChannelCard) return null;

  return (
    <>
      {/* Sub-block header inside the "Needs attention" chapter — the
          gutter already carries the chapter title, so the sub-block uses
          the mockup's compact saffron label + italic note rather than a
          competing 28px serif title. */}
      <div className={styles.divergenceHeader}>
        <span className={styles.divergenceLabel}>Divergences · data/narrative mismatches</span>
        <span className={styles.divergenceNote}>
          The story doesn't match the data &mdash; usually the highest-signal thing on the page.
        </span>
      </div>

      <div>
        {showChannelCard && channelCard && (
          <TriageCard
            key="glean-channel-divergence"
            tone="divergence"
            kind="Channel divergence"
            headline={channelCard.headline}
            evidence={channelCard.evidence}
            sources={[{ origin: "glean", label: "Channel sentiment" }]}
            feedbackSlot={
              accountId ? (
                <IntelligenceCorrection
                  entityId={accountId}
                  entityType="account"
                  field="triage:glean-channel-divergence"
                  itemKey={channelCard.headline}
                  onDismissed={async () => {
                    setHiddenKeys((prev) => new Set(prev).add("glean-channel-divergence"));
                    suppressions.markSuppressed(
                      "triage:glean-channel-divergence",
                      channelCard.headline,
                    );
                  }}
                />
              ) : undefined
            }
          />
        )}

        {visibleFindings.map(({ finding: f, key }) => {
          return (
            <TriageCard
              key={key}
              tone="divergence"
              kind={kindForFinding(f)}
              headline={f.claimText}
              evidence={
                f.evidenceText && f.evidenceText.trim().length > 0 ? f.evidenceText : undefined
              }
              sources={[{ origin: "local", label: f.fieldPath || f.code }]}
              feedbackSlot={
                accountId ? (
                  <IntelligenceCorrection
                    entityId={accountId}
                    entityType="account"
                    field={`triage:${key}`}
                    itemKey={f.claimText}
                    onDismissed={async () => {
                      setHiddenKeys((prev) => new Set(prev).add(key));
                      suppressions.markSuppressed(`triage:${key}`, f.claimText);
                    }}
                  />
                ) : undefined
              }
            />
          );
        })}
      </div>
    </>
  );
}

export function hasDivergenceContent(
  findings: ConsistencyFinding[],
  gleanSignals: HealthOutlookSignals | null,
): boolean {
  if (findings.some(findingHasContent)) return true;
  const channel = gleanSignals?.channelSentiment;
  if (channel?.divergenceDetected && buildChannelCard(channel) !== null) return true;
  return false;
}
