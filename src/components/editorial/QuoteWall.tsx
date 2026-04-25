import { formatShortDate } from "@/lib/utils";
import type { QuoteWallEntry } from "@/types";

import { EditorialEmpty } from "./EditorialEmpty";
import css from "./QuoteWall.module.css";

interface QuoteWallProps {
  quotes?: QuoteWallEntry[] | null;
}

function formatAttribution(entry: QuoteWallEntry): string {
  const speaker = entry.speaker?.trim();
  const role = entry.role?.trim();
  if (speaker && role) return `${speaker}, ${role}`;
  return speaker || role || "Unknown speaker";
}

function formatSource(source?: string | null): string {
  const cleaned = source?.trim();
  if (!cleaned) return "Via Glean";
  if (cleaned.toLowerCase().includes("glean")) return "Via Glean";
  if (cleaned.toLowerCase().startsWith("via ")) return cleaned;
  return `Via ${cleaned}`;
}

function sentimentClass(sentiment: QuoteWallEntry["sentiment"]): string {
  if (!sentiment) return css.neutral;
  return css[sentiment] ?? css.neutral;
}

export function QuoteWall({ quotes }: QuoteWallProps) {
  const entries = (quotes ?? []).filter((entry) => entry.quote.trim().length > 0);

  if (entries.length === 0) {
    return (
      <EditorialEmpty
        title="No quotes captured yet"
        message="No quotes captured yet — they will appear here as transcripts and signals are processed."
      />
    );
  }

  return (
    <div className={css.wall}>
      {entries.map((entry, index) => (
        <article key={`${entry.quote}-${entry.speaker ?? "unknown"}-${index}`} className={css.card}>
          <blockquote className={css.quote}>
            “{entry.quote}”
          </blockquote>
          <div className={css.attribution}>
            {formatAttribution(entry)}
          </div>
          {entry.whyItMatters && (
            <div className={css.why}>
              {entry.whyItMatters}
            </div>
          )}
          <div className={css.footer}>
            <span>{formatSource(entry.source)}</span>
            {entry.date && <span>{formatShortDate(entry.date)}</span>}
            {entry.sentiment && (
              <span className={`${css.badge} ${sentimentClass(entry.sentiment)}`}>
                {entry.sentiment}
              </span>
            )}
          </div>
        </article>
      ))}
    </div>
  );
}
