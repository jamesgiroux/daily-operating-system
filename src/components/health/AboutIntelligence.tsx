/**
 * AboutIntelligence — meta chapter: "About this intelligence" (DOS-203).
 *
 * Renders the source manifest + enrichment freshness as a quiet, italic
 * meta card. Mirrors the mockup's "About this intelligence" block — our
 * own data-capture story, not the customer's.
 */
import type { EntityIntelligence } from "@/types";
import { formatRelativeDate } from "@/lib/utils";
import styles from "./health.module.css";

interface AboutIntelligenceProps {
  intelligence: EntityIntelligence | null;
  /** When true, render the shorter fine-state prose card variant. */
  fine?: boolean;
}

export function AboutIntelligence({ intelligence, fine = false }: AboutIntelligenceProps) {
  if (!intelligence) return null;

  const manifest = intelligence.sourceManifest ?? [];
  const formatCounts = new Map<string, number>();
  for (const e of manifest) {
    const key = (e.format ?? "other").toLowerCase();
    formatCounts.set(key, (formatCounts.get(key) ?? 0) + 1);
  }
  const formats = Array.from(formatCounts.entries())
    .map(([k, n]) => `${n} ${k}${n === 1 ? "" : "s"}`)
    .join(" · ");

  const freshnessLabel = intelligence.enrichedAt
    ? `Last full enrichment ran ${formatRelativeDate(intelligence.enrichedAt)}.`
    : "Enrichment has not yet run for this account.";

  if (fine) {
    return (
      <div className={styles.metaCard}>
        <div className={styles.metaCardProse}>
          {freshnessLabel} No new signals triggered triage. The system is watching —
          it will surface changes as they emerge.
          {formats ? ` Sources: ${formats}.` : ""}
        </div>
      </div>
    );
  }

  return (
    <div className={styles.metaCard}>
      <div className={styles.metaCardLabel}>Data capture</div>
      <div className={styles.metaCardText}>
        {freshnessLabel}
        {formats ? ` Drawn from ${manifest.length} source file${manifest.length === 1 ? "" : "s"} — ${formats}.` : ""}
        {intelligence.sourceFileCount != null && manifest.length !== intelligence.sourceFileCount
          ? ` Manifest shows ${manifest.length} of ${intelligence.sourceFileCount} total files.`
          : ""}
      </div>
    </div>
  );
}
