/**
 * SupportingTension — computed score vs signal trend + dimension bars (DOS-203).
 *
 * Three parts per mockup (lines 915-973 of account-health-outlook-globex.html):
 *   1. Two-block row: Computed score vs Signal trend
 *   2. Italic tension note explaining which to trust
 *   3. Six-dimension bars (RelationshipDimensions)
 *
 * Serde note: `intelligence.health` is the ADR-0097 AccountHealth payload —
 * backend serializes via `#[serde(rename_all = "camelCase")]` on
 * IntelligenceJson so the `health` field arrives under the `health` key.
 * No rename needed; if `intelligence.health` is null at runtime the
 * AccountHealth blob failed to deserialize or was absent in health_json.
 */
import type {
  EntityIntelligence,
  DimensionScore,
  HealthOutlookSignals,
  RelationshipDimensions,
} from "@/types";
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

function bandForScore(score: number): "green" | "yellow" | "red" {
  if (score >= 70) return "green";
  if (score >= 40) return "yellow";
  return "red";
}

const DIMENSION_LABELS: Array<{ key: keyof RelationshipDimensions; label: string }> = [
  { key: "meetingCadence", label: "Meeting Cadence" },
  { key: "emailEngagement", label: "Email Engagement" },
  { key: "stakeholderCoverage", label: "Stakeholder Coverage" },
  { key: "championHealth", label: "Champion Health" },
  { key: "financialProximity", label: "Financial Proximity" },
  { key: "signalMomentum", label: "Signal Momentum" },
];

interface SupportingTensionProps {
  intelligence: EntityIntelligence | null;
  /**
   * Glean enrichment payload. Drives the "Signal trend" label swap — when
   * Glean signals are present the trend is Glean-sourced per the mockup
   * (lines 925-929 of `.docs/mockups/account-health-outlook-globex.html`).
   */
  gleanSignals?: HealthOutlookSignals | null;
}

export function SupportingTension({ intelligence, gleanSignals }: SupportingTensionProps) {
  const health = intelligence?.health;
  if (!health) return null;
  const sufficient = health.sufficientData !== false;

  const scoreDisplay = sufficient ? Math.round(health.score).toString() : "--";
  const bandLabel = health.band ? health.band[0].toUpperCase() + health.band.slice(1) : "Unknown";
  const dir = health.trend?.direction;
  const timeframe = health.trend?.timeframe ?? "";
  const rationale =
    (health.trend?.rationale && health.trend.rationale.trim().length > 0
      ? health.trend.rationale
      : null) ?? null;

  const dims = health.dimensions;

  return (
    <>
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
          <div className={styles.tensionBlockLabel}>
            {gleanSignals ? "Update trend (from Glean)" : "Signal trend"}
          </div>
          <div className={`${styles.tensionBlockValue} ${trendColorClass(dir)}`}>
            {trendLabel(dir)}
          </div>
          <div className={styles.tensionBlockMeta}>
            {rationale ? rationale : "Trend derived from dimension momentum."}
          </div>
        </div>
      </div>

      {rationale ? (
        <p className={styles.tensionNote}>{rationale}</p>
      ) : null}

      {dims ? (
        <div className={styles.dimensions}>
          {DIMENSION_LABELS.map(({ key, label }) => {
            const d: DimensionScore | undefined = dims[key];
            if (!d) return null;
            const raw = Math.round(d.score);
            const clamped = Math.max(0, Math.min(100, raw));
            const band = bandForScore(clamped);
            const fillClass =
              band === "green"
                ? styles.dimFillGreen
                : band === "yellow"
                  ? styles.dimFillYellow
                  : styles.dimFillRed;
            const evidence =
              d.evidence && d.evidence.length > 0
                ? d.evidence.filter((e) => !!e && e.trim().length > 0).join(" · ")
                : null;
            return (
              <div key={key} className={styles.dim}>
                <div className={styles.dimName}>
                  <span>{label}</span>
                  <span className={styles.dimScore}>{raw}</span>
                </div>
                <div className={styles.dimBar}>
                  <div className={`${styles.dimFill} ${fillClass}`} style={{ width: `${clamped}%` }} />
                </div>
                {evidence ? <div className={styles.dimEvidence}>{evidence}</div> : null}
              </div>
            );
          })}
        </div>
      ) : null}
    </>
  );
}
