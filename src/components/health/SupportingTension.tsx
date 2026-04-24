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
import type { CSSProperties } from "react";
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
  { key: "emailEngagement", label: "Engagement" },
  { key: "stakeholderCoverage", label: "Stakeholder Coverage" },
  { key: "keyAdvocateHealth", label: "Champion Health" },
  { key: "financialProximity", label: "Financial Proximity" },
  { key: "signalMomentum", label: "Update Momentum" },
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

/**
 * Short delta string for the Computed score meta line. `"▲ +12"` / `"▼ -8"`
 * / `""` when delta is missing or zero. Renders the arrow that matches
 * the direction rather than the sign of the number.
 */
function formatScoreDelta(delta: number | null | undefined): string {
  if (delta == null || delta === 0) return "";
  const arrow = delta > 0 ? "▲" : "▼";
  const magnitude = Math.abs(Math.round(delta));
  return `${arrow} ${delta > 0 ? "+" : "-"}${magnitude}`;
}

export function SupportingTension({ intelligence, gleanSignals }: SupportingTensionProps) {
  const health = intelligence?.health;
  if (!health) return null;
  const sufficient = health.sufficientData !== false;

  const scoreDisplay = sufficient ? Math.round(health.score).toString() : "--";
  const bandLabel = health.band ? health.band[0].toUpperCase() + health.band.slice(1) : "Unknown";
  const dir = health.trend?.direction;
  const timeframe = health.trend?.timeframe ?? "";
  // DOS-249: `delta` is now a typed field on `IntelligenceHealthTrend`.
  const delta = health.trend?.delta ?? null;
  const deltaStr = formatScoreDelta(delta);
  const rationale =
    health.trend?.rationale && health.trend.rationale.trim().length > 0
      ? health.trend.rationale
      : null;

  // Computed-score meta: "Green · ▲ +12 in 30d · 'improving'"
  const scoreMetaParts: string[] = [];
  if (sufficient) {
    scoreMetaParts.push(bandLabel);
    if (deltaStr) {
      scoreMetaParts.push(timeframe ? `${deltaStr} in ${timeframe}` : deltaStr);
    } else if (timeframe) {
      scoreMetaParts.push(timeframe);
    }
    if (dir) scoreMetaParts.push(`'${dir}'`);
  }
  const scoreMeta = sufficient ? scoreMetaParts.join(" · ") : "Insufficient data";

  // Trend meta: render structured tags when the backend emits them (DOS-249).
  // Format: "Label ▲ · Label · Label ▼" — matches mockup style.
  // Falls back to empty string when no tags are present.
  const tags = health.trend?.tags ?? [];
  const trendMeta =
    tags.length > 0
      ? tags
          .map((t) => {
            const arrow = t.direction === "up" ? " ▲" : t.direction === "down" ? " ▼" : "";
            return `${t.label}${arrow}`;
          })
          .join(" · ")
      : "";

  const dims = health.dimensions;

  return (
    <>
      <div className={styles.supportingTension}>
        <div>
          <div className={styles.tensionBlockLabel}>Computed score</div>
          <div className={`${styles.tensionBlockValue} ${scoreColorClass(health.band)}`}>
            {scoreDisplay}
          </div>
          <div className={styles.tensionBlockMeta}>{scoreMeta}</div>
        </div>
        <div>
          <div className={styles.tensionBlockLabel}>
            {gleanSignals ? "Update trend (from Glean)" : "Signal trend"}
          </div>
          <div className={`${styles.tensionBlockValue} ${trendColorClass(dir)}`}>
            {trendLabel(dir)}
          </div>
          {trendMeta ? (
            <div className={styles.tensionBlockMeta}>{trendMeta}</div>
          ) : null}
        </div>
      </div>

      {/* Tension note: rationale prose lives here only, never duplicated
          into the trend block's meta. When no rationale is emitted, no
          note — don't fabricate an explainer. */}
      {rationale ? <p className={styles.tensionNote}>{rationale}</p> : null}

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
            // Dim-bar fill is data-driven — width is an instance value,
            // not a design-system value. Delivered via a CSS custom
            // property so the actual CSS rule stays in the module
            // (see .dimFill `width: var(--dim-fill)` in health.module.css).
            const fillStyle = { "--dim-fill": `${clamped}%` } as CSSProperties;
            return (
              <div key={key} className={styles.dim}>
                <div className={styles.dimName}>
                  <span>{label}</span>
                  <span className={styles.dimScore}>{raw}</span>
                </div>
                <div className={styles.dimBar}>
                  <div className={`${styles.dimFill} ${fillClass}`} style={fillStyle} />
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
