/**
 * IntelligenceQualityBadge — freshness/completeness indicator for intelligence data.
 *
 * Two modes:
 * 1. Structured quality from backend (preferred for meetings)
 * 2. Time-based freshness from enrichedAt (backward compat for entity heroes)
 */
import css from "./IntelligenceQualityBadge.module.css";

interface StructuredQuality {
  level: "sparse" | "developing" | "ready" | "fresh";
  hasNewSignals: boolean;
  lastEnriched?: string;
}

interface IntelligenceQualityBadgeProps {
  /** Structured quality from backend intelligence assessment */
  quality?: StructuredQuality;
  /** ISO 8601 timestamp of last enrichment (backward compat for entity heroes) */
  enrichedAt?: string;
  /** Whether to show inline label text alongside the dot */
  showLabel?: boolean;
}

type Freshness = "fresh" | "recent" | "stale" | "none";

function computeFreshness(enrichedAt?: string): Freshness {
  if (!enrichedAt) return "none";
  try {
    const enrichedMs = new Date(enrichedAt).getTime();
    const hoursAgo = (Date.now() - enrichedMs) / (1000 * 60 * 60);
    if (hoursAgo < 24) return "fresh";
    if (hoursAgo < 48) return "recent";
    return "stale";
  } catch {
    return "none";
  }
}

const DOT_COLORS: Record<Freshness, string> = {
  fresh: "var(--color-garden-sage)",
  recent: "var(--color-spice-turmeric)",
  stale: "var(--color-text-tertiary)",
  none: "transparent",
};

const LABELS: Record<Freshness, string> = {
  fresh: "Fresh",
  recent: "Building",
  stale: "Sparse",
  none: "No data",
};

const QUALITY_DOT_COLORS: Record<StructuredQuality["level"], string> = {
  sparse: "var(--color-text-tertiary)",
  developing: "var(--color-spice-turmeric)",
  ready: "var(--color-garden-sage)",
  fresh: "var(--color-garden-sage)",
};

const QUALITY_LABELS: Record<StructuredQuality["level"], string> = {
  sparse: "Sparse",
  developing: "Limited",
  ready: "Ready",
  fresh: "Fresh",
};

export function IntelligenceQualityBadge({
  quality,
  enrichedAt,
  showLabel = false,
}: IntelligenceQualityBadgeProps) {
  // Structured quality path (preferred)
  if (quality) {
    const color = QUALITY_DOT_COLORS[quality.level];
    const label = QUALITY_LABELS[quality.level];
    const tooltip = quality.lastEnriched
      ? `${label} — Last updated: ${new Date(quality.lastEnriched).toLocaleString()}`
      : `${label} — Not yet updated`;

    return (
      <span className={css.root} title={tooltip}>
        <span className={css.structuredDotShell}>
          <span
            className={`${css.dot} ${quality.level === "sparse" ? css.mutedDot : ""}`}
            // Runtime quality level determines the dot color.
            style={{ background: color }}
          />
          {quality.hasNewSignals && (
            <span className={css.newSignalDot} />
          )}
        </span>
        {showLabel && (
          <span
            className={css.label}
            // Runtime quality level determines the label color.
            style={{ color }}
          >
            {label}
          </span>
        )}
      </span>
    );
  }

  // Fallback: time-based freshness from enrichedAt
  const freshness = computeFreshness(enrichedAt);

  return (
    <span className={css.root} title={enrichedAt ? `${LABELS[freshness]} — Last updated: ${new Date(enrichedAt).toLocaleString()}` : "Not yet updated"}>
      <span
        className={`${css.dot} ${freshness === "none" ? css.mutedDot : ""}`}
        // Runtime freshness determines the dot color.
        style={{ background: DOT_COLORS[freshness] }}
      />
      {showLabel && (
        <span
          className={css.label}
          // Runtime freshness determines the label color.
          style={{ color: DOT_COLORS[freshness] }}
        >
          {LABELS[freshness]}
        </span>
      )}
    </span>
  );
}
