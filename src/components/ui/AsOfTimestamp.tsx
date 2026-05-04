import clsx from "clsx";
import { parseDate } from "@/lib/utils";
import styles from "./AsOfTimestamp.module.css";

export type AsOfTimestampFormat = "relative" | "absolute" | "both";

export interface AsOfTimestampProps {
  at: string | Date | null;
  format?: AsOfTimestampFormat;
  prefix?: string;
  unavailableLabel?: string;
  className?: string;
}

function normalizeDate(value: string | Date | null): Date | null {
  if (!value) return null;
  if (value instanceof Date) return Number.isNaN(value.getTime()) ? null : value;
  return parseDate(value);
}

function formatRelativeAge(date: Date): string {
  const diffMs = Math.max(0, Date.now() - date.getTime());
  const diffMinutes = Math.floor(diffMs / (1000 * 60));
  const diffHours = Math.floor(diffMinutes / 60);
  const diffDays = Math.floor(diffHours / 24);

  if (diffMinutes < 1) return "now";
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  if (diffDays < 7) return `${diffDays}d ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
  return `${Math.floor(diffDays / 30)}mo ago`;
}

function formatAbsolute(date: Date): string {
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

function formatTimestamp(date: Date, format: AsOfTimestampFormat): string {
  switch (format) {
    case "absolute":
      return formatAbsolute(date);
    case "both":
      return `${formatRelativeAge(date)} · ${formatAbsolute(date)}`;
    case "relative":
    default:
      return formatRelativeAge(date);
  }
}

export function AsOfTimestamp({
  at,
  format = "relative",
  prefix = "As of",
  unavailableLabel = "unavailable",
  className,
}: AsOfTimestampProps) {
  const date = normalizeDate(at);
  const label = date ? formatTimestamp(date, format) : unavailableLabel;

  return (
    <span
      className={clsx(styles.timestamp, className)}
      data-state={date ? "available" : "unavailable"}
      data-ds-name="AsOfTimestamp"
      data-ds-spec="primitives/AsOfTimestamp.md"
    >
      {prefix} {label}
    </span>
  );
}
