/**
 * SupportingTension — computed score vs signal trend (DOS-203).
 *
 * Two-block row surfaces the tension between the *lagging* computed score
 * (mean of dimensions) and the *leading* signal trend direction. When they
 * agree (steady), the row reads as confirmation; when they disagree, the
 * note below explains which to trust (per the mockup).
 */
import type { EntityIntelligence } from "@/types";
import styles from "./health.module.css";

function scoreColorClass(band?: string): string {
  const v = (band ?? "").toLowerCase();
  if (v === "green") return styles.tensionValueGreen;
  if (v === "yellow") return styles.tensionValueYellow;
  if (v === "red") return styles.tensionValueRed;
  return styles.tensionValueNeutral;
}

function trendColorClass(direction?: string): string {
  const v = (direction ?? "").toLowerCase();
  if (v === "improving") return styles.tensionValueGreen;
  if (v === "declining") return styles.tensionValueRed;
  if (v === "volatile") return styles.tensionValueYellow;
  return styles.tensionValueNeutral;
}

function trendLabel(direction?: string): string {
  const v = (direction ?? "").toLowerCase();
  if (v === "improving") return "Strengthening";
  if (v === "declining") return "Worsening";
  if (v === "volatile") return "Volatile";
  if (v === "stable") return "Steady";
  return "Unknown";
}

interface SupportingTensionProps {
  intelligence: EntityIntelligence | null;
}

export function SupportingTension({ intelligence }: SupportingTensionProps) {
  const health = intelligence?.health;
  if (!health) return null;
  const sufficient = health.sufficientData !== false;

  const scoreDisplay = sufficient ? Math.round(health.score).toString() : "--";
  const bandLabel = health.band ? health.band[0].toUpperCase() + health.band.slice(1) : "Unknown";
  const dir = health.trend?.direction;
  const timeframe = health.trend?.timeframe ?? "";

  return (
    <div className={styles.supportingTension}>
      <div>
        <div className={styles.tensionBlockLabel}>Computed score</div>
        <div className={`${styles.tensionBlockValue} ${scoreColorClass(health.band)}`}>
          {scoreDisplay}
        </div>
        <div className={styles.tensionBlockMeta}>
          {sufficient
            ? `${bandLabel} · ${dir ?? "stable"}${timeframe ? ` · ${timeframe}` : ""}`
            : "Insufficient data"}
        </div>
      </div>
      <div>
        <div className={styles.tensionBlockLabel}>Signal trend</div>
        <div className={`${styles.tensionBlockValue} ${trendColorClass(dir)}`}>
          {trendLabel(dir)}
        </div>
        <div className={styles.tensionBlockMeta}>
          {health.trend?.rationale ?? "Trend derived from dimension momentum."}
        </div>
      </div>
    </div>
  );
}
