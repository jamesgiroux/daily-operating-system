import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import type { EntityIntelligence } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { DimensionBar } from "@/components/shared/DimensionBar";

import shared from "@/styles/entity-detail.module.css";
import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountHealthSectionProps {
  health: NonNullable<EntityIntelligence["health"]>;
}

export function AccountHealthSection({ health }: AccountHealthSectionProps) {
  // DOS-84: When fewer than 3 dimensions have data, show "Insufficient Data"
  // Use !== true so undefined (old cached data) also triggers insufficient display
  const isInsufficient = health.sufficientData !== true;

  return (
    <div id="relationship-health" className={`editorial-reveal ${shared.marginLabelSection}`}>
      <div className={shared.marginLabel}>Relationship<br/>Health</div>
      <div className={shared.marginContent}>
        <ChapterHeading title="Relationship Health" />
        <div className={styles.healthHero}>
          <div className={styles.healthScoreNumber} style={isInsufficient ? { opacity: 0.4 } : undefined}>
            {isInsufficient ? "--" : Math.round(health.score)}
          </div>
          <div className={styles.healthMeta}>
            {isInsufficient ? (
              <div className={styles.healthBandYellow}>Insufficient Data</div>
            ) : (
              <div className={
                health.band === "green" ? styles.healthBandGreen
                  : health.band === "red" ? styles.healthBandRed
                  : styles.healthBandYellow
              }>
                {health.band === "green" ? "Healthy"
                  : health.band === "red" ? "At Risk"
                  : "Monitor"}
              </div>
            )}
            {isInsufficient ? (
              <p className={styles.healthNarrative}>
                Fewer than 3 of 6 health dimensions have data. As more meetings, emails, and stakeholder data accumulate, a reliable score will appear.
              </p>
            ) : health.narrative ? (
              <p className={styles.healthNarrative}>{health.narrative}</p>
            ) : null}
            {!isInsufficient && (
              <div className={styles.healthTrendLabel}>
                {health.trend.direction === "improving" && <TrendingUp size={12} strokeWidth={2} />}
                {health.trend.direction === "declining" && <TrendingDown size={12} strokeWidth={2} />}
                {(health.trend.direction === "stable" || health.trend.direction === "volatile") && <Minus size={12} strokeWidth={2} />}
                {health.trend.direction}
                {health.trend.timeframe && ` \u00b7 ${health.trend.timeframe}`}
              </div>
            )}
          </div>
        </div>
        <div className="editorial-reveal-stagger">
          <DimensionBar dimensions={health.dimensions} />
        </div>
      </div>
    </div>
  );
}
