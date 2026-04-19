/**
 * AboutThisDossier — final chapter of the Context tab.
 *
 * Surfaces our own data-quality story: our data capture gap, source coverage,
 * and last enrichment / meeting counts. Always renders so the user sees what
 * the page was built from. Consumes existing EntityIntelligence fields and an
 * optional list of stakeholders with missing assessments — no new schema.
 *
 * Mockup: .meta-section / "About this dossier" in
 *         .docs/mockups/account-context-globex.html
 */
import type { EntityIntelligence } from "@/types";
import { formatShortDate } from "@/lib/utils";

interface AboutThisDossierProps {
  intelligence: EntityIntelligence | null;
  /** Meeting count from accountEvents or detail — optional, shown when present. */
  meetingCount?: number;
  /** Transcript count derived from source manifest format. */
  transcriptCount?: number;
  /**
   * Stakeholders who have attended meetings but have no assessment captured.
   * Drives the "Our data capture gap" card. Pass a minimal shape so the
   * callsite doesn't need to leak full stakeholder types.
   */
  uncharacterizedStakeholders?: { personName: string; meetingCount?: number | null }[];
}

// Mockup .meta-card — charcoal-4 tint with a thin tertiary left accent only.
// No full border, no serif body — sans body, 13px, max 620px readable width.
const card: React.CSSProperties = {
  background: "var(--color-desk-charcoal-4)",
  borderLeft: "2px solid var(--color-text-tertiary)",
  borderRadius: "0 var(--radius-md, 6px) var(--radius-md, 6px) 0",
  padding: "20px 28px",
  marginTop: 16,
};
const cardLabel: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 8,
  fontWeight: 600,
};
const cardText: React.CSSProperties = {
  fontFamily: "var(--font-sans)",
  fontSize: 13,
  lineHeight: 1.55,
  color: "var(--color-text-secondary)",
  maxWidth: 620,
};

export function AboutThisDossier({
  intelligence,
  meetingCount,
  transcriptCount,
  uncharacterizedStakeholders,
}: AboutThisDossierProps) {
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

  const gapStakeholders = uncharacterizedStakeholders ?? [];
  const gapCount = gapStakeholders.length;

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

      {gapCount > 0 && (
        <div style={card}>
          <div style={cardLabel}>Our data capture gap</div>
          <div style={cardText}>
            {gapCount} stakeholder{gapCount === 1 ? "" : "s"} attended meetings but{" "}
            {gapCount === 1 ? "has" : "have"} no characterization
            {gapStakeholders.length <= 3 ? (
              <>
                {" "}—{" "}
                {gapStakeholders.map((s, i) => (
                  <span key={s.personName}>
                    {i > 0 && (i === gapStakeholders.length - 1 ? " and " : ", ")}
                    <strong>{s.personName}</strong>
                  </span>
                ))}
              </>
            ) : null}
            . Assessments require verification in a customer-facing meeting.
          </div>
        </div>
      )}

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

      {!formatSummary && freshnessLine.length === 0 && gapCount === 0 && (
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
