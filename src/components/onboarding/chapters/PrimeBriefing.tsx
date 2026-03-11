import { useState, useCallback, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowRight, Upload, FileText, Headphones, HardDrive, Loader2, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";

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
    const added: string[] = [];

    try {
      await invoke("copy_to_inbox", { paths });
      for (const p of paths) {
        const name = p.split("/").pop() ?? p;
        added.push(name);
      }
    } catch (err) {
      console.error("Failed to copy files to inbox:", err);
    }

    setFilesAdded(prev => [...prev, ...added]);
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
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="Prime Your Briefings"
        epigraph="Give DailyOS context about your work — the more it knows, the better your briefings."
      />

      {/* Path A: Drop zone */}
      <div
        onClick={handleBrowse}
        style={{
          border: `2px dashed ${dragOver ? "var(--color-spice-turmeric)" : "var(--color-rule-heavy)"}`,
          borderRadius: 8,
          padding: 32,
          textAlign: "center",
          background: dragOver ? "rgba(196, 164, 75, 0.05)" : "transparent",
          transition: "all 0.2s ease",
          cursor: "pointer",
        }}
      >
        {processing ? (
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", gap: 8 }}>
            <Loader2 size={20} className="animate-spin" style={{ color: "var(--color-text-tertiary)" }} />
            <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-tertiary)" }}>
              Processing...
            </span>
          </div>
        ) : (
          <>
            <Upload size={24} style={{ color: "var(--color-text-tertiary)", margin: "0 auto 8px" }} />
            <p style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              fontWeight: 500,
              color: "var(--color-text-primary)",
              margin: "0 0 4px",
            }}>
              Drop files here or click to browse
            </p>
            <p style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              margin: 0,
            }}>
              .txt, .md, .pdf, .docx — meeting notes, account briefs, anything relevant
            </p>
          </>
        )}
      </div>

      {/* Files added feedback */}
      {filesAdded.length > 0 && (
        <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
          {filesAdded.map((name, i) => (
            <div key={i} style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <Check size={14} style={{ color: "var(--color-garden-sage)" }} />
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-secondary)" }}>
                {name}
              </span>
            </div>
          ))}
        </div>
      )}

      {/* Path B: Connect feeders */}
      <div style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 20 }}>
        <p style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
          margin: "0 0 12px",
        }}>
          Or connect a source
        </p>
        <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
          {[
            { icon: <Headphones size={16} />, name: "Quill", desc: "Meeting transcripts" },
            { icon: <FileText size={16} />, name: "Granola", desc: "Meeting notes" },
            { icon: <HardDrive size={16} />, name: "Google Drive", desc: "Shared documents" },
          ].map((source) => (
            <div
              key={source.name}
              style={{
                flex: "1 1 140px",
                padding: 16,
                borderTop: "1px solid var(--color-rule-light)",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4, color: "var(--color-text-primary)" }}>
                {source.icon}
                <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: 500 }}>
                  {source.name}
                </span>
              </div>
              <p style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-text-tertiary)", margin: 0 }}>
                {source.desc}
              </p>
            </div>
          ))}
        </div>
        <p style={{
          fontFamily: "var(--font-sans)",
          fontSize: 12,
          color: "var(--color-text-tertiary)",
          marginTop: 8,
        }}>
          You can set these up any time in Settings.
        </p>
      </div>

      {/* Actions */}
      <div className="flex justify-between" style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 20 }}>
        <button
          onClick={onComplete}
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: 0,
          }}
        >
          Skip — I'll add context later
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
