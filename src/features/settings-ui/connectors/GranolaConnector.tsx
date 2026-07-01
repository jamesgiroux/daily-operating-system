import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import clsx from "clsx";
import { AlertCircle, Check, Loader2, LogIn, LogOut, RefreshCw } from "lucide-react";
import {
  FormRow,
  SettingsButton,
  SettingsInput,
  SettingsSectionLabel,
  formRowStyles,
} from "@/features/settings-ui/FormRow";
import { useGranolaAuth } from "@/hooks/useGranolaAuth";
import type { GranolaStatus, GranolaTokenHealth } from "@/types";
import surface from "./ConnectorSurface.module.css";

const DEFAULT_GRANOLA_ENDPOINT = "https://mcp.granola.ai/mcp";
const BACKFILL_DAYS = 365;

export default function GranolaConnection() {
  const granola = useGranolaAuth();
  const previousPhase = useRef(granola.phase);
  const [endpoint, setEndpoint] = useState(DEFAULT_GRANOLA_ENDPOINT);
  const [status, setStatus] = useState<GranolaStatus | null>(null);
  const [tokenHealth, setTokenHealth] = useState<GranolaTokenHealth | null>(null);
  const [backfilling, setBackfilling] = useState(false);

  const isConnected = granola.status.status === "authenticated";

  const refreshStatus = useCallback(async () => {
    try {
      const result = await invoke<GranolaStatus>("get_granola_status");
      setStatus(result);
      if (result.mcpEndpoint) {
        setEndpoint(result.mcpEndpoint);
      }
    } catch (err) {
      console.error("get_granola_status failed:", err);
    }
  }, []);

  const refreshTokenHealth = useCallback(async () => {
    try {
      const health = await invoke<GranolaTokenHealth>("get_granola_token_health");
      setTokenHealth(health);
    } catch {
      setTokenHealth(null);
    }
  }, []);

  useEffect(() => {
    void refreshStatus();
    void refreshTokenHealth();
  }, [refreshStatus, refreshTokenHealth]);

  useEffect(() => {
    const previous = previousPhase.current;
    previousPhase.current = granola.phase;

    if (previous !== "idle" && granola.phase === "idle") {
      void refreshStatus();
      void refreshTokenHealth();
    }
  }, [granola.phase, refreshStatus, refreshTokenHealth]);

  const handleConnect = async () => {
    const target = endpoint.trim() || DEFAULT_GRANOLA_ENDPOINT;
    await granola.connect(target);
    void refreshStatus();
    void refreshTokenHealth();
  };

  const handleDisconnect = async () => {
    await granola.disconnect();
    void refreshStatus();
    void refreshTokenHealth();
  };

  const handlePollIntervalChange = async (minutes: number) => {
    try {
      await invoke("set_granola_poll_interval", { minutes });
      setStatus((current) =>
        current ? { ...current, pollIntervalMinutes: minutes } : current,
      );
    } catch (err) {
      console.error("Failed to set Granola poll interval:", err);
      toast.error("Failed to update poll interval");
    }
  };

  const handleBackfill = async () => {
    setBackfilling(true);
    try {
      const result = await invoke<{ created: number; eligible: number }>("start_granola_backfill", {
        daysBack: BACKFILL_DAYS,
      });
      toast(`Backfill: ${result.created} of ${result.eligible} documents matched`);
      void refreshStatus();
    } catch {
      toast.error("Backfill failed");
    } finally {
      setBackfilling(false);
    }
  };

  const statusLabel = isConnected
    ? granola.email || "Connected"
    : granola.phase === "authorizing"
      ? "Waiting for authorization..."
      : "Not connected";

  const statusDotClass = isConnected
    ? surface.statusDotConnected
    : granola.phase === "authorizing"
      ? surface.statusDotWarning
      : surface.statusDotNeutral;
  const hasSyncStats = Boolean(
    status && (status.pendingSyncs > 0 || status.failedSyncs > 0 || status.completedSyncs > 0),
  );

  return (
    <div>
      <div className={surface.intro}>
        <SettingsSectionLabel>Granola Transcripts</SettingsSectionLabel>
        <p className={`${formRowStyles.description} ${surface.introDescription}`}>
          Sync meeting notes and transcripts from Granola.
        </p>
      </div>

      <div className={formRowStyles.settingRow}>
        <div className={surface.statusSummary}>
          {granola.phase === "authorizing" ? (
            <Loader2 className="animate-spin" size={14} />
          ) : isConnected ? (
            <Check size={14} />
          ) : (
            <AlertCircle size={14} />
          )}
          <span className={clsx(surface.statusDot, statusDotClass)} />
          <span className={surface.statusText}>{statusLabel}</span>
        </div>
        <div className={surface.actionRow}>
          {isConnected ? (
            <SettingsButton
              tone="danger"
              onClick={handleDisconnect}
              disabled={granola.loading}
            >
              {granola.phase === "disconnecting" ? (
                <Loader2 className="animate-spin" size={14} />
              ) : (
                <LogOut size={14} />
              )}
              Disconnect
            </SettingsButton>
          ) : (
            <SettingsButton
              tone="primary"
              onClick={handleConnect}
              disabled={granola.loading}
            >
              {granola.phase === "authorizing" ? (
                <Loader2 className="animate-spin" size={14} />
              ) : (
                <LogIn size={14} />
              )}
              Connect
            </SettingsButton>
          )}
        </div>
      </div>

      <FormRow
        label="MCP endpoint"
        help="Granola OAuth resource"
        controlId="granola-mcp-endpoint"
      >
        <SettingsInput
          id="granola-mcp-endpoint"
          value={endpoint}
          onChange={(event) => setEndpoint(event.target.value)}
          width={300}
          disabled={granola.loading}
        />
      </FormRow>

      {granola.error && (
        <div className={surface.errorRow}>
          <span className={surface.errorText}>{granola.error}</span>
          <SettingsButton tone="borderless" compact onClick={granola.clearError}>
            Clear
          </SettingsButton>
        </div>
      )}

      {tokenHealth?.connected && tokenHealth.status !== "healthy" && (
        <div className={surface.callout}>
          <p className={surface.calloutLabel}>Session Attention</p>
          <p className={surface.calloutText}>
            Granola authorization is {tokenHealth.status}. Reconnect if sync stops.
          </p>
        </div>
      )}

      {isConnected && (
        <>
          {status && hasSyncStats ? (
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
            </div>
          ) : null}

          <div className={formRowStyles.settingRow}>
            <div className={surface.settingCopy}>
              <span className={surface.settingTitle}>Poll interval</span>
              <p className={surface.settingDescription}>
                How often DailyOS checks Granola for new notes
              </p>
            </div>
            <select
              value={status?.pollIntervalMinutes ?? 10}
              onChange={(event) => void handlePollIntervalChange(Number(event.target.value))}
              className={surface.selectControl}
            >
              {[1, 2, 5, 10, 15, 30].map((minutes) => (
                <option key={minutes} value={minutes}>
                  {minutes} min
                </option>
              ))}
            </select>
          </div>

          <div className={formRowStyles.settingRow}>
            <div className={surface.settingCopy}>
              <span className={surface.settingTitle}>Historical backfill</span>
              <p className={surface.settingDescription}>
                Match Granola notes to past meetings from the last {BACKFILL_DAYS} days
              </p>
            </div>
            <SettingsButton
              tone="ghost"
              onClick={handleBackfill}
              disabled={backfilling}
            >
              {backfilling ? (
                <Loader2 className="animate-spin" size={14} />
              ) : (
                <RefreshCw size={14} />
              )}
              {backfilling ? "Running..." : "Start Backfill"}
            </SettingsButton>
          </div>
        </>
      )}
    </div>
  );
}
