import { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useSearch } from "@tanstack/react-router";
import { useInbox } from "@/hooks/useInbox";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import type { InboxFile, InboxFileType } from "@/types";

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
  status: "routed" | "needs_enrichment" | "error";
  classification?: string;
  destination?: string;
  message?: string;
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
  "Filing, sorting, enriching. Living the dream.",
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
  if (value === "error") return "Error";
  if (value === "unprocessed") return "New";
  return value.replace(/_/g, " ");
}

function statusDotColor(value: string): string {
  if (value === "completed" || value === "routed") return "var(--color-garden-sage)";
  if (value === "needs_enrichment") return "var(--color-spice-turmeric)";
  if (value === "error") return "var(--color-spice-terracotta)";
  return "var(--color-text-tertiary)";
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

              invoke<number>("copy_to_inbox", { paths: uniquePaths })
                .then((count) => {
                  if (count > 0) {
                    setDropResult({ count });
                    setTimeout(() => setDropResult(null), 3000);
                    refresh();
                  }
                })
                .catch(() => {});
            }
          } else {
            setIsDragging(false);
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch(() => {});
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
      folioLabel: "Dropbox",
      atmosphereColor: "olive" as const,
      activePage: "dropbox" as const,
      folioActions: (
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {processingAll ? (
            <button
              onClick={cancelAll}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase" as const,
                color: "var(--color-spice-terracotta)",
                background: "none",
                border: "1px solid var(--color-spice-terracotta)",
                borderRadius: 4,
                padding: "2px 10px",
                cursor: "pointer",
              }}
            >
              Cancel
            </button>
          ) : (
            visibleFiles.length > 0 && (
              <button
                onClick={processAll}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  fontWeight: 600,
                  letterSpacing: "0.06em",
                  textTransform: "uppercase" as const,
                  color: "var(--color-text-secondary)",
                  background: "none",
                  border: "1px solid var(--color-rule-heavy)",
                  borderRadius: 4,
                  padding: "2px 10px",
                  cursor: "pointer",
                }}
              >
                Process All
              </button>
            )
          )}
          <button
            onClick={handleRefresh}
            disabled={refreshing || processingAll}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase" as const,
              color: refreshing ? "var(--color-text-tertiary)" : "var(--color-text-secondary)",
              background: "none",
              border: "1px solid var(--color-rule-heavy)",
              borderRadius: 4,
              padding: "2px 10px",
              cursor: refreshing || processingAll ? "default" : "pointer",
              opacity: refreshing || processingAll ? 0.5 : 1,
            }}
          >
            {refreshing ? "..." : "Refresh"}
          </button>
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
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
        {/* Hero */}
        <section style={{ paddingTop: 80, paddingBottom: 24 }}>
          <h1
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 36,
              fontWeight: 400,
              letterSpacing: "-0.02em",
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Inbox
          </h1>
          <div style={{ height: 2, background: "var(--color-desk-charcoal)", marginTop: 16 }} />
        </section>

        {/* Drop zone */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            padding: "80px 0",
            border: `2px dashed ${isDragging ? "var(--color-spice-turmeric)" : "var(--color-rule-heavy)"}`,
            borderRadius: 8,
            textAlign: "center",
            transition: "border-color 0.2s ease, background 0.2s ease",
            background: isDragging ? "rgba(201, 162, 39, 0.04)" : "transparent",
          }}
        >
          <p
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 18,
              fontStyle: "italic",
              color: isDragging ? "var(--color-spice-turmeric)" : "var(--color-text-tertiary)",
              margin: 0,
            }}
          >
            {isDragging ? "Drop files here" : getPersonalityCopy("inbox-empty", personality).title}
          </p>
          {!isDragging && (
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                fontWeight: 300,
                color: "var(--color-text-tertiary)",
                marginTop: 8,
              }}
            >
              {getPersonalityCopy("inbox-empty", personality).message}
            </p>
          )}
        </div>
      </div>
    );
  }

  // ---------------------------------------------------------------------------
  // File list
  // ---------------------------------------------------------------------------
  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* Drop result toast */}
      {dropResult && (
        <div
          style={{
            position: "fixed",
            top: 80,
            left: "50%",
            transform: "translateX(-50%)",
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-garden-sage)",
            background: "var(--color-text-primary)",
            borderRadius: 6,
            padding: "8px 16px",
            zIndex: 50,
          }}
        >
          {dropResult.count} file{dropResult.count === 1 ? "" : "s"} added to inbox
        </div>
      )}

      {/* Result banner */}
      {resultBanner && !processingAll && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "10px 0",
            marginBottom: 16,
            borderBottom: "1px solid var(--color-rule-light)",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
            {resultBanner.routed > 0 && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-garden-sage)",
                }}
              >
                {resultBanner.routed} processed
              </span>
            )}
            {resultBanner.errors > 0 && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-spice-terracotta)",
                }}
              >
                {resultBanner.errors} failed
              </span>
            )}
            {resultBanner.routed === 0 && resultBanner.errors === 0 && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  color: "var(--color-text-tertiary)",
                }}
              >
                Nothing to process
              </span>
            )}
          </div>
          <button
            onClick={() => setResultBanner(null)}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              padding: 0,
            }}
          >
            Dismiss
          </button>
        </div>
      )}

      {/* ═══ HERO ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 24 }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <h1
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 36,
              fontWeight: 400,
              letterSpacing: "-0.02em",
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Inbox
          </h1>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
            }}
          >
            {processingAll
              ? `Processing ${files.length} file${files.length === 1 ? "" : "s"}...`
              : `${visibleFiles.length} file${visibleFiles.length === 1 ? "" : "s"}`}
          </span>
        </div>
        <div style={{ height: 2, background: "var(--color-desk-charcoal)", marginTop: 16 }} />
      </section>

      {/* ═══ DROP ZONE ═══ */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          padding: "20px 0",
          marginBottom: 20,
          border: `1px dashed ${isDragging ? "var(--color-spice-turmeric)" : "var(--color-rule-heavy)"}`,
          borderRadius: 6,
          transition: "border-color 0.2s ease, background 0.2s ease",
          background: isDragging ? "rgba(201, 162, 39, 0.04)" : "transparent",
        }}
      >
        <span
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 14,
            fontStyle: "italic",
            color: isDragging ? "var(--color-spice-turmeric)" : "var(--color-text-tertiary)",
          }}
        >
          {isDragging ? "Drop to add" : "Drop files here"}
        </span>
      </div>

      {/* ═══ BATCH PROCESSING BANNER ═══ */}
      {allProcessing && (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            padding: "24px 0",
            marginBottom: 20,
          }}
        >
          <div
            style={{
              width: 160,
              height: 2,
              background: "var(--color-rule-light)",
              borderRadius: 1,
              overflow: "hidden",
              marginBottom: 12,
            }}
          >
            <div
              style={{
                width: "100%",
                height: "100%",
                background: "var(--color-spice-turmeric)",
                borderRadius: 1,
                animation: "heartbeat 1.5s ease-in-out infinite",
              }}
            />
          </div>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              margin: 0,
            }}
          >
            {processingQuote}
          </p>
        </div>
      )}

      {/* ═══ FILE LIST ═══ */}
      <section>
        <div style={{ display: "flex", flexDirection: "column" }}>
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
            />
          ))}
        </div>
      </section>

      {/* ═══ END MARK ═══ */}
      {visibleFiles.length > 0 && <FinisMarker />}
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
}: {
  file: InboxFile;
  state: FileState;
  processingAll: boolean;
  isLast: boolean;
  onToggleExpand: () => void;
  onProcess: () => void;
  onCancel: () => void;
}) {
  const [hovered, setHovered] = useState(false);
  const isProcessing = state.status === "processing";
  const isError = state.status === "error";
  const classification = classifyFile(file);
  const title = humanizeFilename(file.filename);
  const time = file.modified ? formatModified(file.modified) : "";

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
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "12px 0",
          borderBottom: !isLast && !state.expanded
            ? "1px solid var(--color-rule-light)"
            : "none",
        }}
      >
        {/* Colored dot */}
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: isProcessing
              ? "var(--color-spice-turmeric)"
              : classification.dotColor,
            flexShrink: 0,
          }}
        />

        {/* Expand toggle + title */}
        <button
          type="button"
          onClick={onToggleExpand}
          disabled={isProcessing}
          style={{
            display: "flex",
            flex: 1,
            minWidth: 0,
            alignItems: "baseline",
            gap: 8,
            textAlign: "left",
            background: "none",
            border: "none",
            padding: 0,
            cursor: isProcessing ? "default" : "pointer",
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 15,
              fontWeight: 400,
              color: isProcessing ? "var(--color-text-tertiary)" : "var(--color-text-primary)",
              overflow: "hidden",
              textOverflow: "ellipsis",
              whiteSpace: "nowrap",
            }}
          >
            {title}
          </span>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              letterSpacing: "0.04em",
              color: "var(--color-text-tertiary)",
              flexShrink: 0,
              opacity: 0.6,
            }}
          >
            {classification.label}
          </span>
        </button>

        {/* Right side: status dot + label, time, actions */}
        <div style={{ display: "flex", alignItems: "center", gap: 10, flexShrink: 0 }}>
          {/* Status dot + label */}
          {isProcessing ? (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                letterSpacing: "0.04em",
                color: "var(--color-spice-turmeric)",
              }}
            >
              Processing...
            </span>
          ) : (
            <span style={{ display: "flex", alignItems: "center", gap: 5 }}>
              <span
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: "50%",
                  background: statusDotColor(displayStatus),
                  flexShrink: 0,
                }}
              />
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  letterSpacing: "0.04em",
                  color: isError
                    ? "var(--color-spice-terracotta)"
                    : "var(--color-text-tertiary)",
                }}
              >
                {formatInboxStatus(displayStatus)}
              </span>
            </span>
          )}

          {/* Time */}
          {!isProcessing && !isError && time && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                color: "var(--color-text-tertiary)",
                opacity: 0.5,
              }}
            >
              {time}
            </span>
          )}

          {/* Process / Cancel button */}
          {isProcessing ? (
            <button
              onClick={onCancel}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase" as const,
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                padding: 0,
                cursor: "pointer",
                opacity: hovered ? 1 : 0,
                transition: "opacity 0.15s ease",
              }}
            >
              Cancel
            </button>
          ) : (
            <button
              onClick={(e) => { e.stopPropagation(); onProcess(); }}
              disabled={processingAll}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase" as const,
                color: hovered ? "var(--color-text-secondary)" : "var(--color-text-tertiary)",
                background: "none",
                border: `1px solid ${hovered ? "var(--color-rule-heavy)" : "transparent"}`,
                borderRadius: 4,
                padding: "2px 8px",
                cursor: processingAll ? "default" : "pointer",
                opacity: hovered || processingAll ? 1 : 0,
                transition: "opacity 0.15s ease, border-color 0.15s ease, color 0.15s ease",
              }}
            >
              Process
            </button>
          )}

          {/* Expand indicator */}
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              color: "var(--color-text-tertiary)",
              opacity: 0.4,
              transition: "transform 0.15s ease",
              display: "inline-block",
              transform: state.expanded ? "rotate(180deg)" : "rotate(0deg)",
            }}
          >
            v
          </span>
        </div>
      </div>

      {/* Error inline */}
      {isError && state.error && (
        <div
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-spice-terracotta)",
            padding: "4px 0 8px 20px",
            borderBottom: !isLast ? "1px solid var(--color-rule-light)" : "none",
          }}
        >
          {state.error}
        </div>
      )}

      {/* Expanded content */}
      {!isProcessing && state.expanded && (
        <div
          style={{
            padding: "12px 0 16px 20px",
            borderBottom: !isLast ? "1px solid var(--color-rule-light)" : "none",
          }}
        >
          {state.loadingContent ? (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  style={{
                    height: 14,
                    width: `${100 - i * 25}%`,
                    background: "var(--color-rule-light)",
                    borderRadius: 4,
                    animation: "pulse 1.5s ease-in-out infinite",
                  }}
                />
              ))}
            </div>
          ) : (
            <>
              {/* File metadata line */}
              <div
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  color: "var(--color-text-tertiary)",
                  opacity: 0.6,
                  marginBottom: 8,
                }}
              >
                {file.filename}
              </div>
              <pre
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  lineHeight: 1.6,
                  color: "var(--color-text-secondary)",
                  whiteSpace: "pre-wrap",
                  maxHeight: 256,
                  overflow: "auto",
                  margin: 0,
                }}
              >
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
        <div
          style={{
            height: 2,
            width: "100%",
            background: "var(--color-rule-light)",
            overflow: "hidden",
          }}
        >
          <div
            style={{
              width: "100%",
              height: "100%",
              background: "var(--color-spice-turmeric)",
              borderRadius: 1,
              animation: "heartbeat 1.5s ease-in-out infinite",
            }}
          />
        </div>
      )}
    </div>
  );
}
