/**
 * AboutIntelligence — meta chapter: "About this intelligence" (DOS-203).
 *
 * Renders the source manifest + enrichment freshness as a quiet meta card.
 * Mirrors the mockup's "About this intelligence" block (lines 974-989) — our
 * own data-capture story, not the customer's.
 *
 * The "enrichment has not yet run" empty-state is gated solely on
 * `intelligence.enrichedAt`. Glean is an *optional* upstream signal source;
 * its absence does not mean enrichment didn't run. When `gleanSignals` is
 * provided the manifest call-out includes a Glean subsection; otherwise we
 * describe only the local sources.
 */
import type { EntityIntelligence, HealthOutlookSignals, SourceManifestEntry } from "@/types";
import { formatRelativeDate } from "@/lib/utils";
import styles from "./health.module.css";

interface AboutIntelligenceProps {
  intelligence: EntityIntelligence | null;
  /** Glean enrichment payload, when the optional upstream source ran. */
  gleanSignals?: HealthOutlookSignals | null;
  /** When true, render the shorter fine-state prose card variant. */
  fine?: boolean;
}

/** Did enrichment actually run?
 *
 * Primary signal: `enrichedAt` is a non-empty timestamp.
 *
 * Fallback: any substantive intelligence field is populated. This catches
 * the case where the backend wrote dimensions_json + risks / health / etc.
 * but failed to stamp the `enriched_at` column — the enrichment DID run,
 * the stamp is just missing. We'd rather render the intelligence we have
 * than falsely claim enrichment never ran.
 */
function didEnrich(intelligence: EntityIntelligence): boolean {
  const stamp = intelligence.enrichedAt;
  if (stamp && stamp.trim().length > 0) return true;
  if (intelligence.health) return true;
  if (intelligence.currentState) return true;
  if (intelligence.executiveAssessment && intelligence.executiveAssessment.trim().length > 0) return true;
  if (intelligence.renewalOutlook) return true;
  if ((intelligence.risks?.length ?? 0) > 0) return true;
  if ((intelligence.recentWins?.length ?? 0) > 0) return true;
  return false;
}

/** Pretty "N transcripts · M emails · K pdfs" string from a manifest. */
function formatManifestCounts(manifest: SourceManifestEntry[]): string {
  const counts = new Map<string, number>();
  for (const e of manifest) {
    const key = (e.format ?? "other").toLowerCase();
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  return Array.from(counts.entries())
    .map(([k, n]) => `${n} ${k}${n === 1 ? "" : "s"}`)
    .join(" · ");
}

/** Count non-empty Glean signal blocks to describe what Glean contributed. */
function summariseGlean(g: HealthOutlookSignals): string {
  const parts: string[] = [];
  if (g.championRisk) parts.push("champion risk");
  if (g.productUsageTrend) parts.push("product usage");
  if (g.channelSentiment) parts.push("channel sentiment");
  if (g.transcriptExtraction) parts.push("transcript extraction");
  if (g.commercialSignals) parts.push("commercial signals");
  if (g.advocacyTrack) parts.push("advocacy");
  if (g.quoteWall && g.quoteWall.length > 0) parts.push(`${g.quoteWall.length} quotes`);
  if (parts.length === 0) return "no signals returned";
  return parts.join(" · ");
}

export function AboutIntelligence({ intelligence, gleanSignals, fine = false }: AboutIntelligenceProps) {
  if (!intelligence) return null;

  const hasEnriched = didEnrich(intelligence);

  if (!hasEnriched) {
    return (
      <div className={styles.metaCard}>
        <div className={styles.metaCardLabel}>Our data capture gap</div>
        <div className={styles.metaCardText}>
          Enrichment has not yet run for this account.
        </div>
      </div>
    );
  }

  const manifest = intelligence.sourceManifest ?? [];
  const formats = formatManifestCounts(manifest);
  // Freshness: honour enrichedAt when set; stay silent otherwise so we
  // don't render "Last enrichment ran on the empty string" when the
  // backend failed to stamp the column.
  const stamp = intelligence.enrichedAt;
  const freshness =
    stamp && stamp.trim().length > 0
      ? `Last enrichment ran ${formatRelativeDate(stamp)}.`
      : "";

  if (fine) {
    return (
      <div className={styles.metaCard}>
        <div className={styles.metaCardProse}>
          {freshness ? `${freshness} ` : ""}No new signals triggered triage. The system is watching —
          it will surface changes as they emerge.
          {formats ? ` Sources: ${formats}.` : ""}
        </div>
      </div>
    );
  }

  const manifestBlurb = formats
    ? `Drawn from ${manifest.length} source file${manifest.length === 1 ? "" : "s"} — ${formats}.`
    : manifest.length > 0
      ? `Drawn from ${manifest.length} source file${manifest.length === 1 ? "" : "s"}.`
      : "";

  const shortfall =
    intelligence.sourceFileCount != null &&
    manifest.length > 0 &&
    manifest.length !== intelligence.sourceFileCount
      ? `Manifest shows ${manifest.length} of ${intelligence.sourceFileCount} total files.`
      : "";

  const bodyParts = [freshness, manifestBlurb, shortfall].filter(
    (s) => s.trim().length > 0,
  );
  const body = bodyParts.join(" ");

  return (
    <div className={styles.metaCard}>
      {/* Label matches mockup line 979 — "Our data capture gap" is the
          fixed label for this meta card, whether populated or empty.
          The prior "Data capture" alt label was inconsistent drift. */}
      <div className={styles.metaCardLabel}>Our data capture gap</div>
      <div className={styles.metaCardText}>
        {body.length > 0
          ? body
          : "Enrichment has run, but the source manifest wasn't recorded."}
      </div>
      {gleanSignals ? (
        <div className={styles.metaCardSubsection}>
          <div className={styles.metaCardSubLabel}>Glean</div>
          <div className={styles.metaCardText}>
            Upstream enrichment contributed {summariseGlean(gleanSignals)}.
          </div>
        </div>
      ) : null}
    </div>
  );
}
