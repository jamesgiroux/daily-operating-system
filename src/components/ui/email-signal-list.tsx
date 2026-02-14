import { Badge } from "@/components/ui/badge";
import { cn, formatRelativeDate, formatRelativeDateLong, formatShortDate } from "@/lib/utils";
import type { EmailSignal } from "@/types";

type DateFormat = "relative" | "relative-short" | "absolute";

interface EmailSignalListProps {
  signals: EmailSignal[];
  limit?: number;
  dateFormat?: DateFormat;
  /** Show urgency, sentiment, and confidence metadata under each signal. */
  showMetadata?: boolean;
  className?: string;
}

function formatDetectedAt(iso: string, format: DateFormat): string {
  switch (format) {
    case "relative":
      return formatRelativeDateLong(iso);
    case "relative-short":
      return formatRelativeDate(iso);
    case "absolute":
      return formatShortDate(iso);
  }
}

export function EmailSignalList({
  signals,
  limit = 8,
  dateFormat = "relative-short",
  showMetadata = false,
  className,
}: EmailSignalListProps) {
  const display = signals.slice(0, limit);
  if (display.length === 0) return null;

  return (
    <div className={cn("space-y-2", className)}>
      {display.map((signal, idx) => (
        <div
          key={`${signal.id ?? idx}-${signal.signalType}`}
          className="rounded-md border border-border/70 bg-card/50 px-3 py-2"
        >
          <div className="flex items-center justify-between gap-2">
            <Badge variant="outline" className="text-[10px] uppercase tracking-wide">
              {signal.signalType}
            </Badge>
            {signal.detectedAt && (
              <span className="text-[10px] text-muted-foreground">
                {formatDetectedAt(signal.detectedAt, dateFormat)}
              </span>
            )}
          </div>
          <p className="mt-1 text-sm leading-relaxed">{signal.signalText}</p>
          {showMetadata && (
            <div className="mt-1 flex flex-wrap items-center gap-2 text-[11px] text-muted-foreground">
              {signal.urgency && <span>Urgency: {signal.urgency}</span>}
              {signal.sentiment && <span>Sentiment: {signal.sentiment}</span>}
              {signal.confidence != null && (
                <span>Confidence: {Math.round(signal.confidence * 100)}%</span>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
