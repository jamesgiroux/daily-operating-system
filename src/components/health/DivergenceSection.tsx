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
import type { ConsistencyFinding, HealthOutlookSignals, ChannelSentimentSignal, ChannelReading } from "@/types";
import { TriageCard } from "./TriageCard";
import styles from "./health.module.css";

interface DivergenceSectionProps {
  findings: ConsistencyFinding[];
  gleanSignals: HealthOutlookSignals | null;
}

function kindForFinding(f: ConsistencyFinding): string {
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

export function DivergenceSection({ findings, gleanSignals }: DivergenceSectionProps) {
  const channel = gleanSignals?.channelSentiment ?? null;
  const channelCard = channel?.divergenceDetected ? buildChannelCard(channel) : null;
  const realFindings = findings.filter(findingHasContent);

  if (realFindings.length === 0 && !channelCard) return null;

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
        {channelCard && (
          <TriageCard
            key="glean-channel-divergence"
            tone="divergence"
            kind="Channel divergence"
            headline={channelCard.headline}
            evidence={channelCard.evidence}
            sources={[{ origin: "glean", label: "Channel sentiment" }]}
          />
        )}

        {realFindings.map((f, i) => (
          <TriageCard
            key={`finding-${i}-${f.code}`}
            tone="divergence"
            kind={kindForFinding(f)}
            headline={f.claimText}
            evidence={
              f.evidenceText && f.evidenceText.trim().length > 0 ? f.evidenceText : undefined
            }
            sources={[{ origin: "local", label: f.fieldPath || f.code }]}
          />
        ))}
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
