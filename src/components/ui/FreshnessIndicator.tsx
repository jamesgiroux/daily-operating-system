import clsx from "clsx";
import { formatRelativeDate, parseDate } from "@/lib/utils";
import styles from "./FreshnessIndicator.module.css";

export interface FreshnessFragment {
  text: string;
  stale?: boolean;
}

export type FreshnessFormat = "relative" | "absolute" | "both";
export type FreshnessStaleness = "fresh" | "aging" | "stale";

export interface FreshnessIndicatorProps {
  /** Source-of-truth timestamp for inline trust contexts. */
  at?: string | Date | null;
  /** Back-compat alias used by chapter freshness strips. */
  enrichedAt?: string | null;
  format?: FreshnessFormat;
  /** Back-compat chapter strip formatter. */
  dateFormat?: "relative" | "short";
  /** Hours before the timestamp moves from fresh to aging. */
  stalenessThreshold?: number;
  /** Verb used by chapter strips, e.g. "Updated" or "Enriched". */
  verb?: string;
  /** Ordered fragments shown before the time label in strip mode. */
  fragments?: (string | FreshnessFragment)[];
  variant?: "inline" | "strip";
  className?: string;
}

function normalizeFragment(fragment: string | FreshnessFragment): FreshnessFragment {
  return typeof fragment === "string" ? { text: fragment } : fragment;
}

function normalizeDate(value: string | Date | null | undefined): Date | null {
  if (!value) return null;
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? null : value;
  return parseDate(value);
}

function formatShort(date: Date): string {
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

function formatAbsolute(date: Date): string {
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function formatRelativeAge(date: Date): string {
  const diffMs = Math.max(0, Date.now() - date.getTime());
  const diffMinutes = Math.floor(diffMs / (1000 * 60));
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMinutes < 1) return "now";
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}w`;
  return `${Math.floor(diffDays / 30)}mo`;
}

function stripAgo(label: string): string {
  return label.replace(/\s+ago$/, "");
}

function getStaleness(
  date: Date | null,
  stalenessThreshold = 48,
): FreshnessStaleness {
  if (!date) return "fresh";
  const ageHours = Math.max(0, Date.now() - date.getTime()) / (1000 * 60 * 60);
  if (ageHours < stalenessThreshold) return "fresh";
  if (ageHours < stalenessThreshold * 2) return "aging";
  return "stale";
}

function formatInlineLabel(
  date: Date | null,
  raw: string | Date | null | undefined,
  format: FreshnessFormat,
  staleness: FreshnessStaleness,
): string | null {
  if (!date) return raw ? String(raw) : null;

  const relative = formatRelativeAge(date);
  const relativeLabel = staleness === "stale" ? `stale ${stripAgo(relative)}` : relative;

  switch (format) {
    case "absolute":
      return formatAbsolute(date);
    case "both":
      return `${relativeLabel} · ${formatAbsolute(date)}`;
    case "relative":
    default:
      return relativeLabel;
  }
}

export function FreshnessIndicator({
  at,
  enrichedAt,
  format = "relative",
  dateFormat,
  stalenessThreshold,
  verb = "Updated",
  fragments = [],
  variant,
  className,
}: FreshnessIndicatorProps) {
  const when = at ?? enrichedAt ?? null;
  const date = normalizeDate(when);
  const staleness = getStaleness(date, stalenessThreshold);
  const resolvedVariant = variant ?? (fragments.length > 0 || enrichedAt ? "strip" : "inline");

  if (resolvedVariant === "strip") {
    const timeLabel = when
      ? `${verb} ${
          dateFormat === "relative"
            ? formatRelativeDate(when instanceof Date ? when.toISOString() : when)
            : date
              ? formatShort(date)
              : String(when)
        }`
      : null;
    const parts = fragments.map(normalizeFragment);
    if (timeLabel) parts.push({ text: timeLabel, stale: staleness === "stale" });
    if (parts.length === 0) return null;

    return (
      <div
        className={clsx(styles.root, styles.strip, className)}
        data-staleness={staleness}
        data-ds-name="FreshnessIndicator"
        data-ds-spec="primitives/FreshnessIndicator.md"
      >
        {parts.map((part, index) => (
          <span className={styles.part} data-stale={part.stale ? "true" : undefined} key={index}>
            {index > 0 && (
              <span className={styles.separator} aria-hidden="true">
                ·
              </span>
            )}
            <span className={clsx(styles.text, index === parts.length - 1 && styles.timeText)}>
              {part.text}
            </span>
          </span>
        ))}
      </div>
    );
  }

  const label = formatInlineLabel(date, when, format, staleness);
  if (!label) return null;

  return (
    <span
      className={clsx(styles.root, styles.inline, className)}
      data-staleness={staleness}
      data-ds-name="FreshnessIndicator"
      data-ds-spec="primitives/FreshnessIndicator.md"
    >
      <span className={styles.timeText}>{label}</span>
    </span>
  );
}
