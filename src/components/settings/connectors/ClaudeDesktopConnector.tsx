import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import { styles } from "../styles";

// ═══════════════════════════════════════════════════════════════════════════
// CoworkPluginsSubsection
// ═══════════════════════════════════════════════════════════════════════════

function CoworkPluginsSubsection() {
  const [plugins, setPlugins] = useState<
    { name: string; description: string; filename: string; available: boolean; exported: boolean }[]
  >([]);
  const [exporting, setExporting] = useState<string | null>(null);
  const [exported, setExported] = useState<Record<string, boolean>>({});

  useEffect(() => {
    invoke<
      { name: string; description: string; filename: string; available: boolean; exported: boolean }[]
    >("get_cowork_plugins_status")
      .then((res) => {
        setPlugins(res);
        const initial: Record<string, boolean> = {};
        for (const p of res) {
          if (p.exported) initial[p.name] = true;
        }
        setExported(initial);
      })
      .catch((err) => console.error("Claude Desktop status check failed:", err));
  }, []);

  const handleExport = async (pluginName: string) => {
    setExporting(pluginName);
    try {
      const res = await invoke<{ success: boolean; message: string; path: string | null }>(
        "export_cowork_plugin",
        { pluginName }
      );
      if (res.success) {
        setExported((prev) => ({ ...prev, [pluginName]: true }));
        toast.success(res.message);
      } else {
        toast.error(res.message);
      }
    } catch (e) {
      toast.error(String(e));
    } finally {
      setExporting(null);
    }
  };

  if (plugins.length === 0) return null;

  return (
    <>
      <hr
        style={{
          border: "none",
          borderTop: "1px solid var(--color-rule)",
          margin: "20px 0",
        }}
      />
      <p style={styles.subsectionLabel}>Cowork Plugins</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Install plugins in Claude Desktop's Cowork sidebar for live workspace access.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
        {plugins
          .filter((p) => p.available)
          .map((plugin) => (
            <div
              key={plugin.name}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 12,
              }}
            >
              <div style={{ flex: 1, minWidth: 0 }}>
                <span
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 12,
                    fontWeight: 600,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {plugin.name}
                </span>
                <p
                  style={{
                    ...styles.description,
                    fontSize: 11,
                    margin: "2px 0 0",
                  }}
                >
                  {plugin.description}
                </p>
              </div>
              <button
                style={{
                  ...styles.btn,
                  ...styles.btnGhost,
                  flexShrink: 0,
                  opacity: exporting === plugin.name ? 0.5 : 1,
                }}
                onClick={() => handleExport(plugin.name)}
                disabled={exporting === plugin.name}
              >
                {exporting === plugin.name ? (
                  <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                    <Loader2 size={12} className="animate-spin" /> Saving...
                  </span>
                ) : exported[plugin.name] ? (
                  "Saved to Desktop"
                ) : (
                  "Save to Desktop"
                )}
              </button>
            </div>
          ))}
      </div>

      <p style={{ ...styles.description, fontSize: 11, marginTop: 12, fontStyle: "italic" }}>
        Drag the zip from your Desktop into Claude Desktop's Cowork sidebar to install.
      </p>
    </>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// ClaudeDesktopConnection
// ═══════════════════════════════════════════════════════════════════════════

export default function ClaudeDesktopConnection() {
  const [configuring, setConfiguring] = useState(false);
  const [result, setResult] = useState<{
    success: boolean;
    message: string;
    configPath?: string;
    binaryPath?: string;
  } | null>(null);

  useEffect(() => {
    invoke<{ success: boolean; message: string; configPath: string | null; binaryPath: string | null }>(
      "get_claude_desktop_status"
    )
      .then((res) => {
        setResult({
          success: res.success,
          message: res.message,
          configPath: res.configPath ?? undefined,
          binaryPath: res.binaryPath ?? undefined,
        });
      })
      .catch((err) => console.error("Claude Desktop status check failed:", err));
  }, []);

  const handleConfigure = async () => {
    setConfiguring(true);
    setResult(null);
    try {
      const res = await invoke<{
        success: boolean;
        message: string;
        configPath: string | null;
        binaryPath: string | null;
      }>("configure_claude_desktop");
      setResult({
        success: res.success,
        message: res.message,
        configPath: res.configPath ?? undefined,
        binaryPath: res.binaryPath ?? undefined,
      });
      if (res.success) {
        toast.success("Claude Desktop configured");
      } else {
        toast.error(res.message);
      }
    } catch (e) {
      setResult({
        success: false,
        message: String(e),
      });
      toast.error("Failed to configure Claude Desktop");
    } finally {
      setConfiguring(false);
    }
  };

  return (
    <div>
      <p style={styles.subsectionLabel}>Claude Desktop</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Connect Claude Desktop to query your workspace via MCP
      </p>

      {result && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: 8,
            padding: "10px 0",
            marginBottom: 12,
          }}
        >
          <div
            style={styles.statusDot(
              result.success ? "var(--color-garden-sage)" : "var(--color-spice-terracotta)"
            )}
          />
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-secondary)" }}>
            {result.message}
          </span>
        </div>
      )}

      <button
        style={{
          ...styles.btn,
          ...styles.btnGhost,
          opacity: configuring ? 0.5 : 1,
        }}
        onClick={handleConfigure}
        disabled={configuring}
      >
        {configuring ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <Loader2 size={12} className="animate-spin" /> Configuring...
          </span>
        ) : result?.success ? (
          "Reconfigure"
        ) : (
          "Connect to Claude Desktop"
        )}
      </button>

      <p style={{ ...styles.description, fontSize: 12, marginTop: 12 }}>
        Adds DailyOS as an MCP server in Claude Desktop. After connecting,
        Claude can query your briefing, accounts, projects, and meeting
        history.
      </p>

      <CoworkPluginsSubsection />
    </div>
  );
}
