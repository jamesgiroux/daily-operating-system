import { useState, useCallback, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useInbox } from "@/hooks/useInbox";
import type { InboxFile } from "@/types";
import { cn } from "@/lib/utils";
import {
  AlertCircle,
  Building2,
  Calendar,
  CheckSquare,
  ChevronDown,
  ChevronRight,
  Download,
  FileText,
  Lightbulb,
  RefreshCw,
  X,
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
// File classification (display-side, mirrors Rust classifier patterns)
// =============================================================================

interface FileClassification {
  type: "meeting" | "actions" | "account" | "context" | "unknown";
  label: string;
  icon: LucideIcon;
  borderClass: string;
}

const classifications: Record<FileClassification["type"], Omit<FileClassification, "type">> = {
  meeting:  { label: "Meeting Notes", icon: Calendar,    borderClass: "border-l-primary" },
  actions:  { label: "Action Items",  icon: CheckSquare, borderClass: "border-l-success" },
  account:  { label: "Account",       icon: Building2,   borderClass: "border-l-primary" },
  context:  { label: "Context",       icon: Lightbulb,   borderClass: "border-l-muted-foreground/30" },
  unknown:  { label: "File",          icon: FileText,    borderClass: "" },
};

function classifyByFilename(filename: string): FileClassification {
  const lower = filename.toLowerCase();
  let type: FileClassification["type"] = "unknown";

  if (lower.includes("meeting") || lower.includes("notes") || lower.includes("sync") || lower.includes("standup")) {
    type = "meeting";
  } else if (lower.includes("action") || lower.includes("todo") || lower.includes("task")) {
    type = "actions";
  } else if (lower.includes("account") || lower.includes("dashboard") || lower.includes("customer")) {
    type = "account";
  } else if (lower.includes("context") || lower.includes("brief") || lower.includes("prep")) {
    type = "context";
  }

  return { type, ...classifications[type] };
}

/** Turn `acme-corp-meeting-notes-2026-02-05.md` into `Acme Corp Meeting Notes` */
function humanizeFilename(filename: string): string {
  // Strip extension
  const base = filename.replace(/\.md$/i, "");
  // Strip trailing date patterns (YYYY-MM-DD, YYYYMMDD)
  const withoutDate = base.replace(/[-_]?\d{4}[-_]?\d{2}[-_]?\d{2}$/, "");
  // Replace hyphens/underscores with spaces, title-case
  return (withoutDate || base)
    .replace(/[-_]+/g, " ")
    .trim()
    .replace(/\b\w/g, (c) => c.toUpperCase());
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
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
  // Expand / collapse — click the card
  // ---------------------------------------------------------------------------
  const toggleExpand = useCallback(
    async (filename: string) => {
      // Read current state directly to avoid stale closure
      const current = fileStates[filename] ?? defaultFileState;

      if (current.expanded) {
        updateFileState(filename, { expanded: false });
        return;
      }

      // If we already have content, just expand
      if (current.content) {
        updateFileState(filename, { expanded: true });
        return;
      }

      // Load content
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
  // Process single file: quick classify → AI enrich if needed
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
  // Process all: batch classify → AI enrich remainders sequentially
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
  // Derived state
  // ---------------------------------------------------------------------------
  const processingCount = files.filter(
    (f) => getFileState(f.filename).status === "processing"
  ).length;
  const allProcessing = processingCount === files.length && files.length > 0;

  // ---------------------------------------------------------------------------
  // Loading state
  // ---------------------------------------------------------------------------
  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-32" />
          <Skeleton className="h-4 w-48" />
        </div>
        <div className="space-y-3">
          {[1, 2, 3].map((i) => (
            <div key={i} className="flex gap-4 rounded-lg border p-5">
              <Skeleton className="size-10 shrink-0 rounded-lg" />
              <div className="flex-1 space-y-2">
                <Skeleton className="h-5 w-48" />
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-3 w-24" />
              </div>
            </div>
          ))}
        </div>
      </main>
    );
  }

  // ---------------------------------------------------------------------------
  // Error state
  // ---------------------------------------------------------------------------
  if (error) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="size-5" />
              <p>{error}</p>
            </div>
          </CardContent>
        </Card>
      </main>
    );
  }

  // ---------------------------------------------------------------------------
  // Render
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

          {/* Header */}
          <div className="mb-6 flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">Inbox</h1>
              <p className="text-sm text-muted-foreground">
                {processingAll
                  ? `Processing ${files.length} file${files.length === 1 ? "" : "s"}...`
                  : files.length > 0
                    ? `${files.length} file${files.length === 1 ? "" : "s"} to process`
                    : "Files dropped here get picked up by DailyOS"}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {processingAll ? (
                <Button variant="outline" size="sm" onClick={cancelAll}>
                  <X className="mr-1.5 size-3.5" />
                  Cancel
                </Button>
              ) : (
                files.length > 0 && (
                  <Button variant="outline" size="sm" onClick={processAll}>
                    <Zap className="mr-1.5 size-3.5" />
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
                <RefreshCw className={cn("size-4", refreshing && "animate-spin")} />
              </Button>
            </div>
          </div>

          {/* Drop zone — always visible above files */}
          {files.length > 0 && (
            <div
              className={cn(
                "mb-4 flex items-center justify-center gap-2 rounded-lg border-2 border-dashed py-3 text-sm transition-all",
                isDragging
                  ? "border-primary bg-accent/50 py-8 text-primary"
                  : "border-border/40 text-muted-foreground/40"
              )}
            >
              <Download className="size-4" />
              {isDragging ? "Drop to add" : "Drop files to add"}
            </div>
          )}

          {/* Processing banner */}
          {allProcessing && (
            <Card className="mb-4">
              <CardContent className="flex flex-col items-center justify-center py-10 text-center">
                <div className="mb-4 h-1.5 w-48 overflow-hidden rounded-full bg-muted">
                  <div className="animate-heartbeat h-full w-full rounded-full bg-primary" />
                </div>
                <p className="text-sm text-muted-foreground">{processingQuote}</p>
              </CardContent>
            </Card>
          )}

          {/* Result banner */}
          {resultBanner && !processingAll && (
            <div className="mb-4 flex items-center justify-between rounded-md border bg-muted/50 px-4 py-3">
              <div className="flex items-center gap-4 text-sm">
                {resultBanner.routed > 0 && (
                  <span className="text-success">
                    {resultBanner.routed} file{resultBanner.routed === 1 ? "" : "s"} processed
                  </span>
                )}
                {resultBanner.errors > 0 && (
                  <span className="text-destructive">{resultBanner.errors} failed</span>
                )}
                {resultBanner.routed === 0 && resultBanner.errors === 0 && (
                  <span className="text-muted-foreground">No files to process</span>
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

          {/* File list / empty state */}
          {files.length === 0 ? (
            <div
              className={cn(
                "flex flex-col items-center justify-center rounded-xl border-2 border-dashed py-16 text-center transition-all",
                isDragging
                  ? "border-primary bg-accent/50"
                  : "border-border bg-card"
              )}
            >
              <Download
                className={cn(
                  "mb-4 size-12",
                  isDragging ? "text-primary" : "text-muted-foreground/30"
                )}
              />
              <p className="text-lg font-medium">
                {isDragging ? "Drop files here" : "Inbox is clear"}
              </p>
              <p className="mt-1 text-sm text-muted-foreground">
                {isDragging
                  ? "Files will be copied to your inbox for processing"
                  : "Drag files here or drop them into _inbox/"}
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {files.map((file) => {
                const state = getFileState(file.filename);
                if (state.status === "processed") return null;
                return (
                  <InboxFileCard
                    key={file.filename}
                    file={file}
                    state={state}
                    processingAll={processingAll}
                    onToggleExpand={() => toggleExpand(file.filename)}
                    onProcess={() => processFile(file.filename)}
                    onCancel={() => cancelFile(file.filename)}
                  />
                );
              })}
            </div>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

// =============================================================================
// Inbox File Card — content-first design
// =============================================================================

function InboxFileCard({
  file,
  state,
  processingAll,
  onToggleExpand,
  onProcess,
  onCancel,
}: {
  file: InboxFile;
  state: FileState;
  processingAll: boolean;
  onToggleExpand: () => void;
  onProcess: () => void;
  onCancel: () => void;
}) {
  const isProcessing = state.status === "processing";
  const isError = state.status === "error";
  const classification = classifyByFilename(file.filename);
  const title = humanizeFilename(file.filename);
  const Icon = classification.icon;

  const meta: string[] = [];
  if (file.modified) meta.push(formatModified(file.modified));
  if (file.sizeBytes) meta.push(formatFileSize(file.sizeBytes));

  return (
    <Card
      className={cn(
        "overflow-hidden transition-all",
        !isProcessing && "hover:-translate-y-0.5 hover:shadow-md",
        classification.borderClass && `border-l-4 ${classification.borderClass}`,
      )}
    >
      <CardContent className="p-0">
        {/* Clickable card body */}
        <button
          type="button"
          onClick={onToggleExpand}
          disabled={isProcessing}
          className={cn(
            "flex w-full items-start gap-4 p-5 text-left",
            !isProcessing && "cursor-pointer",
          )}
        >
          {/* Icon */}
          <div
            className={cn(
              "flex size-10 shrink-0 items-center justify-center rounded-lg",
              isProcessing
                ? "animate-pulse bg-primary/10 text-primary"
                : "bg-muted text-muted-foreground"
            )}
          >
            <Icon className="size-5" />
          </div>

          {/* Content */}
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex items-start justify-between gap-3">
              <div className="min-w-0 flex-1">
                <h3 className={cn(
                  "font-medium leading-snug",
                  isProcessing && "text-muted-foreground"
                )}>
                  {title}
                </h3>
                <div className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
                  <span>{classification.label}</span>
                  {meta.length > 0 && (
                    <>
                      <span className="text-border">·</span>
                      <span>{meta.join(" · ")}</span>
                    </>
                  )}
                </div>
              </div>

              {/* Status badges — only for non-default states */}
              <div className="flex shrink-0 items-center gap-2">
                {isProcessing && (
                  <Badge variant="default" className="text-xs">Processing</Badge>
                )}
                {isError && (
                  <Badge variant="destructive" className="text-xs">Error</Badge>
                )}
                {state.expanded ? (
                  <ChevronDown className="size-4 text-muted-foreground" />
                ) : (
                  <ChevronRight className="size-4 text-muted-foreground/40" />
                )}
              </div>
            </div>

            {/* Preview — always shown when collapsed, not processing */}
            {!isProcessing && !state.expanded && file.preview && (
              <p className="line-clamp-2 text-sm leading-relaxed text-muted-foreground">
                {file.preview}
              </p>
            )}
          </div>
        </button>

        {/* Action bar — outside the button to avoid nested interactives */}
        <div className="flex items-center justify-between border-t px-5 py-2">
          <span className="truncate text-xs text-muted-foreground/50 font-mono">
            {file.filename}
          </span>
          <div className="flex shrink-0 gap-1">
            {isProcessing ? (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => { e.stopPropagation(); onCancel(); }}
                className="h-7 text-xs text-muted-foreground hover:text-foreground"
              >
                <X className="mr-1 size-3" />
                Cancel
              </Button>
            ) : (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => { e.stopPropagation(); onProcess(); }}
                disabled={state.status === "processed" || processingAll}
                className="h-7 text-xs"
              >
                <Zap className="mr-1 size-3" />
                Process
              </Button>
            )}
          </div>
        </div>

        {/* Error message */}
        {isError && state.error && (
          <div className="border-t border-destructive/20 bg-destructive/5 px-5 py-3 text-sm text-destructive">
            {state.error}
          </div>
        )}

        {/* Expanded content */}
        {!isProcessing && state.expanded && (
          <div className="border-t bg-muted/30 px-5 py-4">
            {state.loadingContent ? (
              <div className="space-y-2">
                <Skeleton className="h-4 w-full" />
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
              </div>
            ) : (
              <pre className="max-h-64 overflow-auto whitespace-pre-wrap font-mono text-sm leading-relaxed text-muted-foreground">
                {state.content
                  ? state.content.length > 2000
                    ? state.content.slice(0, 2000) + "\n\n... (truncated)"
                    : state.content
                  : "No content available."}
              </pre>
            )}
          </div>
        )}

        {/* Heartbeat progress bar */}
        {isProcessing && (
          <div className="h-1 w-full bg-muted">
            <div className="animate-heartbeat h-full w-full rounded-full bg-primary" />
          </div>
        )}
      </CardContent>
    </Card>
  );
}
