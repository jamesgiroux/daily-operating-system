import { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useInbox } from "@/hooks/useInbox";
import type { InboxFile, InboxFileType } from "@/types";
import { cn } from "@/lib/utils";
import { PageError } from "@/components/PageState";
import {
  Building2,
  Calendar,
  CheckSquare,
  ChevronDown,
  Database,
  Download,
  FileSpreadsheet,
  FileText,
  Image,
  Lightbulb,
  Loader2,
  RefreshCw,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

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
  icon: LucideIcon;
}

const fileTypeClassifications: Record<string, Omit<FileClassification, "type">> = {
  image:       { label: "Image",       icon: Image },
  spreadsheet: { label: "Spreadsheet", icon: FileSpreadsheet },
  document:    { label: "Document",    icon: FileText },
  data:        { label: "Data",        icon: Database },
  text:        { label: "Text",        icon: FileText },
  other:       { label: "File",        icon: FileText },
};

const mdClassifications: Record<string, Omit<FileClassification, "type">> = {
  meeting:  { label: "Meeting Notes", icon: Calendar },
  actions:  { label: "Actions",       icon: CheckSquare },
  account:  { label: "Account",       icon: Building2 },
  context:  { label: "Context",       icon: Lightbulb },
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

  const cls = mdClassifications[mdType] ?? { label: "Markdown", icon: FileText };
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
// Inbox Page
// =============================================================================

export default function InboxPage() {
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
              invoke<number>("copy_to_inbox", { paths })
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
        const result = await invoke<ProcessingResultPayload>("process_inbox_file", { filename });

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
          invoke<{ status: string; message?: string }>("enrich_inbox_file", { filename }),
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
    [updateFileState, refresh]
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
            invoke<{ status: string; message?: string }>("enrich_inbox_file", { filename }),
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
  }, [files, updateFileState, refresh]);

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
  // Loading
  // ---------------------------------------------------------------------------
  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-8 space-y-2">
          <Skeleton className="h-8 w-32" />
          <Skeleton className="h-4 w-48" />
        </div>
        <div className="space-y-0 divide-y rounded-lg border">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="flex items-center gap-3 px-4 py-3">
              <Skeleton className="size-5 shrink-0 rounded" />
              <Skeleton className="h-4 w-40" />
              <div className="flex-1" />
              <Skeleton className="h-3 w-16" />
            </div>
          ))}
        </div>
      </main>
    );
  }

  // ---------------------------------------------------------------------------
  // Error
  // ---------------------------------------------------------------------------
  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} onRetry={refresh} />
      </main>
    );
  }

  // ---------------------------------------------------------------------------
  // Empty state — the drop zone IS the page
  // ---------------------------------------------------------------------------
  if (files.length === 0) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="p-6">
          <div className="mb-8">
            <h1 className="text-2xl font-semibold tracking-tight">Inbox</h1>
            <p className="mt-1 text-sm text-muted-foreground">
              Files dropped here get picked up by DailyOS
            </p>
          </div>

          <div
            className={cn(
              "flex flex-col items-center justify-center rounded-xl border-2 border-dashed py-20 text-center transition-all",
              isDragging
                ? "border-primary bg-accent/50"
                : "border-border"
            )}
          >
            <Download
              className={cn(
                "mb-3 size-10",
                isDragging ? "text-primary" : "text-muted-foreground/20"
              )}
            />
            <p className="font-medium">
              {isDragging ? "Drop files here" : "Inbox is clear"}
            </p>
            <p className="mt-1 text-sm text-muted-foreground">
              {isDragging
                ? "Files will be copied to your inbox"
                : "Drag files here or drop them into _inbox/"}
            </p>
          </div>
        </div>
      </main>
    );
  }

  // ---------------------------------------------------------------------------
  // File list
  // ---------------------------------------------------------------------------
  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          {/* Drop result toast */}
          {dropResult && (
            <div className="mb-4 flex items-center gap-2 rounded-md border border-success/30 bg-success/10 px-4 py-2.5 text-sm text-success">
              <Download className="size-4" />
              {dropResult.count} file{dropResult.count === 1 ? "" : "s"} added to inbox
            </div>
          )}

          {/* Result banner */}
          {resultBanner && !processingAll && (
            <div className="mb-4 flex items-center justify-between rounded-md border bg-muted/50 px-4 py-2.5">
              <div className="flex items-center gap-4 text-sm">
                {resultBanner.routed > 0 && (
                  <span className="text-success">
                    {resultBanner.routed} processed
                  </span>
                )}
                {resultBanner.errors > 0 && (
                  <span className="text-destructive">{resultBanner.errors} failed</span>
                )}
                {resultBanner.routed === 0 && resultBanner.errors === 0 && (
                  <span className="text-muted-foreground">Nothing to process</span>
                )}
              </div>
              <button
                onClick={() => setResultBanner(null)}
                className="text-xs text-muted-foreground hover:text-foreground"
              >
                Dismiss
              </button>
            </div>
          )}

          {/* Header */}
          <div className="mb-6 flex items-end justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">Inbox</h1>
              <p className="mt-1 text-sm text-muted-foreground">
                {processingAll
                  ? `Processing ${files.length} file${files.length === 1 ? "" : "s"}...`
                  : `${visibleFiles.length} file${visibleFiles.length === 1 ? "" : "s"}`}
              </p>
            </div>
            <div className="flex items-center gap-1.5">
              {processingAll ? (
                <Button variant="ghost" size="sm" onClick={cancelAll} className="h-8 text-xs">
                  Cancel
                </Button>
              ) : (
                visibleFiles.length > 0 && (
                  <Button variant="ghost" size="sm" onClick={processAll} className="h-8 text-xs">
                    <Zap className="mr-1 size-3.5" />
                    Process All
                  </Button>
                )
              )}
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={handleRefresh}
                disabled={refreshing || processingAll}
              >
                <RefreshCw className={cn("size-3.5", refreshing && "animate-spin")} />
              </Button>
            </div>
          </div>

          {/* Drop zone */}
          <div
            className={cn(
              "mb-5 flex flex-col items-center justify-center gap-1.5 rounded-lg border bg-card py-8 text-sm transition-all",
              isDragging
                ? "border-primary bg-accent/60 text-primary"
                : "text-muted-foreground"
            )}
          >
            <Download className={cn("size-5", isDragging ? "text-primary" : "text-muted-foreground/40")} />
            <span>{isDragging ? "Drop to add" : "Drop files here"}</span>
          </div>

          {/* Batch processing banner */}
          {allProcessing && (
            <div className="mb-5 flex flex-col items-center py-6 text-center">
              <div className="mb-3 h-1 w-40 overflow-hidden rounded-full bg-muted">
                <div className="animate-heartbeat h-full w-full rounded-full bg-primary" />
              </div>
              <p className="text-sm text-muted-foreground">{processingQuote}</p>
            </div>
          )}

          {/* File list — compact rows inside a single container */}
          <div className="overflow-hidden rounded-lg border">
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
        </div>
      </ScrollArea>
    </main>
  );
}

// =============================================================================
// Inbox Row — compact, scannable
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
  const isProcessing = state.status === "processing";
  const isError = state.status === "error";
  const classification = classifyFile(file);
  const title = humanizeFilename(file.filename);
  const Icon = classification.icon;
  const time = file.modified ? formatModified(file.modified) : "";

  return (
    <div className={cn(!isLast && !state.expanded && "border-b")}>
      {/* Row — the main scannable line */}
      <div className="group flex items-center gap-3 px-4 py-3">
        {/* Icon */}
        <Icon
          className={cn(
            "size-[18px] shrink-0",
            isProcessing ? "text-primary" : "text-muted-foreground/50"
          )}
        />

        {/* Expand toggle + title */}
        <button
          type="button"
          onClick={onToggleExpand}
          disabled={isProcessing}
          className="flex min-w-0 flex-1 items-center gap-2 text-left"
        >
          <span className={cn(
            "truncate text-sm",
            isProcessing && "text-muted-foreground",
          )}>
            {title}
          </span>
          <span className="shrink-0 text-xs text-muted-foreground/50">
            {classification.label}
          </span>
        </button>

        {/* Right side: status / time / actions */}
        <div className="flex shrink-0 items-center gap-2">
          {isProcessing && (
            <Loader2 className="size-3.5 animate-spin text-primary" />
          )}

          {isError && (
            <Badge variant="destructive" className="h-5 text-[10px] px-1.5">Error</Badge>
          )}

          {!isProcessing && !isError && time && (
            <span className="text-xs text-muted-foreground/40">{time}</span>
          )}

          {/* Process button — visible on hover or always on touch */}
          {isProcessing ? (
            <button
              onClick={onCancel}
              className="text-xs text-muted-foreground/40 opacity-0 transition-opacity hover:text-foreground group-hover:opacity-100"
            >
              Cancel
            </button>
          ) : (
            <button
              onClick={(e) => { e.stopPropagation(); onProcess(); }}
              disabled={processingAll}
              className={cn(
                "flex items-center gap-1 rounded-md px-2 py-1 text-xs transition-all",
                "text-muted-foreground/50 hover:bg-muted hover:text-foreground",
                "opacity-0 group-hover:opacity-100",
                // Always visible on touch / when focused
                "focus-visible:opacity-100",
              )}
            >
              <Zap className="size-3" />
              Process
            </button>
          )}

          {/* Expand chevron */}
          <ChevronDown
            className={cn(
              "size-3.5 text-muted-foreground/30 transition-transform",
              state.expanded && "rotate-180"
            )}
          />
        </div>
      </div>

      {/* Error inline */}
      {isError && state.error && (
        <div className="border-t border-destructive/10 bg-destructive/5 px-4 py-2 text-xs text-destructive">
          {state.error}
        </div>
      )}

      {/* Expanded content */}
      {!isProcessing && state.expanded && (
        <div className={cn("border-t bg-muted/20 px-4 py-4", !isLast && "border-b")}>
          {state.loadingContent ? (
            <div className="space-y-2">
              <Skeleton className="h-3.5 w-full" />
              <Skeleton className="h-3.5 w-3/4" />
              <Skeleton className="h-3.5 w-1/2" />
            </div>
          ) : (
            <>
              {/* File metadata line */}
              <div className="mb-3 flex items-center gap-3 text-xs text-muted-foreground/50">
                <span className="font-mono">{file.filename}</span>
              </div>
              <pre className="max-h-64 overflow-auto whitespace-pre-wrap font-mono text-xs leading-relaxed text-muted-foreground">
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
        <div className="h-0.5 w-full bg-muted">
          <div className="animate-heartbeat h-full w-full rounded-full bg-primary" />
        </div>
      )}
    </div>
  );
}
