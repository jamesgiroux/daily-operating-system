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
import s from "./FileListSection.module.css";

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
      className={s.fileIcon}
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
    <div className={s.section}>
      <div className={s.header}>
        <div className={s.sectionTitle}>
          Files{files.length > 0 ? ` \u00B7 ${files.length}` : ""}
        </div>
        <div className={s.headerActions}>
          {indexFeedback && (
            <span className={s.indexFeedback}>
              {indexFeedback}
            </span>
          )}
          {onIndexFiles && (
            <button
              onClick={onIndexFiles}
              disabled={indexing}
              className={s.actionButton}
            >
              {indexing ? "Indexing…" : "Re-index"}
            </button>
          )}
        </div>
      </div>
      {files.length > 0 ? (
        <>
          <ul className={s.fileList}>
            {visibleFiles.map((f) => (
              <li
                key={f.id}
                onClick={() =>
                  invoke("reveal_in_finder", { path: f.absolutePath })
                }
                className={s.fileRow}
              >
                <span>
                  <FileIcon />
                </span>
                <span className={s.filename}>
                  {f.filename}
                </span>
                <span className={s.fileMeta}>
                  {formatFileSize(f.fileSize)}
                </span>
                <span className={s.fileMeta}>
                  {formatRelativeDate(f.modifiedAt)}
                </span>
              </li>
            ))}
          </ul>
          {hasMore && !expanded && (
            <button
              onClick={() => setExpanded(true)}
              className={s.showMoreButton}
            >
              +{files.length - 10} more files
            </button>
          )}
        </>
      ) : (
        <p className={s.emptyMessage}>
          {emptyMessage}
        </p>
      )}
    </div>
  );
}
