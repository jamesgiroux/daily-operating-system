/**
 * DivergenceSection — saffron sub-header + divergence cards (DOS-203).
 *
 * "The story doesn't match the data — usually the highest-signal thing on the page."
 *
 * Pulls from existing `intelligence.consistencyFindings` (deterministic I527
 * contradictions) and from Glean's `channelSentiment.divergenceDetected` flag
 * when present. Returns `null` when empty so the caller can switch to fine state.
 */
import type { ConsistencyFinding, HealthOutlookSignals } from "@/types";
import { TriageCard } from "./TriageCard";
import styles from "./health.module.css";

interface DivergenceSectionProps {
  findings: ConsistencyFinding[];
  gleanSignals: HealthOutlookSignals | null;
}

function kindForFinding(f: ConsistencyFinding): string {
  const severity = f.severity === "high" ? "major" : f.severity === "medium" ? "notable" : "minor";
  return `Data · ${severity} mismatch · ${f.code}`;
}

/** DOS-232 Codex fix: detect individual channels carrying a negative or
 * cooling signal even when the cross-channel `divergenceDetected` flag is
 * false. A single channel reporting negative sentiment is still a
 * divergence-worthy data point the Health chapter must not bury. */
function negativeChannels(
  channel: NonNullable<HealthOutlookSignals["channelSentiment"]>,
): { label: string; reading: { sentiment?: string | null; trend30d?: "warming" | "stable" | "cooling" | null; evidence?: string | null } }[] {
  const entries: { label: string; reading: { sentiment?: string | null; trend30d?: "warming" | "stable" | "cooling" | null; evidence?: string | null } }[] = [];
  const push = (
    label: string,
    reading: { sentiment?: string | null; trend30d?: "warming" | "stable" | "cooling" | null; evidence?: string | null } | null | undefined,
  ) => {
    if (!reading) return;
    const sent = (reading.sentiment ?? "").toLowerCase();
    const trend = reading.trend30d;
    if (sent === "negative" || sent === "mixed" || trend === "cooling") {
      entries.push({ label, reading });
    }
  };
  push("Email", channel.email);
  push("Meetings", channel.meetings);
  push("Support tickets", channel.supportTickets);
  push("Slack", channel.slack);
  return entries;
}

export function DivergenceSection({ findings, gleanSignals }: DivergenceSectionProps) {
  const channel = gleanSignals?.channelSentiment;
  const channelDiverges = !!channel?.divergenceDetected;
  const negChannels = channel ? negativeChannels(channel) : [];
  if (findings.length === 0 && !channelDiverges && negChannels.length === 0) return null;

  return (
    <>
      <div className={styles.divergenceHeader}>
        <span className={styles.divergenceLabel}>Divergences · data/narrative mismatches</span>
        <span className={styles.divergenceNote}>
          The story doesn't match the data — usually the highest-signal thing on the page.
        </span>
      </div>

      <div>
        {channelDiverges && (
          <TriageCard
            key="glean-channel-divergence"
            tone="divergence"
            kind="Channel divergence"
            headline={
              channel?.divergenceSummary ??
              "Tone disagrees across channels — synchronous conversations and asynchronous signals point different directions."
            }
            evidence={
              [channel?.meetings?.evidence, channel?.supportTickets?.evidence, channel?.email?.evidence]
                .filter(Boolean)
                .join(" ")
                .trim() || undefined
            }
            sources={[{ origin: "glean", label: "Channel sentiment" }]}
          />
        )}

        {/* DOS-232: Per-channel negative/cooling readings — only render when
            the top-level divergenceDetected flag is false. If it's true, the
            cross-channel card already summarizes the story. */}
        {!channelDiverges &&
          negChannels.map((c, i) => {
            const trend = c.reading.trend30d;
            const sent = c.reading.sentiment ?? "";
            const headline = trend === "cooling"
              ? `${c.label} sentiment is cooling.`
              : `${c.label} sentiment is ${sent || "negative"}.`;
            return (
              <TriageCard
                key={`channel-${i}-${c.label}`}
                tone="divergence"
                kind={`Channel · ${c.label.toLowerCase()}`}
                headline={headline}
                evidence={c.reading.evidence ?? undefined}
                sources={[{ origin: "glean", label: `Channel sentiment · ${c.label.toLowerCase()}` }]}
              />
            );
          })}

        {findings.map((f, i) => (
          <TriageCard
            key={`finding-${i}-${f.code}`}
            tone="divergence"
            kind={kindForFinding(f)}
            headline={f.claimText}
            evidence={f.evidenceText}
            sources={[{ origin: "local", label: f.fieldPath }]}
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
  if (findings.length > 0) return true;
  const channel = gleanSignals?.channelSentiment;
  if (channel?.divergenceDetected) return true;
  // DOS-232: per-channel negative/cooling readings should fire the
  // divergence chapter even without the cross-channel flag.
  if (channel && negativeChannels(channel).length > 0) return true;
  return false;
}
