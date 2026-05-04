import { useState, useEffect, type CSSProperties } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  SettingsButton,
  SettingsSectionLabel,
  formRowStyles,
} from "@/components/settings/FormRow";
import surface from "./ConnectorSurface.module.css";

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
  const BACKFILL_DAYS = 365;
  const [status, setStatus] = useState<QuillStatusData | null>(null);
  const [testing, setTesting] = useState(false);
  const [backfilling, setBackfilling] = useState(false);

  useEffect(() => {
    invoke<QuillStatusData>("get_quill_status")
      .then(setStatus)
      .catch((err) => console.error("get_quill_status failed:", err)); // Expected: background init on mount
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_quill_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Quill:", err);
      toast.error("Failed to toggle Quill");
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      const ok = await invoke<boolean>("test_quill_connection");
      toast(ok ? "Quill connection successful" : "Quill bridge not available");
    } catch {
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
      <div className={surface.intro}>
        <SettingsSectionLabel>Quill Transcripts</SettingsSectionLabel>
        <p className={`${formRowStyles.description} ${surface.introDescription}`}>
          Automatically sync meeting transcripts from Quill
        </p>
      </div>

      <div className={formRowStyles.settingRow}>
        <div className={surface.settingCopy}>
          <span className={surface.settingTitle}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p className={surface.settingDescription}>
            {status?.enabled
              ? "Transcripts will sync after meetings end"
              : "Quill transcript sync is turned off"}
          </p>
        </div>
        <SettingsButton
          tone="ghost"
          className={!status ? surface.disabledButton : undefined}
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </SettingsButton>
      </div>

      {status?.enabled && (
        <>
          {!status.bridgeExists && (
            <div className={surface.callout}>
              <p className={surface.calloutLabel}>Setup Required</p>
              <p className={surface.calloutText}>
                To connect Quill, enable MCP in the Quill app first:
              </p>
              <ol className={surface.setupList}>
                <li>Open Quill → Settings → Integrations</li>
                <li>Enable MCP Server</li>
                <li>Restart Quill — this creates the bridge file DailyOS needs</li>
              </ol>
              <p className={`${surface.calloutText} ${surface.calloutTextSpaced} ${surface.inlineCode}`}>
                {status.bridgePath}
              </p>
              <p className={`${surface.calloutText} ${surface.calloutTextSpaced}`}>
                Requires Node.js installed on your system
              </p>
            </div>
          )}

          <div className={formRowStyles.settingRow}>
            <div className={surface.statusSummary}>
              <div
                className={surface.statusDot}
                style={{ "--connector-status-color": statusColor } as CSSProperties}
              />
              <span className={surface.statusText}>{statusLabel}</span>
            </div>
            <SettingsButton
              tone="ghost"
              className={testing ? surface.disabledButton : undefined}
              onClick={testConnection}
              disabled={testing}
            >
              {testing ? "Testing..." : "Test Connection"}
            </SettingsButton>
          </div>

          <div className={`${formRowStyles.settingRow} ${formRowStyles.noBorder}`}>
            <div className={surface.settingCopy}>
              <span className={formRowStyles.monoLabel}>Bridge path</span>
              <p className={surface.bridgePathValue}>{status.bridgePath}</p>
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
              {status.abandonedSyncs > 0 && (
                <span className={`${surface.statsLabel} ${surface.statsAbandoned}`}>
                  {status.abandonedSyncs} abandoned
                </span>
              )}
            </div>
          )}

          {status.lastError && (
            <div className={surface.errorRow}>
              <span className={surface.errorText}>{status.lastError}</span>
              {status.lastErrorAt && (
                <span className={surface.errorTimestamp}>
                  {new Date(status.lastErrorAt).toLocaleString()}
                </span>
              )}
            </div>
          )}

          <div className={formRowStyles.settingRow}>
            <div className={surface.settingCopy}>
              <span className={surface.settingTitle}>Poll interval</span>
              <p className={surface.settingDescription}>
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

          <div className={formRowStyles.settingRow}>
            <div className={surface.settingCopy}>
              <span className={surface.settingTitle}>Historical backfill</span>
              <p className={surface.settingDescription}>
                Create sync rows for past meetings (last {BACKFILL_DAYS} days)
              </p>
            </div>
            <SettingsButton
              tone="ghost"
              className={backfilling ? surface.disabledButton : undefined}
              onClick={async () => {
                setBackfilling(true);
                try {
                  const result = await invoke<{ created: number; eligible: number }>("start_quill_backfill", {
                    daysBack: BACKFILL_DAYS,
                  });
                  toast(`Backfill: ${result.created} of ${result.eligible} eligible meetings queued`);
                  const refreshed = await invoke<QuillStatusData>("get_quill_status");
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
            </SettingsButton>
          </div>
        </>
      )}
    </div>
  );
}
