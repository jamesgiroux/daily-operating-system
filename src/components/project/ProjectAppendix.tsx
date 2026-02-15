/**
 * ProjectAppendix — Appendix section for project detail editorial page.
 * Open actions (since TheWork chapter is dropped), milestones (full list),
 * description, notes, files.
 * Simpler than AccountAppendix (no lifecycle events, BUs, portfolio, company context).
 */
import React from "react";
import { Link } from "@tanstack/react-router";
import type { ProjectDetail, ContentFile, Action } from "@/types";
import { formatShortDate } from "@/lib/utils";
import { FileListSection } from "@/components/entity/FileListSection";
import { classifyAction } from "@/lib/entity-utils";

interface ProjectAppendixProps {
  detail: ProjectDetail;
  files: ContentFile[];
  editNotes?: string;
  setEditNotes?: (v: string) => void;
  onSaveNotes?: () => void;
  notesDirty?: boolean;
  onIndexFiles?: () => void;
  indexing?: boolean;
  indexFeedback?: string | null;
  openActions?: Action[];
  addingAction?: boolean;
  setAddingAction?: (v: boolean) => void;
  newActionTitle?: string;
  setNewActionTitle?: (v: string) => void;
  creatingAction?: boolean;
  onCreateAction?: () => void;
}

const sectionLabelStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 11,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 16,
};

const ruleStyle: React.CSSProperties = {
  borderTop: "2px solid var(--color-rule-heavy)",
  paddingTop: 24,
  marginTop: 32,
};

function milestoneStatusColor(status: string): string {
  const lower = status.toLowerCase();
  if (lower === "in_progress" || lower === "active") return "var(--color-garden-sage)";
  if (lower === "completed" || lower === "done") return "var(--color-garden-larkspur)";
  return "var(--color-text-tertiary)";
}

export function ProjectAppendix({
  detail,
  files,
  editNotes,
  setEditNotes,
  onSaveNotes,
  notesDirty,
  onIndexFiles,
  indexing,
  indexFeedback,
  openActions,
  addingAction,
  setAddingAction,
  newActionTitle,
  setNewActionTitle,
  creatingAction,
  onCreateAction,
}: ProjectAppendixProps) {
  const now = new Date();
  const actions = openActions ?? [];
  const hasOverdue = actions.some((a) => classifyAction(a, now) === "overdue");
  const sortedActions = [...actions].sort((a, b) => {
    const ca = classifyAction(a, now);
    const cb = classifyAction(b, now);
    if (ca === "overdue" && cb !== "overdue") return -1;
    if (ca !== "overdue" && cb === "overdue") return 1;
    return 0;
  });
  return (
    <section id="appendix" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <div
        style={{
          borderTop: "3px double var(--color-rule-heavy)",
          paddingTop: 32,
        }}
      >
        <div style={sectionLabelStyle}>Appendix</div>

        {/* Open Actions */}
        {(actions.length > 0 || setAddingAction) && (
          <div style={ruleStyle}>
            <div
              style={{
                ...sectionLabelStyle,
                color: hasOverdue ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
              }}
            >
              Open Actions{actions.length > 0 && ` (${actions.length})`}
            </div>
            {sortedActions.map((a) => {
              const cls = classifyAction(a, now);
              return (
                <Link
                  key={a.id}
                  to="/actions/$actionId"
                  params={{ actionId: a.id }}
                  style={{
                    display: "grid",
                    gridTemplateColumns: "1fr auto auto",
                    gap: "4px 16px",
                    padding: "10px 0 10px 16px",
                    borderBottom: "1px solid var(--color-rule-light)",
                    textDecoration: "none",
                    color: "inherit",
                    position: "relative",
                  }}
                >
                  {cls === "overdue" && (
                    <div
                      style={{
                        position: "absolute",
                        left: 0,
                        top: 10,
                        bottom: 10,
                        width: 3,
                        borderRadius: 2,
                        background: "var(--color-spice-terracotta)",
                      }}
                    />
                  )}
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      color: "var(--color-text-primary)",
                      fontWeight: cls === "overdue" ? 500 : 400,
                    }}
                  >
                    {a.title}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 500,
                      textTransform: "uppercase",
                      color:
                        cls === "overdue"
                          ? "var(--color-spice-terracotta)"
                          : cls === "this-week"
                            ? "var(--color-spice-turmeric)"
                            : "var(--color-text-tertiary)",
                    }}
                  >
                    {cls === "overdue" ? "Overdue" : cls === "this-week" ? "This week" : ""}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color:
                        cls === "overdue"
                          ? "var(--color-spice-terracotta)"
                          : "var(--color-text-tertiary)",
                    }}
                  >
                    {a.dueDate ? formatShortDate(a.dueDate) : "\u2014"}
                  </span>
                </Link>
              );
            })}
            {/* Inline action creation */}
            {setAddingAction && onCreateAction && (
              <div style={{ marginTop: 12 }}>
                {addingAction ? (
                  <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                    <input
                      value={newActionTitle ?? ""}
                      onChange={(e) => setNewActionTitle?.(e.target.value)}
                      placeholder="New action..."
                      autoFocus
                      onKeyDown={(e) => {
                        if (e.key === "Enter" && (newActionTitle ?? "").trim()) onCreateAction();
                        if (e.key === "Escape") setAddingAction(false);
                      }}
                      style={{
                        flex: 1,
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        color: "var(--color-text-primary)",
                        background: "none",
                        border: "none",
                        borderBottom: "1px solid var(--color-rule-light)",
                        outline: "none",
                        padding: "4px 0",
                      }}
                    />
                    <button
                      onClick={onCreateAction}
                      disabled={creatingAction || !(newActionTitle ?? "").trim()}
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        textTransform: "uppercase",
                        letterSpacing: "0.06em",
                        padding: 0,
                      }}
                    >
                      {creatingAction ? "..." : "Add"}
                    </button>
                    <button
                      onClick={() => setAddingAction(false)}
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        background: "none",
                        border: "none",
                        cursor: "pointer",
                        padding: 0,
                      }}
                    >
                      x
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => setAddingAction(true)}
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                      background: "none",
                      border: "none",
                      cursor: "pointer",
                      padding: "4px 0",
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                    }}
                  >
                    + Add Action
                  </button>
                )}
              </div>
            )}
          </div>
        )}

        {/* Milestones (full list) */}
        {detail.milestones.length > 0 && (
          <div style={ruleStyle}>
            <div style={sectionLabelStyle}>Milestones</div>
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "1fr auto auto",
                gap: "8px 24px",
              }}
            >
              {detail.milestones.map((m, i) => (
                <React.Fragment key={i}>
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      color: "var(--color-text-primary)",
                    }}
                  >
                    {m.name}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 500,
                      textTransform: "uppercase",
                      color: milestoneStatusColor(m.status),
                    }}
                  >
                    {m.status.replace(/_/g, " ")}
                  </span>
                  <span
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    {m.targetDate ?? "—"}
                  </span>
                </React.Fragment>
              ))}
            </div>
          </div>
        )}

        {/* Description */}
        {detail.description && (
          <div style={ruleStyle}>
            <div style={sectionLabelStyle}>Description</div>
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                lineHeight: 1.65,
                color: "var(--color-text-secondary)",
                margin: 0,
              }}
            >
              {detail.description}
            </p>
          </div>
        )}

        {/* Notes (editable) */}
        <div style={ruleStyle}>
          <div
            style={{
              display: "flex",
              alignItems: "baseline",
              justifyContent: "space-between",
              marginBottom: 16,
            }}
          >
            <div style={sectionLabelStyle}>Notes</div>
            {notesDirty && onSaveNotes && (
              <button
                onClick={onSaveNotes}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-garden-olive)",
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                  padding: 0,
                }}
              >
                Save
              </button>
            )}
          </div>
          <textarea
            value={editNotes ?? detail.notes ?? ""}
            onChange={(e) => setEditNotes?.(e.target.value)}
            placeholder="Notes about this project…"
            rows={6}
            style={{
              width: "100%",
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              lineHeight: 1.65,
              color: "var(--color-text-primary)",
              background: "none",
              border: "none",
              borderBottom: "1px solid var(--color-rule-light)",
              outline: "none",
              resize: "vertical",
              padding: "8px 0",
            }}
          />
        </div>

        {/* Files */}
        <div style={ruleStyle}>
          <FileListSection
            files={files}
            onIndexFiles={onIndexFiles}
            indexing={indexing}
            indexFeedback={indexFeedback}
            emptyMessage="No files indexed yet."
          />
        </div>
      </div>
    </section>
  );
}

