/**
 * IntelligenceQualityBadge — freshness/completeness indicator for intelligence data.
 *
 * Two modes:
 * 1. Structured quality from backend (preferred for meetings)
 * 2. Time-based freshness from enrichedAt (backward compat for entity heroes)
 */

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
  stale: "var(--color-spice-saffron)",
  none: "var(--color-text-tertiary)",
};

const LABELS: Record<Freshness, string> = {
  fresh: "Fresh",
  recent: "Recent",
  stale: "Stale",
  none: "Not analyzed",
};

const QUALITY_DOT_COLORS: Record<StructuredQuality["level"], string> = {
  sparse: "var(--color-text-tertiary)",
  developing: "var(--color-spice-turmeric)",
  ready: "var(--color-garden-sage)",
  fresh: "var(--color-garden-sage)",
};

const QUALITY_LABELS: Record<StructuredQuality["level"], string> = {
  sparse: "Sparse",
  developing: "Building",
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
      ? `${label} — Last enriched: ${new Date(quality.lastEnriched).toLocaleString()}`
      : `${label} — Not yet enriched`;

    return (
      <span
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
        }}
        title={tooltip}
      >
        <span
          style={{
            position: "relative",
            width: 7,
            height: 7,
            flexShrink: 0,
          }}
        >
          <span
            style={{
              width: 7,
              height: 7,
              borderRadius: "50%",
              background: color,
              display: "block",
              opacity: quality.level === "sparse" ? 0.5 : 1,
            }}
          />
          {quality.hasNewSignals && (
            <span
              style={{
                position: "absolute",
                top: -2,
                right: -2,
                width: 5,
                height: 5,
                borderRadius: "50%",
                background: "var(--color-water-larkspur)",
              }}
            />
          )}
        </span>
        {showLabel && (
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 500,
              letterSpacing: "0.04em",
              color,
              textTransform: "uppercase",
            }}
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
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 6,
      }}
      title={enrichedAt ? `Last enriched: ${new Date(enrichedAt).toLocaleString()}` : "Not yet enriched"}
    >
      <span
        style={{
          width: 7,
          height: 7,
          borderRadius: "50%",
          background: DOT_COLORS[freshness],
          flexShrink: 0,
          opacity: freshness === "none" ? 0.5 : 1,
        }}
      />
      {showLabel && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 500,
            letterSpacing: "0.04em",
            color: DOT_COLORS[freshness],
            textTransform: "uppercase",
          }}
        >
          {LABELS[freshness]}
        </span>
      )}
    </span>
  );
}
