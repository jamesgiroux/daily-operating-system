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

export function DivergenceSection({ findings, gleanSignals }: DivergenceSectionProps) {
  const channel = gleanSignals?.channelSentiment;
  const channelDiverges = !!channel?.divergenceDetected;
  if (findings.length === 0 && !channelDiverges) return null;

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
  if (gleanSignals?.channelSentiment?.divergenceDetected) return true;
  return false;
}
