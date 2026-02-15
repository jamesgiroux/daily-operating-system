/**
 * WatchListPrograms — Account-specific "Active Initiatives" section.
 * Extracted from old WatchList so the generalized entity/WatchList stays clean.
 * Passed as `bottomSection` to the shared WatchList component.
 */
import type { StrategicProgram } from "@/types";

interface WatchListProgramsProps {
  programs: StrategicProgram[];
  onProgramUpdate?: (index: number, updated: StrategicProgram) => void;
  onProgramDelete?: (index: number) => void;
  onAddProgram?: () => void;
}

function statusBadgeStyle(status: string): React.CSSProperties {
  const lower = status.toLowerCase();
  if (lower === "active") {
    return { background: "rgba(126,170,123,0.14)", color: "var(--color-garden-rosemary)" };
  }
  if (lower === "planned" || lower === "planning") {
    return { background: "rgba(143,163,196,0.14)", color: "var(--color-garden-larkspur)" };
  }
  return { background: "rgba(30,37,48,0.06)", color: "var(--color-text-tertiary)" };
}

export function WatchListPrograms({
  programs,
  onProgramUpdate,
  onProgramDelete,
  onAddProgram,
}: WatchListProgramsProps) {
  const activePrograms = programs.filter((p) => p.status !== "Complete");
  if (activePrograms.length === 0 && !onAddProgram) return null;

  return (
    <div style={{ marginTop: 48 }}>
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.1em",
          color: "var(--color-spice-turmeric)",
          marginBottom: 20,
        }}
      >
        Active Initiatives
      </div>

      {activePrograms.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column" }}>
          {activePrograms.map((p) => {
            const originalIndex = programs.indexOf(p);
            return (
              <div
                key={originalIndex}
                style={{
                  display: "flex",
                  flexDirection: "column",
                  padding: "12px 0",
                  borderBottom:
                    originalIndex === programs.indexOf(activePrograms[activePrograms.length - 1])
                      ? "none"
                      : "1px solid rgba(30,37,48,0.06)",
                }}
              >
                <div style={{ display: "flex", alignItems: "baseline", gap: 12 }}>
                  {onProgramUpdate ? (
                    <input
                      value={p.name}
                      onChange={(e) =>
                        onProgramUpdate(originalIndex, { ...p, name: e.target.value })
                      }
                      placeholder="Initiative name"
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        fontWeight: 500,
                        color: "var(--color-text-primary)",
                        flex: 1,
                        background: "none",
                        border: "none",
                        borderBottom: "1px solid transparent",
                        outline: "none",
                        padding: 0,
                      }}
                      onFocus={(e) => {
                        e.currentTarget.style.borderBottomColor = "var(--color-rule-light)";
                      }}
                      onBlur={(e) => {
                        e.currentTarget.style.borderBottomColor = "transparent";
                      }}
                    />
                  ) : (
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        fontWeight: 500,
                        color: "var(--color-text-primary)",
                        flex: 1,
                      }}
                    >
                      {p.name || "Untitled"}
                    </span>
                  )}

                  {onProgramUpdate ? (
                    <select
                      value={p.status}
                      onChange={(e) =>
                        onProgramUpdate(originalIndex, { ...p, status: e.target.value })
                      }
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 9,
                        fontWeight: 500,
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                        padding: "2px 7px",
                        borderRadius: 3,
                        border: "none",
                        cursor: "pointer",
                        ...statusBadgeStyle(p.status),
                      }}
                    >
                      <option value="Active">Active</option>
                      <option value="Planned">Planned</option>
                      <option value="Planning">Planning</option>
                      <option value="On Hold">On Hold</option>
                      <option value="Complete">Complete</option>
                    </select>
                  ) : (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 9,
                        fontWeight: 500,
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                        padding: "2px 7px",
                        borderRadius: 3,
                        ...statusBadgeStyle(p.status),
                      }}
                    >
                      {p.status}
                    </span>
                  )}

                  {onProgramDelete && (
                    <button
                      onClick={() => onProgramDelete(originalIndex)}
                      style={{
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        padding: 0,
                      }}
                    >
                      x
                    </button>
                  )}
                </div>

                {p.notes && (
                  <p
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      lineHeight: 1.5,
                      color: "var(--color-text-tertiary)",
                      margin: "4px 0 0",
                    }}
                  >
                    {p.notes}
                  </p>
                )}
              </div>
            );
          })}
        </div>
      )}

      {onAddProgram && (
        <button
          onClick={onAddProgram}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: "8px 0",
            textTransform: "uppercase",
            letterSpacing: "0.06em",
          }}
        >
          + Add Initiative
        </button>
      )}
    </div>
  );
}
