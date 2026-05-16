import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { BarChart3, Loader2, ShieldCheck } from "lucide-react";
import { toast } from "sonner";
import { Switch } from "@/components/ui/Switch";
import {
  SettingsButton,
  SettingsRule,
  SettingsSectionLabel,
  formRowStyles,
} from "@/features/settings-ui/FormRow";
import styles from "./PrivacyPanel.module.css";

type MetricValuePreview =
  | { type: "count"; value: number }
  | { type: "duration"; valueMs: number }
  | { type: "percentile"; quantile: string; valueMs: number }
  | { type: "boolean"; value: boolean };

interface AggregateMetricPreview {
  metricName: string;
  metricValue: MetricValuePreview;
  abilityName?: string | null;
  abilityVersion?: string | null;
  signalType?: string | null;
  outcome?: "success" | "failure" | "skipped" | "timeout" | null;
  bucketStart: string;
  buildVersion: string;
}

interface AggregateTelemetryStatus {
  enabled: boolean;
  optInSplashDismissed: boolean;
  catalog: string[];
  preview: AggregateMetricPreview[];
}

function formatMetricValue(value: MetricValuePreview) {
  switch (value.type) {
    case "count":
      return `${value.value}`;
    case "duration":
      return `${value.valueMs}ms`;
    case "percentile":
      return `p${value.quantile} ${value.valueMs}ms`;
    case "boolean":
      return value.value ? "true" : "false";
  }
}

export default function PrivacyPanel() {
  const [status, setStatus] = useState<AggregateTelemetryStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [confirmingEnable, setConfirmingEnable] = useState(false);

  async function refresh() {
    const next = await invoke<AggregateTelemetryStatus>("get_aggregate_telemetry_status");
    setStatus(next);
    setLoading(false);
  }

  useEffect(() => {
    refresh().catch((err) => {
      console.error("Telemetry status load failed:", err);
      setLoading(false);
    });
    let unlisten: (() => void) | undefined;
    void listen("telemetry-status-updated", () => {
      refresh().catch((err) => console.error("Telemetry status refresh failed:", err));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  async function setTelemetryEnabled(enabled: boolean) {
    setSaving(true);
    try {
      const next = await invoke<AggregateTelemetryStatus>("set_aggregate_telemetry_enabled", {
        enabled,
      });
      setStatus(next);
      setConfirmingEnable(false);
      toast.success(enabled ? "Anonymous metrics are on" : "Anonymous metrics are off");
    } catch (err) {
      toast.error(`Telemetry update failed: ${err}`);
    } finally {
      setSaving(false);
    }
  }

  const enabled = status?.enabled ?? false;
  const preview = status?.preview ?? [];

  return (
    <div>
      <SettingsSectionLabel>Anonymous product metrics</SettingsSectionLabel>
      <p className={formRowStyles.descriptionLead}>
        Send anonymous aggregate counts, timings, percentages, and booleans so
        DailyOS can measure whether context quality and reliability are improving.
      </p>
      <p className={formRowStyles.descriptionSmallBottom8}>
        Never sent: account names, people, saved facts or context text, source
        identifiers, prompts, file paths, content hashes, invocation IDs, or
        email content.
      </p>

      <div className={styles.statusRow}>
        <div className={styles.statusCopy}>
          <ShieldCheck size={15} className={styles.oliveIcon} />
          <span className={formRowStyles.descriptionTight}>
            {enabled ? "Anonymous metrics are on" : "Anonymous metrics are off"}
          </span>
        </div>
        <Switch
          checked={enabled}
          disabled={loading || saving}
          aria-label="Anonymous product metrics"
          onCheckedChange={(checked) => {
            if (checked) {
              setConfirmingEnable(true);
            } else {
              void setTelemetryEnabled(false);
            }
          }}
        />
      </div>

      {confirmingEnable && (
        <div className={styles.confirmationPanel}>
          <div className={styles.previewHeader}>
            <BarChart3 size={14} className={styles.turmericIcon} />
            <span className={formRowStyles.descriptionTight}>
              Last 24 hours sample
            </span>
          </div>
          {preview.length > 0 ? (
            <div className={styles.previewList}>
              {preview.slice(0, 8).map((metric, idx) => (
                <div
                  key={`${metric.metricName}-${metric.bucketStart}-${idx}`}
                  className={styles.metricRow}
                >
                  <span>{metric.metricName}</span>
                  <span>{formatMetricValue(metric.metricValue)}</span>
                </div>
              ))}
            </div>
          ) : (
            <p className={formRowStyles.descriptionSmallBottom8}>
              No aggregate rows have been collected in the current 24-hour window.
            </p>
          )}
          <div className={styles.confirmActions}>
            <SettingsButton
              tone="primary"
              onClick={() => void setTelemetryEnabled(true)}
              disabled={saving}
              muted={saving}
            >
              {saving ? <Loader2 size={12} className="animate-spin" /> : <ShieldCheck size={12} />}
              Turn on
            </SettingsButton>
            <SettingsButton tone="ghost" onClick={() => setConfirmingEnable(false)}>
              Keep off
            </SettingsButton>
          </div>
        </div>
      )}

      <div className={styles.catalog}>
        {(status?.catalog ?? []).map((metric) => (
          <span key={metric} className={styles.catalogItem}>
            {metric}
          </span>
        ))}
      </div>

      <SettingsRule />
    </div>
  );
}
