import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { styles } from "./styles";
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
// Toggle
// ═══════════════════════════════════════════════════════════════════════════

function Toggle({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (next: boolean) => void;
}) {
  return (
    <button
      type="button"
      className={s.switch}
      data-checked={checked}
      onClick={() => onChange(!checked)}
      aria-pressed={checked}
    >
      <span className={s.switchThumb} />
    </button>
  );
}

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
      <p style={styles.subsectionLabel}>Notifications</p>
      <p className={s.description}>
        Control which native alerts DailyOS sends to your notification center.
      </p>

      {/* Daily briefing alerts */}
      <div className={s.toggleRow}>
        <div className={s.toggleInfo}>
          <span className={s.toggleLabel}>Daily briefing alerts</span>
          <span className={s.toggleDescription}>
            Notifies when your morning briefing is ready
          </span>
        </div>
        <Toggle
          checked={config.workflowCompletion}
          onChange={(v) => save({ ...config, workflowCompletion: v })}
        />
      </div>

      {/* Meeting notes alerts */}
      <div className={s.toggleRow}>
        <div className={s.toggleInfo}>
          <span className={s.toggleLabel}>Meeting notes alerts</span>
          <span className={s.toggleDescription}>
            Notifies when meeting transcripts are processed (rate-limited to once per 5 minutes)
          </span>
        </div>
        <Toggle
          checked={config.transcriptReady}
          onChange={(v) => save({ ...config, transcriptReady: v })}
        />
      </div>

      {/* Connection alerts */}
      <div className={s.toggleRow}>
        <div className={s.toggleInfo}>
          <span className={s.toggleLabel}>Connection alerts</span>
          <span className={s.toggleDescription}>
            Notifies when Google account needs reconnection
          </span>
        </div>
        <Toggle
          checked={config.authExpiry}
          onChange={(v) => save({ ...config, authExpiry: v })}
        />
      </div>

      {/* Quiet hours */}
      <div className={s.quietHoursSection}>
        <div className={s.quietHoursToggle}>
          <div className={s.toggleInfo}>
            <span className={s.toggleLabel}>Quiet hours</span>
            <span className={s.toggleDescription}>
              Suppress all notifications during these hours
            </span>
          </div>
          <Toggle checked={quietEnabled} onChange={handleQuietToggle} />
        </div>

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
