/**
 * ProjectAppendix — Appendix section for project detail editorial page.
 * Milestones (full list), description, notes, files.
 * Actions moved to TheWork chapter (I351).
 * Simpler than AccountAppendix (no lifecycle events, BUs, portfolio, company context).
 */
import React from "react";
import type { ProjectDetail, ContentFile } from "@/types";
import { FileListSection } from "@/components/entity/FileListSection";

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
}: ProjectAppendixProps) {
  return (
    <section id="appendix" style={{ scrollMarginTop: 60, paddingTop: 80 }}>
      <div
        style={{
          borderTop: "3px double var(--color-rule-heavy)",
          paddingTop: 32,
        }}
      >
        <div style={sectionLabelStyle}>Appendix</div>

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

