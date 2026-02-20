import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { styles } from "../styles";

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
  const [status, setStatus] = useState<GranolaStatusData | null>(null);
  const [backfilling, setBackfilling] = useState(false);

  useEffect(() => {
    invoke<GranolaStatusData>("get_granola_status")
      .then(setStatus)
      .catch(() => {});
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
      <p style={styles.subsectionLabel}>Granola Transcripts</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Sync meeting notes from Granola's local cache (no API key required)
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Notes will sync from Granola cache"
              : "Granola transcript sync is turned off"}
          </p>
        </div>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: !status ? 0.5 : 1 }}
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {status?.enabled && (
        <>
          <div style={styles.settingRow}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={styles.statusDot(statusColor)} />
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
                {statusLabel}
              </span>
            </div>
          </div>

          {(status.pendingSyncs > 0 || status.failedSyncs > 0 || status.completedSyncs > 0) && (
            <div style={{ display: "flex", gap: 16, paddingTop: 8 }}>
              {status.completedSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-garden-olive)" }}>
                  {status.completedSyncs} synced
                </span>
              )}
              {status.pendingSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-golden-turmeric)" }}>
                  {status.pendingSyncs} pending
                </span>
              )}
              {status.failedSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-spice-terracotta)" }}>
                  {status.failedSyncs} failed
                </span>
              )}
            </div>
          )}

          <div style={styles.settingRow}>
            <div>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                Poll interval
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
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
                }
              }}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                padding: "4px 8px",
                border: "1px solid var(--color-border)",
                borderRadius: 4,
                background: "var(--color-surface)",
                color: "var(--color-text-primary)",
              }}
            >
              {[1, 2, 5, 10, 15, 30].map((m) => (
                <option key={m} value={m}>
                  {m} min
                </option>
              ))}
            </select>
          </div>

          <div style={styles.settingRow}>
            <div>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                Historical backfill
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Match Granola cache documents to past meetings
              </p>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost, opacity: backfilling ? 0.5 : 1 }}
              onClick={async () => {
                setBackfilling(true);
                try {
                  const result = await invoke<{ created: number; eligible: number }>("start_granola_backfill");
                  toast(`Backfill: ${result.created} of ${result.eligible} documents matched`);
                  const refreshed = await invoke<GranolaStatusData>("get_granola_status");
                  setStatus(refreshed);
                } catch (err) {
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
