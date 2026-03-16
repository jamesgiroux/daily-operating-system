import { useState, useEffect, useCallback } from "react";
import { Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import type { DriveStatusData, DriveWatchedSource } from "@/types";
import { styles } from "../styles";

export default function GoogleConnection() {
  const {
    status,
    email,
    loading,
    phase,
    error,
    justConnected,
    connect,
    disconnect,
    clearError,
  } = useGoogleAuth();

  // Drive status
  const [driveStatus, setDriveStatus] = useState<DriveStatusData | null>(null);
  const [driveSources, setDriveSources] = useState<DriveWatchedSource[]>([]);
  const [driveSyncing, setDriveSyncing] = useState(false);

  const loadDriveStatus = useCallback(async () => {
    try {
      const s = await invoke<DriveStatusData>("get_google_drive_status");
      setDriveStatus(s);
    } catch (err) {
      console.error("get_google_drive_status failed:", err);
    }
  }, []);

  const loadDriveSources = useCallback(async () => {
    try {
      const items = await invoke<DriveWatchedSource[]>("get_google_drive_watches");
      setDriveSources(items);
    } catch (err) {
      console.error("get_google_drive_watches failed:", err);
    }
  }, []);

  useEffect(() => {
    if (status.status === "authenticated") {
      loadDriveStatus();
      loadDriveSources();
    }
  }, [status.status, loadDriveStatus, loadDriveSources]);

  async function handleDriveSyncNow() {
    setDriveSyncing(true);
    try {
      await invoke("trigger_drive_sync_now");
      setTimeout(async () => {
        await loadDriveStatus();
        await loadDriveSources();
        setDriveSyncing(false);
      }, 2000);
    } catch (err) {
      console.error("Drive sync failed:", err);
      setDriveSyncing(false);
    }
  }

  async function handleRemoveDriveSource(sourceId: string) {
    try {
      await invoke("remove_google_drive_watch", { watchId: sourceId });
      setDriveSources((prev) => prev.filter((s) => s.id !== sourceId));
      setDriveStatus((prev) =>
        prev ? { ...prev, watchedCount: Math.max(0, prev.watchedCount - 1) } : prev
      );
    } catch (err) {
      console.error("Failed to remove source:", err);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Google</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        {status.status === "authenticated"
          ? "Calendar, email, and Drive documents for your briefings"
          : "Connect Google for calendar, email, and document access"}
      </p>

      {error && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "10px 0",
            borderBottom: "1px solid var(--color-spice-terracotta)",
            marginBottom: 12,
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-spice-terracotta)",
            }}
          >
            {error}
          </span>
          <button
            style={{
              ...styles.btn,
              fontSize: 10,
              padding: "2px 8px",
              color: "var(--color-spice-terracotta)",
              border: "none",
            }}
            onClick={clearError}
          >
            Dismiss
          </button>
        </div>
      )}

      {status.status === "authenticated" ? (
        <>
          {/* Authentication status */}
          <div style={styles.settingRow}>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <div style={styles.statusDot("var(--color-garden-sage)")} />
                <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                  {email}
                </span>
              </div>
              {justConnected && (
                <p
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 12,
                    color: "var(--color-garden-sage)",
                    marginTop: 4,
                  }}
                >
                  Connected successfully.
                </p>
              )}
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost, opacity: loading || phase === "authorizing" ? 0.5 : 1 }}
              onClick={disconnect}
              disabled={loading || phase === "authorizing"}
            >
              {loading ? (
                <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                  <Loader2 size={12} className="animate-spin" /> ...
                </span>
              ) : (
                "Disconnect"
              )}
            </button>
          </div>

          {/* Drive section */}
          {driveStatus && (
            <>
              <hr style={styles.thinRule} />
              <div style={{ marginTop: 12 }}>
                <p
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    fontWeight: 600,
                    letterSpacing: "0.06em",
                    textTransform: "uppercase",
                    color: "var(--color-text-tertiary)",
                    marginBottom: 8,
                  }}
                >
                  Google Drive
                </p>

                {/* Drive status + sync button */}
                <div style={styles.settingRow}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <div style={styles.statusDot(driveStatus.watchedCount > 0 ? "var(--color-garden-olive)" : "var(--color-text-tertiary)")} />
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-secondary)",
                      }}
                    >
                      {driveStatus.watchedCount} watched document{driveStatus.watchedCount === 1 ? "" : "s"}
                    </span>
                  </div>
                  <button
                    style={{
                      ...styles.btn,
                      ...styles.btnGhost,
                      opacity: driveSyncing ? 0.5 : 1,
                    }}
                    onClick={handleDriveSyncNow}
                    disabled={driveSyncing}
                  >
                    {driveSyncing ? "Syncing..." : "Sync Now"}
                  </button>
                </div>

                {driveStatus.lastSyncAt && (
                  <div
                    style={{
                      padding: "8px 0",
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                    }}
                  >
                    Last synced: {new Date(driveStatus.lastSyncAt).toLocaleString()}
                  </div>
                )}

                {/* Watched sources list */}
                {driveSources.length > 0 && (
                  <div style={{ marginTop: 12 }}>
                    <p
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        fontWeight: 600,
                        textTransform: "uppercase",
                        color: "var(--color-text-tertiary)",
                        marginBottom: 8,
                      }}
                    >
                      Watched Files
                    </p>
                    {driveSources.map((source) => (
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
                              fontSize: 12,
                              color: "var(--color-text-primary)",
                              overflow: "hidden",
                              textOverflow: "ellipsis",
                              whiteSpace: "nowrap",
                            }}
                          >
                            {source.name}
                          </span>
                        </div>
                        <button
                          style={{
                            ...styles.btn,
                            ...styles.btnGhost,
                            fontSize: 10,
                            padding: "2px 8px",
                            flexShrink: 0,
                          }}
                          onClick={() => handleRemoveDriveSource(source.id)}
                        >
                          Remove
                        </button>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </>
          )}
        </>
      ) : status.status === "tokenexpired" ? (
        <div style={styles.settingRow}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={styles.statusDot("var(--color-spice-terracotta)")} />
            <span style={styles.description}>Session expired</span>
          </div>
          <button
            style={{ ...styles.btn, ...styles.btnDanger, opacity: loading ? 0.5 : 1 }}
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : phase === "authorizing" ? (
              "Waiting..."
            ) : (
              "Reconnect"
            )}
          </button>
        </div>
      ) : (
        <div style={styles.settingRow}>
          <span style={styles.description}>Not connected</span>
          <button
            style={{ ...styles.btn, ...styles.btnPrimary, opacity: loading ? 0.5 : 1 }}
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : phase === "authorizing" ? (
              "Waiting for authorization..."
            ) : (
              "Connect"
            )}
          </button>
        </div>
      )}
    </div>
  );
}
