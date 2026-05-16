import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { BarChart3, Loader2, ShieldCheck, ShieldOff } from "lucide-react";
import { toast } from "sonner";
import styles from "./TelemetryOptInSplash.module.css";

interface AggregateTelemetryStatus {
  enabled: boolean;
  optInSplashDismissed: boolean;
  catalog: string[];
}

export function TelemetryOptInSplash() {
  const [status, setStatus] = useState<AggregateTelemetryStatus | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<AggregateTelemetryStatus>("get_aggregate_telemetry_status")
      .then(setStatus)
      .catch((err) => console.error("Telemetry splash status load failed:", err));
  }, []);

  async function keepOff() {
    setSaving(true);
    try {
      const next = await invoke<AggregateTelemetryStatus>("dismiss_aggregate_telemetry_splash");
      setStatus(next);
    } catch (err) {
      toast.error(`Telemetry preference failed: ${err}`);
    } finally {
      setSaving(false);
    }
  }

  async function turnOn() {
    setSaving(true);
    try {
      const next = await invoke<AggregateTelemetryStatus>("set_aggregate_telemetry_enabled", {
        enabled: true,
      });
      setStatus(next);
      toast.success("Anonymous metrics are on");
    } catch (err) {
      toast.error(`Telemetry preference failed: ${err}`);
    } finally {
      setSaving(false);
    }
  }

  if (!status || status.optInSplashDismissed) return null;

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="telemetry-opt-in-title"
      className={styles.overlay}
    >
      <section className={styles.panel}>
        <div className={styles.header}>
          <BarChart3 size={18} className={styles.turmericIcon} />
          <h2
            id="telemetry-opt-in-title"
            className={styles.title}
          >
            Share anonymous product metrics
          </h2>
        </div>

        <p className={styles.body}>
          DailyOS can send a small set of anonymous aggregate measurements to
          help validate whether context quality and reliability are improving
          across installs. The default is off.
        </p>

        <div className={styles.points}>
          <div className={styles.point}>
            <ShieldCheck size={14} className={styles.olivePointIcon} />
            <p className={styles.pointText}>
              Sent only when enabled: counts, durations, percentiles, and booleans for{" "}
              {status.catalog.join(", ")}.
            </p>
          </div>
          <div className={styles.point}>
            <ShieldOff size={14} className={styles.terracottaPointIcon} />
            <p className={styles.pointText}>
              Never sent: names, account data, saved facts or context text,
              source references, prompts, hashes, file paths, or invocation IDs.
            </p>
          </div>
        </div>

        <div className={styles.actions}>
          <button
            type="button"
            onClick={keepOff}
            disabled={saving}
            className={styles.secondaryButton}
          >
            Keep off
          </button>
          <button
            type="button"
            onClick={turnOn}
            disabled={saving}
            className={styles.primaryButton}
          >
            {saving && <Loader2 size={12} className="animate-spin" />}
            Turn on
          </button>
        </div>
      </section>
    </div>
  );
}

export default TelemetryOptInSplash;
