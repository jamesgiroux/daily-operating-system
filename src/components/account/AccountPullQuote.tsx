import type { EntityIntelligence } from "@/types";

import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountPullQuoteProps {
  intelligence: EntityIntelligence;
}

export function AccountPullQuote({ intelligence }: AccountPullQuoteProps) {
  if (!intelligence.pullQuote && !intelligence.executiveAssessment) return null;

  const quote = intelligence.pullQuote
    || (() => {
      const text = intelligence.executiveAssessment!;
      const match = text.match(/^(.+?[.!?])(?:\s|\n|$)/);
      return match ? match[1] : text.split("\n\n")[0]?.slice(0, 200);
    })();

  if (!quote) return null;

  return (
    <div className={`editorial-reveal-slow ${styles.pullQuote}`}>
      <blockquote className={styles.pullQuoteText}>
        {quote}
      </blockquote>
      <cite className={styles.pullQuoteAttribution}>From the executive assessment</cite>
    </div>
  );
}
