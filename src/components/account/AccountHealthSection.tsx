import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import type { EntityIntelligence, ConsistencyFinding } from "@/types";
import { hasBleedFlag } from "@/lib/contamination-guard";
import { ContaminationWarning } from "@/components/ui/ContaminationWarning";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { DimensionBar } from "@/components/shared/DimensionBar";

import shared from "@/styles/entity-detail.module.css";
import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountHealthSectionProps {
  health: NonNullable<EntityIntelligence["health"]>;
  /** DOS-83: Consistency findings for bleed detection. */
  consistencyFindings?: ConsistencyFinding[];
}

export function AccountHealthSection({ health, consistencyFindings }: AccountHealthSectionProps) {
  const narrativeBleed = hasBleedFlag(consistencyFindings, "health.narrative");
  return (
    <div id="relationship-health" className={`editorial-reveal ${shared.marginLabelSection}`}>
      <div className={shared.marginLabel}>Relationship<br/>Health</div>
      <div className={shared.marginContent}>
        <ChapterHeading title="Relationship Health" />
        <div className={styles.healthHero}>
          <div className={styles.healthScoreNumber}>
            {Math.round(health.score)}
          </div>
          <div className={styles.healthMeta}>
            <div className={
              health.band === "green" ? styles.healthBandGreen
                : health.band === "red" ? styles.healthBandRed
                : styles.healthBandYellow
            }>
              {health.band === "green" ? "Healthy"
                : health.band === "red" ? "At Risk"
                : "Monitor"}
            </div>
            {health.narrative && !narrativeBleed && (
              <p className={styles.healthNarrative}>{health.narrative}</p>
            )}
            {health.narrative && narrativeBleed && (
              <ContaminationWarning variant="badge" />
            )}
            <div className={styles.healthTrendLabel}>
              {health.trend.direction === "improving" && <TrendingUp size={12} strokeWidth={2} />}
              {health.trend.direction === "declining" && <TrendingDown size={12} strokeWidth={2} />}
              {(health.trend.direction === "stable" || health.trend.direction === "volatile") && <Minus size={12} strokeWidth={2} />}
              {health.trend.direction}
              {health.trend.timeframe && ` \u00b7 ${health.trend.timeframe}`}
            </div>
          </div>
        </div>
        <div className="editorial-reveal-stagger">
          <DimensionBar dimensions={health.dimensions} />
        </div>
      </div>
    </div>
  );
}
