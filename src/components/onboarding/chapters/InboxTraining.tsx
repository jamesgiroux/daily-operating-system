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
import styles from "../onboarding.module.css";
import type { CopyToInboxReport } from "@/types";

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
        .catch((err) => console.error("listen drag-drop failed:", err));
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
      const report = await invoke<CopyToInboxReport>("copy_to_inbox", { paths });
      if (report.copiedCount > 0) {
        processFiles(report.copiedFilenames);
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
          invoke("enrich_inbox_file", { filename }).catch((err) => console.error("enrich_inbox_file failed:", err));
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
      <div className={`${styles.flexCol} ${styles.gap24}`}>
        <ChapterHeading
          title="Drop things in, context comes out."
        />

        <p className={styles.bodyText}>
          Every productivity app teaches you to manage. DailyOS flips the script.
          Drop meeting notes, transcripts, or documents into your inbox — the system
          classifies, extracts actions, and routes them automatically. This is the core behavior.
        </p>

        {/* Drop zone — editorial rules, no dashed border */}
        <div
          className={`${styles.dropArea} ${isDragging ? styles.dropAreaActive : ""}`}
        >
          <Inbox size={32} className={styles.dropAreaIcon} />
          <p className={styles.dropAreaText}>
            Drag a file here to try it
          </p>
        </div>

        <div className={styles.flexCenter}>
          <span className={styles.hintText}>or</span>
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
        <div className={styles.flexEnd}>
          <button
            className={styles.skipButton}
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
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title={phase === "done" ? "Processing complete" : "Processing your file..."}
      />

      {/* File progress */}
      <div className={`${styles.flexCol} ${styles.gap12}`}>
        {files.map((file) => (
          <div
            key={file.filename}
            className={styles.fileProgressItem}
          >
            <div className={`${styles.flexRow} ${styles.gap8}`}>
              <FileText size={16} className={`${styles.flexShrink0} ${styles.tertiaryText}`} />
              <span className={styles.fileName}>
                {file.filename}
              </span>
            </div>

            <div className={`${styles.flexCol} ${styles.gap6} ${styles.pl24}`}>
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
                <StepIndicator label="AI analysis running..." status="current" />
              )}
            </div>
          </div>
        ))}
      </div>

      {/* Continue */}
      {phase === "done" && (
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          {files.some((f) => f.step === "enriching") && (
            <p className={`${styles.hintText} ${styles.textCenter}`}>
              AI analysis continues in the background — check your Inbox page in a minute.
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
    <div className={styles.stepIndicator}>
      {status === "done" && (
        <Check size={12} className={`${styles.flexShrink0} ${styles.sageColor}`} />
      )}
      {status === "current" && (
        <Loader2 size={12} className={`animate-spin ${styles.flexShrink0} ${styles.accentColor}`} />
      )}
      {status === "pending" && (
        <div className={styles.pendingDot} />
      )}
      {status === "error" && (
        <Sparkles size={12} className={`${styles.flexShrink0} ${styles.dangerColor}`} />
      )}
      <span
        className={
          status === "done"
            ? styles.primaryText
            : status === "error"
              ? styles.dangerColor
              : styles.tertiaryText
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
