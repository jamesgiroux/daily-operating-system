/**
 * ProjectAppendix — Appendix section for project detail editorial page.
 * Milestones (full list), description, notes, files.
 * Actions moved to TheWork chapter (I351).
 * Simpler than AccountAppendix (no lifecycle events, BUs, portfolio, company context).
 */
import React from "react";
import type { ProjectDetail, ContentFile } from "@/types";
import { FileListSection } from "@/components/entity/FileListSection";
import { ContextEntryList } from "@/components/entity/ContextEntryList";

interface ProjectAppendixProps {
  detail: ProjectDetail;
  files: ContentFile[];
  // Context entries
  contextEntries?: { id: string; title: string; content: string; createdAt: string }[];
  onCreateContextEntry?: (title: string, content: string) => void;
  onUpdateContextEntry?: (id: string, title: string, content: string) => void;
  onDeleteContextEntry?: (id: string) => void;
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
  contextEntries,
  onCreateContextEntry,
  onUpdateContextEntry,
  onDeleteContextEntry,
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

        {/* Context */}
        <div style={ruleStyle}>
          <div style={sectionLabelStyle}>Context</div>
          {onCreateContextEntry && onUpdateContextEntry && onDeleteContextEntry && contextEntries ? (
            <ContextEntryList
              entries={contextEntries}
              onCreate={onCreateContextEntry}
              onUpdate={onUpdateContextEntry}
              onDelete={onDeleteContextEntry}
              addLabel="+ Add context entry"
              placeholders={{
                title: "e.g., 'Architecture decision' or 'Risk identified'",
                content: "What happened and why it matters...",
              }}
            />
          ) : (
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: "var(--color-text-tertiary)",
                fontStyle: "italic",
                margin: 0,
              }}
            >
              No context entries.
            </p>
          )}
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

