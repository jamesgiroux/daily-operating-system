import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";
import {
  SettingsButton,
  SettingsRule,
  SettingsSectionLabel,
  SettingsStatusDot,
  formRowStyles,
} from "@/components/settings/FormRow";

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
      .catch((err) => console.error("Claude Desktop status check failed:", err)); // Expected: background init on mount
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
      <SettingsRule />
      <SettingsSectionLabel>Cowork Plugins</SettingsSectionLabel>
      <p className={formRowStyles.descriptionLead}>
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
                  className={formRowStyles.descriptionTinyTop2}
                >
                  {plugin.description}
                </p>
              </div>
              <SettingsButton
                tone="ghost"
                className={formRowStyles.buttonFlexShrink}
                onClick={() => handleExport(plugin.name)}
                disabled={exporting === plugin.name}
              >
                {exporting === plugin.name ? (
                  <>
                    <Loader2 size={12} className="animate-spin" /> Saving...
                  </>
                ) : exported[plugin.name] ? (
                  "Saved to Desktop"
                ) : (
                  "Save to Desktop"
                )}
              </SettingsButton>
            </div>
          ))}
      </div>

      <p className={formRowStyles.descriptionTinyItalic}>
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
      .catch((err) => console.error("Claude Desktop status check failed:", err)); // Expected: background init on mount
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
      <SettingsSectionLabel>Claude Desktop</SettingsSectionLabel>
      <p className={formRowStyles.descriptionLead}>
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
          <SettingsStatusDot
            color={result.success ? "var(--color-garden-sage)" : "var(--color-spice-terracotta)"}
          />
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-secondary)" }}>
            {result.message}
          </span>
        </div>
      )}

      <SettingsButton
        tone="ghost"
        onClick={handleConfigure}
        disabled={configuring}
      >
        {configuring ? (
          <>
            <Loader2 size={12} className="animate-spin" /> Configuring...
          </>
        ) : result?.success ? (
          "Reconfigure"
        ) : (
          "Connect to Claude Desktop"
        )}
      </SettingsButton>

      <p className={formRowStyles.descriptionSmallTop4}>
        Adds DailyOS as an MCP server in Claude Desktop. After connecting,
        Claude can query your briefing, accounts, projects, and meeting
        history.
      </p>
      <p style={{
        fontFamily: "var(--font-sans)",
        fontSize: 12,
        color: "var(--color-text-tertiary)",
        margin: "8px 0 0",
        fontStyle: "italic",
      }}>
        After connecting, restart Claude Desktop for the MCP server to take effect.
      </p>

      <CoworkPluginsSubsection />
    </div>
  );
}
