import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Globe, HardDrive, RefreshCw, Check, AlertCircle } from "lucide-react";
import { styles } from "@/components/settings/styles";

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

interface ContextModeLocal {
  mode: "Local";
}

interface ContextModeGlean {
  mode: "Glean";
  endpoint: string;
  keychain_key: string;
  strategy: "Additive" | "Governed";
}

type ContextMode = ContextModeLocal | ContextModeGlean;

// ═══════════════════════════════════════════════════════════════════════════
// Component
// ═══════════════════════════════════════════════════════════════════════════

export default function ContextSourceSection() {
  const [mode, setMode] = useState<ContextMode>({ mode: "Local" });
  const [hasToken, setHasToken] = useState(false);
  const [endpoint, setEndpoint] = useState(
    "https://automattic-be.glean.com/mcp/default"
  );
  const [strategy, setStrategy] = useState<"Additive" | "Governed">(
    "Additive"
  );
  const [tokenInput, setTokenInput] = useState("");
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  const load = useCallback(async () => {
    try {
      const [currentMode, tokenStatus] = await Promise.all([
        invoke<ContextMode>("get_context_mode"),
        invoke<boolean>("get_glean_token_status"),
      ]);
      setMode(currentMode);
      setHasToken(tokenStatus);
      if (currentMode.mode === "Glean") {
        setEndpoint(currentMode.endpoint);
        setStrategy(currentMode.strategy);
      }
    } catch {
      // DB not ready yet — defaults are fine
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const handleSaveGlean = async () => {
    setSaving(true);
    try {
      // Save token if provided
      if (tokenInput.trim()) {
        await invoke("save_glean_token", { token: tokenInput.trim() });
        setTokenInput("");
        setHasToken(true);
      }

      // Save mode
      const newMode: ContextModeGlean = {
        mode: "Glean",
        endpoint,
        keychain_key: "com.dailyos.desktop.glean",
        strategy,
      };
      await invoke("set_context_mode", { mode: newMode });
      setMode(newMode);
      setDirty(false);
      toast.success("Context source updated. Restart the app to apply.");
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
      toast.success("Switched to local mode. Restart the app to apply.");
    } catch (e) {
      toast.error(`Failed to save: ${e}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDisconnectGlean = async () => {
    try {
      await invoke("delete_glean_token");
      await invoke("set_context_mode", { mode: { mode: "Local" } });
      setMode({ mode: "Local" });
      setHasToken(false);
      toast.success("Glean disconnected. Restart the app to apply.");
    } catch (e) {
      toast.error(`Failed to disconnect: ${e}`);
    }
  };

  const isGlean = mode.mode === "Glean";

  return (
    <div style={{ marginBottom: 32 }}>
      <h3 style={styles.subsectionLabel}>Context Source</h3>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Where DailyOS gathers context for briefings and intelligence. Local mode
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
            {hasToken ? (
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
                  Token required
                </span>
              </>
            )}
          </div>

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
            style={{ ...styles.input, marginBottom: 16 }}
          />

          {/* OAuth token */}
          {!hasToken && (
            <>
              <label style={styles.fieldLabel}>OAuth Token</label>
              <input
                type="password"
                value={tokenInput}
                onChange={(e) => setTokenInput(e.target.value)}
                placeholder="Paste token from Glean MCP Configurator"
                style={{ ...styles.input, marginBottom: 16 }}
              />
            </>
          )}

          {/* Strategy */}
          <label style={styles.fieldLabel}>Strategy</label>
          <div style={{ display: "flex", gap: 12, marginBottom: 16 }}>
            <label
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: "var(--color-text-secondary)",
                cursor: "pointer",
              }}
            >
              <input
                type="radio"
                name="glean-strategy"
                checked={strategy === "Additive"}
                onChange={() => {
                  setStrategy("Additive");
                  setDirty(true);
                }}
              />
              Additive
            </label>
            <label
              style={{
                display: "flex",
                alignItems: "center",
                gap: 6,
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: "var(--color-text-secondary)",
                cursor: "pointer",
              }}
            >
              <input
                type="radio"
                name="glean-strategy"
                checked={strategy === "Governed"}
                onChange={() => {
                  setStrategy("Governed");
                  setDirty(true);
                }}
              />
              Governed
            </label>
          </div>
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-text-tertiary)",
              margin: "0 0 16px",
            }}
          >
            {strategy === "Additive"
              ? "Glean is the primary context source. Local signals (Gmail, Calendar) are still active."
              : "Glean is the only context source. Gmail polling and local file enrichment are disabled."}
          </p>

          {/* Actions */}
          <div style={{ display: "flex", gap: 12 }}>
            {dirty && (
              <button
                onClick={handleSaveGlean}
                disabled={saving || (!hasToken && !tokenInput.trim())}
                style={{
                  ...styles.btn,
                  ...styles.btnPrimary,
                  display: "flex",
                  alignItems: "center",
                  gap: 6,
                  opacity:
                    saving || (!hasToken && !tokenInput.trim()) ? 0.5 : 1,
                }}
              >
                {saving ? (
                  <RefreshCw size={12} className="animate-spin" />
                ) : (
                  <Check size={12} />
                )}
                {saving ? "Saving…" : "Save & Restart Required"}
              </button>
            )}
            {isGlean && (
              <button
                onClick={handleDisconnectGlean}
                style={{
                  ...styles.btn,
                  ...styles.btnDanger,
                }}
              >
                Disconnect
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
