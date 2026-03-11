import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { EntityPicker } from "@/components/ui/entity-picker";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PickerFile {
  id: string;
  name: string;
  mimeType: string;
}

type WatchMode = "once" | "watch";

// Map Google MIME types to our DB type column
function driveTypeFromMime(mime: string): string {
  if (mime.includes("spreadsheet") || mime.includes("sheet")) return "spreadsheet";
  if (mime.includes("presentation") || mime.includes("slide")) return "presentation";
  if (mime.includes("folder")) return "folder";
  return "document";
}

// ---------------------------------------------------------------------------
// Google Picker loader
// ---------------------------------------------------------------------------

let pickerApiLoaded = false;
let gapiLoaded = false;

function loadGapi(): Promise<void> {
  if (gapiLoaded) return Promise.resolve();
  return new Promise((resolve, reject) => {
    if (document.querySelector('script[src*="apis.google.com/js/api.js"]')) {
      gapiLoaded = true;
      resolve();
      return;
    }
    const script = document.createElement("script");
    script.src = "https://apis.google.com/js/api.js";
    script.onload = () => {
      gapiLoaded = true;
      resolve();
    };
    script.onerror = () => reject(new Error("Failed to load Google API script"));
    document.head.appendChild(script);
  });
}

function loadPickerApi(): Promise<void> {
  if (pickerApiLoaded) return Promise.resolve();
  return new Promise((resolve, reject) => {
    window.gapi.load("picker", {
      callback: () => {
        pickerApiLoaded = true;
        resolve();
      },
      onerror: () => reject(new Error("Failed to load Picker API")),
    });
  });
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface GoogleDriveImportModalProps {
  open: boolean;
  onClose: () => void;
  onImported: () => void;
}

export function GoogleDriveImportModal({
  open,
  onClose,
  onImported,
}: GoogleDriveImportModalProps) {
  const [pickerFiles, setPickerFiles] = useState<PickerFile[]>([]);
  const [watchMode, setWatchMode] = useState<WatchMode>("once");
  const [entityId, setEntityId] = useState<string | null>(null);
  const [entityName, setEntityName] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [pickerOpen, setPickerOpen] = useState(false);
  const backdropRef = useRef<HTMLDivElement>(null);

  // Reset state when modal opens
  useEffect(() => {
    if (open) {
      setPickerFiles([]);
      setWatchMode("once");
      setEntityId(null);
      setEntityName(null);
      setSubmitting(false);
    }
  }, [open]);

  // Open Google Picker on mount when no files selected yet
  // But don't show the modal backdrop until the picker is done
  useEffect(() => {
    if (open && pickerFiles.length === 0 && !pickerOpen) {
      // Delay opening picker to avoid rendering both modals at once
      const timer = setTimeout(() => openPicker(), 100);
      return () => clearTimeout(timer);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  const openPicker = useCallback(async () => {
    setPickerOpen(true);
    try {
      // Get access token from backend
      let token: string;
      try {
        token = await invoke<string>("get_google_access_token");
      } catch (err) {
        const errMsg = typeof err === "string" ? err : "Token error";
        console.error("get_google_access_token failed:", err);
        toast.error(`Token error: ${errMsg}`);
        setPickerOpen(false);
        onClose();
        return;
      }

      try {
        await loadGapi();
      } catch (err) {
        console.error("loadGapi failed:", err);
        toast.error("Failed to load Google API");
        setPickerOpen(false);
        onClose();
        return;
      }

      try {
        await loadPickerApi();
      } catch (err) {
        console.error("loadPickerApi failed:", err);
        toast.error("Failed to load Google Picker");
        setPickerOpen(false);
        onClose();
        return;
      }

      const google = window.google;
      if (!google?.picker) {
        toast.error("Google Picker API not available");
        setPickerOpen(false);
        onClose();
        return;
      }

      const docsView = new google.picker.DocsView()
        .setIncludeFolders(true)
        .setSelectFolderEnabled(true);

      const picker = new google.picker.PickerBuilder()
        .addView(docsView)
        .setOAuthToken(token)
        .enableFeature(google.picker.Feature.MULTISELECT_ENABLED)
        .setCallback((data: google.picker.ResponseObject) => {
          if (data.action === google.picker.Action.PICKED) {
            const files: PickerFile[] = data.docs.map((doc: google.picker.DocumentObject) => ({
              id: doc.id,
              name: doc.name,
              mimeType: doc.mimeType,
            }));
            setPickerFiles(files);
          } else if (data.action === google.picker.Action.CANCEL) {
            // If no files were previously selected, close the modal
            setPickerFiles((prev) => {
              if (prev.length === 0) onClose();
              return prev;
            });
          }
          setPickerOpen(false);
        })
        .build();

      picker.setVisible(true);
    } catch (err) {
      console.error("Picker error:", err);
      toast.error(`Picker error: ${typeof err === "string" ? err : "Unknown error"}`);
      setPickerOpen(false);
      onClose();
    }
  }, [onClose]);

  const handleSubmit = useCallback(async () => {
    if (!entityId || pickerFiles.length === 0) return;
    setSubmitting(true);
    try {
      for (const file of pickerFiles) {
        if (watchMode === "watch") {
          await invoke("add_google_drive_watch", {
            googleId: file.id,
            name: file.name,
            fileType: driveTypeFromMime(file.mimeType),
            googleDocUrl: null,
            entityId: entityId,
            entityType: "account",
          });
        } else {
          await invoke("import_google_drive_file", {
            googleId: file.id,
            name: file.name,
            entityId: entityId,
            entityType: "account",
          });
        }
      }
      toast(
        `${pickerFiles.length} file${pickerFiles.length === 1 ? "" : "s"} ${
          watchMode === "watch" ? "watching" : "importing"
        }`
      );
      onImported();
      onClose();
    } catch (err) {
      const msg = typeof err === "string" ? err : "Failed to add Drive source";
      toast.error(msg);
    } finally {
      setSubmitting(false);
    }
  }, [entityId, pickerFiles, watchMode, onImported, onClose]);

  if (!open) return null;

  // Only show the modal form after files are selected (Picker has closed and files exist)
  // This prevents rendering both the Picker and the modal form at the same time
  if (pickerFiles.length === 0) {
    // Picker is open or loading - don't show any modal UI yet
    return null;
  }

  return (
    <div
      ref={backdropRef}
      onClick={(e) => {
        if (e.target === backdropRef.current) onClose();
      }}
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 100,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        background: "var(--color-overlay-light)",
        backdropFilter: "blur(2px)",
      }}
    >
      <div
        style={{
          width: 480,
          maxHeight: "80vh",
          background: "var(--color-bg-primary, #faf8f2)",
          borderRadius: 8,
          border: "1px solid var(--color-rule-light)",
          overflow: "hidden",
          display: "flex",
          flexDirection: "column",
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: "20px 24px 16px",
            borderBottom: "1px solid var(--color-rule-light)",
          }}
        >
          <h2
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 20,
              fontWeight: 400,
              letterSpacing: "-0.01em",
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Import from Google Drive
          </h2>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
              margin: "6px 0 0",
            }}
          >
            Select files or folders to import and link to an entity
          </p>
        </div>

        {/* Body */}
        <div style={{ padding: "16px 24px", flex: 1, overflow: "auto" }}>
          {/* Selected files */}
          {pickerFiles.length > 0 && (
            <div style={{ marginBottom: 20 }}>
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  fontWeight: 600,
                  letterSpacing: "0.06em",
                  textTransform: "uppercase",
                  color: "var(--color-text-tertiary)",
                }}
              >
                Selected Files
              </span>
              <div style={{ marginTop: 8 }}>
                {pickerFiles.map((file) => (
                  <div
                    key={file.id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      padding: "6px 0",
                      borderBottom: "1px solid var(--color-rule-light)",
                    }}
                  >
                    <span
                      style={{
                        width: 6,
                        height: 6,
                        borderRadius: "50%",
                        background: "var(--color-garden-sage)",
                        flexShrink: 0,
                      }}
                    />
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        color: "var(--color-text-primary)",
                        flex: 1,
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {file.name}
                    </span>
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        flexShrink: 0,
                      }}
                    >
                      {driveTypeFromMime(file.mimeType)}
                    </span>
                  </div>
                ))}
              </div>
              <button
                onClick={openPicker}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  fontWeight: 500,
                  letterSpacing: "0.04em",
                  color: "var(--color-text-tertiary)",
                  background: "none",
                  border: "none",
                  padding: "6px 0 0",
                  cursor: "pointer",
                  textDecoration: "underline",
                  textUnderlineOffset: 2,
                }}
              >
                Change selection
              </button>
            </div>
          )}

          {/* Watch mode toggle */}
          <div style={{ marginBottom: 20 }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: "var(--color-text-tertiary)",
                display: "block",
                marginBottom: 8,
              }}
            >
              Import Mode
            </span>
            <div style={{ display: "flex", gap: 8 }}>
              {(["once", "watch"] as const).map((mode) => (
                <button
                  key={mode}
                  onClick={() => setWatchMode(mode)}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                    fontWeight: watchMode === mode ? 600 : 400,
                    letterSpacing: "0.04em",
                    color:
                      watchMode === mode
                        ? "var(--color-garden-olive)"
                        : "var(--color-text-tertiary)",
                    background: "none",
                    border: `1px solid ${
                      watchMode === mode
                        ? "var(--color-garden-olive)"
                        : "var(--color-rule-heavy)"
                    }`,
                    borderRadius: 4,
                    padding: "6px 14px",
                    cursor: "pointer",
                    transition: "all 0.15s ease",
                  }}
                >
                  {mode === "once" ? "Import once" : "Watch for changes"}
                </button>
              ))}
            </div>
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 12,
                color: "var(--color-text-tertiary)",
                margin: "6px 0 0",
              }}
            >
              {watchMode === "once"
                ? "Import content now. No ongoing sync."
                : "Import now and check for updates on each sync cycle."}
            </p>
          </div>

          {/* Entity picker */}
          <div style={{ marginBottom: 12 }}>
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: "var(--color-text-tertiary)",
                display: "block",
                marginBottom: 8,
              }}
            >
              Link to Entity
            </span>
            <EntityPicker
              value={entityId}
              onChange={(id, name) => {
                setEntityId(id);
                setEntityName(name ?? null);
              }}
              placeholder="Select account or project..."
            />
            {entityName && (
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  color: "var(--color-text-secondary)",
                  margin: "6px 0 0",
                }}
              >
                Files will be linked to <strong>{entityName}</strong>
              </p>
            )}
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            padding: "12px 24px 16px",
            borderTop: "1px solid var(--color-rule-light)",
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
          }}
        >
          <button
            onClick={onClose}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "1px solid var(--color-rule-heavy)",
              borderRadius: 4,
              padding: "4px 14px",
              cursor: "pointer",
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleSubmit}
            disabled={submitting || !entityId || pickerFiles.length === 0}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: "var(--color-garden-olive)",
              background: "none",
              border: "1px solid var(--color-garden-olive)",
              borderRadius: 4,
              padding: "4px 14px",
              cursor:
                submitting || !entityId || pickerFiles.length === 0
                  ? "default"
                  : "pointer",
              opacity: submitting || !entityId || pickerFiles.length === 0 ? 0.5 : 1,
              transition: "all 0.15s ease",
            }}
          >
            {submitting ? "Importing..." : "Import"}
          </button>
        </div>
      </div>
    </div>
  );
}
