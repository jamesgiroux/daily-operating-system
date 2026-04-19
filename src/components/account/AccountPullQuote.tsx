import type { EntityIntelligence } from "@/types";
import { formatRelativeDate } from "@/lib/utils";

import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountPullQuoteProps {
  intelligence: EntityIntelligence;
  /** DOS-18: render as the Thesis chapter (serif 40px, label + freshness meta). */
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
      <section
        className="editorial-reveal-slow"
        style={{
          padding: "64px 0 48px",
          borderBottom: "1px solid var(--color-rule-light)",
        }}
      >
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            textTransform: "uppercase",
            letterSpacing: "0.14em",
            color: "var(--color-text-tertiary)",
            marginBottom: 28,
          }}
        >
          The thesis
        </div>
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 40,
            fontWeight: 400,
            fontStyle: "italic",
            lineHeight: 1.25,
            letterSpacing: "-0.01em",
            color: "var(--color-text-primary)",
            maxWidth: 820,
            margin: 0,
          }}
        >
          <span aria-hidden className={styles.pullquoteMark}>
            &ldquo;
          </span>
          {quote}
          <span aria-hidden className={styles.pullquoteMark}>
            &rdquo;
          </span>
        </p>
        {(metaParts.length > 0 || onRefresh) && (
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              textTransform: "uppercase",
              letterSpacing: "0.08em",
              color: "var(--color-text-tertiary)",
              marginTop: 32,
              display: "flex",
              alignItems: "center",
              gap: 16,
              flexWrap: "wrap",
            }}
          >
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
