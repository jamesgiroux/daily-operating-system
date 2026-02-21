/** Format an ISO timestamp as display time: "9:30 AM" */
export function formatDisplayTime(isoString: string): string {
  try {
    const d = new Date(isoString);
    return d.toLocaleTimeString("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    });
  } catch {
    return "";
  }
}

/** Format duration from two ISO timestamps: "45m" / "1h" / "1h 30m" */
export function formatDurationFromIso(
  startIso: string,
  endIso?: string,
): string | null {
  if (!endIso) return null;
  try {
    const startMs = new Date(startIso).getTime();
    const endMs = new Date(endIso).getTime();
    if (endMs <= startMs) return null;
    const mins = Math.round((endMs - startMs) / 60000);
    if (mins < 60) return `${mins}m`;
    const hrs = Math.floor(mins / 60);
    const rem = mins % 60;
    return rem > 0 ? `${hrs}h ${rem}m` : `${hrs}h`;
  } catch {
    return null;
  }
}
