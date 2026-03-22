import { useState, useEffect, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { styles } from "../styles";
import surface from "./ConnectorSurface.module.css";

interface GranolaStatusData {
  enabled: boolean;
  cacheExists: boolean;
  cachePath: string;
  documentCount: number;
  pendingSyncs: number;
  failedSyncs: number;
  completedSyncs: number;
  lastSyncAt: string | null;
  pollIntervalMinutes: number;
}

export default function GranolaConnection() {
  const BACKFILL_DAYS = 365;
  const [status, setStatus] = useState<GranolaStatusData | null>(null);
  const [backfilling, setBackfilling] = useState(false);

  useEffect(() => {
    invoke<GranolaStatusData>("get_granola_status")
      .then(setStatus)
      .catch((err) => console.error("get_granola_status failed:", err)); // Expected: background init on mount
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_granola_enabled", { enabled: newEnabled });
      const refreshed = await invoke<GranolaStatusData>("get_granola_status");
      setStatus(refreshed);
    } catch (err) {
      console.error("Failed to toggle Granola:", err);
      toast.error("Failed to toggle Granola");
    }
  }

  const statusLabel = !status
    ? "Loading..."
    : !status.cacheExists
      ? "Cache not found"
      : `Cache found (${status.documentCount} documents)`;

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : !status.cacheExists
      ? "var(--color-spice-terracotta)"
      : "var(--color-garden-olive)";

  return (
    <div>
      <div className={surface.intro}>
        <p style={styles.subsectionLabel}>Granola Transcripts</p>
        <p style={styles.description} className={surface.introDescription}>
          Sync meeting notes from Granola&apos;s local cache (no API key required)
        </p>
      </div>

      <div style={styles.settingRow}>
        <div className={surface.settingCopy}>
          <span className={surface.settingTitle}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p className={surface.settingDescription}>
            {status?.enabled
              ? "Notes will sync from Granola cache"
              : "Granola transcript sync is turned off"}
          </p>
        </div>
        <button
          style={{ ...styles.btn, ...styles.btnGhost }}
          className={!status ? surface.disabledButton : undefined}
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {status?.enabled && (
        <>
          {!status.cacheExists && (
            <div className={surface.callout}>
              <p className={surface.calloutLabel}>Not Found</p>
              <p className={surface.calloutText}>
                Granola must be installed and have recorded at least one meeting for its local cache to exist.
              </p>
              <p className={`${surface.calloutText} ${surface.calloutTextSpaced}`}>
                Expected path: <span className={surface.inlineCode}>~/Library/Application Support/Granola/</span>
              </p>
            </div>
          )}

          <div style={styles.settingRow}>
            <div className={surface.statusSummary}>
              <div
                className={surface.statusDot}
                style={{ "--connector-status-color": statusColor } as CSSProperties}
              />
              <span className={surface.statusText}>{statusLabel}</span>
            </div>
          </div>

          {(status.pendingSyncs > 0 || status.failedSyncs > 0 || status.completedSyncs > 0) && (
            <div className={surface.statsRow}>
              {status.completedSyncs > 0 && (
                <span className={`${surface.statsLabel} ${surface.statsSynced}`}>
                  {status.completedSyncs} synced
                </span>
              )}
              {status.pendingSyncs > 0 && (
                <span className={`${surface.statsLabel} ${surface.statsPending}`}>
                  {status.pendingSyncs} pending
                </span>
              )}
              {status.failedSyncs > 0 && (
                <span className={`${surface.statsLabel} ${surface.statsFailed}`}>
                  {status.failedSyncs} failed
                </span>
              )}
            </div>
          )}

          <div style={styles.settingRow}>
            <div className={surface.settingCopy}>
              <span className={surface.settingTitle}>Poll interval</span>
              <p className={surface.settingDescription}>
                How often to check the Granola cache for new notes
              </p>
            </div>
            <select
              value={status.pollIntervalMinutes}
              onChange={async (e) => {
                const minutes = Number(e.target.value);
                try {
                  await invoke("set_granola_poll_interval", { minutes });
                  setStatus({ ...status, pollIntervalMinutes: minutes });
                } catch (err) {
                  console.error("Failed to set poll interval:", err);
                  toast.error("Failed to update poll interval");
                }
              }}
              className={surface.selectControl}
            >
              {[1, 2, 5, 10, 15, 30].map((m) => (
                <option key={m} value={m}>
                  {m} min
                </option>
              ))}
            </select>
          </div>

          <div style={styles.settingRow}>
            <div className={surface.settingCopy}>
              <span className={surface.settingTitle}>Historical backfill</span>
              <p className={surface.settingDescription}>
                Match Granola cache documents to past meetings (last {BACKFILL_DAYS} days)
              </p>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost }}
              className={backfilling ? surface.disabledButton : undefined}
              onClick={async () => {
                setBackfilling(true);
                try {
                  const result = await invoke<{ created: number; eligible: number }>("start_granola_backfill", {
                    daysBack: BACKFILL_DAYS,
                  });
                  toast(`Backfill: ${result.created} of ${result.eligible} documents matched`);
                  const refreshed = await invoke<GranolaStatusData>("get_granola_status");
                  setStatus(refreshed);
                } catch {
                  toast.error("Backfill failed");
                } finally {
                  setBackfilling(false);
                }
              }}
              disabled={backfilling}
            >
              {backfilling ? "Running..." : "Start Backfill"}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
