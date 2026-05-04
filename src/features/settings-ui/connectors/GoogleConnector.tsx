import { useState, useEffect, useCallback } from "react";
import { Loader2 } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import type { DriveStatusData, DriveWatchedSource } from "@/types";
import {
  SettingsButton,
  SettingsRule,
  SettingsSectionLabel,
  SettingsStatusDot,
  formRowStyles,
} from "@/components/settings/FormRow";

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
      console.error("get_google_drive_status failed:", err); // Expected: background init
    }
  }, []);

  const loadDriveSources = useCallback(async () => {
    try {
      const items = await invoke<DriveWatchedSource[]>("get_google_drive_watches");
      setDriveSources(items);
    } catch (err) {
      console.error("get_google_drive_watches failed:", err); // Expected: background init
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
      toast.error("Drive sync failed");
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
      toast.error("Failed to remove source");
    }
  }

  return (
    <div>
      <SettingsSectionLabel>Google</SettingsSectionLabel>
      <p className={formRowStyles.descriptionLead}>
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
          <SettingsButton tone="borderless" compact onClick={clearError}>
            Dismiss
          </SettingsButton>
        </div>
      )}

      {status.status === "authenticated" ? (
        <>
          {/* Authentication status */}
          <div className={formRowStyles.settingRow}>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <SettingsStatusDot color="var(--color-garden-sage)" />
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
            <SettingsButton
              tone="ghost"
              onClick={disconnect}
              disabled={loading || phase === "authorizing"}
            >
              {loading ? (
                <>
                  <Loader2 size={12} className="animate-spin" /> ...
                </>
              ) : (
                "Disconnect"
              )}
            </SettingsButton>
          </div>

          {/* Drive section */}
          {driveStatus && (
            <>
              <SettingsRule />
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
                <div className={formRowStyles.settingRow}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <SettingsStatusDot color={driveStatus.watchedCount > 0 ? "var(--color-garden-olive)" : "var(--color-text-tertiary)"} />
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
                  <SettingsButton
                    tone="ghost"
                    onClick={handleDriveSyncNow}
                    disabled={driveSyncing}
                  >
                    {driveSyncing ? "Syncing..." : "Sync Now"}
                  </SettingsButton>
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
                        <SettingsButton
                          tone="ghost"
                          compact
                          className={formRowStyles.buttonFlexShrink}
                          onClick={() => handleRemoveDriveSource(source.id)}
                        >
                          Remove
                        </SettingsButton>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </>
          )}
        </>
      ) : status.status === "tokenexpired" ? (
        <div className={formRowStyles.settingRow}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <SettingsStatusDot color="var(--color-spice-terracotta)" />
            <span className={formRowStyles.description}>Session expired</span>
          </div>
          <SettingsButton
            tone="danger"
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <>
                <Loader2 size={12} className="animate-spin" /> ...
              </>
            ) : phase === "authorizing" ? (
              "Waiting..."
            ) : (
              "Reconnect"
            )}
          </SettingsButton>
        </div>
      ) : (
        <div className={formRowStyles.settingRow}>
          <span className={formRowStyles.description}>Not connected</span>
          <SettingsButton
            tone="primary"
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <>
                <Loader2 size={12} className="animate-spin" /> ...
              </>
            ) : phase === "authorizing" ? (
              "Waiting for authorization..."
            ) : (
              "Connect"
            )}
          </SettingsButton>
        </div>
      )}
    </div>
  );
}
