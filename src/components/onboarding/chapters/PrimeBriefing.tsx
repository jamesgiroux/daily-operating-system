import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowRight, Upload, FileText, Headphones, HardDrive, Loader2, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import styles from "../onboarding.module.css";

interface PrimeBriefingProps {
  onComplete: () => void;
}

const VALID_EXTENSIONS = ["txt", "md", "pdf", "docx"];

function hasValidExtension(path: string): boolean {
  const lower = path.toLowerCase();
  return VALID_EXTENSIONS.some(ext => lower.endsWith(`.${ext}`));
}

export function PrimeBriefing({ onComplete }: PrimeBriefingProps) {
  const [processing, setProcessing] = useState(false);
  const [filesAdded, setFilesAdded] = useState<string[]>([]);
  const [dragOver, setDragOver] = useState(false);

  // Tauri native drag-drop listener (provides real file paths)
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    try {
      getCurrentWebview()
        .onDragDropEvent((event) => {
          if (event.payload.type === "over") {
            setDragOver(true);
          } else if (event.payload.type === "leave") {
            setDragOver(false);
          } else if (event.payload.type === "drop") {
            setDragOver(false);
            const paths = event.payload.paths;
            if (paths && paths.length > 0) {
              const valid = paths.filter(hasValidExtension);
              if (valid.length > 0) {
                handleFilePaths(valid);
              }
            }
          }
        })
        .then((fn) => {
          unlisten = fn;
        })
        .catch((err) => console.error("listen drag-drop failed:", err));
    } catch {
      // Drag-drop not available outside Tauri webview
    }

    return () => {
      unlisten?.();
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const handleFilePaths = useCallback(async (paths: string[]) => {
    setProcessing(true);

    try {
      const copiedCount = await invoke<number>("copy_to_inbox", { paths });
      if (copiedCount > 0) {
        const names = paths.map(p => p.split("/").pop() ?? p);
        setFilesAdded(prev => [...prev, ...names.slice(0, copiedCount)]);
      } else {
        console.warn("No files were copied — they may be outside permitted directories");
      }
    } catch (err) {
      console.error("Failed to copy files to inbox:", err);
    }

    setProcessing(false);
  }, []);

  const handleBrowse = useCallback(async () => {
    try {
      const selected = await open({
        multiple: true,
        filters: [{ name: "Documents", extensions: VALID_EXTENSIONS }],
      });
      if (!selected) return;

      const paths = Array.isArray(selected) ? selected : [selected];
      await handleFilePaths(paths);
    } catch {
      // User cancelled
    }
  }, [handleFilePaths]);

  return (
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Prime Your Briefings"
        epigraph="Give DailyOS context about your work — the more it knows, the better your briefings."
      />

      {/* Path A: Drop zone */}
      <div
        onClick={handleBrowse}
        className={`${styles.dropZone} ${dragOver ? styles.dropZoneActive : ""}`}
      >
        {processing ? (
          <div className={styles.flexCenterGap8}>
            <Loader2 size={20} className={`animate-spin ${styles.tertiaryText}`} />
            <span className={styles.processingIndicator}>
              Processing...
            </span>
          </div>
        ) : (
          <>
            <Upload size={24} className={styles.uploadIcon} />
            <p className={styles.dropZoneLabel}>
              Drop files here or click to browse
            </p>
            <p className={styles.dropZoneHint}>
              .txt, .md, .pdf, .docx — meeting notes, account briefs, anything relevant
            </p>
          </>
        )}
      </div>

      {/* Files added feedback */}
      {filesAdded.length > 0 && (
        <div className={`${styles.flexCol} ${styles.gap8}`}>
          {filesAdded.map((name, i) => (
            <div key={i} className={`${styles.flexRow} ${styles.gap8}`}>
              <Check size={14} className={styles.sageColor} />
              <span className={styles.fileAddedName}>
                {name}
              </span>
            </div>
          ))}
          <p className={styles.fileAddedMessage}>
            DailyOS is primed. Context will build from what you just gave it, and from your connectors as they run.
          </p>
        </div>
      )}

      {/* Path B: Connect feeders */}
      <div className={styles.ruleSection}>
        <p className={styles.sectionLabelLg}>
          Or connect a source
        </p>
        <div className={styles.flexWrapRow}>
          {[
            { icon: <Headphones size={16} />, name: "Quill", desc: "Meeting transcripts" },
            { icon: <FileText size={16} />, name: "Granola", desc: "Meeting notes" },
            { icon: <HardDrive size={16} />, name: "Google Drive", desc: "Shared documents" },
          ].map((source) => (
            <div
              key={source.name}
              className={styles.sourceCard}
            >
              <div className={styles.sourceHeader}>
                {source.icon}
                <span className={styles.sourceName}>
                  {source.name}
                </span>
              </div>
              <p className={styles.sourceDesc}>
                {source.desc}
              </p>
              <p className={styles.sourceComingSoon}>
                Coming soon
              </p>
            </div>
          ))}
        </div>
        <p className={styles.settingsHint}>
          You can set these up any time in Settings.
        </p>
      </div>

      {/* Actions */}
      <div className={`flex justify-between ${styles.ruleSection}`}>
        <button
          onClick={onComplete}
          className={styles.skipLink}
        >
          Skip — I'll feed it manually later
        </button>
        <Button onClick={onComplete}>
          Go to Dashboard
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>

      <FinisMarker />
    </div>
  );
}
