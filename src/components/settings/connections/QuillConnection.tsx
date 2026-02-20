import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { styles } from "../styles";

interface QuillStatusData {
  enabled: boolean;
  bridgeExists: boolean;
  bridgePath: string;
  pendingSyncs: number;
  failedSyncs: number;
  completedSyncs: number;
  lastSyncAt: string | null;
  lastError: string | null;
  lastErrorAt: string | null;
  abandonedSyncs: number;
  pollIntervalMinutes: number;
}

export default function QuillConnection() {
  const [status, setStatus] = useState<QuillStatusData | null>(null);
  const [testing, setTesting] = useState(false);
  const [backfilling, setBackfilling] = useState(false);

  useEffect(() => {
    invoke<QuillStatusData>("get_quill_status")
      .then(setStatus)
      .catch((err) => console.error("get_quill_status failed:", err));
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_quill_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Quill:", err);
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      const ok = await invoke<boolean>("test_quill_connection");
      toast(ok ? "Quill connection successful" : "Quill bridge not available");
    } catch (err) {
      toast.error("Connection test failed");
    } finally {
      setTesting(false);
    }
  }

  const statusLabel = !status
    ? "Loading..."
    : !status.bridgeExists
      ? "Bridge not found"
      : status.lastSyncAt
        ? `Last sync: ${new Date(status.lastSyncAt).toLocaleString()}`
        : "Connected, no syncs yet";

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : !status.bridgeExists
      ? "var(--color-spice-terracotta)"
      : "var(--color-garden-olive)";

  return (
    <div>
      <p style={styles.subsectionLabel}>Quill Transcripts</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Automatically sync meeting transcripts from Quill
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Transcripts will sync after meetings end"
              : "Quill transcript sync is turned off"}
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
            <button
              style={{ ...styles.btn, ...styles.btnGhost, opacity: testing ? 0.5 : 1 }}
              onClick={testConnection}
              disabled={testing}
            >
              {testing ? "Testing..." : "Test Connection"}
            </button>
          </div>

          <div style={{ ...styles.settingRow, borderBottom: "none" }}>
            <div>
              <span style={styles.monoLabel}>Bridge path</span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2, fontFamily: "var(--font-mono)" }}>
                {status.bridgePath}
              </p>
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
              {status.abandonedSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-text-tertiary)" }}>
                  {status.abandonedSyncs} abandoned
                </span>
              )}
            </div>
          )}

          {status.lastError && (
            <div style={{ paddingTop: 8 }}>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-spice-terracotta)" }}>
                {status.lastError}
              </span>
              {status.lastErrorAt && (
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", marginLeft: 8 }}>
                  {new Date(status.lastErrorAt).toLocaleString()}
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
                How often to check for new transcripts
              </p>
            </div>
            <select
              value={status.pollIntervalMinutes}
              onChange={async (e) => {
                const minutes = Number(e.target.value);
                try {
                  await invoke("set_quill_poll_interval", { minutes });
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
                Create sync rows for past meetings (last 90 days)
              </p>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost, opacity: backfilling ? 0.5 : 1 }}
              onClick={async () => {
                setBackfilling(true);
                try {
                  const result = await invoke<{ created: number; eligible: number }>("start_quill_backfill");
                  toast(`Backfill: ${result.created} of ${result.eligible} eligible meetings queued`);
                  const refreshed = await invoke<QuillStatusData>("get_quill_status");
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
