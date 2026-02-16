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
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
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
    try {
      const count = await invoke<number>("copy_to_inbox", { paths });
      if (count > 0) {
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

    const initial: FileProgress[] = filenames.map((f) => ({
      filename: f,
      step: "received" as FileStep,
    }));
    setFiles(initial);

    for (let i = 0; i < filenames.length; i++) {
      const filename = filenames[i];

      setFiles((prev) =>
        prev.map((f, idx) =>
          idx === i ? { ...f, step: "classifying" as FileStep } : f
        )
      );

      try {
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
      <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
        <ChapterHeading
          title="Drop things in, intelligence comes out."
        />

        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
            margin: 0,
          }}
        >
          Every productivity app teaches you to manage. DailyOS flips the script.
          Drop meeting notes, transcripts, or documents into your inbox — the system
          classifies, extracts actions, and routes them automatically. This is the core behavior.
        </p>

        {/* Drop zone — editorial rules, no dashed border */}
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: 12,
            borderTop: "1px solid var(--color-rule-light)",
            borderBottom: "1px solid var(--color-rule-light)",
            padding: "32px 0",
            transition: "all 0.15s ease",
            ...(isDragging
              ? {
                  borderTopColor: "var(--color-spice-turmeric)",
                  borderBottomColor: "var(--color-spice-turmeric)",
                  background: "var(--color-paper-warm-white)",
                }
              : {}),
          }}
        >
          <Inbox size={32} style={{ color: "var(--color-text-tertiary)", opacity: 0.4 }} />
          <p style={{ fontSize: 14, color: "var(--color-text-tertiary)", margin: 0 }}>
            Drag a file here to try it
          </p>
        </div>

        <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 12 }}>
          <span style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>or</span>
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
        <div style={{ display: "flex", justifyContent: "flex-end" }}>
          <button
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.04em",
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
            }}
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
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title={phase === "done" ? "Processing complete" : "Processing your file..."}
      />

      {/* File progress */}
      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {files.map((file) => (
          <div
            key={file.filename}
            style={{
              borderTop: "1px solid var(--color-rule-light)",
              paddingTop: 16,
              display: "flex",
              flexDirection: "column",
              gap: 8,
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <FileText size={16} style={{ flexShrink: 0, color: "var(--color-text-tertiary)" }} />
              <span
                style={{
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                  whiteSpace: "nowrap",
                }}
              >
                {file.filename}
              </span>
            </div>

            <div style={{ display: "flex", flexDirection: "column", gap: 6, paddingLeft: 24 }}>
              <StepIndicator label="File received" status={file.step === "received" ? "current" : "done"} />
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
              {file.step === "enriching" && (
                <StepIndicator label="AI enrichment running..." status="current" />
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Continue */}
      {phase === "done" && (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {files.some((f) => f.step === "enriching") && (
            <p style={{ fontSize: 12, color: "var(--color-text-tertiary)", textAlign: "center" }}>
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
    <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12 }}>
      {status === "done" && (
        <Check size={12} style={{ flexShrink: 0, color: "var(--color-garden-sage)" }} />
      )}
      {status === "current" && (
        <Loader2 size={12} className="animate-spin" style={{ flexShrink: 0, color: "var(--color-spice-turmeric)" }} />
      )}
      {status === "pending" && (
        <div
          style={{
            width: 12,
            height: 12,
            borderRadius: "50%",
            border: "1px solid var(--color-rule-heavy)",
            flexShrink: 0,
          }}
        />
      )}
      {status === "error" && (
        <Sparkles size={12} style={{ flexShrink: 0, color: "var(--color-spice-terracotta)" }} />
      )}
      <span
        style={{
          color:
            status === "done"
              ? "var(--color-text-primary)"
              : status === "error"
                ? "var(--color-spice-terracotta)"
                : "var(--color-text-tertiary)",
        }}
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
