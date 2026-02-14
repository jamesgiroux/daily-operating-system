import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/**
 * Parse a date string with WebKit compatibility.
 *
 * WebKit/Safari rejects formats like "2026-02-13 11:30 AM" that V8 accepts.
 * Falls back to extracting the YYYY-MM-DD portion when native parsing fails.
 */
export function parseDate(dateStr: string): Date | null {
  const date = new Date(dateStr);
  if (!isNaN(date.getTime())) return date;
  // WebKit fallback: extract date portion from "YYYY-MM-DD ..." format
  const dateOnly = dateStr.match(/^(\d{4}-\d{2}-\d{2})/)?.[1];
  if (dateOnly) {
    const fallback = new Date(dateOnly);
    if (!isNaN(fallback.getTime())) return fallback;
  }
  return null;
}

/** Format a date string as short month + day (e.g. "Feb 13"). */
export function formatShortDate(dateStr: string): string {
  const date = parseDate(dateStr);
  if (!date) return dateStr;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

/** Format a date string as a full date (e.g. "Thu, Feb 13, 2026"). */
export function formatFullDate(dateStr: string): string {
  const date = parseDate(dateStr);
  if (!date) return dateStr;
  return date.toLocaleDateString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
    year: "numeric",
  });
}

/** Format a date string as a relative label (e.g. "Today", "3d ago", "2w ago"). */
export function formatRelativeDate(dateStr: string): string {
  const date = parseDate(dateStr);
  if (!date) return "";
  const now = new Date();
  const diffDays = Math.floor(
    (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
  );

  if (diffDays === 0) return "Today";
  if (diffDays === 1) return "Yesterday";
  if (diffDays < 7) return `${diffDays}d ago`;
  if (diffDays < 30) return `${Math.floor(diffDays / 7)}w ago`;
  return `${Math.floor(diffDays / 30)}mo ago`;
}

/** Format a date string as a long relative label (e.g. "Today", "3 days ago", "2 weeks ago"). */
export function formatRelativeDateLong(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffDays = Math.floor(
      (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
    );

    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays} day${diffDays !== 1 ? "s" : ""} ago`;
    if (diffDays < 30) {
      const weeks = Math.floor(diffDays / 7);
      return `${weeks} week${weeks !== 1 ? "s" : ""} ago`;
    }
    const months = Math.floor(diffDays / 30);
    return `${months} month${months !== 1 ? "s" : ""} ago`;
  } catch {
    return "";
  }
}

/** Format a date string as bidirectional relative (handles future dates: "Tomorrow", "In 3 days"). */
export function formatBidirectionalDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    const now = new Date();
    const diffMs = date.getTime() - now.getTime();
    const diffDays = Math.round(diffMs / (1000 * 60 * 60 * 24));

    if (diffDays === 0) {
      return date.toLocaleTimeString(undefined, {
        hour: "numeric",
        minute: "2-digit",
      });
    }
    if (diffDays === 1) {
      return `Tomorrow ${date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })}`;
    }
    if (diffDays === -1) return "Yesterday";
    if (diffDays < -1) return `${Math.abs(diffDays)} days ago`;
    if (diffDays <= 7) return `In ${diffDays} days`;
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr.split("T")[0] ?? dateStr;
  }
}

/** Format a date as weekday + time (e.g. "Monday at 3:00 PM"). */
export function formatDayTime(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    if (isNaN(date.getTime())) return "";
    return (
      date.toLocaleDateString("en-US", { weekday: "long" }) +
      " at " +
      date.toLocaleTimeString("en-US", {
        hour: "numeric",
        minute: "2-digit",
      })
    );
  } catch {
    return "";
  }
}


/** Format ARR as human-readable ($1.2M, $500K, etc.). */
export function formatArr(arr: number): string {
  if (arr >= 1_000_000) return `${(arr / 1_000_000).toFixed(1)}M`;
  if (arr >= 1_000) return `${(arr / 1_000).toFixed(0)}K`;
  return arr.toFixed(0);
}

/** Format bytes as human-readable file size (1.2 KB, 3.4 MB, etc.). */
export function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024)
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GB`;
}

export function stripMarkdown(text: string): string {
  return text
    .replace(/\*\*(.+?)\*\*/g, "$1")   // **bold**
    .replace(/\*(.+?)\*/g, "$1")       // *italic*
    .replace(/__(.+?)__/g, "$1")       // __bold__
    .replace(/_(.+?)_/g, "$1")         // _italic_
    .replace(/`(.+?)`/g, "$1")         // `code`
    .replace(/\[(.+?)\]\(.+?\)/g, "$1"); // [text](url)
}
