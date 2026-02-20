/**
 * IntelligenceQualityBadge — freshness/completeness indicator for intelligence data.
 *
 * Visual: small colored dot + label text.
 * - Green dot: fresh (<24h since enriched)
 * - Amber dot: stale (>48h since enriched)
 * - Gray dot: no intelligence data
 */

interface IntelligenceQualityBadgeProps {
  /** ISO 8601 timestamp of last enrichment */
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

export function IntelligenceQualityBadge({
  enrichedAt,
  showLabel = false,
}: IntelligenceQualityBadgeProps) {
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
