import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";
import { styles } from "@/components/settings/styles";
import type {
  PostMeetingCaptureConfig,
  FeatureDefinition,
  AiModelConfig,
  HygieneStatusView,
  HygieneNarrativeView,
} from "@/types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTime(iso?: string): string {
  if (!iso) return "--";
  try {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

function ChevronSvg({ open }: { open: boolean }) {
  return (
    <svg
      width="12"
      height="12"
      viewBox="0 0 12 12"
      fill="none"
      style={{
        transition: "transform 0.2s ease",
        transform: open ? "rotate(90deg)" : "rotate(0deg)",
        flexShrink: 0,
      }}
    >
      <path
        d="M4.5 2.5L8 6L4.5 9.5"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// UpdateSection — app version + update check
// ---------------------------------------------------------------------------

type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "available"; update: Update }
  | { phase: "installing" }
  | { phase: "error"; message: string };

function UpdateSection() {
  const [appVersion, setAppVersion] = useState<string>("");
  const [state, setState] = useState<UpdateState>({ phase: "idle" });

  useEffect(() => {
    getVersion().then(setAppVersion).catch(() => {});
  }, []);

  async function handleCheck() {
    setState({ phase: "checking" });
    try {
      const update = await check();
      if (update) {
        setState({ phase: "available", update });
      } else {
        toast.success("You're on the latest version");
        setState({ phase: "idle" });
      }
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`Update check failed: ${message}`);
      setState({ phase: "error", message });
    }
  }

  async function handleInstall() {
    if (state.phase !== "available") return;
    const { update } = state;
    setState({ phase: "installing" });
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      toast.error(`Update failed: ${message}`);
      setState({ phase: "error", message });
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Updates</p>
      <p style={{ ...styles.description, marginBottom: 12 }}>
        {appVersion ? `DailyOS v${appVersion}` : "DailyOS"}
      </p>

      {state.phase === "idle" || state.phase === "error" ? (
        <div style={styles.settingRow}>
          <span style={styles.description}>
            {state.phase === "error" ? "Update check failed" : "Check for new versions"}
          </span>
          <button style={{ ...styles.btn, ...styles.btnGhost }} onClick={handleCheck}>
            Check for Updates
          </button>
        </div>
      ) : state.phase === "checking" ? (
        <div style={styles.settingRow}>
          <span style={styles.description}>Checking for updates...</span>
          <button style={{ ...styles.btn, ...styles.btnGhost, opacity: 0.5 }} disabled>
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> Checking
            </span>
          </button>
        </div>
      ) : state.phase === "available" ? (
        <div>
          <div style={styles.settingRow}>
            <div>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  fontWeight: 500,
                  color: "var(--color-text-primary)",
                }}
              >
                v{state.update.version} available
              </span>
              {state.update.body && (
                <p style={{ ...styles.description, fontSize: 12, marginTop: 4 }}>
                  {state.update.body}
                </p>
              )}
            </div>
            <button style={{ ...styles.btn, ...styles.btnPrimary }} onClick={handleInstall}>
              Install &amp; Restart
            </button>
          </div>
        </div>
      ) : state.phase === "installing" ? (
        <div style={styles.settingRow}>
          <span style={styles.description}>Installing update...</span>
          <button style={{ ...styles.btn, ...styles.btnGhost, opacity: 0.5 }} disabled>
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> Installing
            </span>
          </button>
        </div>
      ) : null}
    </div>
  );
}

// ---------------------------------------------------------------------------
// HealthOneLiner — last briefing + intelligence health summary
// ---------------------------------------------------------------------------

function HealthOneLiner() {
  const [lastBriefing, setLastBriefing] = useState<string | null>(null);
  const [healthSummary, setHealthSummary] = useState<string | null>(null);

  useEffect(() => {
    invoke<Record<string, unknown>>("get_config")
      .then((config) => {
        const schedules = config.schedules as Record<string, unknown> | undefined;
        if (schedules?.lastBriefingTime) {
          setLastBriefing(formatTime(schedules.lastBriefingTime as string));
        } else if (schedules?.dailyBriefingTime) {
          setLastBriefing(String(schedules.dailyBriefingTime));
        }
      })
      .catch(() => {});

    invoke<HygieneStatusView>("get_intelligence_hygiene_status")
      .then((status) => {
        const parts: string[] = [];
        if (status.gaps.length > 0) {
          parts.push(`${status.gaps.length} gap${status.gaps.length !== 1 ? "s" : ""}`);
        }
        if (status.totalFixes > 0) {
          parts.push(`${status.totalFixes} fix${status.totalFixes !== 1 ? "es" : ""} applied`);
        }
        if (parts.length === 0) {
          setHealthSummary("All clear");
        } else {
          setHealthSummary(parts.join(", "));
        }
      })
      .catch(() => {});
  }, []);

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 16 }}>
      {lastBriefing && (
        <p style={{ ...styles.monoLabel, margin: 0 }}>
          Last briefing: {lastBriefing}
        </p>
      )}
      {healthSummary && (
        <p style={{ ...styles.monoLabel, margin: 0 }}>
          Intelligence health: {healthSummary}
        </p>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// AiModelsSection
// ---------------------------------------------------------------------------

const modelOptions = ["haiku", "sonnet", "opus"] as const;

const tierDescriptions: Record<string, { label: string; description: string }> = {
  synthesis: {
    label: "Synthesis",
    description: "Intelligence, briefings, weekly narrative",
  },
  extraction: {
    label: "Extraction",
    description: "Emails, meeting preps",
  },
  mechanical: {
    label: "Mechanical",
    description: "Inbox classification, transcripts",
  },
};

function AiModelsSection() {
  const [aiModels, setAiModels] = useState<AiModelConfig | null>(null);

  useEffect(() => {
    invoke<{ aiModels?: AiModelConfig }>("get_config")
      .then((config) => {
        setAiModels(
          config.aiModels ?? { synthesis: "sonnet", extraction: "sonnet", mechanical: "haiku" },
        );
      })
      .catch(() => {});
  }, []);

  async function handleModelChange(tier: string, model: string) {
    if (!aiModels) return;
    try {
      await invoke("set_ai_model", { tier, model });
      setAiModels({ ...aiModels, [tier]: model });
      toast.success(`${tierDescriptions[tier]?.label ?? tier} model set to ${model}`);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update model");
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>AI Models</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Choose which Claude model handles each type of operation
      </p>
      <div style={{ display: "flex", flexDirection: "column" }}>
        {(["synthesis", "extraction", "mechanical"] as const).map((tier) => {
          const info = tierDescriptions[tier];
          const current = aiModels?.[tier] ?? "sonnet";
          return (
            <div key={tier} style={styles.settingRow}>
              <div>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {info.label}
                </span>
                <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                  {info.description}
                </p>
              </div>
              <div style={{ display: "flex", gap: 4 }}>
                {modelOptions.map((model) => (
                  <button
                    key={model}
                    style={{
                      ...styles.btn,
                      ...(current === model ? styles.btnPrimary : styles.btnGhost),
                      padding: "3px 10px",
                      opacity: !aiModels ? 0.5 : 1,
                    }}
                    onClick={() => handleModelChange(tier, model)}
                    disabled={!aiModels}
                  >
                    {model}
                  </button>
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// FeaturesSection
// ---------------------------------------------------------------------------

function FeaturesSection() {
  const [features, setFeatures] = useState<FeatureDefinition[]>([]);

  useEffect(() => {
    invoke<FeatureDefinition[]>("get_features")
      .then(setFeatures)
      .catch(() => {});
  }, []);

  async function toggleFeature(key: string, currentEnabled: boolean) {
    try {
      await invoke("set_feature_enabled", { feature: key, enabled: !currentEnabled });
      setFeatures((prev) =>
        prev.map((f) => (f.key === key ? { ...f, enabled: !currentEnabled } : f)),
      );
    } catch (err) {
      console.error("Failed to toggle feature:", err);
    }
  }

  if (features.length === 0) return null;

  return (
    <div>
      <p style={styles.subsectionLabel}>Features</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Enable or disable pipeline operations
      </p>
      <div style={{ display: "flex", flexDirection: "column" }}>
        {features.map((feature) => (
          <div key={feature.key} style={styles.settingRow}>
            <div>
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {feature.label}
                </span>
                {feature.csOnly && (
                  <span style={{ ...styles.monoLabel, fontSize: 10 }}>CS</span>
                )}
              </div>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                {feature.description}
              </p>
            </div>
            <button
              style={{
                ...styles.btn,
                ...(feature.enabled ? styles.btnPrimary : styles.btnGhost),
                padding: "3px 10px",
              }}
              onClick={() => toggleFeature(feature.key, feature.enabled)}
            >
              {feature.enabled ? "Enabled" : "Disabled"}
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// HygieneSection — intelligence hygiene config + narrative
// ---------------------------------------------------------------------------

interface HygieneConfig {
  hygieneScanIntervalHours: number;
  hygieneAiBudget: number;
  hygienePreMeetingHours: number;
}

const scanIntervalOptions = [1, 2, 4, 8] as const;
const aiBudgetOptions = [5, 10, 20, 50] as const;
const preMeetingOptions = [2, 4, 12, 24] as const;

function HygieneSection() {
  const navigate = useNavigate();
  const [status, setStatus] = useState<HygieneStatusView | null>(null);
  const [narrative, setNarrative] = useState<HygieneNarrativeView | null>(null);
  const [loading, setLoading] = useState(true);
  const [runningNow, setRunningNow] = useState(false);
  const [hygieneConfig, setHygieneConfig] = useState<HygieneConfig>({
    hygieneScanIntervalHours: 4,
    hygieneAiBudget: 10,
    hygienePreMeetingHours: 12,
  });

  async function loadStatus() {
    try {
      const result = await invoke<HygieneStatusView>("get_intelligence_hygiene_status");
      setStatus(result);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to load hygiene status");
    } finally {
      setLoading(false);
    }
    invoke<HygieneNarrativeView | null>("get_hygiene_narrative")
      .then(setNarrative)
      .catch(() => {});
  }

  useEffect(() => {
    loadStatus();
    invoke<HygieneConfig & Record<string, unknown>>("get_config")
      .then((config) => {
        setHygieneConfig({
          hygieneScanIntervalHours: config.hygieneScanIntervalHours ?? 4,
          hygieneAiBudget: config.hygieneAiBudget ?? 10,
          hygienePreMeetingHours: config.hygienePreMeetingHours ?? 12,
        });
      })
      .catch(() => {});
  }, []);

  async function runScanNow() {
    setRunningNow(true);
    try {
      const updated = await invoke<HygieneStatusView>("run_hygiene_scan_now");
      setStatus(updated);
      invoke<HygieneNarrativeView | null>("get_hygiene_narrative")
        .then(setNarrative)
        .catch(() => {});
      toast.success("Hygiene scan complete");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to run hygiene scan");
    } finally {
      setRunningNow(false);
    }
  }

  async function handleHygieneConfigChange(
    field: "scanIntervalHours" | "aiBudget" | "preMeetingHours",
    value: number,
  ) {
    try {
      await invoke("set_hygiene_config", {
        [field === "scanIntervalHours"
          ? "scanIntervalHours"
          : field === "aiBudget"
            ? "aiBudget"
            : "preMeetingHours"]: value,
      });
      setHygieneConfig((prev) => ({
        ...prev,
        ...(field === "scanIntervalHours" && { hygieneScanIntervalHours: value }),
        ...(field === "aiBudget" && { hygieneAiBudget: value }),
        ...(field === "preMeetingHours" && { hygienePreMeetingHours: value }),
      }));
      toast.success("Hygiene configuration updated");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update hygiene config");
    }
  }

  if (loading) {
    return (
      <div>
        <p style={styles.subsectionLabel}>Intelligence Hygiene</p>
        <div
          style={{
            height: 24,
            width: 200,
            background: "var(--color-rule-light)",
            borderRadius: 4,
            marginBottom: 12,
            animation: "pulse 1.5s ease-in-out infinite",
          }}
        />
        <div
          style={{
            height: 80,
            background: "var(--color-rule-light)",
            borderRadius: 4,
            animation: "pulse 1.5s ease-in-out infinite",
          }}
        />
      </div>
    );
  }

  if (!status) {
    return (
      <div>
        <p style={styles.subsectionLabel}>Intelligence Hygiene</p>
        <p style={styles.description}>
          No scan completed yet -- runs automatically after startup.
        </p>
      </div>
    );
  }

  const severityDotColor = (severity: string) => {
    switch (severity) {
      case "critical":
        return "var(--color-spice-terracotta)";
      case "medium":
        return "var(--color-spice-turmeric)";
      default:
        return "var(--color-text-tertiary)";
    }
  };

  return (
    <div>
      <p style={styles.subsectionLabel}>Intelligence Hygiene</p>

      {/* Narrative prose (when available) */}
      {narrative && (
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 17,
            color: "var(--color-text-secondary)",
            lineHeight: 1.55,
            maxWidth: 580,
            margin: "0 0 16px",
          }}
        >
          {narrative.narrative}
        </p>
      )}

      {/* Fixes -- what the system healed */}
      {status.totalFixes > 0 && (
        <div style={{ marginBottom: 24 }}>
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-garden-sage)",
              marginBottom: 8,
            }}
          >
            Healed
          </p>
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {status.fixDetails.length > 0
              ? status.fixDetails.map((fix, i) => (
                  <div
                    key={i}
                    style={{ display: "flex", alignItems: "center", gap: 8 }}
                  >
                    <div
                      style={{
                        width: 6,
                        height: 6,
                        borderRadius: "50%",
                        backgroundColor: "var(--color-garden-sage)",
                        flexShrink: 0,
                      }}
                    />
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-secondary)",
                      }}
                    >
                      {fix.description}
                      {fix.entityName && (
                        <span style={{ color: "var(--color-text-tertiary)" }}>
                          {" \u2014 "}{fix.entityName}
                        </span>
                      )}
                    </span>
                  </div>
                ))
              : status.fixes.map((fix) => (
                  <div
                    key={fix.key}
                    style={{ display: "flex", alignItems: "center", gap: 8 }}
                  >
                    <div
                      style={{
                        width: 6,
                        height: 6,
                        borderRadius: "50%",
                        backgroundColor: "var(--color-garden-sage)",
                        flexShrink: 0,
                      }}
                    />
                    <span
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-secondary)",
                      }}
                    >
                      {fix.label}
                    </span>
                  </div>
                ))}
          </div>
        </div>
      )}

      {/* Gaps -- remaining issues (clickable) */}
      {status.gaps.length > 0 && (
        <div style={{ marginBottom: 24 }}>
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              textTransform: "uppercase",
              letterSpacing: "0.1em",
              color: "var(--color-spice-terracotta)",
              marginBottom: 8,
            }}
          >
            Remaining
          </p>
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            {status.gaps.map((gap) => {
              const isClickable = gap.action.kind === "navigate" && gap.action.route;
              return (
                <div
                  key={gap.key}
                  role={isClickable ? "button" : undefined}
                  tabIndex={isClickable ? 0 : undefined}
                  onClick={
                    isClickable
                      ? () => navigate({ to: gap.action.route! })
                      : undefined
                  }
                  onKeyDown={
                    isClickable
                      ? (e: React.KeyboardEvent) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.preventDefault();
                            navigate({ to: gap.action.route! });
                          }
                        }
                      : undefined
                  }
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: 8,
                    cursor: isClickable ? "pointer" : "default",
                    padding: "4px 0",
                    borderRadius: 4,
                  }}
                >
                  <div
                    style={{
                      width: 6,
                      height: 6,
                      borderRadius: "50%",
                      backgroundColor: severityDotColor(gap.impact),
                      flexShrink: 0,
                    }}
                  />
                  <span
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: isClickable
                        ? "var(--color-text-primary)"
                        : "var(--color-text-secondary)",
                      textDecoration: isClickable ? "underline" : "none",
                      textDecorationColor: "var(--color-rule-light)",
                      textUnderlineOffset: 2,
                    }}
                  >
                    {gap.label}
                  </span>
                  {isClickable && (
                    <span
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        textTransform: "uppercase",
                      }}
                    >
                      {gap.action.label}
                    </span>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Scan timestamp */}
      {status.lastScanTime && (
        <p
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            color: "var(--color-text-tertiary)",
            margin: "0 0 24px",
          }}
        >
          Last scan: {formatTime(status.lastScanTime)}
        </p>
      )}

      {/* Configuration */}
      <div style={{ marginBottom: 32 }}>
        <p style={styles.subsectionLabel}>Configuration</p>
        <div style={{ display: "flex", flexDirection: "column" }}>
          <div style={styles.settingRow}>
            <div>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                }}
              >
                Scan Interval
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                How often hygiene runs
              </p>
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              {scanIntervalOptions.map((v) => (
                <button
                  key={v}
                  style={{
                    ...styles.btn,
                    ...(hygieneConfig.hygieneScanIntervalHours === v
                      ? styles.btnPrimary
                      : styles.btnGhost),
                    padding: "3px 10px",
                  }}
                  onClick={() => handleHygieneConfigChange("scanIntervalHours", v)}
                >
                  {v}hr
                </button>
              ))}
            </div>
          </div>
          <div style={styles.settingRow}>
            <div>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                }}
              >
                Daily AI Budget
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Max AI enrichments per day
              </p>
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              {aiBudgetOptions.map((v) => (
                <button
                  key={v}
                  style={{
                    ...styles.btn,
                    ...(hygieneConfig.hygieneAiBudget === v
                      ? styles.btnPrimary
                      : styles.btnGhost),
                    padding: "3px 10px",
                  }}
                  onClick={() => handleHygieneConfigChange("aiBudget", v)}
                >
                  {v}
                </button>
              ))}
            </div>
          </div>
          <div style={styles.settingRow}>
            <div>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-primary)",
                }}
              >
                Pre-Meeting Window
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Refresh intel before meetings
              </p>
            </div>
            <div style={{ display: "flex", gap: 4 }}>
              {preMeetingOptions.map((v) => (
                <button
                  key={v}
                  style={{
                    ...styles.btn,
                    ...(hygieneConfig.hygienePreMeetingHours === v
                      ? styles.btnPrimary
                      : styles.btnGhost),
                    padding: "3px 10px",
                  }}
                  onClick={() => handleHygieneConfigChange("preMeetingHours", v)}
                >
                  {v}hr
                </button>
              ))}
            </div>
          </div>
        </div>
      </div>

      {/* Action buttons */}
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <button
          style={{
            ...styles.btn,
            ...styles.btnPrimary,
            opacity: runningNow || status.isRunning ? 0.5 : 1,
          }}
          onClick={runScanNow}
          disabled={runningNow || status.isRunning}
        >
          {runningNow || status.isRunning ? (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> Scanning...
            </span>
          ) : (
            "Run Hygiene Scan Now"
          )}
        </button>
        <button
          style={{ ...styles.btn, color: "var(--color-text-tertiary)", border: "none" }}
          onClick={loadStatus}
        >
          Refresh
        </button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// CaptureSection — post-meeting capture toggle + delay
// ---------------------------------------------------------------------------

function CaptureSection() {
  const [captureConfig, setCaptureConfig] = useState<PostMeetingCaptureConfig | null>(null);

  useEffect(() => {
    invoke<PostMeetingCaptureConfig>("get_capture_settings")
      .then(setCaptureConfig)
      .catch(() => {});
  }, []);

  async function toggleCapture() {
    if (!captureConfig) return;
    const newEnabled = !captureConfig.enabled;
    try {
      await invoke("set_capture_enabled", { enabled: newEnabled });
      setCaptureConfig({ ...captureConfig, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle capture:", err);
    }
  }

  async function updateDelay(minutes: number) {
    if (!captureConfig) return;
    try {
      await invoke("set_capture_delay", { delayMinutes: minutes });
      setCaptureConfig({ ...captureConfig, delayMinutes: minutes });
    } catch (err) {
      console.error("Failed to update delay:", err);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Post-Meeting Capture</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Prompt for quick outcomes after customer meetings
      </p>
      <div style={styles.settingRow}>
        <div>
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-primary)",
            }}
          >
            {captureConfig?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {captureConfig?.enabled
              ? "Prompts appear after customer meetings end"
              : "Post-meeting prompts are turned off"}
          </p>
        </div>
        <button
          style={{
            ...styles.btn,
            ...styles.btnGhost,
            opacity: !captureConfig ? 0.5 : 1,
          }}
          onClick={toggleCapture}
          disabled={!captureConfig}
        >
          {captureConfig?.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {captureConfig?.enabled && (
        <div style={{ ...styles.settingRow, marginTop: 8 }}>
          <div>
            <span
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-text-primary)",
              }}
            >
              Prompt delay
            </span>
            <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
              Wait before showing the prompt
            </p>
          </div>
          <div style={{ display: "flex", gap: 4 }}>
            {[2, 5, 10].map((mins) => (
              <button
                key={mins}
                style={{
                  ...styles.btn,
                  ...(captureConfig.delayMinutes === mins
                    ? styles.btnPrimary
                    : styles.btnGhost),
                  padding: "3px 10px",
                }}
                onClick={() => updateDelay(mins)}
              >
                {mins}m
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// SystemStatus — main exported component
// ---------------------------------------------------------------------------

export default function SystemStatus() {
  const [advancedOpen, setAdvancedOpen] = useState(false);

  return (
    <div>
      {/* Always visible: version, last briefing, health one-liner */}
      <UpdateSection />
      <HealthOneLiner />

      <hr style={{ ...styles.thinRule, margin: "24px 0" }} />

      {/* Advanced disclosure */}
      <button
        onClick={() => setAdvancedOpen(!advancedOpen)}
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
          background: "none",
          border: "none",
          cursor: "pointer",
          padding: "4px 0",
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 600,
          letterSpacing: "0.06em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
        }}
      >
        Advanced
        <ChevronSvg open={advancedOpen} />
      </button>

      {advancedOpen && (
        <div style={{ display: "flex", flexDirection: "column", gap: 32, marginTop: 24 }}>
          <AiModelsSection />
          <FeaturesSection />
          <HygieneSection />
          <CaptureSection />
        </div>
      )}
    </div>
  );
}
