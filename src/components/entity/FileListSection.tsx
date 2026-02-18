/**
 * FileListSection — Shared file list for entity appendix sections.
 * 4-column grid: icon | filename | size | date. Click to reveal in Finder.
 * Collapses to 10 files with expand button.
 * Used by AccountAppendix, ProjectAppendix, PersonAppendix.
 */
import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ContentFile } from "@/types";
import { formatFileSize, formatRelativeDate } from "@/lib/utils";

interface FileListSectionProps {
  files: ContentFile[];
  onIndexFiles?: () => void;
  indexing?: boolean;
  indexFeedback?: string | null;
  emptyMessage?: string;
}

/* ── Document icon SVG (14x14) ── */

function FileIcon() {
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 14 14"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={{ color: "var(--color-text-tertiary)", flexShrink: 0 }}
    >
      <path
        d="M3 1.5h5l3 3v8a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1v-10a1 1 0 0 1 1-1Z"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
      <path
        d="M8 1.5v3h3"
        stroke="currentColor"
        strokeWidth="1"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

/* ── Shared styles ── */

const sectionTitleStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  textTransform: "uppercase",
  letterSpacing: "0.1em",
  color: "var(--color-text-tertiary)",
  marginBottom: 12,
};

const monoActionButtonStyle: React.CSSProperties = {
  fontFamily: "var(--font-mono)",
  fontSize: 10,
  fontWeight: 500,
  color: "var(--color-text-tertiary)",
  background: "none",
  border: "none",
  cursor: "pointer",
  textTransform: "uppercase",
  letterSpacing: "0.06em",
  padding: 0,
};

/* ── Component ── */

export function FileListSection({
  files,
  onIndexFiles,
  indexing,
  indexFeedback,
  emptyMessage = "No files indexed.",
}: FileListSectionProps) {
  const [expanded, setExpanded] = useState(false);

  const visibleFiles = expanded ? files : files.slice(0, 10);
  const hasMore = files.length > 10;

  return (
    <div style={{ marginBottom: 40 }}>
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          justifyContent: "space-between",
        }}
      >
        <div style={sectionTitleStyle}>
          Files{files.length > 0 ? ` \u00B7 ${files.length}` : ""}
        </div>
        <div style={{ display: "flex", gap: 12, alignItems: "baseline" }}>
          {indexFeedback && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                color: "var(--color-garden-sage)",
              }}
            >
              {indexFeedback}
            </span>
          )}
          {onIndexFiles && (
            <button
              onClick={onIndexFiles}
              disabled={indexing}
              style={{
                ...monoActionButtonStyle,
                cursor: indexing ? "default" : "pointer",
              }}
            >
              {indexing ? "Indexing…" : "Re-index"}
            </button>
          )}
        </div>
      </div>
      {files.length > 0 ? (
        <>
          <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
            {visibleFiles.map((f, idx) => (
              <li
                key={f.id}
                onClick={() =>
                  invoke("reveal_in_finder", { path: f.absolutePath })
                }
                style={{
                  display: "grid",
                  gridTemplateColumns: "20px 1fr auto auto",
                  gap: 10,
                  padding: "7px 0",
                  borderBottom:
                    idx === visibleFiles.length - 1
                      ? "none"
                      : "1px solid var(--color-rule-light)",
                  alignItems: "center",
                  fontSize: 13,
                  cursor: "pointer",
                }}
              >
                <span>
                  <FileIcon />
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {f.filename}
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    color: "var(--color-text-tertiary)",
                  }}
                >
                  {formatFileSize(f.fileSize)}
                </span>
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    color: "var(--color-text-tertiary)",
                  }}
                >
                  {formatRelativeDate(f.modifiedAt)}
                </span>
              </li>
            ))}
          </ul>
          {hasMore && !expanded && (
            <button
              onClick={() => setExpanded(true)}
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 4,
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.06em",
                color: "var(--color-text-tertiary)",
                cursor: "pointer",
                padding: "6px 0",
                marginTop: 4,
                border: "none",
                background: "none",
              }}
            >
              +{files.length - 10} more files
            </button>
          )}
        </>
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
          {emptyMessage}
        </p>
      )}
    </div>
  );
}
