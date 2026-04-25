/**
 * WatchListPrograms — Account-specific "Active Initiatives" section.
 * Extracted from old WatchList so the generalized entity/WatchList stays clean.
 * Passed as `bottomSection` to the shared WatchList component.
 */
import type { StrategicProgram } from "@/types";
import styles from "./WatchListPrograms.module.css";

interface WatchListProgramsProps {
  programs: StrategicProgram[];
  onProgramUpdate?: (index: number, updated: StrategicProgram) => void;
  onProgramDelete?: (index: number) => void;
  onAddProgram?: () => void;
}

function statusBadgeStyle(status: string): React.CSSProperties {
  // Badge colors are data-driven by program status, so they stay inline.
  const lower = status.toLowerCase();
  if (lower === "active") {
    return { background: "var(--color-garden-sage-14)", color: "var(--color-garden-rosemary)" };
  }
  if (lower === "planned" || lower === "planning") {
    return { background: "var(--color-garden-larkspur-14)", color: "var(--color-garden-larkspur)" };
  }
  return { background: "var(--color-rule-light)", color: "var(--color-text-tertiary)" };
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
    <div className={styles.section}>
      <div className={styles.heading}>Active Initiatives</div>

      {activePrograms.length > 0 && (
        <div className={styles.programList}>
          {activePrograms.map((p) => {
            const originalIndex = programs.indexOf(p);
            return (
              <div key={originalIndex} className={styles.program}>
                <div className={styles.programHeader}>
                  {onProgramUpdate ? (
                    <input
                      value={p.name}
                      onChange={(e) =>
                        onProgramUpdate(originalIndex, { ...p, name: e.target.value })
                      }
                      placeholder="Initiative name"
                      className={styles.programNameInput}
                    />
                  ) : (
                    <span className={styles.programName}>
                      {p.name || "Untitled"}
                    </span>
                  )}

                  {onProgramUpdate ? (
                    <>
                      {/* Status badge colors are data-driven by program status. */}
                    <select
                      value={p.status}
                      onChange={(e) =>
                        onProgramUpdate(originalIndex, { ...p, status: e.target.value })
                      }
                      className={styles.statusSelect}
                      style={statusBadgeStyle(p.status)}
                    >
                      <option value="Active">Active</option>
                      <option value="Planned">Planned</option>
                      <option value="Planning">Planning</option>
                      <option value="On Hold">On Hold</option>
                      <option value="Complete">Complete</option>
                    </select>
                    </>
                  ) : (
                    <>
                      {/* Status badge colors are data-driven by program status. */}
                    <span
                      className={styles.statusBadge}
                      style={statusBadgeStyle(p.status)}
                    >
                      {p.status}
                    </span>
                    </>
                  )}

                  {onProgramDelete && (
                    <button
                      onClick={() => onProgramDelete(originalIndex)}
                      className={styles.deleteButton}
                    >
                      x
                    </button>
                  )}
                </div>

                {p.notes && (
                  <p className={styles.notes}>
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
          className={styles.addButton}
        >
          + Add Initiative
        </button>
      )}
    </div>
  );
}
