/**
 * FolioToolsDropdown.tsx
 *
 * Self-contained tools dropdown for the FolioBar actions slot.
 * Accepts action callbacks as props — no parent state closures.
 * Owns its own open/close state and click-outside handler.
 */

import { useState, useEffect } from "react";
import styles from "./FolioToolsDropdown.module.css";

interface FolioToolsDropdownProps {
  onCreateChild: () => void;
  onMerge: () => void;
  onArchive: () => void;
  onUnarchive: () => void;
  onIndexFiles: () => void;
  isArchived: boolean;
  isIndexing: boolean;
  hasDetail: boolean;
}

export function FolioToolsDropdown({
  onCreateChild,
  onMerge,
  onArchive,
  onUnarchive,
  onIndexFiles,
  isArchived,
  isIndexing,
  hasDetail,
}: FolioToolsDropdownProps) {
  const [open, setOpen] = useState(false);

  // Close on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick() { setOpen(false); }
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, [open]);

  return (
    <div className={styles.wrapper}>
      <button
        onClick={(e) => { e.stopPropagation(); setOpen(o => !o); }}
        className={styles.button}
      >
        Tools {open ? "\u25b4" : "\u25be"}
      </button>
      {open && (
        <div className={styles.dropdown}>
          {hasDetail && (
            <button
              className={styles.item}
              onClick={() => { setOpen(false); onCreateChild(); }}
            >
              + Business Unit
            </button>
          )}
          <button
            className={styles.item}
            onClick={() => { setOpen(false); onMerge(); }}
          >
            Merge Into...
          </button>
          <button
            className={styles.item}
            onClick={() => { setOpen(false); onIndexFiles(); }}
            disabled={isIndexing}
          >
            {isIndexing ? "Indexing\u2026" : "Index Files"}
          </button>
          <div className={styles.separator} />
          {isArchived ? (
            <button
              className={styles.item}
              onClick={() => { setOpen(false); onUnarchive(); }}
            >
              Unarchive
            </button>
          ) : hasDetail ? (
            <button
              className={styles.item}
              onClick={() => { setOpen(false); onArchive(); }}
            >
              Archive
            </button>
          ) : null}
        </div>
      )}
    </div>
  );
}
