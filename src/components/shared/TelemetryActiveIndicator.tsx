import { useEffect, useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { BarChart3 } from "lucide-react";
import styles from "./TelemetryActiveIndicator.module.css";

interface AggregateTelemetryStatus {
  enabled: boolean;
}

export function TelemetryActiveIndicator() {
  const navigate = useNavigate();
  const [enabled, setEnabled] = useState(false);

  async function refresh() {
    const status = await invoke<AggregateTelemetryStatus>("get_aggregate_telemetry_status");
    setEnabled(status.enabled);
  }

  useEffect(() => {
    refresh().catch((err) => console.error("Telemetry indicator status load failed:", err));
    let unlisten: (() => void) | undefined;
    void listen("telemetry-status-updated", () => {
      refresh().catch((err) => console.error("Telemetry indicator refresh failed:", err));
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, []);

  if (!enabled) return null;

  return (
    <button
      type="button"
      aria-label="Anonymous metrics are on"
      title="Anonymous metrics are on"
      onClick={() => void navigate({ to: "/settings", search: { tab: "data" } })}
      className={styles.indicator}
    >
      <BarChart3 size={16} strokeWidth={1.8} />
    </button>
  );
}

export default TelemetryActiveIndicator;
