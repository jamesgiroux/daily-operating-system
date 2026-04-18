/**
 * ChapterFreshness — monospace strip under ChapterHeading.
 *
 * Renders a small row of metadata fragments plus a relative "updated" label
 * derived from `enrichedAt` (or a supplied override). Stale fragments can be
 * accented in saffron by passing `{ stale: true, text }`.
 *
 * Mockup: .freshness-strip in .docs/mockups/account-context-globex.html
 */
import { parseDate, formatRelativeDate } from "@/lib/utils";

export interface FreshnessFragment {
  text: string;
  stale?: boolean;
}

interface ChapterFreshnessProps {
  /** Source-of-truth enrichment timestamp. */
  enrichedAt?: string | null;
  /** Override timestamp (e.g. chapter-specific sourcedAt). Falls back to enrichedAt. */
  at?: string | null;
  /** Verb — "Updated" (default), "Enriched", "Refreshed". */
  verb?: string;
  /** Ordered fragments shown before the time label, separated by middots. */
  fragments?: (string | FreshnessFragment)[];
  /** Absolute date formatter override. Defaults to locale short month + day. */
  dateFormat?: "relative" | "short";
}

function normalizeFragment(f: string | FreshnessFragment): FreshnessFragment {
  return typeof f === "string" ? { text: f } : f;
}

function formatShort(dateStr: string): string {
  const d = parseDate(dateStr);
  if (!d) return dateStr;
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function ChapterFreshness({
  enrichedAt,
  at,
  verb = "Updated",
  fragments = [],
  dateFormat = "short",
}: ChapterFreshnessProps) {
  const when = at ?? enrichedAt ?? null;
  const timeLabel = when
    ? dateFormat === "relative"
      ? `${verb} ${formatRelativeDate(when)}`
      : `${verb} ${formatShort(when)}`
    : null;

  const parts = [...fragments.map(normalizeFragment)];
  if (timeLabel) parts.push({ text: timeLabel });

  if (parts.length === 0) return null;

  return (
    <div
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 10,
        textTransform: "uppercase",
        letterSpacing: "0.08em",
        color: "var(--color-text-tertiary)",
        margin: "6px 0 32px",
        display: "flex",
        flexWrap: "wrap",
        gap: "0 8px",
      }}
    >
      {parts.map((f, i) => (
        <span key={i} style={{ display: "inline-flex", alignItems: "center", gap: 8 }}>
          {i > 0 && <span aria-hidden style={{ color: "var(--color-text-tertiary)" }}>·</span>}
          <span style={f.stale ? { color: "var(--color-spice-saffron)" } : undefined}>
            {f.text}
          </span>
        </span>
      ))}
    </div>
  );
}
