import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface GoogleDriveImportModalProps {
  open: boolean;
  onClose: () => void;
  onImported: () => void;
}

export function GoogleDriveImportModal({ open, onClose, onImported }: GoogleDriveImportModalProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) {
    return null;
  }

  const handleClose = () => {
    setError(null);
    onClose();
  };

  const handleGetAccessToken = async () => {
    try {
      setLoading(true);
      setError(null);
      await invoke("get_google_access_token");
      console.log("Access token retrieved for Google Drive");
      // TODO: Initialize Google Picker with token
      // For now, call onImported to refresh the inbox
      onImported();
      handleClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to get access token");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0, 0, 0, 0.5)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 100,
      }}
      onClick={handleClose}
    >
      <div
        style={{
          background: "var(--color-ui-background)",
          borderRadius: 8,
          padding: 24,
          maxWidth: 500,
          width: "90%",
          maxHeight: "80vh",
          overflow: "auto",
          boxShadow: "0 10px 40px rgba(0, 0, 0, 0.2)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 20 }}>
          <h2
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 24,
              fontWeight: 400,
              margin: 0,
              color: "var(--color-text-primary)",
            }}
          >
            Import from Google Drive
          </h2>
          <button
            onClick={handleClose}
            style={{
              background: "none",
              border: "none",
              fontSize: 24,
              cursor: "pointer",
              color: "var(--color-text-tertiary)",
              padding: 0,
            }}
          >
            ×
          </button>
        </div>

        {/* Content */}
        <div style={{ marginBottom: 20 }}>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-secondary)",
              lineHeight: 1.6,
              margin: "0 0 16px 0",
            }}
          >
            Select files or folders from your Google Drive to import. Files will be placed in your workspace and
            automatically processed.
          </p>

          {error && (
            <div
              style={{
                padding: 12,
                background: "rgba(220, 38, 38, 0.1)",
                border: "1px solid var(--color-spice-terracotta)",
                borderRadius: 4,
                color: "var(--color-spice-terracotta)",
                fontSize: 13,
                fontFamily: "var(--font-mono)",
                marginBottom: 16,
              }}
            >
              {error}
            </div>
          )}

          {/* Placeholder: Google Picker will be initialized here */}
          <div
            style={{
              padding: 40,
              background: "var(--color-rule-light)",
              borderRadius: 4,
              textAlign: "center",
              color: "var(--color-text-tertiary)",
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              minHeight: 200,
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
              justifyContent: "center",
            }}
          >
            <div style={{ marginBottom: 12 }}>📁 Google Drive Picker</div>
            <div style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
              Click "Open Picker" to select files from your Google Drive
            </div>
          </div>
        </div>

        {/* Actions */}
        <div style={{ display: "flex", gap: 12, justifyContent: "flex-end" }}>
          <button
            onClick={handleClose}
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
              padding: "6px 14px",
              cursor: "pointer",
            }}
          >
            Cancel
          </button>
          <button
            onClick={handleGetAccessToken}
            disabled={loading}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: loading ? "var(--color-text-tertiary)" : "var(--color-garden-sage)",
              background: "none",
              border: loading ? "1px solid var(--color-rule-heavy)" : "1px solid var(--color-garden-sage)",
              borderRadius: 4,
              padding: "6px 14px",
              cursor: loading ? "default" : "pointer",
              opacity: loading ? 0.5 : 1,
            }}
          >
            {loading ? "Opening..." : "Open Picker"}
          </button>
        </div>
      </div>
    </div>
  );
}
