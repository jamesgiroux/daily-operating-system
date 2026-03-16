/**
 * HealthBadge — Shared health score visualization.
 *
 * Three sizes:
 * - compact: dot + score (for list rows)
 * - standard: dot + score + trend arrow
 * - hero: large score + band tint + trend rationale + confidence qualifier
 *
 * I502: Wired across AccountsPage, AccountHero, MeetingDetailPage,
 * DailyBriefing, and WeekPage.
 */
import { TrendingUp, TrendingDown, Minus, Zap } from "lucide-react";
import type { IntelligenceHealthTrend, HealthDivergence } from "@/types";
import styles from "./HealthBadge.module.css";

interface HealthBadgeProps {
  score: number;
  band: "green" | "yellow" | "red";
  trend: IntelligenceHealthTrend;
  confidence?: number;
  source?: string;
  size?: "compact" | "standard" | "hero";
  showScore?: boolean;
  divergence?: HealthDivergence | null;
}

const bandDotClass: Record<string, string> = {
  green: styles.dotGreen,
  yellow: styles.dotYellow,
  red: styles.dotRed,
};

const trendDirectionClass: Record<string, string> = {
  improving: styles.trendImproving,
  stable: styles.trendStable,
  declining: styles.trendDeclining,
  volatile: styles.trendVolatile,
};

const heroTintClass: Record<string, string> = {
  green: styles.heroTintGreen,
  yellow: styles.heroTintYellow,
  red: styles.heroTintRed,
};

const divergenceClass: Record<string, string> = {
  minor: styles.divergenceMinor,
  notable: styles.divergenceNotable,
  critical: styles.divergenceCritical,
};

function TrendIcon({ direction, size }: { direction: string; size: number }) {
  const props = { size, strokeWidth: 2 };
  switch (direction) {
    case "improving":
      return <TrendingUp {...props} />;
    case "declining":
      return <TrendingDown {...props} />;
    case "volatile":
      return <Zap {...props} />;
    case "stable":
    default:
      return <Minus {...props} />;
  }
}

export function HealthBadge({
  score,
  band,
  trend,
  confidence,
  source,
  size = "standard",
  showScore = true,
  divergence,
}: HealthBadgeProps) {
  if (size === "hero") {
    return (
      <div className={`${styles.hero} ${heroTintClass[band] ?? ""}`}>
        <div className={styles.heroRow}>
          <span className={`${styles.dot} ${styles.dotHero} ${bandDotClass[band] ?? ""}`} />
          {showScore && (
            <span className={`${styles.score} ${styles.scoreHero}`}>{Math.round(score)}</span>
          )}
          <span className={`${styles.trend} ${trendDirectionClass[trend.direction] ?? ""}`}>
            <TrendIcon direction={trend.direction} size={20} />
          </span>
        </div>

        {trend.rationale && (
          <p className={styles.heroRationale}>{trend.rationale}</p>
        )}

        <div className={styles.heroMeta}>
          {confidence != null && confidence < 0.5 && (
            <span className={styles.confidenceQualifier}>Limited data</span>
          )}
          {source && (
            <span className={styles.sourceLabel}>{source}</span>
          )}
          {divergence && (
            <span className={`${styles.divergence} ${divergenceClass[divergence.severity] ?? ""}`}>
              {divergence.severity === "critical" ? "Divergence" : divergence.severity}
              {divergence.leadingIndicator && " (leading)"}
            </span>
          )}
        </div>
      </div>
    );
  }

  // compact and standard
  const dotSizeClass = size === "compact" ? styles.dotCompact : styles.dotStandard;
  const scoreSizeClass = size === "compact" ? styles.scoreCompact : styles.scoreStandard;

  return (
    <span className={styles.badge}>
      <span className={`${styles.dot} ${dotSizeClass} ${bandDotClass[band] ?? ""}`} />
      {showScore && (
        <span className={`${styles.score} ${scoreSizeClass}`}>{Math.round(score)}</span>
      )}
      {size === "standard" && (
        <span className={`${styles.trend} ${trendDirectionClass[trend.direction] ?? ""}`}>
          <TrendIcon direction={trend.direction} size={14} />
        </span>
      )}
    </span>
  );
}
