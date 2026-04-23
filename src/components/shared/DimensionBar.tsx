/**
 * DimensionBar — Renders 6 relationship health dimensions as horizontal bars.
 *
 * Labels use product vocabulary (ADR-0083): "Meeting Cadence", "Email Engagement",
 * "Stakeholder Coverage", "Champion Health", "Financial Proximity", "Momentum".
 *
 * Each bar shows: label, score, colored fill, trend arrow, expandable evidence.
 *
 * I502: Used in StateOfPlay for account detail pages.
 */
import { useState } from "react";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import type { RelationshipDimensions, DimensionScore } from "@/types";
import styles from "./DimensionBar.module.css";

interface DimensionBarProps {
  dimensions: RelationshipDimensions;
}

interface DimensionConfig {
  key: keyof RelationshipDimensions;
  label: string;
}

const DIMENSIONS: DimensionConfig[] = [
  { key: "meetingCadence", label: "Meeting Cadence" },
  { key: "emailEngagement", label: "Email Engagement" },
  { key: "stakeholderCoverage", label: "Stakeholder Coverage" },
  { key: "keyAdvocateHealth", label: "Champion Health" },
  { key: "financialProximity", label: "Financial Proximity" },
  { key: "signalMomentum", label: "Momentum" },
];

function getBarColorClass(score: number): string {
  if (score > 60) return styles.barFillGreen;
  if (score >= 30) return styles.barFillYellow;
  return styles.barFillRed;
}

function getTrendClass(trend: string): string {
  switch (trend) {
    case "improving":
      return styles.trendImproving;
    case "declining":
      return styles.trendDeclining;
    case "stable":
    default:
      return styles.trendStable;
  }
}

function TrendIcon({ trend }: { trend: string }) {
  const props = { size: 12, strokeWidth: 2 };
  switch (trend) {
    case "improving":
      return <TrendingUp {...props} />;
    case "declining":
      return <TrendingDown {...props} />;
    case "stable":
    default:
      return <Minus {...props} />;
  }
}

function DimensionRow({ config, dimension }: { config: DimensionConfig; dimension: DimensionScore }) {
  const [showEvidence, setShowEvidence] = useState(false);
  const hasEvidence = dimension.evidence && dimension.evidence.length > 0;
  const fillPct = Math.min(100, Math.max(0, dimension.score));

  return (
    <div className={styles.dimension}>
      <div className={styles.dimensionHeader}>
        <span className={styles.dimensionLabel}>
          {config.label}
          <span className={`${styles.trend} ${getTrendClass(dimension.trend)}`}>
            <TrendIcon trend={dimension.trend} />
          </span>
        </span>
        <span className={styles.dimensionScore}>
          {Math.round(dimension.score)}
          <span className={styles.dimensionWeight}> / w{dimension.weight}</span>
        </span>
      </div>
      <div className={styles.barTrack}>
        <div
          className={`${styles.barFill} ${getBarColorClass(dimension.score)}`}
          style={{ width: `${fillPct}%` }}
        />
      </div>
      {hasEvidence && (
        <>
          <button
            className={styles.evidenceToggle}
            onClick={() => setShowEvidence(!showEvidence)}
          >
            {showEvidence ? "Hide evidence" : "Show evidence"}
          </button>
          {showEvidence && (
            <div className={styles.evidenceList}>
              {dimension.evidence!.map((item, i) => (
                <div key={i} className={styles.evidenceItem}>{item}</div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

export function DimensionBar({ dimensions }: DimensionBarProps) {
  return (
    <div className={styles.container}>
      {DIMENSIONS.map((config) => {
        const dimension = dimensions[config.key];
        if (!dimension) return null;
        return (
          <DimensionRow key={config.key} config={config} dimension={dimension} />
        );
      })}
    </div>
  );
}
