import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { ClayStatusData } from "@/types";
import { styles } from "../styles";

interface SmitheryStatus {
  connected: boolean;
  hasApiKey: boolean;
  namespace: string | null;
  connectionId: string | null;
}

interface SmitheryDetected {
  apiKey: string;
  namespace: string;
  connectionId: string | null;
}

export default function ClayConnection() {
  const [status, setStatus] = useState<ClayStatusData | null>(null);
  const [smithery, setSmithery] = useState<SmitheryStatus | null>(null);
  const [testing, setTesting] = useState(false);
  const [enriching, setEnriching] = useState(false);
  const [detecting, setDetecting] = useState(false);

  // Manual config state
  const [manualKey, setManualKey] = useState("");
  const [manualConnId, setManualConnId] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<ClayStatusData>("get_clay_status")
      .then(setStatus)
      .catch((err) => console.error("get_clay_status failed:", err));

    invoke<SmitheryStatus>("get_smithery_status")
      .then(setSmithery)
      .catch((err) => console.error("get_smithery_status failed:", err));
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_clay_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Clay:", err);
      toast.error("Failed to update contact enrichment");
    }
  }

  async function handleDetect() {
    setDetecting(true);
    try {
      const detected = await invoke<SmitheryDetected>("detect_smithery_settings");
      // Save API key to keychain
      await invoke("save_smithery_api_key", { key: detected.apiKey });
      // Save namespace + connection ID (if auto-detected) to config
      await invoke("set_smithery_connection", {
        namespace: detected.namespace,
        connectionId: detected.connectionId ?? "",
      });
      setManualConnId("");
      const refreshed = await invoke<SmitheryStatus>("get_smithery_status");
      // Ensure namespace is visible even if backend stored empty connectionId
      setSmithery({ ...refreshed, namespace: detected.namespace });
      if (detected.connectionId) {
        toast(`Clay connected via Smithery (${detected.connectionId})`);
      } else {
        toast("Smithery detected. Enter your Clay connection ID below.");
      }
    } catch (err) {
      const msg = typeof err === "string" ? err : "Detection failed";
      toast.error(msg);
    } finally {
      setDetecting(false);
    }
  }

  async function handleSaveManual() {
    if (!manualKey.trim() || !manualConnId.trim()) return;
    setSaving(true);
    try {
      // Save API key to keychain
      await invoke("save_smithery_api_key", { key: manualKey.trim() });

      // Parse namespace from API key or use a default
      // Smithery keys start with "smry_" — the namespace comes from detect or manual entry
      // For now, detect to get the namespace
      let namespace = "";
      try {
        const detected = await invoke<SmitheryDetected>("detect_smithery_settings");
        namespace = detected.namespace;
      } catch {
        // If detect fails, ask user — but for now use the connection ID prefix
        toast.error("Could not detect Smithery namespace. Run: npx @smithery/cli login");
        setSaving(false);
        return;
      }

      await invoke("set_smithery_connection", {
        namespace,
        connectionId: manualConnId.trim(),
      });

      setManualKey("");
      setManualConnId("");
      toast("Smithery connection saved");

      const refreshed = await invoke<SmitheryStatus>("get_smithery_status");
      setSmithery(refreshed);
    } catch (err) {
      toast.error("Failed to save connection");
    } finally {
      setSaving(false);
    }
  }

  async function handleSaveConnectionId() {
    if (!manualConnId.trim()) return;
    setSaving(true);
    try {
      const ns = smithery?.namespace;
      if (!ns) {
        toast.error("Namespace not set. Run detect first.");
        setSaving(false);
        return;
      }
      await invoke("set_smithery_connection", {
        namespace: ns,
        connectionId: manualConnId.trim(),
      });
      setManualConnId("");
      toast("Connection ID saved");
      const refreshed = await invoke<SmitheryStatus>("get_smithery_status");
      setSmithery(refreshed);
    } catch (err) {
      toast.error("Failed to save connection ID");
    } finally {
      setSaving(false);
    }
  }

  async function handleDisconnect() {
    try {
      await invoke("disconnect_smithery");
      setSmithery({ connected: false, hasApiKey: false, namespace: null, connectionId: null });
      toast("Smithery disconnected");
    } catch (err) {
      toast.error("Failed to disconnect");
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      await invoke<boolean>("test_clay_connection");
      toast("Clay connection successful via Smithery");
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
      toast(`Queued ${result.queued} contacts for updates`);
      const refreshed = await invoke<ClayStatusData>("get_clay_status");
      setStatus(refreshed);
    } catch (err) {
      toast.error("Bulk update failed");
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
    : `${status.enrichedCount} updated \u00b7 ${status.pendingCount} pending`;

  const needsConnectionId = smithery?.hasApiKey && smithery?.namespace && !smithery?.connectionId;

  return (
    <div>
      <p style={styles.subsectionLabel}>Clay Contact Updates</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Update contacts with social profiles, bios, and company data via Clay + Smithery
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Contacts will be updated with Clay data"
              : "Clay updates are turned off"}
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
          {!smithery?.connected && !smithery?.hasApiKey && (
            <div style={{
              padding: "12px 16px",
              borderRadius: 6,
              border: "1px solid var(--color-rule-light)",
              background: "var(--color-paper-linen)",
              marginBottom: 16,
            }}>
              <p style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.08em",
                color: "var(--color-text-tertiary)",
                marginBottom: 6,
                marginTop: 0,
              }}>
                Prerequisites
              </p>
              <p style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                lineHeight: 1.6,
                color: "var(--color-text-secondary)",
                margin: 0,
              }}>
                Requires a Smithery account connected to Clay.
              </p>
              <ol style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                lineHeight: 1.6,
                color: "var(--color-text-secondary)",
                margin: "4px 0 0",
                paddingLeft: 20,
              }}>
                <li>Run <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>npx @smithery/cli login</code> to authenticate with Smithery</li>
                <li>Connect Clay via <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>npx @smithery/cli mcp add clay-inc/clay-mcp</code></li>
                <li>Return here and click Detect to auto-configure</li>
              </ol>
            </div>
          )}

          {/* Smithery connection */}
          <div style={styles.settingRow}>
            <div>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                {smithery?.connected ? "Connected via Smithery" : "Smithery Connect"}
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                {smithery?.connected
                  ? `${smithery.namespace} / ${smithery.connectionId}`
                  : "Connect Clay through Smithery for managed OAuth"}
              </p>
            </div>
            {smithery?.connected ? (
              <button
                style={{ ...styles.btn, ...styles.btnGhost }}
                onClick={handleDisconnect}
              >
                Disconnect
              </button>
            ) : (
              <button
                style={{ ...styles.btn, ...styles.btnPrimary, opacity: detecting ? 0.5 : 1 }}
                onClick={handleDetect}
                disabled={detecting}
              >
                {detecting ? "Detecting..." : "Detect Smithery"}
              </button>
            )}
          </div>

          {/* Connection ID input — shown after detect finds API key + namespace but no connection ID */}
          {needsConnectionId && (
            <div style={{ ...styles.settingRow, borderBottom: "none" }}>
              <div style={{ flex: 1 }}>
                <span style={styles.monoLabel}>Connection ID</span>
                <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                  Run <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>npx @smithery/cli mcp add clay-inc/clay-mcp</code> then enter the connection ID
                </p>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
                  <input
                    type="text"
                    value={manualConnId}
                    onChange={(e) => setManualConnId(e.target.value)}
                    placeholder="e.g. clay-mcp-vGfX"
                    style={{ ...styles.input, width: 200 }}
                  />
                  {manualConnId.trim() && (
                    <button
                      style={{ ...styles.btn, ...styles.btnPrimary, opacity: saving ? 0.5 : 1 }}
                      onClick={handleSaveConnectionId}
                      disabled={saving}
                    >
                      {saving ? "Saving..." : "Save"}
                    </button>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Manual setup fallback — shown when nothing is detected */}
          {!smithery?.connected && !smithery?.hasApiKey && (
            <div style={{ ...styles.settingRow, borderBottom: "none" }}>
              <div style={{ flex: 1 }}>
                <span style={styles.monoLabel}>Manual Setup</span>
                <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                  Paste your Smithery API key and Clay connection ID
                </p>
                <div style={{ display: "flex", flexDirection: "column", gap: 8, marginTop: 8 }}>
                  <input
                    type="password"
                    value={manualKey}
                    onChange={(e) => setManualKey(e.target.value)}
                    placeholder="Smithery API key"
                    style={{ ...styles.input, width: 300 }}
                  />
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <input
                      type="text"
                      value={manualConnId}
                      onChange={(e) => setManualConnId(e.target.value)}
                      placeholder="Connection ID (e.g. clay-mcp-vGfX)"
                      style={{ ...styles.input, width: 240 }}
                    />
                    {manualKey.trim() && manualConnId.trim() && (
                      <button
                        style={{ ...styles.btn, ...styles.btnPrimary, opacity: saving ? 0.5 : 1 }}
                        onClick={handleSaveManual}
                        disabled={saving}
                      >
                        {saving ? "Saving..." : "Save"}
                      </button>
                    )}
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Status + actions */}
          {smithery?.connected && (
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
                    disabled={testing}
                  >
                    {testing ? "Testing..." : "Test Connection"}
                  </button>
                  <button
                    style={{ ...styles.btn, ...styles.btnGhost, opacity: enriching ? 0.5 : 1 }}
                    onClick={handleBulkEnrich}
                    disabled={enriching}
                  >
                    {enriching ? "Updating..." : "Update All"}
                  </button>
                </div>
              </div>

              <div style={styles.settingRow}>
                <div>
                  <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                    Auto-update new contacts
                  </span>
                  <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                    Automatically update when new people are created
                  </p>
                </div>
                <button
                  style={{ ...styles.btn, ...styles.btnGhost }}
                  onClick={toggleAutoEnrich}
                >
                  {status.autoEnrichOnCreate ? "Disable" : "Enable"}
                </button>
              </div>

              {status.lastEnrichmentAt && (
                <div style={{ padding: "8px 0", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                  Last updated: {new Date(status.lastEnrichmentAt).toLocaleString()}
                </div>
              )}
            </>
          )}
        </>
      )}
    </div>
  );
}
