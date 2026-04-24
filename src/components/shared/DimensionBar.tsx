/**
 * DimensionBar — Renders 6 relationship health dimensions as horizontal bars.
 *
 * Labels are resolved from the active preset's intelligence.dimensionLabels
 * (DOS-177), falling back to hardcoded ADR-0083 vocabulary when no preset
 * is configured or the key is absent.
 *
 * Each bar shows: label, score, colored fill, trend arrow, expandable evidence.
 *
 * I502: Used in StateOfPlay for account detail pages.
 */
import { useState } from "react";
import { TrendingUp, TrendingDown, Minus } from "lucide-react";
import type { RelationshipDimensions, DimensionScore } from "@/types";
import { useActivePreset } from "@/hooks/useActivePreset";
import styles from "./DimensionBar.module.css";

interface DimensionBarProps {
  dimensions: RelationshipDimensions;
}

interface DimensionConfig {
  /** camelCase key on RelationshipDimensions */
  key: keyof RelationshipDimensions;
  /** snake_case key matching preset intelligence.dimensionLabels */
  presetKey: string;
  /** Hardcoded fallback label (ADR-0083 vocabulary) */
  defaultLabel: string;
}

const DIMENSIONS: DimensionConfig[] = [
  { key: "meetingCadence", presetKey: "meeting_cadence", defaultLabel: "Meeting Cadence" },
  { key: "emailEngagement", presetKey: "email_engagement", defaultLabel: "Email Engagement" },
  { key: "stakeholderCoverage", presetKey: "stakeholder_coverage", defaultLabel: "Stakeholder Coverage" },
  { key: "keyAdvocateHealth", presetKey: "key_advocate_health", defaultLabel: "Champion Health" },
  { key: "financialProximity", presetKey: "financial_proximity", defaultLabel: "Financial Proximity" },
  { key: "signalMomentum", presetKey: "signal_momentum", defaultLabel: "Momentum" },
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

function DimensionRow({
  label,
  dimension,
  dimKey,
}: {
  label: string;
  dimension: DimensionScore;
  dimKey: string;
}) {
  const [showEvidence, setShowEvidence] = useState(false);
  const hasEvidence = dimension.evidence && dimension.evidence.length > 0;
  const fillPct = Math.min(100, Math.max(0, dimension.score));

  return (
    <div className={styles.dimension}>
      <div className={styles.dimensionHeader}>
        <span className={styles.dimensionLabel}>
          {label}
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
  const preset = useActivePreset();
  const presetLabels = preset?.intelligence?.dimensionLabels ?? {};

  return (
    <div className={styles.container}>
      {DIMENSIONS.map((config) => {
        const dimension = dimensions[config.key];
        if (!dimension) return null;
        const label = presetLabels[config.presetKey] ?? config.defaultLabel;
        return (
          <DimensionRow key={config.key} dimKey={config.key} label={label} dimension={dimension} />
        );
      })}
    </div>
  );
}
