import { useState, useEffect, useCallback } from "react";
import { Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { styles } from "../styles";
import type { DriveStatusData } from "@/types";

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

  const [driveStatus, setDriveStatus] = useState<DriveStatusData | null>(null);
  const [driveLoading, setDriveLoading] = useState(false);

  const loadDriveStatus = useCallback(async () => {
    try {
      const s = await invoke<DriveStatusData>("get_google_drive_status");
      setDriveStatus(s);
    } catch (err) {
      console.error("get_google_drive_status failed:", err);
    }
  }, []);

  useEffect(() => {
    if (status.status === "authenticated") {
      loadDriveStatus();
    }
  }, [status.status, loadDriveStatus]);

  async function toggleDriveEnabled() {
    if (!driveStatus) return;
    setDriveLoading(true);
    try {
      await invoke("set_google_drive_enabled", { enabled: !driveStatus.enabled });
      setDriveStatus({ ...driveStatus, enabled: !driveStatus.enabled });
      toast(driveStatus.enabled ? "Drive sync disabled" : "Drive sync enabled");
    } catch (err) {
      toast.error("Failed to toggle Drive");
      console.error(err);
    } finally {
      setDriveLoading(false);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Google Account</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        {status.status === "authenticated"
          ? "Calendar, email, and Google Drive connected"
          : "Connect Google to sync Calendar, Gmail, and Google Drive"}
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
          <div style={{ marginTop: 24, paddingTop: 16, borderTop: "1px solid var(--color-rule-light)" }}>
            <p style={styles.subsectionLabel}>Google Drive</p>
            <p style={{ ...styles.description, marginBottom: 16 }}>
              Sync Google Drive documents as context for your insights
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
                  {driveStatus?.enabled ? "Enabled" : "Disabled"}
                </span>
                <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                  {driveStatus?.enabled
                    ? "Documents sync on schedule"
                    : "Drive sync is off"}
                </p>
              </div>
              <button
                style={{
                  ...styles.btn,
                  ...styles.btnGhost,
                  opacity: driveLoading ? 0.5 : 1,
                }}
                onClick={toggleDriveEnabled}
                disabled={driveLoading}
              >
                {driveLoading ? (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                    <Loader2 size={12} className="animate-spin" /> ...
                  </span>
                ) : driveStatus?.enabled ? (
                  "Disable"
                ) : (
                  "Enable"
                )}
              </button>
            </div>

            {driveStatus?.enabled && driveStatus.watchedCount > 0 && (
              <div style={{ ...styles.description, fontSize: 12, marginTop: 8 }}>
                {driveStatus.watchedCount} document{driveStatus.watchedCount !== 1 ? "s" : ""} syncing
              </div>
            )}
          </div>
        </div>
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
