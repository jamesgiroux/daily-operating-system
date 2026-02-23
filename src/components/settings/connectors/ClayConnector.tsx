import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { ClayStatusData } from "@/types";
import { styles } from "../styles";

export default function ClayConnection() {
  const [status, setStatus] = useState<ClayStatusData | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [testing, setTesting] = useState(false);
  const [enriching, setEnriching] = useState(false);

  useEffect(() => {
    invoke<ClayStatusData>("get_clay_status")
      .then((s) => {
        setStatus(s);
        if (s.apiKeySet) setApiKey("\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022");
      })
      .catch((err) => console.error("get_clay_status failed:", err));
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_clay_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Clay:", err);
    }
  }

  async function saveApiKey() {
    const trimmed = apiKey.trim();
    if (!trimmed || trimmed === "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022") return;
    try {
      await invoke("set_clay_api_key", { key: trimmed });
      setApiKeyDirty(false);
      setStatus((prev) => prev ? { ...prev, apiKeySet: true } : prev);
      toast("Clay API key saved");
    } catch (err) {
      toast.error("Failed to save API key");
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      await invoke<boolean>("test_clay_connection");
      toast("Clay connection successful");
    } catch (err) {
      toast.error("Clay connection failed");
    } finally {
      setTesting(false);
    }
  }

  async function handleBulkEnrich() {
    setEnriching(true);
    try {
      const result = await invoke<{ queued: number; totalUnenriched: number }>("start_clay_bulk_enrich");
      toast(`Queued ${result.queued} people for enrichment`);
      const refreshed = await invoke<ClayStatusData>("get_clay_status");
      setStatus(refreshed);
    } catch (err) {
      toast.error("Bulk enrichment failed");
    } finally {
      setEnriching(false);
    }
  }

  async function toggleAutoEnrich() {
    if (!status) return;
    const newValue = !status.autoEnrichOnCreate;
    try {
      await invoke("set_clay_auto_enrich", { enabled: newValue });
      setStatus({ ...status, autoEnrichOnCreate: newValue });
    } catch (err) {
      console.error("Failed to toggle auto-enrich:", err);
    }
  }

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : status.enabled && status.enrichedCount > 0
      ? "var(--color-garden-olive)"
      : "var(--color-text-tertiary)";

  const statusLabel = !status
    ? "Loading..."
    : `${status.enrichedCount} enriched \u00b7 ${status.pendingCount} pending`;

  return (
    <div>
      <p style={styles.subsectionLabel}>Clay Contact Enrichment</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Enrich contacts with social profiles, bios, and company data from Clay.earth
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Contacts will be enriched with Clay data"
              : "Clay enrichment is turned off"}
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
            <div style={{ display: "flex", gap: 8 }}>
              <button
                style={{ ...styles.btn, ...styles.btnGhost, opacity: testing ? 0.5 : 1 }}
                onClick={testConnection}
                disabled={testing || !status.apiKeySet}
              >
                {testing ? "Testing..." : "Test Connection"}
              </button>
              <button
                style={{ ...styles.btn, ...styles.btnGhost, opacity: enriching ? 0.5 : 1 }}
                onClick={handleBulkEnrich}
                disabled={enriching}
              >
                {enriching ? "Enriching..." : "Enrich All"}
              </button>
            </div>
          </div>

          <div style={styles.settingRow}>
            <div>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                Auto-enrich new contacts
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Automatically enrich when new people are created
              </p>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost }}
              onClick={toggleAutoEnrich}
            >
              {status.autoEnrichOnCreate ? "Disable" : "Enable"}
            </button>
          </div>

          <div style={{ ...styles.settingRow, borderBottom: "none" }}>
            <div style={{ flex: 1 }}>
              <span style={styles.monoLabel}>API Key</span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Clay API key for contact enrichment
              </p>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => {
                    setApiKey(e.target.value);
                    setApiKeyDirty(true);
                  }}
                  placeholder="Enter Clay API key"
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

          {status.lastEnrichmentAt && (
            <div style={{ padding: "8px 0", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
              Last enrichment: {new Date(status.lastEnrichmentAt).toLocaleString()}
            </div>
          )}
        </>
      )}
    </div>
  );
}
