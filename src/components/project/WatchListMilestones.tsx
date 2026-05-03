/**
 * WatchListMilestones — Project-specific bottom section for WatchList.
 * Displays active milestones in editorial style. Read-only.
 */
import type { ProjectMilestone } from "@/types";

interface WatchListMilestonesProps {
  milestones: ProjectMilestone[];
}

function milestoneStatusStyle(status: string): { background: string; color: string } {
  const lower = status.toLowerCase();
  if (lower === "in_progress" || lower === "active") {
    return { background: "var(--color-garden-sage-14)", color: "var(--color-garden-rosemary)" };
  }
  if (lower === "planned") {
    return { background: "var(--color-garden-larkspur-14)", color: "var(--color-garden-larkspur)" };
  }
  if (lower === "completed" || lower === "done") {
    return { background: "var(--color-rule-light)", color: "var(--color-text-tertiary)" };
  }
  return { background: "var(--color-rule-light)", color: "var(--color-text-tertiary)" };
}

export function WatchListMilestones({ milestones }: WatchListMilestonesProps) {
  const active = milestones.filter(
    (m) => m.status.toLowerCase() !== "completed" && m.status.toLowerCase() !== "done",
  );
  if (active.length === 0) return null;

  return (
    <div style={{ marginTop: 48 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: "var(--color-project)",
          marginBottom: 20,
        }}
      >
        Milestones
      </div>

      <div style={{ display: "flex", flexDirection: "column" }}>
        {active.map((m, i) => (
          <div
            key={i}
            style={{
              display: "flex",
              alignItems: "baseline",
              gap: 12,
              padding: "12px 0",
              borderBottom:
                i === active.length - 1 ? "none" : "1px solid var(--color-rule-light)",
            }}
          >
            <span
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-text-primary)",
                flex: 1,
              }}
            >
              {m.name}
            </span>

            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 9,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                padding: "2px 7px",
                borderRadius: 3,
                ...milestoneStatusStyle(m.status),
              }}
            >
              {m.status.replace(/_/g, " ")}
            </span>

            {m.targetDate && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-text-tertiary)",
                  whiteSpace: "nowrap",
                }}
              >
                {m.targetDate}
              </span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
