/**
 * AboutThisDossier — final chapter of the Context tab.
 *
 * Surfaces our own data-quality story: source coverage, last enrichment,
 * meeting/transcript counts. Always renders so the user sees what the
 * page was built from. Consumes existing EntityIntelligence fields only —
 * `enrichedAt`, `sourceFileCount`, `sourceManifest`. No new schema.
 *
 * Mockup: .meta-section / "About this dossier" in .docs/mockups/account-context-globex.html
 */
import type { EntityIntelligence } from "@/types";
import { formatShortDate } from "@/lib/utils";

interface AboutThisDossierProps {
  intelligence: EntityIntelligence | null;
  /** Meeting count from accountEvents or detail — optional, shown when present. */
  meetingCount?: number;
  /** Transcript count derived from source manifest format. */
  transcriptCount?: number;
}

const card: React.CSSProperties = {
  background: "var(--color-paper-warm-white)",
  border: "1px solid var(--color-rule-light)",
  borderRadius: "var(--radius-md, 6px)",
  padding: "20px 24px",
  marginBottom: 16,
};
const cardLabel: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  textTransform: "uppercase",
  letterSpacing: "0.12em",
  color: "var(--color-text-tertiary)",
  marginBottom: 10,
  fontWeight: 600,
};
const cardText: React.CSSProperties = {
  fontFamily: "var(--font-serif)",
  fontSize: 15,
  lineHeight: 1.6,
  color: "var(--color-text-secondary)",
};

export function AboutThisDossier({ intelligence, meetingCount, transcriptCount }: AboutThisDossierProps) {
  const enrichedAt = intelligence?.enrichedAt;
  const sourceCount = intelligence?.sourceFileCount;
  const manifest = intelligence?.sourceManifest ?? [];

  // Group source manifest entries by format for a coverage line.
  const byFormat = manifest.reduce<Record<string, number>>((acc, entry) => {
    const key = entry.format ?? "other";
    acc[key] = (acc[key] ?? 0) + 1;
    return acc;
  }, {});
  const formatSummary = Object.entries(byFormat)
    .sort((a, b) => b[1] - a[1])
    .map(([format, count]) => `${count} ${format}`)
    .join(" · ");

  const freshnessLine: string[] = [];
  if (meetingCount != null) freshnessLine.push(`${meetingCount} meeting${meetingCount === 1 ? "" : "s"} on record`);
  if (transcriptCount != null) freshnessLine.push(`${transcriptCount} with transcripts`);
  if (enrichedAt) freshnessLine.push(`Last full dossier enrichment: ${formatShortDate(enrichedAt)}`);

  return (
    <section style={{ paddingTop: 80 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          textTransform: "uppercase",
          letterSpacing: "0.14em",
          color: "var(--color-text-tertiary)",
          marginBottom: 20,
          fontWeight: 600,
        }}
      >
        About this dossier
      </div>

      {formatSummary && (
        <div style={card}>
          <div style={cardLabel}>Source coverage</div>
          <div style={cardText}>
            Synthesized from <strong>{sourceCount ?? manifest.length}</strong> source file
            {(sourceCount ?? manifest.length) === 1 ? "" : "s"} — {formatSummary}. Gaps
            in commercial and relationship-fabric fields require manual capture and are
            not yet part of the enrichment loop.
          </div>
        </div>
      )}

      {freshnessLine.length > 0 && (
        <div style={card}>
          <div style={cardLabel}>Freshness</div>
          <div style={cardText}>{freshnessLine.join(" · ")}.</div>
        </div>
      )}

      {!formatSummary && freshnessLine.length === 0 && (
        <div style={card}>
          <div style={cardLabel}>Freshness</div>
          <div style={cardText}>
            No enrichment has run yet. Source manifest will populate once meetings,
            transcripts, and docs are captured.
          </div>
        </div>
      )}
    </section>
  );
}
