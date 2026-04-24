/**
 * OutlookPanel — renewal outlook (DOS-203).
 *
 * Compact 3-cell grid per mockup: Confidence / Benchmark / Recommended start.
 * Matches `.docs/mockups/account-health-outlook-globex.html` lines 888-913.
 *
 * Data sources (all from `intelligence.agreementOutlook` + contractContext):
 *   - Confidence cell: `outlook.confidence` ("high"/"moderate"/"low"), detail
 *     summarises up to three `outlook.riskFactors`.
 *   - Benchmark cell: peer-cohort renewal rate — NOT wired yet. Cell is
 *     suppressed until the backend field lands (DOS-204); the grid collapses
 *     to 2-col via the `.outlookGridTwoCol` modifier class (no inline styles).
 *   - Recommended start cell: prefers the AI-emitted
 *     `outlook.recommendedStart` (e.g. "2026-08-01" → "Aug 1"); falls back
 *     to `renewalDate - 120d` arithmetic only when the AI didn't emit one.
 *
 * A turmeric-rimmed pull-quote below the grid renders `outlook.expansionPotential`
 * when present — this is the renewal narrative field from the AI and matches
 * the mockup's editorial block that sits under the grid. When neither
 * `expansionPotential` nor `confidence` is populated, the entire panel
 * returns null rather than showing a dead frame.
 */
import type { EntityIntelligence, AgreementOutlook } from "@/types";
import styles from "./health.module.css";

const RENEWAL_RUNWAY_DAYS = 120;

/**
 * Compute the renewal "call" — the verdict rendered as the chapter title.
 * The title is the judgment, not a label: what are we saying about this
 * account's next commercial moment?
 *
 *   confidence "low"                              → "Churn risk"
 *   confidence "high" + substantive expansion     → "Expansion"
 *   everything else (incl. missing, moderate)     → "Renewal"
 *
 * "Substantive expansion" = the AI emitted a non-empty narrative >80 chars
 * that isn't just "none identified" / "no expansion surfaced". At that
 * threshold the signal is stable enough to name the call "Expansion"
 * rather than default "Renewal".
 */
export function renewalCallVerdict(outlook: AgreementOutlook | null | undefined): string {
  const conf = (outlook?.confidence ?? "").toLowerCase();
  if (conf === "low") return "Churn risk";
  const expansion = (outlook?.expansionPotential ?? "").trim();
  const expansionLooksNegative = /^(none|no\b|not\s)/i.test(expansion);
  if (
    conf === "high" &&
    expansion.length > 80 &&
    !expansionLooksNegative
  ) {
    return "Expansion";
  }
  return "Renewal";
}

function confidenceColorClass(c?: string): string {
  const v = (c ?? "").toLowerCase();
  if (v === "high") return styles.outlookValueHigh;
  if (v === "low") return styles.outlookValueLow;
  if (v === "moderate" || v === "medium") return styles.outlookValueNeutral;
  return styles.outlookValueNeutral;
}

/**
 * Build the Confidence cell detail — summarises up to three risk factors
 * inline. Per the mockup: "Moderate. Risk factors: X, Y, Z." One-line-ish
 * synthesis rather than dumping the first riskFactor in full.
 */
function confidenceCell(ro: AgreementOutlook): { label: string; detail: string } {
  const raw = (ro.confidence ?? "").trim();
  if (!raw) return { label: "—", detail: "Confidence not yet captured." };
  const cap = raw[0].toUpperCase() + raw.slice(1);
  const risks = (ro.riskFactors ?? [])
    .map((r) => r.trim())
    .filter((r) => r.length > 0);
  if (risks.length === 0) {
    return { label: cap, detail: `${cap}. No open risk factors surfaced.` };
  }
  // Compress each risk to its first clause (up to the first `—` or `:`) so
  // the joined summary stays readable. Full prose already lives in the
  // pull-quote below.
  const summary = risks
    .slice(0, 3)
    .map((r) => {
      const headClause = r.split(/[—:]/)[0].trim();
      return headClause.replace(/\.$/, "");
    })
    .filter((s) => s.length > 0)
    .join("; ");
  const moreCount = risks.length - Math.min(3, risks.length);
  const trailer = moreCount > 0 ? ` (+${moreCount} more)` : "";
  return {
    label: cap,
    detail: `${cap}. Risk factors: ${summary}${trailer}.`,
  };
}

function daysBetween(from: Date, to: Date): number {
  const ms = to.getTime() - from.getTime();
  return Math.floor(ms / (1000 * 60 * 60 * 24));
}

function formatShortMonthDay(iso: string): string | null {
  try {
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return null;
    return d.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  } catch {
    return null;
  }
}

/**
 * Recommended start cell — prefers the AI-emitted `recommendedStart` date
 * when present. Falls back to the renewal-date math.
 */
function recommendedStartCell(
  aiRecommendedStart: string | undefined,
  renewalDate?: string,
): { label: string; detail: string; className: string } {
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  // AI-emitted start wins when we can parse it.
  if (aiRecommendedStart) {
    const ai = new Date(aiRecommendedStart);
    if (!Number.isNaN(ai.getTime())) {
      const label = formatShortMonthDay(aiRecommendedStart) ?? aiRecommendedStart;
      const daysUntilStart = daysBetween(today, ai);
      let detail: string;
      if (daysUntilStart < 0) {
        detail = "Window is open now — begin the renewal conversation today.";
      } else if (daysUntilStart === 0) {
        detail = "Window opens today — begin the renewal conversation now.";
      } else if (renewalDate) {
        const renewal = new Date(renewalDate);
        if (!Number.isNaN(renewal.getTime())) {
          const daysUntilRenewal = daysBetween(today, renewal);
          if (daysUntilRenewal >= 0) {
            detail = `${daysUntilRenewal} days to renewal · window opens in ${daysUntilStart} days.`;
          } else {
            detail = `Window opens in ${daysUntilStart} days.`;
          }
        } else {
          detail = `Window opens in ${daysUntilStart} days.`;
        }
      } else {
        detail = `Window opens in ${daysUntilStart} days.`;
      }
      return { label, detail, className: styles.outlookValuePrep };
    }
  }

  // Fallback: compute from renewalDate - 120d runway.
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
      detail: `${daysUntilRenewal} days to renewal · window is open.`,
      className: styles.outlookValuePrep,
    };
  }
  return {
    label: `In ${daysUntilStart}d`,
    detail: `${daysUntilRenewal} days to renewal · window opens in ${daysUntilStart} days.`,
    className: styles.outlookValuePrep,
  };
}

interface OutlookPanelProps {
  intelligence: EntityIntelligence | null;
}

export function OutlookPanel({ intelligence }: OutlookPanelProps) {
  const outlook = intelligence?.agreementOutlook;
  if (!outlook) return null;

  const conf = confidenceCell(outlook);
  const start = recommendedStartCell(
    outlook.recommendedStart,
    intelligence?.contractContext?.renewalDate,
  );

  // TODO(DOS-204): when peer benchmark cohort data lands, render the
  // Benchmark cell between Confidence and Recommended start. Today the cell
  // is suppressed — the grid collapses to 2-col via .outlookGridTwoCol.
  const hasBenchmark = false;

  const gridClassName = `${styles.outlookGrid} ${hasBenchmark ? "" : styles.outlookGridTwoCol}`;

  // DOS-249: Pull-quote narrative — prefer `renewalNarrative` (dedicated field
  // added in DOS-249). Fall back to `expansionPotential` for backward compat with
  // accounts enriched before the field was added.
  const pullQuote = (outlook.renewalNarrative ?? outlook.expansionPotential ?? "").trim();

  return (
    <>
      <div className={gridClassName}>
        <div>
          <div className={styles.outlookBlockLabel}>Confidence</div>
          <div className={`${styles.outlookBlockValue} ${confidenceColorClass(outlook.confidence)}`}>
            {conf.label}
          </div>
          <div className={styles.outlookBlockDetail}>{conf.detail}</div>
        </div>

        <div>
          <div className={styles.outlookBlockLabel}>Recommended start</div>
          <div className={`${styles.outlookBlockValue} ${start.className}`}>{start.label}</div>
          <div className={styles.outlookBlockDetail}>{start.detail}</div>
        </div>
      </div>

      {pullQuote.length > 0 ? (
        <blockquote className={styles.outlookPullQuote}>{pullQuote}</blockquote>
      ) : null}
    </>
  );
}
