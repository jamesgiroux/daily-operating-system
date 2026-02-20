import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { LinearStatusData } from "@/types";
import { styles } from "../styles";

export default function LinearConnection() {
  const [status, setStatus] = useState<LinearStatusData | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [testing, setTesting] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [viewerName, setViewerName] = useState<string | null>(null);

  useEffect(() => {
    invoke<LinearStatusData>("get_linear_status")
      .then((s) => {
        setStatus(s);
        if (s.apiKeySet) setApiKey("\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022");
      })
      .catch((err) => console.error("get_linear_status failed:", err));
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_linear_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Linear:", err);
    }
  }

  async function saveApiKey() {
    const trimmed = apiKey.trim();
    if (!trimmed || trimmed === "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022") return;
    try {
      await invoke("set_linear_api_key", { key: trimmed });
      setApiKeyDirty(false);
      setStatus((prev) => prev ? { ...prev, apiKeySet: true } : prev);
      toast("Linear API key saved");
    } catch (err) {
      toast.error("Failed to save API key");
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      const name = await invoke<string>("test_linear_connection");
      setViewerName(name);
      toast(`Connected as ${name}`);
    } catch (err) {
      toast.error("Linear connection failed");
      setViewerName(null);
    } finally {
      setTesting(false);
    }
  }

  async function handleSync() {
    setSyncing(true);
    try {
      await invoke("start_linear_sync");
      toast("Linear sync started");
      setTimeout(async () => {
        try {
          const refreshed = await invoke<LinearStatusData>("get_linear_status");
          setStatus(refreshed);
        } catch {}
        setSyncing(false);
      }, 3000);
    } catch (err) {
      toast.error("Sync failed");
      setSyncing(false);
    }
  }

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : status.enabled && status.issueCount > 0
      ? "var(--color-garden-olive)"
      : "var(--color-text-tertiary)";

  const statusLabel = !status
    ? "Loading..."
    : `${status.issueCount} issues \u00b7 ${status.projectCount} projects`;

  return (
    <div>
      <p style={styles.subsectionLabel}>Linear Issue Tracking</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Sync your assigned issues and projects from Linear
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Issues and projects will sync from Linear"
              : "Linear sync is turned off"}
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
              {viewerName && (
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                  ({viewerName})
                </span>
              )}
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                style={{ ...styles.btn, ...styles.btnGhost, opacity: testing ? 0.5 : 1 }}
                onClick={testConnection}
                disabled={testing || !status.apiKeySet}
              >
                {testing ? "Testing..." : "Test Connection"}
              </button>
              <button
                style={{ ...styles.btn, ...styles.btnGhost, opacity: syncing ? 0.5 : 1 }}
                onClick={handleSync}
                disabled={syncing}
              >
                {syncing ? "Syncing..." : "Sync Now"}
              </button>
            </div>
          </div>

          <div style={{ ...styles.settingRow, borderBottom: "none" }}>
            <div style={{ flex: 1 }}>
              <span style={styles.monoLabel}>API Key</span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Personal API key from Linear settings
              </p>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => {
                    setApiKey(e.target.value);
                    setApiKeyDirty(true);
                  }}
                  placeholder="Enter Linear API key"
                  style={{
                    ...styles.input,
                    width: 260,
                  }}
                />
                {apiKeyDirty && apiKey.trim() && (
                  <button
                    style={{ ...styles.btn, ...styles.btnPrimary }}
                    onClick={saveApiKey}
                  >
                    Save
                  </button>
                )}
              </div>
            </div>
          </div>

          {status.lastSyncAt && (
            <div style={{ padding: "8px 0", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
              Last sync: {new Date(status.lastSyncAt).toLocaleString()}
            </div>
          )}
        </>
      )}
    </div>
  );
}
