import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Globe, HardDrive, RefreshCw, Check, AlertCircle, Loader2 } from "lucide-react";
import { styles } from "@/components/settings/styles";
import { useGleanAuth } from "@/hooks/useGleanAuth";
import type { GleanTokenHealth } from "@/types";

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

interface ContextModeLocal {
  mode: "Local";
}

interface ContextModeGlean {
  mode: "Glean";
  endpoint: string;
}

type ContextMode = ContextModeLocal | ContextModeGlean;

// ═══════════════════════════════════════════════════════════════════════════
// Component
// ═══════════════════════════════════════════════════════════════════════════

export default function ContextSourceSection() {
  const [mode, setMode] = useState<ContextMode>({ mode: "Local" });
  const [endpoint, setEndpoint] = useState("");
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);
  const [tokenHealth, setTokenHealth] = useState<GleanTokenHealth | null>(null);

  const glean = useGleanAuth();
  const previousGleanPhase = useRef(glean.phase);
  const isConnected = glean.status.status === "authenticated";

  const load = useCallback(async () => {
    try {
      const currentMode = await invoke<ContextMode>("get_context_mode");
      setMode(currentMode);
      if (currentMode.mode === "Glean") {
        setEndpoint(currentMode.endpoint);
      }
    } catch {
      // DB not ready yet — defaults are fine
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const refreshTokenHealth = useCallback(async () => {
    try {
      const health = await invoke<GleanTokenHealth>("get_glean_token_health");
      setTokenHealth(health);
    } catch {
      setTokenHealth(null);
    }
  }, []);

  useEffect(() => {
    refreshTokenHealth();
    const interval = window.setInterval(refreshTokenHealth, 6 * 60 * 60 * 1000);
    return () => {
      window.clearInterval(interval);
    };
  }, [refreshTokenHealth]);

  useEffect(() => {
    const previousPhase = previousGleanPhase.current;
    previousGleanPhase.current = glean.phase;

    if (previousPhase !== "idle" && glean.phase === "idle") {
      void refreshTokenHealth();
    }
  }, [glean.phase, refreshTokenHealth]);

  const handleConnectGlean = async () => {
    if (!endpoint.trim()) {
      toast.error("MCP endpoint is required");
      return;
    }
    await glean.connect(endpoint.trim());
  };

  const handleSaveGlean = async () => {
    setSaving(true);
    try {
      const newMode: ContextModeGlean = {
        mode: "Glean",
        endpoint: endpoint.trim(),
      };
      await invoke("set_context_mode", { mode: newMode });
      setMode(newMode);
      setDirty(false);
      toast.success("Context source updated.");
    } catch (e) {
      toast.error(`Failed to save: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleSwitchToLocal = async () => {
    setSaving(true);
    try {
      await invoke("set_context_mode", { mode: { mode: "Local" } });
      setMode({ mode: "Local" });
      setDirty(false);
      toast.success("Switched to local mode.");
    } catch (e) {
      toast.error(`Failed to save: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDisconnectGlean = async () => {
    try {
      await glean.disconnect();
      await invoke("set_context_mode", { mode: { mode: "Local" } });
      setMode({ mode: "Local" });
      toast.success("Glean disconnected. Glean-derived data was cleared locally.");
    } catch (e) {
      toast.error(`Failed to disconnect: ${e}`);
    }
  };

  const isGlean = mode.mode === "Glean";

  return (
    <div style={{ marginBottom: 32 }}>
      <h3 style={styles.subsectionLabel}>Context Source</h3>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Where DailyOS gathers context for briefings and analysis. Local mode
        uses your workspace files and connectors. Glean mode uses your
        organization's knowledge graph.
      </p>

      {/* Mode selector */}
      <div style={{ display: "flex", gap: 12, marginBottom: 20 }}>
        <button
          onClick={() => {
            if (isGlean) {
              handleSwitchToLocal();
            }
          }}
          style={{
            ...styles.btn,
            ...(isGlean ? styles.btnGhost : styles.btnPrimary),
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "8px 16px",
          }}
        >
          <HardDrive size={14} />
          Local
          {!isGlean && <Check size={12} />}
        </button>
        <button
          onClick={() => {
            if (!isGlean) {
              setDirty(true);
            }
          }}
          style={{
            ...styles.btn,
            ...(isGlean ? styles.btnPrimary : styles.btnGhost),
            display: "flex",
            alignItems: "center",
            gap: 6,
            padding: "8px 16px",
          }}
        >
          <Globe size={14} />
          Glean
          {isGlean && <Check size={12} />}
        </button>
      </div>

      {/* Glean configuration panel */}
      {(isGlean || dirty) && (
        <div
          style={{
            padding: 20,
            borderRadius: 8,
            border: "1px solid var(--color-rule-light)",
            background: "var(--color-surface-inset)",
          }}
        >
          {/* Connection status */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              marginBottom: 16,
            }}
          >
            {isConnected ? (
              <>
                <Check
                  size={14}
                  style={{ color: "var(--color-garden-olive)" }}
                />
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-garden-olive)",
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                  }}
                >
                  Connected
                </span>
                {glean.email && (
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 12,
                      color: "var(--color-text-secondary)",
                      marginLeft: 4,
                    }}
                  >
                    {glean.email}
                  </span>
                )}
              </>
            ) : glean.phase === "authorizing" ? (
              <>
                <Loader2
                  size={14}
                  className="animate-spin"
                  style={{ color: "var(--color-spice-turmeric)" }}
                />
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-spice-turmeric)",
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                  }}
                >
                  Waiting for authorization...
                </span>
              </>
            ) : (
              <>
                <AlertCircle
                  size={14}
                  style={{ color: "var(--color-spice-turmeric)" }}
                />
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-spice-turmeric)",
                    textTransform: "uppercase",
                    letterSpacing: "0.06em",
                  }}
                >
                  Not connected
                </span>
              </>
            )}
          </div>

          {/* Error message */}
          {glean.error && (
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 12,
                color: "var(--color-earth-terracotta)",
                margin: "0 0 12px",
              }}
            >
              {glean.error}
            </p>
          )}

          {tokenHealth?.connected && tokenHealth.status !== "healthy" && (
            <div
              style={{
                marginBottom: 12,
                padding: 12,
                borderRadius: 6,
                border: "1px solid var(--color-spice-turmeric)",
                background: "var(--color-spice-turmeric-8)",
              }}
            >
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--color-text-primary)",
                  margin: "0 0 8px 0",
                }}
              >
                {tokenHealth.status === "expired"
                  ? "Your Glean token has expired. Reconnect now to resume enrichment."
                  : `Your Glean token expires in about ${tokenHealth.expiresInHours} hour${tokenHealth.expiresInHours === 1 ? "" : "s"}.`}
              </p>
              <button
                onClick={handleConnectGlean}
                style={{ ...styles.btn, ...styles.btnPrimary }}
              >
                Reconnect
              </button>
            </div>
          )}

          {/* Endpoint */}
          <label style={styles.fieldLabel}>MCP Endpoint</label>
          <input
            type="text"
            value={endpoint}
            onChange={(e) => {
              setEndpoint(e.target.value);
              setDirty(true);
            }}
            placeholder="https://your-org.glean.com/mcp/default"
            style={{ ...styles.input, marginBottom: 0 }}
            disabled={glean.loading}
          />
          <p style={{
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            margin: "4px 0 12px",
          }}>
            Your Glean admin must enable MCP access for your organization. A browser window will open for you to authorize with your Glean account.
          </p>

          {/* Connect / Disconnect button */}
          {!isConnected ? (
            <button
              onClick={handleConnectGlean}
              disabled={
                glean.loading || !endpoint.trim()
              }
              style={{
                ...styles.btn,
                ...styles.btnPrimary,
                display: "flex",
                alignItems: "center",
                gap: 6,
                marginBottom: 16,
                opacity:
                  glean.loading || !endpoint.trim()
                    ? 0.5
                    : 1,
              }}
            >
              {glean.loading ? (
                <Loader2 size={12} className="animate-spin" />
              ) : (
                <Globe size={12} />
              )}
              {glean.loading ? "Connecting..." : "Connect to Glean"}
            </button>
          ) : (
            <>
              {/* Mode info */}
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  color: "var(--color-text-tertiary)",
                  margin: "0 0 16px",
                }}
              >
                Glean is the primary context source. Local sources (Gmail, Calendar) are still active.
              </p>

              {/* Actions */}
              <div style={{ display: "flex", gap: 12 }}>
                {dirty && (
                  <button
                    onClick={handleSaveGlean}
                    disabled={saving}
                    style={{
                      ...styles.btn,
                      ...styles.btnPrimary,
                      display: "flex",
                      alignItems: "center",
                      gap: 6,
                      opacity: saving ? 0.5 : 1,
                    }}
                  >
                    {saving ? (
                      <RefreshCw size={12} className="animate-spin" />
                    ) : (
                      <Check size={12} />
                    )}
                    {saving ? "Saving..." : "Save & Restart Required"}
                  </button>
                )}
                <button
                  onClick={handleDisconnectGlean}
                  style={{
                    ...styles.btn,
                    ...styles.btnDanger,
                  }}
                >
                  Disconnect
                </button>
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
