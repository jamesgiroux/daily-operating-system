import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { DriveStatusData, DriveWatchedSource } from "@/types";
import { styles } from "../styles";

export default function GoogleDriveConnector() {
  const [status, setStatus] = useState<DriveStatusData | null>(null);
  const [sources, setSources] = useState<DriveWatchedSource[]>([]);
  const [syncing, setSyncing] = useState(false);

  const loadStatus = useCallback(async () => {
    try {
      const s = await invoke<DriveStatusData>("get_google_drive_status");
      setStatus(s);
    } catch (err) {
      console.error("get_google_drive_status failed:", err);
    }
  }, []);

  const loadSources = useCallback(async () => {
    try {
      const items = await invoke<DriveWatchedSource[]>("get_google_drive_watches");
      setSources(items);
    } catch (err) {
      console.error("get_google_drive_watches failed:", err);
    }
  }, []);

  useEffect(() => {
    loadStatus();
    loadSources();
  }, [loadStatus, loadSources]);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_google_drive_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Drive:", err);
      toast.error("Failed to toggle Google Drive");
    }
  }

  async function handleSyncNow() {
    setSyncing(true);
    try {
      await invoke("trigger_drive_sync_now");
      toast("Drive sync started");
      // Refresh status after a short delay
      setTimeout(async () => {
        await loadStatus();
        await loadSources();
        setSyncing(false);
      }, 3000);
    } catch (err) {
      toast.error("Drive sync failed");
      setSyncing(false);
    }
  }

  async function handleRemoveSource(sourceId: string) {
    try {
      await invoke("remove_google_drive_watch", { watch_id: sourceId });
      setSources((prev) => prev.filter((s) => s.id !== sourceId));
      setStatus((prev) =>
        prev ? { ...prev, watchedCount: Math.max(0, prev.watchedCount - 1) } : prev
      );
      toast("Drive source removed");
    } catch (err) {
      toast.error("Failed to remove source");
    }
  }

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : status.enabled && status.watchedCount > 0
      ? "var(--color-garden-olive)"
      : "var(--color-text-tertiary)";

  const statusLabel = !status
    ? "Loading..."
    : `${status.watchedCount} watched source${status.watchedCount === 1 ? "" : "s"}`;

  function formatDocType(type: string): string {
    switch (type) {
      case "document": return "Doc";
      case "spreadsheet": return "Sheet";
      case "presentation": return "Slides";
      case "folder": return "Folder";
      default: return type;
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Google Drive</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Import documents and spreadsheets from Google Drive as entity context
      </p>

      <div style={styles.settingRow}>
        <div>
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-primary)",
            }}
          >
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Drive documents will sync on schedule"
              : "Google Drive sync is turned off"}
          </p>
        </div>
        <button
          style={{
            ...styles.btn,
            ...styles.btnGhost,
            opacity: !status ? 0.5 : 1,
          }}
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {status?.enabled && (
        <>
          {/* Status + sync */}
          <div style={styles.settingRow}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={styles.statusDot(statusColor)} />
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--color-text-secondary)",
                }}
              >
                {statusLabel}
              </span>
            </div>
            <button
              style={{
                ...styles.btn,
                ...styles.btnGhost,
                opacity: syncing ? 0.5 : 1,
              }}
              onClick={handleSyncNow}
              disabled={syncing}
            >
              {syncing ? "Syncing..." : "Sync Now"}
            </button>
          </div>

          {status.lastSyncAt && (
            <div
              style={{
                padding: "8px 0",
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
              }}
            >
              Last sync: {new Date(status.lastSyncAt).toLocaleString()}
            </div>
          )}

          {/* Watched sources list */}
          {sources.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <hr style={styles.thinRule} />
              <p style={{ ...styles.monoLabel, marginBottom: 8 }}>
                Watched Sources
              </p>
              {sources.map((source) => (
                <div
                  key={source.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "6px 0",
                    borderBottom: "1px solid var(--color-rule-light)",
                  }}
                >
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: 8,
                      flex: 1,
                      minWidth: 0,
                    }}
                  >
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-primary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}
                    >
                      {source.name}
                    </span>
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        flexShrink: 0,
                        textTransform: "uppercase",
                        letterSpacing: "0.04em",
                      }}
                    >
                      {formatDocType(source.type)}
                    </span>
                    {source.lastSyncedAt && (
                      <span
                        style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 10,
                          color: "var(--color-text-tertiary)",
                          opacity: 0.6,
                          flexShrink: 0,
                        }}
                      >
                        {new Date(source.lastSyncedAt).toLocaleDateString(
                          undefined,
                          { month: "short", day: "numeric" }
                        )}
                      </span>
                    )}
                  </div>
                  <button
                    style={{
                      ...styles.btn,
                      ...styles.btnGhost,
                      fontSize: 10,
                      padding: "2px 8px",
                      flexShrink: 0,
                    }}
                    onClick={() => handleRemoveSource(source.id)}
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          )}

          {sources.length === 0 && (
            <div style={{ marginTop: 16 }}>
              <p
                style={{
                  ...styles.description,
                  fontSize: 12,
                  fontStyle: "italic",
                }}
              >
                No watched sources. Use the Inbox page to import Google Drive files.
              </p>
            </div>
          )}
        </>
      )}
    </div>
  );
}
