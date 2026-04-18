/**
 * OutlookPanel — renewal outlook (DOS-203).
 *
 * Three-cell grid: Confidence / Benchmark / Recommended start.
 * Confidence comes from `intelligence.renewalOutlook.confidence` (string
 * label: "low" | "moderate" | "high").  Recommended start from the same
 * block. Peer benchmark is a stub — no schema yet (DOS-204 will wire) — so
 * we render the "not yet captured" UI sentinel per no-new-schema rule.
 */
import type { EntityIntelligence, RenewalOutlook } from "@/types";
import { formatRelativeDate } from "@/lib/utils";
import styles from "./health.module.css";

function confidenceToClass(c?: string): string {
  const v = (c ?? "").toLowerCase();
  if (v === "high") return styles.outlookValueHigh;
  if (v === "low") return styles.outlookValueLow;
  return styles.outlookValueNeutral;
}

function confidenceLabel(c?: string): string {
  const v = (c ?? "").toLowerCase();
  if (!v) return "Unknown";
  return v[0].toUpperCase() + v.slice(1);
}

function riskFactorsSummary(ro: RenewalOutlook): string {
  if (!ro.riskFactors || ro.riskFactors.length === 0) {
    return "No open risk factors surfaced in the current enrichment.";
  }
  const head = ro.riskFactors.slice(0, 3).join("; ");
  return head;
}

interface OutlookPanelProps {
  intelligence: EntityIntelligence | null;
}

export function OutlookPanel({ intelligence }: OutlookPanelProps) {
  const outlook = intelligence?.renewalOutlook;
  if (!outlook) return null;

  const recommendedStartLabel = outlook.recommendedStart
    ? formatRelativeDate(outlook.recommendedStart)
    : "Not scheduled";

  return (
    <div className={styles.outlookGrid}>
      <div>
        <div className={styles.outlookBlockLabel}>Confidence</div>
        <div
          className={`${styles.outlookBlockValue} ${confidenceToClass(outlook.confidence)}`}
        >
          {confidenceLabel(outlook.confidence)}
        </div>
        <div className={styles.outlookBlockDetail}>{riskFactorsSummary(outlook)}</div>
      </div>

      <div>
        <div className={styles.outlookBlockLabel}>Benchmark</div>
        <div className={`${styles.outlookBlockValue}`} style={{ color: "var(--color-text-tertiary)" }}>
          Not yet captured
        </div>
        <div className={styles.outlookBlockDetail}>
          Peer benchmark enrichment ships in a later pass — the cohort comparison
          will populate once the benchmarking pipeline is wired.
        </div>
      </div>

      <div>
        <div className={styles.outlookBlockLabel}>Recommended start</div>
        <div className={`${styles.outlookBlockValue} ${styles.outlookValueNeutral}`}>
          {recommendedStartLabel}
        </div>
        <div className={styles.outlookBlockDetail}>
          {outlook.expansionPotential ?? "Standard renewal runway. Open the commercial conversation by this date."}
        </div>
      </div>
    </div>
  );
}
