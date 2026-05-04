import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FormRow, SettingsSectionLabel } from "@/components/settings/FormRow";
import { Switch } from "@/components/ui/Switch";
import s from "./NotificationSection.module.css";

// ═══════════════════════════════════════════════════════════════════════════
// Types
// ═══════════════════════════════════════════════════════════════════════════

interface NotificationConfig {
  workflowCompletion: boolean;
  transcriptReady: boolean;
  authExpiry: boolean;
  quietHoursStart: number | null;
  quietHoursEnd: number | null;
}

interface BackendConfig {
  notifications: NotificationConfig;
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

function formatHour(hour: number): string {
  const h = hour % 12 || 12;
  const ampm = hour < 12 ? "AM" : "PM";
  return `${h}:00 ${ampm}`;
}

const HOUR_OPTIONS = Array.from({ length: 24 }, (_, i) => i);

// ═══════════════════════════════════════════════════════════════════════════
// NotificationSection
// ═══════════════════════════════════════════════════════════════════════════

export default function NotificationSection() {
  const [config, setConfig] = useState<NotificationConfig | null>(null);
  const [saveMessage, setSaveMessage] = useState("");

  useEffect(() => {
    invoke<BackendConfig>("get_config")
      .then((cfg) => setConfig(cfg.notifications))
      .catch((err) => console.warn("Failed to load notification config:", err));
  }, []);

  const save = useCallback(async (next: NotificationConfig) => {
    setConfig(next);
    setSaveMessage("");
    try {
      await invoke("set_notification_config", { config: next });
      setSaveMessage("Saved");
      setTimeout(() => setSaveMessage(""), 2000);
    } catch (err) {
      console.error("Failed to save notification config:", err);
      setSaveMessage("Failed to save");
    }
  }, []);

  if (!config) return null;

  const quietEnabled =
    config.quietHoursStart !== null && config.quietHoursEnd !== null;

  function handleQuietToggle(enabled: boolean) {
    if (!config) return;
    if (enabled) {
      save({ ...config, quietHoursStart: 22, quietHoursEnd: 7 });
    } else {
      save({ ...config, quietHoursStart: null, quietHoursEnd: null });
    }
  }

  return (
    <div className={s.container}>
      <SettingsSectionLabel>Notifications</SettingsSectionLabel>
      <p className={s.description}>
        Control which native alerts DailyOS sends to your notification center.
      </p>

      {/* Daily briefing alerts */}
      <FormRow
        label="Daily briefing alerts"
        help="Notifies when your daily briefing is ready"
      >
        <Switch
          aria-label="Daily briefing alerts"
          checked={config.workflowCompletion}
          onCheckedChange={(v) => save({ ...config, workflowCompletion: v })}
        />
      </FormRow>

      {/* Meeting notes alerts */}
      <FormRow
        label="Meeting notes alerts"
        help="Notifies when meeting transcripts are processed (rate-limited to once per 5 minutes)"
      >
        <Switch
          aria-label="Meeting notes alerts"
          checked={config.transcriptReady}
          onCheckedChange={(v) => save({ ...config, transcriptReady: v })}
        />
      </FormRow>

      {/* Connection alerts */}
      <FormRow
        label="Connection alerts"
        help="Notifies when Google account needs reconnection"
      >
        <Switch
          aria-label="Connection alerts"
          checked={config.authExpiry}
          onCheckedChange={(v) => save({ ...config, authExpiry: v })}
        />
      </FormRow>

      {/* Quiet hours */}
      <div className={s.quietHoursSection}>
        <FormRow
          label="Quiet hours"
          help="Suppress all notifications during these hours"
        >
          <Switch
            aria-label="Quiet hours"
            checked={quietEnabled}
            onCheckedChange={handleQuietToggle}
          />
        </FormRow>

        {quietEnabled && (
          <div className={s.quietHoursRow}>
            <span className={s.hourLabel}>From</span>
            <select
              className={s.hourSelect}
              value={config.quietHoursStart ?? 22}
              onChange={(e) =>
                save({ ...config, quietHoursStart: parseInt(e.target.value, 10) })
              }
            >
              {HOUR_OPTIONS.map((h) => (
                <option key={h} value={h}>
                  {formatHour(h)}
                </option>
              ))}
            </select>
            <span className={s.hourLabel}>to</span>
            <select
              className={s.hourSelect}
              value={config.quietHoursEnd ?? 7}
              onChange={(e) =>
                save({ ...config, quietHoursEnd: parseInt(e.target.value, 10) })
              }
            >
              {HOUR_OPTIONS.map((h) => (
                <option key={h} value={h}>
                  {formatHour(h)}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      {saveMessage && <div className={s.saveStatus}>{saveMessage}</div>}
    </div>
  );
}
