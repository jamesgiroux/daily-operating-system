/**
 * OutlookPanel — renewal outlook (DOS-203).
 *
 * Compact 3-cell grid per mockup: Confidence / Benchmark / Recommended start.
 * No prose body — this is a numbers panel, not an essay. The rich editorial
 * outlook renders separately via `AccountOutlook` below this panel.
 *
 * Recommended-start math (derived from `intelligence.contractContext.renewalDate`):
 *   daysUntilRenewal = floor((renewalDate - today) / 1d)
 *   daysUntilStart   = max(0, daysUntilRenewal - 120)     // 120d standard runway
 *   daysUntilRenewal <  0  → "Start immediately" + "Renewal overdue"
 *   daysUntilStart   == 0  → "Start now" + "N days to renewal"
 *   otherwise              → "In Nd" + "M days to renewal"
 *
 * Benchmark cohort is not yet wired (DOS-204 peer benchmarking pipeline).
 * We render "—" until the data exists — never fabricate a number.
 */
import type { EntityIntelligence, RenewalOutlook } from "@/types";
import styles from "./health.module.css";

const RENEWAL_RUNWAY_DAYS = 120;

function confidenceColorClass(c?: string): string {
  const v = (c ?? "").toLowerCase();
  if (v === "high") return styles.outlookValueHigh;
  if (v === "low") return styles.outlookValueLow;
  if (v === "moderate" || v === "medium") return styles.outlookValueNeutral;
  return styles.outlookValueNeutral;
}

function confidenceCell(ro: RenewalOutlook): { label: string; detail: string } {
  const raw = (ro.confidence ?? "").trim();
  if (!raw) return { label: "—", detail: "Confidence not yet captured." };
  const cap = raw[0].toUpperCase() + raw.slice(1);
  const firstRisk = ro.riskFactors?.find((r) => !!r && r.trim().length > 0);
  const detail = firstRisk ? firstRisk.trim() : "No open risk factors surfaced.";
  return { label: cap, detail };
}

function daysBetween(from: Date, to: Date): number {
  const ms = to.getTime() - from.getTime();
  return Math.floor(ms / (1000 * 60 * 60 * 24));
}

function recommendedStartCell(renewalDate?: string): {
  label: string;
  detail: string;
  className: string;
} {
  if (!renewalDate) {
    return {
      label: "—",
      detail: "No renewal date on file.",
      className: styles.outlookValueNeutral,
    };
  }
  const target = new Date(renewalDate);
  if (Number.isNaN(target.getTime())) {
    return {
      label: "—",
      detail: "Renewal date not parseable.",
      className: styles.outlookValueNeutral,
    };
  }
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const daysUntilRenewal = daysBetween(today, target);

  if (daysUntilRenewal < 0) {
    return {
      label: "Start immediately",
      detail: "Renewal overdue — open the commercial conversation today.",
      className: styles.outlookValueLow,
    };
  }

  const daysUntilStart = Math.max(0, daysUntilRenewal - RENEWAL_RUNWAY_DAYS);
  if (daysUntilStart === 0) {
    return {
      label: "Start now",
      detail: `${daysUntilRenewal} day${daysUntilRenewal === 1 ? "" : "s"} to renewal.`,
      className: styles.outlookValueLow,
    };
  }
  return {
    label: `In ${daysUntilStart}d`,
    detail: `${daysUntilRenewal} days to renewal. Runway window opens in ${daysUntilStart} days.`,
    className: styles.outlookValueNeutral,
  };
}

interface OutlookPanelProps {
  intelligence: EntityIntelligence | null;
}

export function OutlookPanel({ intelligence }: OutlookPanelProps) {
  const outlook = intelligence?.renewalOutlook;
  if (!outlook) return null;

  const conf = confidenceCell(outlook);
  const start = recommendedStartCell(intelligence?.contractContext?.renewalDate);

  return (
    <div className={styles.outlookGrid}>
      <div>
        <div className={styles.outlookBlockLabel}>Confidence</div>
        <div className={`${styles.outlookBlockValue} ${confidenceColorClass(outlook.confidence)}`}>
          {conf.label}
        </div>
        <div className={styles.outlookBlockDetail}>{conf.detail}</div>
      </div>

      <div>
        <div className={styles.outlookBlockLabel}>Benchmark</div>
        {/* TODO(DOS-204): peer benchmark cohort comparison — not wired yet. */}
        <div className={`${styles.outlookBlockValue} ${styles.outlookValueNeutral}`}>—</div>
        <div className={styles.outlookBlockDetail}>Peer cohort benchmark not yet captured.</div>
      </div>

      <div>
        <div className={styles.outlookBlockLabel}>Recommended start</div>
        <div className={`${styles.outlookBlockValue} ${start.className}`}>{start.label}</div>
        <div className={styles.outlookBlockDetail}>{start.detail}</div>
      </div>
    </div>
  );
}
