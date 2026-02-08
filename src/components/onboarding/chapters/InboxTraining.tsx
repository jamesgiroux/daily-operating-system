import { useState, useEffect, useCallback } from "react";
import {
  ArrowRight,
  Check,
  FileText,
  Inbox,
  Loader2,
  Sparkles,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";

export interface InboxProcessingState {
  filesDropped: number;
  filesProcessed: number;
  processingInProgress: boolean;
  results: Array<{
    filename: string;
    status: "routed" | "needs_enrichment" | "error";
    classification?: string;
    destination?: string;
  }>;
}

interface InboxTrainingProps {
  onNext: (state: InboxProcessingState) => void;
}

type FileStep = "received" | "classifying" | "classified" | "enriching";

interface FileProgress {
  filename: string;
  step: FileStep;
  classification?: string;
  needsEnrichment?: boolean;
  destination?: string;
  error?: string;
}

export function InboxTraining({ onNext }: InboxTrainingProps) {
  const [phase, setPhase] = useState<"intro" | "processing" | "done">("intro");
  const [isDragging, setIsDragging] = useState(false);
  const [files, setFiles] = useState<FileProgress[]>([]);

  // Drag-drop listener
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
              handleFileDrop(paths);
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
      // Drag-drop not available outside Tauri
    }
    return () => {
      unlisten?.();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleFileDrop = useCallback(async (paths: string[]) => {
    // Copy files to inbox
    try {
      const count = await invoke<number>("copy_to_inbox", { paths });
      if (count > 0) {
        // Get filenames from paths
        const filenames = paths
          .map((p) => p.split("/").pop() ?? p)
          .slice(0, count);
        processFiles(filenames);
      }
    } catch {
      // silently handle
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleUseSample() {
    try {
      const filename = await invoke<string>("install_inbox_sample");
      processFiles([filename]);
    } catch {
      // silently handle
    }
  }

  async function processFiles(filenames: string[]) {
    setPhase("processing");

    // Initialize file progress
    const initial: FileProgress[] = filenames.map((f) => ({
      filename: f,
      step: "received" as FileStep,
    }));
    setFiles(initial);

    // Process each file sequentially
    for (let i = 0; i < filenames.length; i++) {
      const filename = filenames[i];

      // Step: classifying
      setFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { ...f, step: "classifying" as FileStep } : f
        )
      );

      try {
        // Small delay so users see the "classifying" state
        await new Promise((r) => setTimeout(r, 500));

        const result = await invoke<{
          status: string;
          classification?: string;
          destination?: string;
          message?: string;
        }>("process_inbox_file", { filename });

        if (result.status === "routed") {
          setFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    step: "classified" as FileStep,
                    classification: result.classification,
                    destination: result.destination,
                  }
                : f
            )
          );
        } else if (result.status === "needs_enrichment") {
          setFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? {
                    ...f,
                    step: "enriching" as FileStep,
                    needsEnrichment: true,
                  }
                : f
            )
          );

          // Fire AI enrichment in background (don't await)
          invoke("enrich_inbox_file", { filename }).catch(() => {});
        } else if (result.status === "error") {
          setFiles((prev) =>
            prev.map((f, idx) =>
              idx === i
                ? { ...f, step: "classified" as FileStep, error: result.message }
                : f
            )
          );
        }
      } catch {
        setFiles((prev) =>
          prev.map((f, idx) =>
            idx === i
              ? { ...f, step: "classified" as FileStep, error: "Processing failed" }
              : f
          )
        );
      }
    }

    setPhase("done");
  }

  function buildState(): InboxProcessingState {
    return {
      filesDropped: files.length,
      filesProcessed: files.filter(
        (f) => f.step === "classified" || f.step === "enriching"
      ).length,
      processingInProgress: files.some((f) => f.step === "enriching"),
      results: files.map((f) => ({
        filename: f.filename,
        status: f.error
          ? ("error" as const)
          : f.needsEnrichment
            ? ("needs_enrichment" as const)
            : ("routed" as const),
        classification: f.classification,
        destination: f.destination,
      })),
    };
  }

  // Phase A: Introduction
  if (phase === "intro") {
    return (
      <div className="space-y-6">
        <div className="space-y-2">
          <h2 className="text-2xl font-semibold tracking-tight">
            Drop things in, intelligence comes out.
          </h2>
        </div>

        <p className="text-sm text-muted-foreground leading-relaxed">
          Every productivity app teaches you to manage. DailyOS flips the script.
          Drop meeting notes, transcripts, or documents into your inbox — the system
          classifies, extracts actions, and routes them automatically. This is the core behavior.
        </p>

        {/* Drop zone */}
        <div
          className={`flex flex-col items-center justify-center gap-3 rounded-lg border-2 border-dashed p-8 transition-colors ${
            isDragging
              ? "border-primary bg-primary/5"
              : "border-muted-foreground/20"
          }`}
        >
          <Inbox className="size-8 text-muted-foreground/40" />
          <p className="text-sm text-muted-foreground">
            Drag a file here to try it
          </p>
        </div>

        <div className="flex items-center gap-2 justify-center">
          <span className="text-xs text-muted-foreground">or</span>
        </div>

        <Button
          variant="outline"
          className="w-full"
          onClick={handleUseSample}
        >
          <FileText className="mr-2 size-4" />
          Use example meeting notes
        </Button>

        {/* Skip */}
        <div className="flex justify-end">
          <button
            className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            onClick={() =>
              onNext({
                filesDropped: 0,
                filesProcessed: 0,
                processingInProgress: false,
                results: [],
              })
            }
          >
            Skip
          </button>
        </div>
      </div>
    );
  }

  // Phase B / C: Processing / Done
  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">
          {phase === "done" ? "Processing complete" : "Processing your file..."}
        </h2>
      </div>

      {/* File progress */}
      <div className="space-y-3">
        {files.map((file) => (
          <div
            key={file.filename}
            className="rounded-lg border bg-muted/30 p-4 space-y-2"
          >
            <div className="flex items-center gap-2">
              <FileText className="size-4 text-muted-foreground shrink-0" />
              <span className="text-sm font-medium truncate">
                {file.filename}
              </span>
            </div>

            <div className="space-y-1.5 pl-6">
              {/* Step 1: File received */}
              <StepIndicator
                label="File received"
                status={file.step === "received" ? "current" : "done"}
              />

              {/* Step 2: Classifying */}
              {file.step !== "received" && (
                <StepIndicator
                  label="Classifying..."
                  status={
                    file.step === "classifying"
                      ? "current"
                      : file.step === "classified" || file.step === "enriching"
                        ? "done"
                        : "pending"
                  }
                />
              )}

              {/* Step 3: Result */}
              {(file.step === "classified" || file.step === "enriching") && (
                <StepIndicator
                  label={
                    file.error
                      ? file.error
                      : file.needsEnrichment
                        ? "Needs AI analysis"
                        : `Classified as ${formatClassification(file.classification)}`
                  }
                  status={file.error ? "error" : "done"}
                />
              )}

              {/* Step 4: AI enrichment */}
              {file.step === "enriching" && (
                <StepIndicator
                  label="AI enrichment running..."
                  status="current"
                />
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Continue */}
      {phase === "done" && (
        <div className="space-y-3">
          {files.some((f) => f.step === "enriching") && (
            <p className="text-xs text-muted-foreground text-center">
              AI enrichment continues in the background — check your Inbox page in a minute.
            </p>
          )}
          <Button className="w-full" onClick={() => onNext(buildState())}>
            Continue
            <ArrowRight className="ml-2 size-4" />
          </Button>
        </div>
      )}
    </div>
  );
}

function StepIndicator({
  label,
  status,
}: {
  label: string;
  status: "pending" | "current" | "done" | "error";
}) {
  return (
    <div className="flex items-center gap-2 text-xs">
      {status === "done" && (
        <Check className="size-3 text-green-600 shrink-0" />
      )}
      {status === "current" && (
        <Loader2 className="size-3 animate-spin text-primary shrink-0" />
      )}
      {status === "pending" && (
        <div className="size-3 rounded-full border shrink-0" />
      )}
      {status === "error" && (
        <Sparkles className="size-3 text-amber-500 shrink-0" />
      )}
      <span
        className={
          status === "done"
            ? "text-foreground"
            : status === "error"
              ? "text-amber-600"
              : "text-muted-foreground"
        }
      >
        {label}
      </span>
    </div>
  );
}

function formatClassification(c?: string): string {
  if (!c) return "document";
  return c
    .replace(/_/g, " ")
    .replace(/\b\w/g, (l) => l.toUpperCase());
}
