import { useState, useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { open } from "@tauri-apps/plugin-dialog";
import { useSearch, useNavigate } from "@tanstack/react-router";

import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { usePersonality, type Personality } from "@/hooks/usePersonality";
import { toast } from "sonner";
import {
  User,
  Cpu,
  Layers,
  Building2,
  FolderKanban,
  Globe,
  Activity,
  Check,
  Loader2,
  X,
  RefreshCw,
  Play,
  ToggleRight,
} from "lucide-react";
import type {
  PostMeetingCaptureConfig,
  FeatureDefinition,
  EntityMode,
  AiModelConfig,
  SettingsTabId,
  HygieneStatusView,
  HygieneNarrativeView,
} from "@/types";

// ═══════════════════════════════════════════════════════════════════════════
// Types & Constants
// ═══════════════════════════════════════════════════════════════════════════

interface Config {
  workspacePath: string;
  entityMode: EntityMode;
  developerMode: boolean;
  schedules: {
    today: ScheduleEntry;
    archive: ScheduleEntry;
  };
}

interface ScheduleEntry {
  enabled: boolean;
  cron: string;
  timezone: string;
}

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

function parseSettingsTab(value: unknown): SettingsTabId {
  if (
    value === "profile" ||
    value === "integrations" ||
    value === "workflows" ||
    value === "intelligence" ||
    value === "hygiene" ||
    value === "diagnostics"
  ) {
    return value;
  }
  return "profile";
}

// ═══════════════════════════════════════════════════════════════════════════
// Shared editorial styles
// ═══════════════════════════════════════════════════════════════════════════

const styles = {
  subsectionLabel: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    fontWeight: 600,
    letterSpacing: "0.06em",
    textTransform: "uppercase" as const,
    color: "var(--color-text-tertiary)",
    margin: 0,
    marginBottom: 12,
  },
  fieldLabel: {
    fontFamily: "var(--font-sans)",
    fontSize: 13,
    fontWeight: 500,
    color: "var(--color-text-secondary)",
    marginBottom: 4,
    display: "block" as const,
  },
  input: {
    width: "100%",
    fontFamily: "var(--font-sans)",
    fontSize: 14,
    color: "var(--color-text-primary)",
    background: "none",
    border: "none",
    borderBottom: "1px solid var(--color-rule-light)",
    padding: "8px 0",
    outline: "none",
  },
  btn: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    fontWeight: 600,
    letterSpacing: "0.06em",
    textTransform: "uppercase" as const,
    background: "none",
    borderRadius: 4,
    padding: "4px 14px",
    cursor: "pointer",
    transition: "all 0.15s ease",
  },
  btnPrimary: {
    color: "var(--color-garden-olive)",
    border: "1px solid var(--color-garden-olive)",
  },
  btnGhost: {
    color: "var(--color-text-tertiary)",
    border: "1px solid var(--color-rule-heavy)",
  },
  btnDanger: {
    color: "var(--color-spice-terracotta)",
    border: "1px solid var(--color-spice-terracotta)",
  },
  description: {
    fontFamily: "var(--font-sans)",
    fontSize: 13,
    color: "var(--color-text-tertiary)",
    lineHeight: 1.5,
    margin: 0,
  },
  settingRow: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "12px 0",
    borderBottom: "1px solid var(--color-rule-light)",
  },
  statusDot: (color: string) => ({
    width: 8,
    height: 8,
    borderRadius: "50%",
    background: color,
    flexShrink: 0 as const,
  }),
  monoLabel: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    fontWeight: 500,
    letterSpacing: "0.04em",
    color: "var(--color-text-tertiary)",
  },
  sectionGap: {
    marginBottom: 48,
  },
  thinRule: {
    height: 1,
    background: "var(--color-rule-light)",
    border: "none",
    margin: "16px 0",
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// Main page
// ═══════════════════════════════════════════════════════════════════════════

const CHAPTER_DEFS = [
  { id: "settings-profile", label: "Profile", icon: <User size={18} strokeWidth={1.5} /> },
  { id: "settings-integrations", label: "Integrations", icon: <Globe size={18} strokeWidth={1.5} /> },
  { id: "settings-workflows", label: "Workflows", icon: <Play size={18} strokeWidth={1.5} /> },
  { id: "settings-intelligence", label: "Intelligence", icon: <Cpu size={18} strokeWidth={1.5} /> },
  { id: "settings-hygiene", label: "Hygiene", icon: <Activity size={18} strokeWidth={1.5} /> },
];

const DIAGNOSTICS_CHAPTER = {
  id: "settings-diagnostics",
  label: "Diagnostics",
  icon: <ToggleRight size={18} strokeWidth={1.5} />,
};

export default function SettingsPage() {
  const search = useSearch({ from: "/settings" });
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  const [runResult, setRunResult] = useState<{ workflow: string; success: boolean; message: string } | null>(null);
  const scrolledRef = useRef(false);

  useEffect(() => {
    async function loadConfig() {
      try {
        const result = await invoke<Config>("get_config");
        setConfig(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load config");
      } finally {
        setLoading(false);
      }
    }
    loadConfig();
  }, []);

  // Deep-link scroll: if ?tab=X, scroll to that section on mount
  useEffect(() => {
    if (loading || scrolledRef.current) return;
    const tab = parseSettingsTab(search.tab);
    if (tab !== "profile") {
      const el = document.getElementById(`settings-${tab}`);
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "start" });
        scrolledRef.current = true;
      }
    }
  }, [loading, search.tab]);

  async function handleRunWorkflow(workflow: string) {
    setRunning(workflow);
    setRunResult(null);
    try {
      const result = await invoke<string>("run_workflow", { workflow });
      setRunResult({ workflow, success: true, message: result });
    } catch (err) {
      setRunResult({
        workflow,
        success: false,
        message: err instanceof Error ? err.message : "Unknown error",
      });
    } finally {
      setRunning(null);
    }
  }

  async function handleReloadConfig() {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Config>("reload_configuration");
      setConfig(result);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to reload config");
    } finally {
      setLoading(false);
    }
  }

  // Chapters: include diagnostics only in dev mode
  const chapters = useMemo(() => {
    if (import.meta.env.DEV) {
      return [...CHAPTER_DEFS, DIAGNOSTICS_CHAPTER];
    }
    return CHAPTER_DEFS;
  }, []);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Settings",
      atmosphereColor: "olive" as const,
      activePage: "settings" as const,
      chapters,
    }),
    [chapters],
  );
  useRegisterMagazineShell(shellConfig);

  if (loading) {
    return <EditorialLoading count={5} />;
  }

  if (error) {
    return (
      <EditorialError
        message={`${error} — Create a config file at ~/.dailyos/config.json with your workspace path.`}
        onRetry={handleReloadConfig}
      />
    );
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ HERO ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 40 }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 42,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            color: "var(--color-text-primary)",
            margin: 0,
          }}
        >
          Settings
        </h1>
        <div
          style={{
            height: 2,
            background: "var(--color-desk-charcoal)",
            marginTop: 16,
          }}
        />
      </section>

      {/* ═══ CHAPTER 1: PROFILE ═══ */}
      <section id="settings-profile" style={styles.sectionGap}>
        <ChapterHeading title="Profile" epigraph="Who you are and how your workspace is organized." />
        <UpdateCard />
        <div style={{ height: 32 }} />
        <UserProfileCard />
        <div style={{ height: 32 }} />
        <UserDomainsCard />
        <div style={{ height: 32 }} />
        <EntityModeCard
          currentMode={config?.entityMode ?? "account"}
          onModeChange={(mode) => setConfig(config ? { ...config, entityMode: mode } : null)}
        />
        <div style={{ height: 32 }} />
        <WorkspaceCard
          workspacePath={config?.workspacePath ?? ""}
          onPathChange={(path) => setConfig(config ? { ...config, workspacePath: path } : null)}
        />
        <div style={{ height: 32 }} />
        <PersonalityCard />
      </section>

      {/* ═══ CHAPTER 2: INTEGRATIONS ═══ */}
      <section id="settings-integrations" style={styles.sectionGap}>
        <ChapterHeading title="Integrations" epigraph="External services that feed your intelligence layer." />
        <GoogleAccountCard />
        <div style={{ height: 32 }} />
        <ClaudeDesktopCard />
        <div style={{ height: 32 }} />
        <hr style={{ border: "none", borderTop: "1px solid var(--color-rule-light)", margin: 0 }} />
        <div style={{ height: 32 }} />
        <QuillSettingsCard />
      </section>

      {/* ═══ CHAPTER 3: WORKFLOWS ═══ */}
      <section id="settings-workflows" style={styles.sectionGap}>
        <ChapterHeading title="Workflows" epigraph="Automated schedules and manual triggers." />
        <CaptureSettingsCard />
        <div style={{ height: 32 }} />
        <SchedulesSection config={config} running={running} onRun={handleRunWorkflow} />
        {runResult && (
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: 8,
              padding: "12px 0",
              borderBottom: "1px solid var(--color-rule-light)",
              marginTop: 16,
            }}
          >
            <div
              style={styles.statusDot(
                runResult.success ? "var(--color-garden-sage)" : "var(--color-spice-terracotta)"
              )}
            />
            <span
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: runResult.success ? "var(--color-garden-sage)" : "var(--color-spice-terracotta)",
              }}
            >
              {runResult.message}
            </span>
          </div>
        )}
        <div style={{ height: 32 }} />
        <ManualRunSection running={running} onRun={handleRunWorkflow} />
      </section>

      {/* ═══ CHAPTER 4: INTELLIGENCE ═══ */}
      <section id="settings-intelligence" style={styles.sectionGap}>
        <ChapterHeading title="Intelligence" epigraph="Feature toggles and AI model configuration." />
        <FeaturesCard />
        <div style={{ height: 32 }} />
        <AiModelsCard />
      </section>

      {/* ═══ CHAPTER 5: HYGIENE ═══ */}
      <section id="settings-hygiene" style={styles.sectionGap}>
        <ChapterHeading title="Hygiene" epigraph="Proactive intelligence maintenance with clear next actions." />
        <IntelligenceHygieneCard />
      </section>

      {/* ═══ CHAPTER 6: DIAGNOSTICS (dev only) ═══ */}
      {import.meta.env.DEV && (
        <section id="settings-diagnostics" style={styles.sectionGap}>
          <ChapterHeading title="Diagnostics" epigraph="Developer tools and debugging utilities." />
          <DeveloperToggle config={config} setConfig={setConfig} />
          <div style={{ height: 32 }} />
          <MeetingBackfillCard />
        </section>
      )}

      <FinisMarker />
      <div style={{ height: 80 }} />
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// Developer Toggle — extracted from inline diagnostics Card
// ═══════════════════════════════════════════════════════════════════════════

function DeveloperToggle({
  config,
  setConfig,
}: {
  config: Config | null;
  setConfig: (c: Config | null) => void;
}) {
  return (
    <div>
      <p style={styles.subsectionLabel}>Developer Tools</p>
      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            Developer Tools
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            Show the devtools panel (wrench icon)
          </p>
        </div>
        <button
          style={{
            ...styles.btn,
            ...(config?.developerMode ? styles.btnPrimary : styles.btnGhost),
          }}
          onClick={async () => {
            const next = !config?.developerMode;
            try {
              const updated = await invoke<Config>("set_developer_mode", { enabled: next });
              setConfig(updated);
              toast.success(next ? "Developer tools enabled — reloading..." : "Developer tools disabled — reloading...");
              setTimeout(() => window.location.reload(), 500);
            } catch (e) {
              toast.error(String(e));
            }
          }}
        >
          {config?.developerMode ? "On" : "Off"}
        </button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// Schedules Section
// ═══════════════════════════════════════════════════════════════════════════

function SchedulesSection({
  config,
  running,
  onRun,
}: {
  config: Config | null;
  running: string | null;
  onRun: (workflow: string) => void;
}) {
  return (
    <div>
      <p style={styles.subsectionLabel}>Schedules</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Automated workflow execution times
      </p>
      {config?.schedules && (
        <>
          <ScheduleRow
            label="Morning Briefing"
            schedule={config.schedules.today}
            running={running === "today"}
            onRun={() => onRun("today")}
          />
          <ScheduleRow
            label="Nightly Archive"
            schedule={config.schedules.archive}
            running={running === "archive"}
            onRun={() => onRun("archive")}
          />
        </>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// Manual Run Section
// ═══════════════════════════════════════════════════════════════════════════

function ManualRunSection({
  running,
  onRun,
}: {
  running: string | null;
  onRun: (workflow: string) => void;
}) {
  return (
    <div>
      <p style={styles.subsectionLabel}>Manual Run</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Trigger workflows manually without waiting for schedule
      </p>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 10 }}>
        <button
          style={{ ...styles.btn, ...styles.btnPrimary, opacity: running !== null ? 0.5 : 1 }}
          onClick={() => onRun("today")}
          disabled={running !== null}
        >
          {running === "today" ? (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> Running...
            </span>
          ) : (
            "Run Daily Briefing"
          )}
        </button>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: running !== null ? 0.5 : 1 }}
          onClick={() => onRun("week")}
          disabled={running !== null}
        >
          {running === "week" ? (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> Running...
            </span>
          ) : (
            "Run Weekly Briefing"
          )}
        </button>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: running !== null ? 0.5 : 1 }}
          onClick={() => onRun("archive")}
          disabled={running !== null}
        >
          {running === "archive" ? (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> Running...
            </span>
          ) : (
            "Run Archive"
          )}
        </button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// UpdateCard
// ═══════════════════════════════════════════════════════════════════════════

type UpdateState =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "available"; update: Update }
  | { phase: "installing" }
  | { phase: "error"; message: string };

function UpdateCard() {
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

// ═══════════════════════════════════════════════════════════════════════════
// ClaudeDesktopCard
// ═══════════════════════════════════════════════════════════════════════════

function ClaudeDesktopCard() {
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
      .catch(() => {});
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
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// GoogleAccountCard
// ═══════════════════════════════════════════════════════════════════════════

function GoogleAccountCard() {
  const {
    status,
    email,
    loading,
    phase,
    error,
    justConnected,
    connect,
    disconnect,
    clearError,
  } = useGoogleAuth();

  return (
    <div>
      <p style={styles.subsectionLabel}>Google Account</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        {status.status === "authenticated"
          ? "Calendar and meeting features active"
          : "Connect Google for calendar awareness and meeting features"}
      </p>

      {error && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "10px 0",
            borderBottom: "1px solid var(--color-spice-terracotta)",
            marginBottom: 12,
          }}
        >
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-spice-terracotta)",
            }}
          >
            {error}
          </span>
          <button
            style={{
              ...styles.btn,
              fontSize: 10,
              padding: "2px 8px",
              color: "var(--color-spice-terracotta)",
              border: "none",
            }}
            onClick={clearError}
          >
            Dismiss
          </button>
        </div>
      )}

      {status.status === "authenticated" ? (
        <div style={styles.settingRow}>
          <div>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={styles.statusDot("var(--color-garden-sage)")} />
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                {email}
              </span>
            </div>
            {justConnected && (
              <p
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  color: "var(--color-garden-sage)",
                  marginTop: 4,
                }}
              >
                Connected successfully.
              </p>
            )}
          </div>
          <button
            style={{ ...styles.btn, ...styles.btnGhost, opacity: loading || phase === "authorizing" ? 0.5 : 1 }}
            onClick={disconnect}
            disabled={loading || phase === "authorizing"}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : (
              "Disconnect"
            )}
          </button>
        </div>
      ) : status.status === "tokenexpired" ? (
        <div style={styles.settingRow}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={styles.statusDot("var(--color-spice-terracotta)")} />
            <span style={styles.description}>Session expired</span>
          </div>
          <button
            style={{ ...styles.btn, ...styles.btnDanger, opacity: loading ? 0.5 : 1 }}
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : phase === "authorizing" ? (
              "Waiting..."
            ) : (
              "Reconnect"
            )}
          </button>
        </div>
      ) : (
        <div style={styles.settingRow}>
          <span style={styles.description}>Not connected</span>
          <button
            style={{ ...styles.btn, ...styles.btnPrimary, opacity: loading ? 0.5 : 1 }}
            onClick={connect}
            disabled={loading}
          >
            {loading ? (
              <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
                <Loader2 size={12} className="animate-spin" /> ...
              </span>
            ) : phase === "authorizing" ? (
              "Waiting for authorization..."
            ) : (
              "Connect"
            )}
          </button>
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// PersonalityCard
// ═══════════════════════════════════════════════════════════════════════════

const PERSONALITY_OPTIONS = [
  {
    value: "professional",
    label: "Professional",
    description: "Straightforward, clean copy",
    example: "No data yet.",
  },
  {
    value: "friendly",
    label: "Friendly",
    description: "Warm, encouraging tone",
    example: "Nothing here yet — we'll have this ready for you soon.",
  },
  {
    value: "playful",
    label: "Playful",
    description: "Personality-rich, fun",
    example: "The hamsters are still running. Data incoming.",
  },
] as const;

function PersonalityCard() {
  const { personality, setPersonality: setCtxPersonality } = usePersonality();

  async function handleChange(value: string) {
    const previous = personality;
    setCtxPersonality(value as Personality);
    try {
      await invoke("set_personality", { personality: value });
      toast.success("Personality updated");
    } catch (err) {
      setCtxPersonality(previous);
      toast.error(typeof err === "string" ? err : "Failed to update personality");
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Personality</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Sets the tone for empty states, loading messages, and notifications
      </p>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {PERSONALITY_OPTIONS.map((option) => {
          const isSelected = personality === option.value;
          return (
            <button
              key={option.value}
              onClick={() => handleChange(option.value)}
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "flex-start",
                gap: 4,
                padding: "12px 16px",
                textAlign: "left" as const,
                background: "none",
                border: isSelected
                  ? "1px solid var(--color-desk-charcoal)"
                  : "1px solid var(--color-rule-light)",
                borderRadius: 4,
                cursor: "pointer",
                transition: "border-color 0.15s ease",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {option.label}
                </span>
                {isSelected && <Check size={14} style={{ color: "var(--color-garden-sage)" }} />}
              </div>
              <span style={{ ...styles.description, fontSize: 12 }}>
                {option.description}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-serif)",
                  fontSize: 12,
                  fontStyle: "italic",
                  color: "var(--color-text-tertiary)",
                  marginTop: 2,
                }}
              >
                "{option.example}"
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// UserProfileCard
// ═══════════════════════════════════════════════════════════════════════════

function UserProfileCard() {
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [title, setTitle] = useState("");
  const [focus, setFocus] = useState("");
  const [loading, setLoading] = useState(true);

  // Track initial values to detect actual changes on blur
  const initial = useRef({ name: "", company: "", title: "", focus: "" });

  useEffect(() => {
    invoke<{
      userName?: string;
      userCompany?: string;
      userTitle?: string;
      userFocus?: string;
    }>("get_config")
      .then((config) => {
        const n = config.userName ?? "";
        const c = config.userCompany ?? "";
        const t = config.userTitle ?? "";
        const f = config.userFocus ?? "";
        setName(n);
        setCompany(c);
        setTitle(t);
        setFocus(f);
        initial.current = { name: n, company: c, title: t, focus: f };
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  async function saveIfChanged() {
    const current = { name: name.trim(), company: company.trim(), title: title.trim(), focus: focus.trim() };
    if (
      current.name === initial.current.name &&
      current.company === initial.current.company &&
      current.title === initial.current.title &&
      current.focus === initial.current.focus
    ) return;
    try {
      await invoke("set_user_profile", {
        name: current.name || null,
        company: current.company || null,
        title: current.title || null,
        focus: current.focus || null,
        domain: null,
      });
      initial.current = current;
      toast.success("Profile updated");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update profile");
    }
  }

  if (loading) {
    return (
      <div>
        <p style={styles.subsectionLabel}>About You</p>
        <div
          style={{
            height: 40,
            background: "var(--color-rule-light)",
            borderRadius: 4,
            animation: "pulse 1.5s ease-in-out infinite",
          }}
        />
      </div>
    );
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>About You</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Helps DailyOS personalize your briefings and meeting prep
      </p>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: "20px 32px",
        }}
      >
        <div>
          <label htmlFor="profile-name" style={styles.fieldLabel}>Name</label>
          <input
            id="profile-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Jamie"
            style={styles.input}
          />
        </div>
        <div>
          <label htmlFor="profile-company" style={styles.fieldLabel}>Company</label>
          <input
            id="profile-company"
            value={company}
            onChange={(e) => setCompany(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Acme Inc."
            style={styles.input}
          />
        </div>
        <div>
          <label htmlFor="profile-title" style={styles.fieldLabel}>Title</label>
          <input
            id="profile-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Customer Success Manager"
            style={styles.input}
          />
        </div>
        <div>
          <label htmlFor="profile-focus" style={styles.fieldLabel}>Current focus</label>
          <input
            id="profile-focus"
            value={focus}
            onChange={(e) => setFocus(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Driving Q2 renewals"
            style={styles.input}
          />
        </div>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// UserDomainsCard
// ═══════════════════════════════════════════════════════════════════════════

function UserDomainsCard() {
  const [domains, setDomains] = useState<string[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<{ userDomains?: string[]; userDomain?: string }>("get_config")
      .then((config) => {
        const loaded = config.userDomains ?? (config.userDomain ? [config.userDomain] : []);
        setDomains(loaded.filter(Boolean));
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  async function saveDomains(next: string[]) {
    setSaving(true);
    try {
      const updated = await invoke<{ userDomains?: string[]; userDomain?: string }>(
        "set_user_domains",
        { domains: next.join(", ") },
      );
      const saved = updated.userDomains ?? (updated.userDomain ? [updated.userDomain] : []);
      setDomains(saved.filter(Boolean));
      toast.success("Domains updated");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update domains");
    } finally {
      setSaving(false);
    }
  }

  function addDomain(raw: string) {
    const d = raw.trim().toLowerCase().replace(/^@/, "");
    if (!d || domains.includes(d)) return;
    const next = [...domains, d];
    setDomains(next);
    setInputValue("");
    saveDomains(next);
  }

  function removeDomain(d: string) {
    const next = domains.filter((x) => x !== d);
    setDomains(next);
    saveDomains(next);
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if ((e.key === "," || e.key === "Enter" || e.key === "Tab") && inputValue.trim()) {
      e.preventDefault();
      addDomain(inputValue);
    }
    if (e.key === "Backspace" && !inputValue && domains.length > 0) {
      removeDomain(domains[domains.length - 1]);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Your Domains</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Your organization's email domains -- used to distinguish internal vs external meetings
      </p>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: 6,
          borderBottom: "1px solid var(--color-rule-light)",
          padding: "8px 0",
          minHeight: 36,
        }}
      >
        {domains.map((d) => (
          <span
            key={d}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-primary)",
              background: "var(--color-rule-light)",
              padding: "2px 8px",
              borderRadius: 3,
            }}
          >
            {d}
            <button
              type="button"
              onClick={() => removeDomain(d)}
              disabled={saving}
              style={{
                background: "none",
                border: "none",
                padding: 0,
                cursor: "pointer",
                color: "var(--color-text-tertiary)",
                display: "flex",
                alignItems: "center",
              }}
            >
              <X size={12} />
            </button>
          </span>
        ))}
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value.replace(",", ""))}
          onKeyDown={handleKeyDown}
          onBlur={() => {
            if (inputValue.trim()) addDomain(inputValue);
          }}
          placeholder={domains.length === 0 ? "example.com" : ""}
          style={{
            minWidth: 120,
            flex: 1,
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            color: "var(--color-text-primary)",
            background: "transparent",
            border: "none",
            outline: "none",
          }}
          disabled={loading}
        />
        {saving && <Loader2 size={14} className="animate-spin" style={{ color: "var(--color-text-tertiary)" }} />}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// CaptureSettingsCard
// ═══════════════════════════════════════════════════════════════════════════

function CaptureSettingsCard() {
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
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {captureConfig?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {captureConfig?.enabled
              ? "Prompts appear after customer meetings end"
              : "Post-meeting prompts are turned off"}
          </p>
        </div>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: !captureConfig ? 0.5 : 1 }}
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
                  ...(captureConfig.delayMinutes === mins ? styles.btnPrimary : styles.btnGhost),
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

// ═══════════════════════════════════════════════════════════════════════════
// QuillSettingsCard
// ═══════════════════════════════════════════════════════════════════════════

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

function QuillSettingsCard() {
  const [status, setStatus] = useState<QuillStatusData | null>(null);
  const [testing, setTesting] = useState(false);
  const [backfilling, setBackfilling] = useState(false);

  useEffect(() => {
    invoke<QuillStatusData>("get_quill_status")
      .then(setStatus)
      .catch(() => {});
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_quill_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Quill:", err);
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      const ok = await invoke<boolean>("test_quill_connection");
      toast(ok ? "Quill connection successful" : "Quill bridge not available");
    } catch (err) {
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
      <p style={styles.subsectionLabel}>Quill Transcripts</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Automatically sync meeting transcripts from Quill
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Transcripts will sync after meetings end"
              : "Quill transcript sync is turned off"}
          </p>
        </div>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: !status ? 0.5 : 1 }}
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {status?.enabled && (
        <>
          <div style={styles.settingRow}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={styles.statusDot(statusColor)} />
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
                {statusLabel}
              </span>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost, opacity: testing ? 0.5 : 1 }}
              onClick={testConnection}
              disabled={testing}
            >
              {testing ? "Testing..." : "Test Connection"}
            </button>
          </div>

          <div style={{ ...styles.settingRow, borderBottom: "none" }}>
            <div>
              <span style={styles.monoLabel}>Bridge path</span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2, fontFamily: "var(--font-mono)" }}>
                {status.bridgePath}
              </p>
            </div>
          </div>

          {(status.pendingSyncs > 0 || status.failedSyncs > 0 || status.completedSyncs > 0) && (
            <div style={{ display: "flex", gap: 16, paddingTop: 8 }}>
              {status.completedSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-garden-olive)" }}>
                  {status.completedSyncs} synced
                </span>
              )}
              {status.pendingSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-golden-turmeric)" }}>
                  {status.pendingSyncs} pending
                </span>
              )}
              {status.failedSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-spice-terracotta)" }}>
                  {status.failedSyncs} failed
                </span>
              )}
              {status.abandonedSyncs > 0 && (
                <span style={{ ...styles.monoLabel, color: "var(--color-text-tertiary)" }}>
                  {status.abandonedSyncs} abandoned
                </span>
              )}
            </div>
          )}

          {status.lastError && (
            <div style={{ paddingTop: 8 }}>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-spice-terracotta)" }}>
                {status.lastError}
              </span>
              {status.lastErrorAt && (
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", marginLeft: 8 }}>
                  {new Date(status.lastErrorAt).toLocaleString()}
                </span>
              )}
            </div>
          )}

          <div style={styles.settingRow}>
            <div>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                Poll interval
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
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
                }
              }}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 13,
                padding: "4px 8px",
                border: "1px solid var(--color-border)",
                borderRadius: 4,
                background: "var(--color-surface)",
                color: "var(--color-text-primary)",
              }}
            >
              {[1, 2, 5, 10, 15, 30].map((m) => (
                <option key={m} value={m}>
                  {m} min
                </option>
              ))}
            </select>
          </div>

          <div style={styles.settingRow}>
            <div>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
                Historical backfill
              </span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Create sync rows for past meetings (last 90 days)
              </p>
            </div>
            <button
              style={{ ...styles.btn, ...styles.btnGhost, opacity: backfilling ? 0.5 : 1 }}
              onClick={async () => {
                setBackfilling(true);
                try {
                  const result = await invoke<{ created: number; eligible: number }>("start_quill_backfill");
                  toast(`Backfill: ${result.created} of ${result.eligible} eligible meetings queued`);
                  const refreshed = await invoke<QuillStatusData>("get_quill_status");
                  setStatus(refreshed);
                } catch (err) {
                  toast.error("Backfill failed");
                } finally {
                  setBackfilling(false);
                }
              }}
              disabled={backfilling}
            >
              {backfilling ? "Running..." : "Start Backfill"}
            </button>
          </div>
        </>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// FeaturesCard
// ═══════════════════════════════════════════════════════════════════════════

function FeaturesCard() {
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

// ═══════════════════════════════════════════════════════════════════════════
// AiModelsCard
// ═══════════════════════════════════════════════════════════════════════════

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

function AiModelsCard() {
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

// ═══════════════════════════════════════════════════════════════════════════
// ScheduleRow
// ═══════════════════════════════════════════════════════════════════════════

function cronToHumanTime(cron: string): string {
  const parts = cron.split(" ");
  if (parts.length < 2) return cron;
  const minute = parseInt(parts[0], 10);
  const hour = parseInt(parts[1], 10);
  if (isNaN(minute) || isNaN(hour)) return cron;
  const h = hour % 12 || 12;
  const ampm = hour < 12 ? "AM" : "PM";
  const m = minute.toString().padStart(2, "0");
  return `${h}:${m} ${ampm}`;
}

function ScheduleRow({
  label,
  schedule,
  running,
  onRun,
}: {
  label: string;
  schedule: ScheduleEntry;
  running: boolean;
  onRun: () => void;
}) {
  return (
    <div style={styles.settingRow}>
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
            {label}
          </span>
          <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
            <div
              style={styles.statusDot(
                schedule.enabled ? "var(--color-garden-sage)" : "var(--color-text-tertiary)"
              )}
            />
            <span style={styles.monoLabel}>
              {schedule.enabled ? "Enabled" : "Disabled"}
            </span>
          </div>
        </div>
        <p
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            marginTop: 4,
          }}
        >
          {cronToHumanTime(schedule.cron)}{" "}
          <span style={{ opacity: 0.6 }}>({schedule.timezone})</span>
        </p>
      </div>
      <button
        style={{
          background: "none",
          border: "none",
          cursor: running ? "default" : "pointer",
          color: "var(--color-text-tertiary)",
          padding: 4,
          opacity: running ? 0.5 : 1,
        }}
        onClick={onRun}
        disabled={running}
      >
        {running ? (
          <RefreshCw size={16} className="animate-spin" />
        ) : (
          <Play size={16} />
        )}
      </button>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// EntityModeCard
// ═══════════════════════════════════════════════════════════════════════════

const entityModeOptions: { id: EntityMode; title: string; description: string; icon: typeof Building2 }[] = [
  {
    id: "account",
    title: "Account-based",
    description: "External relationships -- customers, clients, partners",
    icon: Building2,
  },
  {
    id: "project",
    title: "Project-based",
    description: "Internal efforts -- features, campaigns, initiatives",
    icon: FolderKanban,
  },
  {
    id: "both",
    title: "Both",
    description: "Relationships and initiatives",
    icon: Layers,
  },
];

function EntityModeCard({
  currentMode,
  onModeChange,
}: {
  currentMode: EntityMode;
  onModeChange: (mode: EntityMode) => void;
}) {
  const [saving, setSaving] = useState(false);

  async function handleSelect(mode: EntityMode) {
    if (mode === currentMode || saving) return;
    setSaving(true);
    try {
      await invoke("set_entity_mode", { mode });
      onModeChange(mode);
      toast.success("Entity mode updated -- reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update entity mode");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Work Mode</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        How you organize your work -- shapes workspace structure and sidebar
      </p>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {entityModeOptions.map((option) => {
          const Icon = option.icon;
          const isSelected = currentMode === option.id;
          return (
            <button
              key={option.id}
              type="button"
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                padding: "12px 16px",
                textAlign: "left" as const,
                background: "none",
                border: isSelected
                  ? "1px solid var(--color-desk-charcoal)"
                  : "1px solid var(--color-rule-light)",
                borderRadius: 4,
                cursor: saving && !isSelected ? "default" : "pointer",
                opacity: saving && !isSelected ? 0.5 : 1,
                transition: "all 0.15s ease",
              }}
              onClick={() => handleSelect(option.id)}
              disabled={saving}
            >
              <Icon size={18} style={{ color: "var(--color-text-tertiary)", flexShrink: 0 }} />
              <div style={{ flex: 1 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {option.title}
                </span>
                <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                  {option.description}
                </p>
              </div>
              {isSelected && <Check size={16} style={{ color: "var(--color-garden-sage)", flexShrink: 0 }} />}
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// WorkspaceCard
// ═══════════════════════════════════════════════════════════════════════════

function WorkspaceCard({
  workspacePath,
  onPathChange,
}: {
  workspacePath: string;
  onPathChange: (path: string) => void;
}) {
  const [saving, setSaving] = useState(false);

  async function handleChooseWorkspace() {
    const selected = await open({
      directory: true,
      title: "Choose workspace directory",
    });
    if (!selected) return;

    setSaving(true);
    try {
      await invoke("set_workspace_path", { path: selected });
      onPathChange(selected);
      toast.success("Workspace updated -- reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set workspace");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Workspace</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        The directory where DailyOS stores briefings, actions, and files
      </p>
      <div style={styles.settingRow}>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            color: "var(--color-text-primary)",
          }}
        >
          {workspacePath || "Not configured"}
        </span>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: saving ? 0.5 : 1 }}
          onClick={handleChooseWorkspace}
          disabled={saving}
        >
          {saving ? (
            <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
              <Loader2 size={12} className="animate-spin" /> ...
            </span>
          ) : (
            "Change"
          )}
        </button>
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// Intelligence Hygiene Card (I213)
// ═══════════════════════════════════════════════════════════════════════════

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

interface HygieneConfig {
  hygieneScanIntervalHours: number;
  hygieneAiBudget: number;
  hygienePreMeetingHours: number;
}

const scanIntervalOptions = [1, 2, 4, 8] as const;
const aiBudgetOptions = [5, 10, 20, 50] as const;
const preMeetingOptions = [2, 4, 12, 24] as const;

function IntelligenceHygieneCard() {
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
        [field === "scanIntervalHours" ? "scanIntervalHours" : field === "aiBudget" ? "aiBudget" : "preMeetingHours"]: value,
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
      <p style={styles.description}>
        No scan completed yet -- runs automatically after startup.
      </p>
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

      {/* Fixes — what the system healed */}
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

      {/* Gaps — remaining issues (clickable) */}
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
                    color: isClickable ? "var(--color-text-primary)" : "var(--color-text-secondary)",
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
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
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
                    ...(hygieneConfig.hygieneScanIntervalHours === v ? styles.btnPrimary : styles.btnGhost),
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
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
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
                    ...(hygieneConfig.hygieneAiBudget === v ? styles.btnPrimary : styles.btnGhost),
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
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
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
                    ...(hygieneConfig.hygienePreMeetingHours === v ? styles.btnPrimary : styles.btnGhost),
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
          {(runningNow || status.isRunning) ? (
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

// ═══════════════════════════════════════════════════════════════════════════
// MeetingBackfillCard
// ═══════════════════════════════════════════════════════════════════════════

function MeetingBackfillCard() {
  const [isRunning, setIsRunning] = useState(false);
  const [result, setResult] = useState<{ created: number; skipped: number; errors: string[] } | null>(null);

  async function runBackfill() {
    setIsRunning(true);
    setResult(null);

    try {
      const [created, skipped, errors] = await invoke<[number, number, string[]]>("backfill_historical_meetings");
      setResult({ created, skipped, errors });

      if (errors.length === 0) {
        toast.success(`Backfilled ${created} meetings (${skipped} already existed)`);
      } else {
        toast.warning(`Backfilled ${created} meetings with ${errors.length} errors`);
      }
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to run backfill");
      setResult({ created: 0, skipped: 0, errors: [String(err)] });
    } finally {
      setIsRunning(false);
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>Historical Meeting Backfill</p>
      <p style={{ ...styles.description, marginBottom: 12 }}>
        Import historical meeting files from your workspace into the database.
        Scans account and project directories for meeting files (transcripts, notes, summaries)
        and creates database records + entity links for any meetings not already in the system.
      </p>
      <p style={{ ...styles.description, fontSize: 12, marginBottom: 16 }}>
        Looks for files in: <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>02-Meetings/</code>,{" "}
        <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>03-Call-Transcripts/</code>,{" "}
        <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>Call-Transcripts/</code>,{" "}
        <code style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>Meeting-Notes/</code>
      </p>

      {result && (
        <div style={{ padding: "12px 0", borderBottom: "1px solid var(--color-rule-light)", marginBottom: 16 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div
              style={styles.statusDot(
                result.errors.length === 0 ? "var(--color-garden-sage)" : "var(--color-spice-turmeric)"
              )}
            />
            <span
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                fontWeight: 500,
                color: "var(--color-text-primary)",
              }}
            >
              Created {result.created} meetings, skipped {result.skipped}
            </span>
          </div>

          {result.errors.length > 0 && (
            <div style={{ marginTop: 8 }}>
              <p
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  fontWeight: 600,
                  color: "var(--color-spice-terracotta)",
                  marginBottom: 4,
                }}
              >
                Errors:
              </p>
              <div style={{ maxHeight: 128, overflowY: "auto" }}>
                {result.errors.map((err, i) => (
                  <p
                    key={i}
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      margin: 0,
                      marginBottom: 2,
                    }}
                  >
                    {err}
                  </p>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      <button
        style={{
          ...styles.btn,
          ...styles.btnPrimary,
          opacity: isRunning ? 0.5 : 1,
          width: "100%",
          textAlign: "center" as const,
        }}
        onClick={runBackfill}
        disabled={isRunning}
      >
        {isRunning ? (
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <Loader2 size={12} className="animate-spin" /> Scanning directories...
          </span>
        ) : (
          "Run Backfill"
        )}
      </button>
    </div>
  );
}
