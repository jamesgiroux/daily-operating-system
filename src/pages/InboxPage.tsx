import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useSearch } from "@tanstack/react-router";
import { toast } from "sonner";
import { useInbox } from "@/hooks/useInbox";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { EditorialPageHeader } from "@/components/editorial/EditorialPageHeader";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import { GoogleDriveImportModal } from "@/components/inbox/GoogleDriveImportModal";
import type { CopyToInboxReport, InboxFile, InboxFileType } from "@/types";
import styles from "./InboxPage.module.css";

// =============================================================================
// Types
// =============================================================================

type FileStatus = "new" | "processing" | "processed" | "error";

interface FileState {
  status: FileStatus;
  error?: string;
  expanded: boolean;
  content?: string;
  loadingContent: boolean;
}

const defaultFileState: FileState = {
  status: "new",
  expanded: false,
  loadingContent: false,
};

interface ProcessingResultPayload {
  status: "routed" | "needs_enrichment" | "needs_entity" | "error";
  classification?: string;
  destination?: string;
  message?: string;
  suggestedName?: string;
}

interface PickerAccount {
  id: string;
  name: string;
  parentName?: string;
  accountType: string;
}

// =============================================================================
// File classification
// =============================================================================

interface FileClassification {
  type: string;
  label: string;
  dotColor: string;
}

const fileTypeClassifications: Record<string, Omit<FileClassification, "type">> = {
  image:       { label: "Image",       dotColor: "var(--color-garden-sage)" },
  spreadsheet: { label: "Spreadsheet", dotColor: "var(--color-spice-turmeric)" },
  document:    { label: "Document",    dotColor: "var(--color-text-tertiary)" },
  data:        { label: "Data",        dotColor: "var(--color-spice-turmeric)" },
  text:        { label: "Text",        dotColor: "var(--color-text-tertiary)" },
  other:       { label: "File",        dotColor: "var(--color-text-tertiary)" },
};

const mdClassifications: Record<string, Omit<FileClassification, "type">> = {
  meeting:  { label: "Meeting Notes", dotColor: "var(--color-spice-terracotta)" },
  actions:  { label: "Actions",       dotColor: "var(--color-spice-terracotta)" },
  account:  { label: "Account",       dotColor: "var(--color-spice-turmeric)" },
  context:  { label: "Context",       dotColor: "var(--color-garden-sage)" },
};

function classifyFile(file: InboxFile): FileClassification {
  const fileType: InboxFileType = file.fileType ?? "other";

  if (fileType !== "markdown") {
    const cls = fileTypeClassifications[fileType] ?? fileTypeClassifications.other;
    return { type: fileType, ...cls };
  }

  const lower = file.filename.toLowerCase();
  let mdType = "markdown";

  if (lower.includes("meeting") || lower.includes("notes") || lower.includes("sync") || lower.includes("standup")) {
    mdType = "meeting";
  } else if (lower.includes("action") || lower.includes("todo") || lower.includes("task")) {
    mdType = "actions";
  } else if (lower.includes("account") || lower.includes("dashboard") || lower.includes("customer")) {
    mdType = "account";
  } else if (lower.includes("context") || lower.includes("brief") || lower.includes("prep")) {
    mdType = "context";
  }

  const cls = mdClassifications[mdType] ?? { label: "Markdown", dotColor: "var(--color-text-tertiary)" };
  return { type: mdType, ...cls };
}

function humanizeFilename(filename: string): string {
  const base = filename.replace(/\.(md|txt|csv|tsv|json|yaml|yml|xml|toml|xlsx|xls|docx|doc|pdf|rtf|png|jpg|jpeg|gif|webp|svg|heic|numbers|pages|ods|odt)$/i, "");
  const withoutDate = base.replace(/[-_]?\d{4}[-_]?\d{2}[-_]?\d{2}$/, "");
  return (withoutDate || base)
    .replace(/[-_]+/g, " ")
    .trim()
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function formatModified(isoDate: string): string {
  try {
    const date = new Date(isoDate);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60_000);
    const diffHours = Math.floor(diffMs / 3_600_000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;

    return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  } catch {
    return "";
  }
}

// =============================================================================
// Processing helpers
// =============================================================================

const ENRICH_TIMEOUT_MS = 180_000;

function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error("Processing timed out — try again")), ms)
    ),
  ]);
}

const processingQuotes = [
  "Doing the boring parts. You're welcome.",
  "AI is reading your files so you don't have to.",
  "Crunching the hard stuff...",
  "Filing, sorting, processing. Living the dream.",
  "This is what peak productivity looks like.",
];

function getProcessingQuote(): string {
  return processingQuotes[Math.floor(Math.random() * processingQuotes.length)];
}

// =============================================================================
// Status formatting
// =============================================================================

function formatInboxStatus(value: string): string {
  if (value === "completed") return "Processed";
  if (value === "routed") return "Processed";
  if (value === "needs_enrichment") return "Needs AI";
  if (value === "needs_entity") return "Needs assignment";
  if (value === "error") return "Error";
  if (value === "unprocessed") return "New";
  return value.replace(/_/g, " ");
}

function statusDotColor(value: string): string {
  if (value === "completed" || value === "routed") return "var(--color-garden-sage)";
  if (value === "needs_enrichment") return "var(--color-spice-turmeric)";
  if (value === "needs_entity") return "var(--color-spice-turmeric)";
  if (value === "error") return "var(--color-spice-terracotta)";
  return "var(--color-text-tertiary)";
}

function getStatusTooltip(status: string): string {
  switch (status) {
    case "needs_enrichment": return "This file needs AI analysis to determine where it belongs";
    case "needs_entity": return "AI has analyzed this file — confirm which account or project it belongs to";
    case "processing": return "AI is currently analyzing this file";
    case "routed": return "This file has been classified and filed";
    case "completed": return "This file has been classified and filed";
    case "error": return "Something went wrong processing this file";
    default: return "";
  }
}

// =============================================================================
// Inbox Page
// =============================================================================

export default function InboxPage() {
  const { personality } = usePersonality();
  const { entityId } = useSearch({ from: "/inbox" });
  const { files, loading, error, refresh } = useInbox();
  const [refreshing, setRefreshing] = useState(false);
  const [processingAll, setProcessingAll] = useState(false);
  const [fileStates, setFileStates] = useState<Record<string, FileState>>({});
  const [resultBanner, setResultBanner] = useState<{
    routed: number;
    errors: number;
  } | null>(null);
  const cancelledRef = useRef<Set<string>>(new Set());
  const [processingQuote] = useState(getProcessingQuote);
  const [isDragging, setIsDragging] = useState(false);
  const [dropResult, setDropResult] = useState<{ count: number } | null>(null);
  const lastDropRef = useRef<{ signature: string; at: number } | null>(null);
  const [driveModalOpen, setDriveModalOpen] = useState(false);
  const [showHelp, setShowHelp] = useState(false);

  // ---------------------------------------------------------------------------
  // Tauri drag-drop listener
  // ---------------------------------------------------------------------------
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type === "over") {
            setIsDragging(true);
          } else if (event.payload.type === "drop") {
            setIsDragging(false);
            const paths = event.payload.paths;
            if (paths && paths.length > 0) {
              const uniquePaths = Array.from(new Set(paths));
              const signature = [...uniquePaths].sort().join("|");
              const now = Date.now();
              const previous = lastDropRef.current;

              // Ignore burst-duplicate drop events from the webview bridge (I203).
              if (
                previous &&
                previous.signature === signature &&
                now - previous.at < 1500
              ) {
                return;
              }
              lastDropRef.current = { signature, at: now };

              invoke<CopyToInboxReport>("copy_to_inbox", { paths: uniquePaths })
                .then((report) => {
                  if (report.copiedCount > 0) {
                    setDropResult({ count: report.copiedCount });
                    setTimeout(() => setDropResult(null), 3000);
                    refresh();
                  }
                })
                .catch((err) => {
                  console.error("copy_to_inbox failed:", err);
                  toast.error("Failed to import dropped files");
                });
            }
          } else {
            setIsDragging(false);
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch((err) => console.error("listen drag-drop failed:", err)); // Expected: system event listener setup
    } catch {
      // Drag-drop not available outside Tauri webview
    }

    return () => {
      unlisten?.();
    };
  }, [refresh]);

  // ---------------------------------------------------------------------------
  // Hydrate file states from backend processing status
  // ---------------------------------------------------------------------------
  useEffect(() => {
    setFileStates((prev) => {
      const next = { ...prev };
      for (const file of files) {
        const existing = prev[file.filename];
        // Only seed from backend if we don't already have active local state
        if (!existing || existing.status === "new") {
          if (file.processingStatus === "error") {
            next[file.filename] = {
              ...defaultFileState,
              status: "error",
              error: file.processingError,
            };
          }
        }
      }
      return next;
    });
  }, [files]);

  // ---------------------------------------------------------------------------
  // State helpers — reads from `prev` to avoid stale closures
  // ---------------------------------------------------------------------------
  const getFileState = useCallback(
    (filename: string): FileState => fileStates[filename] ?? defaultFileState,
    [fileStates]
  );

  const updateFileState = useCallback(
    (filename: string, update: Partial<FileState>) => {
      setFileStates((prev) => {
        const current = prev[filename] ?? defaultFileState;
        return { ...prev, [filename]: { ...current, ...update } };
      });
    },
    []
  );

  const handleRefresh = useCallback(async () => {
    setRefreshing(true);
    await refresh();
    setRefreshing(false);
  }, [refresh]);

  // ---------------------------------------------------------------------------
  // Cancel
  // ---------------------------------------------------------------------------
  const cancelFile = useCallback(
    (filename: string) => {
      cancelledRef.current.add(filename);
      updateFileState(filename, { status: "new", error: undefined });
    },
    [updateFileState]
  );

  const cancelAll = useCallback(() => {
    for (const file of files) {
      if (getFileState(file.filename).status === "processing") {
        cancelledRef.current.add(file.filename);
        updateFileState(file.filename, { status: "new", error: undefined });
      }
    }
    setProcessingAll(false);
  }, [files, getFileState, updateFileState]);

  // ---------------------------------------------------------------------------
  // Expand / collapse
  // ---------------------------------------------------------------------------
  const toggleExpand = useCallback(
    async (filename: string) => {
      const current = fileStates[filename] ?? defaultFileState;

      if (current.expanded) {
        updateFileState(filename, { expanded: false });
        return;
      }

      if (current.content) {
        updateFileState(filename, { expanded: true });
        return;
      }

      updateFileState(filename, { expanded: true, loadingContent: true });
      try {
        const content = await invoke<string>("get_inbox_file_content", { filename });
        updateFileState(filename, { content, loadingContent: false });
      } catch {
        updateFileState(filename, {
          content: "Failed to load file content.",
          loadingContent: false,
        });
      }
    },
    [fileStates, updateFileState]
  );

  // ---------------------------------------------------------------------------
  // Process single file
  // ---------------------------------------------------------------------------
  const processFile = useCallback(
    async (filename: string) => {
      cancelledRef.current.delete(filename);
      updateFileState(filename, { status: "processing", error: undefined });
      try {
        const result = await invoke<ProcessingResultPayload>("process_inbox_file", {
          filename,
          entityId,
        });

        if (cancelledRef.current.has(filename)) {
          cancelledRef.current.delete(filename);
          return;
        }

        if (result.status === "routed") {
          updateFileState(filename, { status: "processed" });
          setTimeout(() => refresh(), 500);
          return;
        }

        if (result.status === "needs_entity") {
          // Entity not found — leave in inbox for user to assign
          updateFileState(filename, { status: "new" });
          setTimeout(() => refresh(), 300);
          return;
        }

        if (result.status === "error") {
          updateFileState(filename, {
            status: "error",
            error: result.message || "Processing failed",
          });
          return;
        }

        // Auto-escalate to AI enrichment
        const enrichResult = await withTimeout(
          invoke<{ status: string; message?: string }>("enrich_inbox_file", {
            filename,
            entityId,
          }),
          ENRICH_TIMEOUT_MS
        );

        if (cancelledRef.current.has(filename)) {
          cancelledRef.current.delete(filename);
          return;
        }

        if (enrichResult.status === "routed" || enrichResult.status === "archived") {
          updateFileState(filename, { status: "processed" });
          setTimeout(() => refresh(), 500);
        } else if (enrichResult.status === "needs_entity") {
          // AI identified an entity that doesn't exist — refresh to show picker
          updateFileState(filename, { status: "new" });
          setTimeout(() => refresh(), 300);
        } else {
          updateFileState(filename, {
            status: "error",
            error: enrichResult.message || "Processing failed",
          });
        }
      } catch (err) {
        if (cancelledRef.current.has(filename)) {
          cancelledRef.current.delete(filename);
          return;
        }
        updateFileState(filename, {
          status: "error",
          error: err instanceof Error ? err.message : "Processing failed",
        });
      }
    },
    [entityId, updateFileState, refresh]
  );

  // ---------------------------------------------------------------------------
  // Process all
  // ---------------------------------------------------------------------------
  const processAll = useCallback(async () => {
    setProcessingAll(true);
    setResultBanner(null);
    cancelledRef.current.clear();

    for (const file of files) {
      updateFileState(file.filename, { status: "processing", error: undefined });
    }

    let routed = 0;
    let errors = 0;
    const needsEnrichment: string[] = [];

    try {
      const results = await invoke<[string, ProcessingResultPayload][]>("process_all_inbox");

      for (const [filename, result] of results) {
        if (result.status === "routed") {
          updateFileState(filename, { status: "processed" });
          routed++;
        } else if (result.status === "needs_enrichment") {
          needsEnrichment.push(filename);
        } else if (result.status === "needs_entity") {
          // Entity not found — leave in inbox for user to assign via picker
          updateFileState(filename, { status: "new" });
        } else {
          updateFileState(filename, {
            status: "error",
            error: result.message || "Failed",
          });
          errors++;
        }
      }

      for (const filename of needsEnrichment) {
        if (cancelledRef.current.has(filename)) {
          cancelledRef.current.delete(filename);
          updateFileState(filename, { status: "new" });
          continue;
        }

        try {
          const enrichResult = await withTimeout(
            invoke<{ status: string; message?: string }>("enrich_inbox_file", {
              filename,
              entityId,
            }),
            ENRICH_TIMEOUT_MS
          );

          if (cancelledRef.current.has(filename)) {
            cancelledRef.current.delete(filename);
            updateFileState(filename, { status: "new" });
            continue;
          }

          if (enrichResult.status === "routed" || enrichResult.status === "archived") {
            updateFileState(filename, { status: "processed" });
            routed++;
          } else if (enrichResult.status === "needs_entity") {
            updateFileState(filename, { status: "new" });
          } else {
            updateFileState(filename, {
              status: "error",
              error: enrichResult.message || "Processing failed",
            });
            errors++;
          }
        } catch (err) {
          if (!cancelledRef.current.has(filename)) {
            updateFileState(filename, {
              status: "error",
              error: err instanceof Error ? err.message : "Processing failed",
            });
            errors++;
          } else {
            cancelledRef.current.delete(filename);
            updateFileState(filename, { status: "new" });
          }
        }
      }

      setResultBanner({ routed, errors });
      if (routed > 0) {
        setTimeout(() => refresh(), 500);
      }
    } catch (err) {
      for (const file of files) {
        if (!cancelledRef.current.has(file.filename)) {
          updateFileState(file.filename, {
            status: "error",
            error: err instanceof Error ? err.message : "Batch processing failed",
          });
        }
      }
    } finally {
      setProcessingAll(false);
    }
  }, [entityId, files, updateFileState, refresh]);

  // ---------------------------------------------------------------------------
  // Derived
  // ---------------------------------------------------------------------------
  const visibleFiles = files.filter(
    (f) => getFileState(f.filename).status !== "processed"
  );
  const processingCount = files.filter(
    (f) => getFileState(f.filename).status === "processing"
  ).length;
  const allProcessing = processingCount === files.length && files.length > 0;

  // ---------------------------------------------------------------------------
  // Magazine shell registration
  // ---------------------------------------------------------------------------
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Inbox",
      atmosphereColor: "olive" as const,
      activePage: "dropbox" as const,
      folioActions: (
        <div className={styles.folioActions}>
          <button
            onClick={() => setDriveModalOpen(true)}
            disabled={processingAll}
            className={processingAll ? styles.folioButtonDisabled : styles.folioButtonDefault}
          >
            Google Drive
          </button>
          {processingAll ? (
            <button
              onClick={cancelAll}
              className={styles.folioButtonCancel}
            >
              Cancel
            </button>
          ) : visibleFiles.length > 0 ? (
            <button
              onClick={processAll}
              className={styles.folioButtonDefault}
            >
              Process All
            </button>
          ) : null}
          <button
            onClick={handleRefresh}
            disabled={refreshing || processingAll}
            className={refreshing || processingAll ? styles.folioButtonDisabled : styles.folioButtonDefault}
          >
            {refreshing ? "..." : "Refresh"}
          </button>
          <button className={styles.helpButton} onClick={() => setShowHelp(true)} title="How inbox works">?</button>
        </div>
      ),
    }),
    [processingAll, visibleFiles.length, cancelAll, processAll, handleRefresh, refreshing],
  );
  useRegisterMagazineShell(shellConfig);

  // ---------------------------------------------------------------------------
  // Loading
  // ---------------------------------------------------------------------------
  if (loading) {
    return <EditorialLoading count={4} />;
  }

  // ---------------------------------------------------------------------------
  // Error
  // ---------------------------------------------------------------------------
  if (error) {
    return <EditorialError message={error} onRetry={refresh} />;
  }

  // ---------------------------------------------------------------------------
  // Empty state — the drop zone IS the page
  // ---------------------------------------------------------------------------
  if (files.length === 0) {
    return (
      <div className={styles.pageContainer}>
        <EditorialPageHeader title="Inbox" scale="standard" width="standard" />

        {/* Drop zone */}
        <div className={`${styles.dropZoneLarge} ${isDragging ? styles.dropZoneLargeDragging : ""}`}>
          <p className={`${styles.dropZoneText} ${isDragging ? styles.dropZoneTextDragging : ""}`}>
            {isDragging ? "Drop files here" : getPersonalityCopy("inbox-empty", personality).title}
          </p>
          {!isDragging && (
            <>
              <p className={styles.dropZoneHint}>
                {getPersonalityCopy("inbox-empty", personality).message}
              </p>
              <button
                onClick={() => setDriveModalOpen(true)}
                className={styles.driveImportButton}
              >
                Import from Google Drive
              </button>
            </>
          )}
        </div>

        <GoogleDriveImportModal
          open={driveModalOpen}
          onClose={() => setDriveModalOpen(false)}
          onImported={refresh}
        />
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // File list
  // ---------------------------------------------------------------------------
  return (
    <div className={styles.pageContainer}>
      {/* Drop result toast */}
      {dropResult && (
        <div className={styles.dropResultToast}>
          {dropResult.count} file{dropResult.count === 1 ? "" : "s"} added to inbox
        </div>
      )}

      {/* Result banner */}
      {resultBanner && !processingAll && (
        <div className={styles.resultBanner}>
          <div className={styles.resultBannerStats}>
            {resultBanner.routed > 0 && (
              <span className={styles.resultStatProcessed}>
                {resultBanner.routed} processed
              </span>
            )}
            {resultBanner.errors > 0 && (
              <span className={styles.resultStatFailed}>
                {resultBanner.errors} failed
              </span>
            )}
            {resultBanner.routed === 0 && resultBanner.errors === 0 && (
              <span className={styles.resultStatEmpty}>
                Nothing to process
              </span>
            )}
          </div>
          <button
            onClick={() => setResultBanner(null)}
            className={styles.resultDismiss}
          >
            Dismiss
          </button>
        </div>
      )}

      <EditorialPageHeader
        title="Inbox"
        scale="standard"
        width="standard"
        meta={
          processingAll
            ? `Processing ${files.length} file${files.length === 1 ? "" : "s"}...`
            : `${visibleFiles.length} file${visibleFiles.length === 1 ? "" : "s"}`
        }
      />

      {/* DROP ZONE */}
      <div className={`${styles.dropZoneCompact} ${isDragging ? styles.dropZoneCompactDragging : ""}`}>
        <span className={isDragging ? styles.dropZoneTextCompactDragging : styles.dropZoneTextCompact}>
          {isDragging ? "Drop to add" : "Drop files here"}
        </span>
      </div>

      {/* BATCH PROCESSING BANNER */}
      {allProcessing && (
        <div className={styles.batchBanner}>
          <div className={styles.batchProgressTrack}>
            <div className={styles.batchProgressBar} />
          </div>
          <p className={styles.batchQuote}>
            {processingQuote}
          </p>
        </div>
      )}

      {/* FILE LIST */}
      <section>
        <div className={styles.fileList}>
          {visibleFiles.map((file, i) => (
            <InboxRow
              key={file.filename}
              file={file}
              state={getFileState(file.filename)}
              processingAll={processingAll}
              isLast={i === visibleFiles.length - 1}
              onToggleExpand={() => toggleExpand(file.filename)}
              onProcess={() => processFile(file.filename)}
              onCancel={() => cancelFile(file.filename)}
              onAssignEntity={async (entityId: string) => {
                updateFileState(file.filename, { status: "processing" });
                try {
                  await invoke("process_inbox_file", { filename: file.filename, entityId });
                  updateFileState(file.filename, { status: "processed" });
                  setTimeout(() => refresh(), 500);
                } catch {
                  updateFileState(file.filename, { status: "error", error: "Assignment failed" });
                }
              }}
            />
          ))}
        </div>
      </section>

      {/* END MARK */}
      <FinisMarker />

      <GoogleDriveImportModal
        open={driveModalOpen}
        onClose={() => setDriveModalOpen(false)}
        onImported={refresh}
      />

      {showHelp && (
        <div className={styles.helpOverlay} onClick={() => setShowHelp(false)}>
          <div className={styles.helpCard} onClick={(e) => e.stopPropagation()}>
            <h3 className={styles.helpTitle}>How your inbox works</h3>
            <div className={styles.helpSteps}>
              <div className={styles.helpStep}>
                <span className={styles.helpStepNumber}>1</span>
                <div>
                  <strong>Drop</strong>
                  <p>Drag meeting notes, transcripts, or documents into the inbox.</p>
                </div>
              </div>
              <div className={styles.helpStep}>
                <span className={styles.helpStepNumber}>2</span>
                <div>
                  <strong>Classify</strong>
                  <p>DailyOS detects the file type and content.</p>
                </div>
              </div>
              <div className={styles.helpStep}>
                <span className={styles.helpStepNumber}>3</span>
                <div>
                  <strong>Match</strong>
                  <p>AI identifies which account or project the file relates to.</p>
                </div>
              </div>
              <div className={styles.helpStep}>
                <span className={styles.helpStepNumber}>4</span>
                <div>
                  <strong>File</strong>
                  <p>Content is routed to the right place automatically.</p>
                </div>
              </div>
            </div>
            <button className={styles.helpCloseButton} onClick={() => setShowHelp(false)}>Got it</button>
          </div>
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Inbox Row — editorial, scannable
// =============================================================================

function InboxRow({
  file,
  state,
  processingAll,
  isLast,
  onToggleExpand,
  onProcess,
  onCancel,
  onAssignEntity,
}: {
  file: InboxFile;
  state: FileState;
  processingAll: boolean;
  isLast: boolean;
  onToggleExpand: () => void;
  onProcess: () => void;
  onCancel: () => void;
  onAssignEntity: (entityId: string) => void;
}) {
  const [hovered, setHovered] = useState(false);
  const [accounts, setAccounts] = useState<PickerAccount[]>([]);
  const isProcessing = state.status === "processing";
  const isError = state.status === "error";
  const needsEntity = file.processingStatus === "needs_entity";
  const classification = classifyFile(file);
  const title = humanizeFilename(file.filename);
  const time = file.modified ? formatModified(file.modified) : "";

  // Load accounts for entity picker when file needs entity assignment
  useEffect(() => {
    if (needsEntity && accounts.length === 0) {
      invoke<PickerAccount[]>("get_accounts_for_picker")
        .then(setAccounts)
        .catch(() => {});
    }
  }, [needsEntity, accounts.length]);

  // Determine status display
  const displayStatus = isProcessing
    ? "processing"
    : isError
      ? "error"
      : file.processingStatus ?? "unprocessed";

  return (
    <div
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      {/* Row */}
      <div className={`${styles.rowContainer} ${!isLast && !state.expanded ? styles.rowBorder : ""}`}>
        {/* Colored dot */}
        <span
          className={styles.rowDot}
          style={{
            background: isProcessing
              ? "var(--color-spice-turmeric)"
              : classification.dotColor,
          }}
        />

        {/* Expand toggle + title */}
        <button
          type="button"
          onClick={onToggleExpand}
          disabled={isProcessing}
          className={styles.rowTitleButton}
        >
          <span className={`${styles.rowTitle} ${isProcessing ? styles.rowTitleProcessing : ""}`}>
            {title}
          </span>
          <span className={styles.rowClassificationLabel}>
            {classification.label}
          </span>
        </button>

        {/* Right side: status dot + label, time, actions */}
        <div className={styles.rowRight}>
          {/* Status dot + label */}
          {isProcessing ? (
            <span className={styles.statusProcessing}>
              Processing...
            </span>
          ) : (
            <span className={styles.statusGroup} title={getStatusTooltip(displayStatus)}>
              <span
                className={styles.statusDot}
                style={{ background: statusDotColor(displayStatus) }}
              />
              <span className={`${styles.statusLabel} ${isError ? styles.statusLabelError : ""}`}>
                {formatInboxStatus(displayStatus)}
              </span>
            </span>
          )}

          {/* Time */}
          {!isProcessing && !isError && time && (
            <span className={styles.rowTime}>
              {time}
            </span>
          )}

          {/* Process / Cancel button */}
          {isProcessing ? (
            <button
              onClick={onCancel}
              className={`${styles.rowCancelButton} ${hovered ? styles.rowCancelButtonVisible : ""}`}
            >
              Cancel
            </button>
          ) : (
            <button
              onClick={(e) => { e.stopPropagation(); onProcess(); }}
              disabled={processingAll}
              className={`${styles.rowProcessButton} ${hovered ? styles.rowProcessButtonHovered : ""}`}
            >
              Process
            </button>
          )}

          {/* Expand indicator */}
          <span className={`${styles.expandIndicator} ${state.expanded ? styles.expandIndicatorOpen : ""}`}>
            v
          </span>
        </div>
      </div>

      {/* Error inline */}
      {isError && state.error && (
        <div className={`${styles.errorInline} ${!isLast ? styles.rowBorder : ""}`}>
          {state.error}
        </div>
      )}

      {/* Needs entity — inline picker */}
      {needsEntity && (
        <div className={`${styles.entityPickerRow} ${!isLast ? styles.rowBorder : ""}`}>
          {file.suggestedEntityName && (
            <span className={styles.entityPickerSuggestion}>
              Suggested: {file.suggestedEntityName}
            </span>
          )}
          <select
            defaultValue=""
            onChange={(e) => {
              if (e.target.value) onAssignEntity(e.target.value);
            }}
            className={styles.entityPickerSelect}
          >
            <option value="" disabled>
              Assign to account...
            </option>
            {accounts.map((a) => (
              <option key={a.id} value={a.id}>
                {a.parentName ? `${a.parentName} > ${a.name}` : a.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Expanded content */}
      {!isProcessing && state.expanded && (
        <div className={`${styles.expandedContent} ${!isLast ? styles.rowBorder : ""}`}>
          {state.loadingContent ? (
            <div className={styles.expandedLoadingSkeleton}>
              <div className={styles.skeletonLine1} />
              <div className={styles.skeletonLine2} />
              <div className={styles.skeletonLine3} />
            </div>
          ) : (
            <>
              {/* File metadata line */}
              <div className={styles.fileMetaLine}>
                {file.filename}
              </div>
              <pre className={styles.fileContentPre}>
                {state.content
                  ? state.content.length > 2000
                    ? state.content.slice(0, 2000) + "\n\n... (truncated)"
                    : state.content
                  : "No content available."}
              </pre>
            </>
          )}
        </div>
      )}

      {/* Processing progress bar */}
      {isProcessing && (
        <div className={styles.progressTrack}>
          <div className={styles.progressBar} />
        </div>
      )}
    </div>
  );
}
