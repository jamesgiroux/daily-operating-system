import type { EntityIntelligence } from "@/types";
import { formatRelativeDate } from "@/lib/utils";

import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountPullQuoteProps {
  intelligence: EntityIntelligence;
  /** render as the Thesis chapter (serif 40px, label + freshness meta). */
  variant?: "default" | "thesis";
  /** Source count fragments for freshness meta (e.g., "14 meetings · 5 transcripts"). */
  freshnessFragments?: string[];
  /**
   * Optional refresh affordance for the thesis variant. When provided, a
   * turmeric "Refresh" anchor renders aligned to the right of the meta row.
   */
  onRefresh?: () => void;
}

export function AccountPullQuote({ intelligence, variant = "default", freshnessFragments, onRefresh }: AccountPullQuoteProps) {
  if (!intelligence.pullQuote && !intelligence.executiveAssessment) return null;

  const quote = intelligence.pullQuote
    || (() => {
      const text = intelligence.executiveAssessment!;
      const match = text.match(/^(.+?[.!?])(?:\s|\n|$)/);
      return match ? match[1] : text.split("\n\n")[0]?.slice(0, 200);
    })();

  if (!quote) return null;

  if (variant === "thesis") {
    const metaParts: string[] = [];
    if (freshnessFragments) metaParts.push(...freshnessFragments);
    if (intelligence.enrichedAt) metaParts.push(`Updated ${formatRelativeDate(intelligence.enrichedAt)}`);

    return (
      <section className={`editorial-reveal-slow ${styles.thesisSection}`}>
        <div className={styles.thesisLabel}>The thesis</div>
        <p className={styles.thesisQuote}>
          <span aria-hidden className={styles.pullquoteMark}>
            &ldquo;
          </span>
          {quote}
          <span aria-hidden className={styles.pullquoteMark}>
            &rdquo;
          </span>
        </p>
        {(metaParts.length > 0 || onRefresh) && (
          <div className={styles.thesisMeta}>
            {metaParts.length > 0 && <span>{metaParts.join(" · ")}</span>}
            {onRefresh && (
              <button
                type="button"
                onClick={onRefresh}
                className={styles.thesisRefresh}
                aria-label="Refresh thesis"
              >
                Refresh
              </button>
            )}
          </div>
        )}
      </section>
    );
  }

  return (
    <div className={`editorial-reveal-slow ${styles.pullQuote}`}>
      <blockquote className={styles.pullQuoteText}>
        {quote}
      </blockquote>
      <cite className={styles.pullQuoteAttribution}>From the executive assessment</cite>
    </div>
  );
}
