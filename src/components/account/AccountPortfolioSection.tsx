import { useNavigate } from "@tanstack/react-router";
import { formatArr } from "@/lib/utils";
import type { EntityIntelligence, AccountChildSummary } from "@/types";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

import shared from "@/styles/entity-detail.module.css";
import styles from "@/pages/AccountDetailEditorial.module.css";

function getHealthColorClass(health: string): string {
  if (health === "green") return styles.healthGreen;
  if (health === "red") return styles.healthRed;
  return styles.healthYellow;
}

function getHealthDotClass(health: string): string {
  if (health === "green") return styles.healthDotGreen;
  if (health === "red") return styles.healthDotRed;
  return styles.healthDotYellow;
}

interface AccountPortfolioSectionProps {
  children: AccountChildSummary[];
  intelligence: EntityIntelligence | null;
}

export function AccountPortfolioSection({
  children,
  intelligence,
}: AccountPortfolioSectionProps) {
  const navigate = useNavigate();

  return (
    <section id="portfolio" className={`editorial-reveal ${shared.chapterSectionWithPadding}`}>
      <ChapterHeading title="Portfolio" />

      {/* Health summary */}
      {intelligence?.portfolio?.healthSummary && (
        <div className={shared.portfolioHealthSummary}>
          <p className={shared.portfolioHealthSummaryText}>
            {intelligence.portfolio.healthSummary}
          </p>
        </div>
      )}

      {/* Portfolio narrative */}
      {intelligence?.portfolio?.portfolioNarrative && (
        <div className={shared.portfolioNarrative}>
          <p className={shared.portfolioNarrativeText}>
            {intelligence.portfolio.portfolioNarrative}
          </p>
        </div>
      )}

      {/* Hotspots */}
      {intelligence?.portfolio?.hotspots && intelligence.portfolio.hotspots.length > 0 && (
        <div className={shared.portfolioHotspotsSection}>
          <div className={shared.portfolioSectionLabelTerracotta}>
            Needs Attention
          </div>
          {intelligence.portfolio.hotspots.map((hotspot, i) => (
            <div
              key={hotspot.childId}
              className={
                i === intelligence.portfolio!.hotspots.length - 1
                  ? shared.hotspotRow
                  : shared.hotspotRowBorder
              }
            >
              <span className={shared.hotspotDot} />
              <div className={shared.hotspotContent}>
                <button
                  onClick={() =>
                    navigate({
                      to: "/accounts/$accountId",
                      params: { accountId: hotspot.childId },
                    })
                  }
                  className={styles.hotspotLinkTurmeric}
                >
                  {hotspot.childName}
                </button>
                <p className={shared.hotspotReason}>
                  {hotspot.reason}
                </p>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Cross-BU patterns */}
      {intelligence?.portfolio?.crossBuPatterns && intelligence.portfolio.crossBuPatterns.length > 0 && (
        <div className={shared.crossPatternsBlock}>
          <div className={shared.portfolioSectionLabelLarkspur}>
            Cross-BU Patterns
          </div>
          {intelligence.portfolio.crossBuPatterns.map((pattern, i) => (
            <p
              key={i}
              className={i === 0 ? shared.crossPatternTextFirst : shared.crossPatternTextSubsequent}
            >
              {pattern}
            </p>
          ))}
        </div>
      )}

      {/* Business Units list */}
      <div className={shared.childListSection}>
        <div className={shared.portfolioSectionLabelTertiary}>
          Business Units
        </div>
        {children.map((child, i) => (
          <div
            key={child.id}
            className={
              i === children.length - 1
                ? shared.childRow
                : shared.childRowBorder
            }
          >
            <button
              onClick={() =>
                navigate({
                  to: "/accounts/$accountId",
                  params: { accountId: child.id },
                })
              }
              className={shared.childNameButton}
            >
              {child.name}
              {child.accountType && child.accountType !== "customer" && (
                <span className={shared.childTypeBadge}>
                  {child.accountType === "partner" ? "Partner" : "Internal"}
                </span>
              )}
            </button>
            {child.health && (
              <span className={`${shared.statusIndicator} ${getHealthColorClass(child.health)}`}>
                <span className={getHealthDotClass(child.health)} />
                {child.health === "green"
                  ? "Healthy"
                  : child.health === "red"
                    ? "At Risk"
                    : "Monitor"}
              </span>
            )}
            {child.arr != null && (
              <span className={shared.secondaryMetric}>
                ${formatArr(child.arr)}
              </span>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
