import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  Loader2,
  Play,
  RefreshCw,
  Check,
  Building2,
  FolderKanban,
  Layers,
  HardDrive,
} from "lucide-react";
import type { EntityMode, FeatureFlags } from "@/types";
import { styles } from "@/components/settings/styles";

// ═══════════════════════════════════════════════════════════════════════════
// Types
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

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

function cronToHumanTime(cron: string): string {
  const parts = cron.split(" ");
  if (parts.length < 2) return cron;
  const minute = parts[0];
  const hour = parts[1];
  const dayOfWeek = parts[4] ?? "*";

  // "1 */4 * * *" → "Every 4 hours (daily)"
  if (hour.startsWith("*/")) {
    const interval = parseInt(hour.slice(2), 10);
    const dayLabel = dayOfWeek === "1-5" ? "weekdays" : "daily";
    return `Every ${interval} hours (${dayLabel})`;
  }

  // "0 */2 * * *" → "Every 2 hours (daily)"
  if (hour.startsWith("*/")) {
    const interval = parseInt(hour.slice(2), 10);
    return `Every ${interval} hours`;
  }

  // Fixed time: "0 8 * * 1-5" → "8:00 AM (weekdays)"
  const h = parseInt(hour, 10);
  const m = parseInt(minute, 10);
  if (isNaN(h) || isNaN(m)) return cron;
  const hDisplay = h % 12 || 12;
  const ampm = h < 12 ? "AM" : "PM";
  const mDisplay = m.toString().padStart(2, "0");
  const dayLabel = dayOfWeek === "1-5" ? " (weekdays)" : dayOfWeek === "1" ? " (Mondays)" : "";
  return `${hDisplay}:${mDisplay} ${ampm}${dayLabel}`;
}

// ═══════════════════════════════════════════════════════════════════════════
// Entity mode options
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

// ═══════════════════════════════════════════════════════════════════════════
// DeveloperToggle
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
            {config?.developerMode
              ? "Active — using isolated database and workspace"
              : "Switches to an isolated sandbox (separate database, workspace, and auth)"}
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
// EntityModeSelector
// ═══════════════════════════════════════════════════════════════════════════

function EntityModeSelector({
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
      toast.success("Work mode updated -- reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update work mode");
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
// ScheduleRow
// ═══════════════════════════════════════════════════════════════════════════

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
// SchedulesSection
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
// ManualRunSection
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

// ═══════════════════════════════════════════════════════════════════════════
// DatabaseStorageCard (I614)
// ═══════════════════════════════════════════════════════════════════════════

interface DbGrowthReport {
  fileSizeBytes: number;
  fileSizeDisplay: string;
  tableCounts: { tableName: string; rowCount: number }[];
  reportedAt: string;
}

interface AiUsageBreakdownCount {
  label: string;
  count: number;
}

interface AiUsageTrendPoint {
  date: string;
  callCount: number;
  estimatedPromptTokens: number;
  estimatedOutputTokens: number;
  estimatedTotalTokens: number;
  totalDurationMs: number;
}

interface AiUsageDiagnostics {
  today: AiUsageTrendPoint;
  operationCounts: AiUsageBreakdownCount[];
  modelCounts: AiUsageBreakdownCount[];
  budgetLimit: number;
  budgetRemaining: number;
  estimatedDailyTokenBudget: number;
  estimatedTokenBudgetRemaining: number;
  backgroundPause: {
    paused: boolean;
    pausedUntil?: string | null;
    reason?: string | null;
    rolling4hTokens: number;
    backgroundCalls4h: number;
    timeoutRateLast20: number;
    consecutiveBackgroundTimeouts: number;
  };
  trend: AiUsageTrendPoint[];
}

const TABLE_LABELS: Record<string, string> = {
  signal_events: "Signals",
  email_signals: "Email signals",
  emails: "Emails",
  entity_assessment: "Assessments",
  captured_commitments: "Commitments",
  content_embeddings: "Embeddings",
  person_relationships: "Relationships",
  meetings: "Meetings",
};

function DatabaseStorageCard() {
  const [report, setReport] = useState<DbGrowthReport | null>(null);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    invoke<DbGrowthReport>("get_db_growth_report")
      .then(setReport)
      .catch((err) => console.warn("get_db_growth_report failed:", err));
  }, []);

  if (!report) return null;

  const isWarning = report.fileSizeBytes >= 300_000_000;
  const isDanger = report.fileSizeBytes >= 500_000_000;

  return (
    <div style={{ marginTop: 8 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
        <HardDrive
          size={14}
          style={{
            color: isDanger
              ? "var(--color-terracotta)"
              : isWarning
                ? "var(--color-turmeric)"
                : "var(--color-text-tertiary)",
          }}
        />
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 500,
            textTransform: "uppercase" as const,
            letterSpacing: "0.06em",
            color: "var(--color-text-tertiary)",
          }}
        >
          Database Storage
        </span>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            fontWeight: 600,
            color: isDanger
              ? "var(--color-terracotta)"
              : isWarning
                ? "var(--color-turmeric)"
                : "var(--color-text-primary)",
            marginLeft: "auto",
          }}
        >
          {report.fileSizeDisplay}
        </span>
      </div>

      {isDanger && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-terracotta)",
            marginBottom: 8,
          }}
        >
          Database exceeds 500 MB. Old data is automatically purged daily.
        </p>
      )}

      <button
        onClick={() => setExpanded(!expanded)}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--color-text-tertiary)",
          background: "none",
          border: "none",
          cursor: "pointer",
          padding: 0,
          display: "flex",
          alignItems: "center",
          gap: 4,
        }}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{
            width: 12,
            height: 12,
            transform: expanded ? "rotate(180deg)" : "none",
            transition: "transform 0.2s ease",
          }}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
        Table details
      </button>

      {expanded && (
        <div style={{ marginTop: 8 }}>
          {report.tableCounts.map((tc) => (
            <div
              key={tc.tableName}
              style={{
                display: "flex",
                justifyContent: "space-between",
                padding: "3px 0",
                fontFamily: "var(--font-mono)",
                fontSize: 12,
              }}
            >
              <span style={{ color: "var(--color-text-secondary)" }}>
                {TABLE_LABELS[tc.tableName] ?? tc.tableName}
              </span>
              <span style={{ color: "var(--color-text-tertiary)" }}>
                {tc.rowCount.toLocaleString()}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function AiUsageCard() {
  const [usage, setUsage] = useState<AiUsageDiagnostics | null>(null);

  useEffect(() => {
    invoke<AiUsageDiagnostics>("get_ai_usage_diagnostics")
      .then(setUsage)
      .catch((err) => console.warn("get_ai_usage_diagnostics failed:", err));
  }, []);

  if (!usage) return null;

  return (
    <div style={{ marginTop: 8 }}>
      <p style={styles.subsectionLabel}>AI Usage</p>
      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(140px, 1fr))", gap: 12 }}>
        <div style={{ border: "1px solid var(--color-rule-light)", padding: "12px 14px" }}>
          <div style={styles.monoLabel}>Today</div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 28, color: "var(--color-text-primary)", marginTop: 4 }}>
            {usage.today.estimatedTotalTokens.toLocaleString()}
          </div>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 4 }}>
            estimated tokens across {usage.today.callCount} calls
          </p>
        </div>
        <div style={{ border: "1px solid var(--color-rule-light)", padding: "12px 14px" }}>
          <div style={styles.monoLabel}>Budget</div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 28, color: "var(--color-text-primary)", marginTop: 4 }}>
            {usage.estimatedTokenBudgetRemaining.toLocaleString()}
          </div>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 4 }}>
            est. tokens remaining today
          </p>
        </div>
        <div style={{ border: "1px solid var(--color-rule-light)", padding: "12px 14px" }}>
          <div style={styles.monoLabel}>Hygiene Budget</div>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 28, color: "var(--color-text-primary)", marginTop: 4 }}>
            {usage.budgetRemaining}/{usage.budgetLimit}
          </div>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 4 }}>
            background AI calls left today
          </p>
        </div>
      </div>

      <div style={{ marginTop: 16 }}>
        <div style={{ ...styles.monoLabel, marginBottom: 8 }}>Background Guard</div>
        <div style={{ border: "1px solid var(--color-rule-light)", padding: "12px 14px" }}>
          <div style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-primary)" }}>
            {usage.backgroundPause.paused ? "Paused" : "Running"}
          </div>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 6 }}>
            {usage.backgroundPause.paused
              ? usage.backgroundPause.reason ?? "Background AI is temporarily paused"
              : `${usage.backgroundPause.rolling4hTokens.toLocaleString()} tokens in the last 4 hours`}
          </p>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 6 }}>
            Timeout rate: {(usage.backgroundPause.timeoutRateLast20 * 100).toFixed(0)}% across recent background calls
          </p>
        </div>
      </div>

      <div style={{ marginTop: 16 }}>
        <div style={{ ...styles.monoLabel, marginBottom: 8 }}>Top Operations</div>
        {usage.operationCounts.length > 0 ? (
          usage.operationCounts.map((entry) => (
            <div
              key={entry.label}
              style={{
                display: "flex",
                justifyContent: "space-between",
                padding: "6px 0",
                borderBottom: "1px solid var(--color-rule-light)",
                fontFamily: "var(--font-mono)",
                fontSize: 12,
              }}
            >
              <span style={{ color: "var(--color-text-secondary)" }}>{entry.label}</span>
              <span style={{ color: "var(--color-text-tertiary)" }}>{entry.count}</span>
            </div>
          ))
        ) : (
          <p style={{ ...styles.description, fontSize: 12 }}>No AI calls recorded today.</p>
        )}
      </div>

      <div style={{ marginTop: 16 }}>
        <div style={{ ...styles.monoLabel, marginBottom: 8 }}>Models</div>
        {usage.modelCounts.length > 0 ? (
          usage.modelCounts.map((entry) => (
            <div
              key={entry.label}
              style={{
                display: "flex",
                justifyContent: "space-between",
                padding: "6px 0",
                borderBottom: "1px solid var(--color-rule-light)",
                fontFamily: "var(--font-mono)",
                fontSize: 12,
              }}
            >
              <span style={{ color: "var(--color-text-secondary)" }}>{entry.label}</span>
              <span style={{ color: "var(--color-text-tertiary)" }}>{entry.count}</span>
            </div>
          ))
        ) : (
          <p style={{ ...styles.description, fontSize: 12 }}>No model usage recorded today.</p>
        )}
      </div>

      <div style={{ marginTop: 16 }}>
        <div style={{ ...styles.monoLabel, marginBottom: 8 }}>7-Day Trend</div>
        {usage.trend.length > 0 ? (
          usage.trend.map((point) => (
            <div
              key={point.date}
              style={{
                display: "grid",
                gridTemplateColumns: "90px 1fr auto",
                gap: 12,
                alignItems: "center",
                padding: "6px 0",
                borderBottom: "1px solid var(--color-rule-light)",
              }}
            >
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                {point.date}
              </span>
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
                {point.callCount} calls · {point.estimatedTotalTokens.toLocaleString()} tokens · {(point.totalDurationMs / 1000).toFixed(1)}s
              </span>
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                in {point.estimatedPromptTokens.toLocaleString()} / out {point.estimatedOutputTokens.toLocaleString()}
              </span>
            </div>
          ))
        ) : (
          <p style={{ ...styles.description, fontSize: 12 }}>No usage recorded in the last 7 days.</p>
        )}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// ArchivedAccountsSection
// ═══════════════════════════════════════════════════════════════════════════

function ArchivedAccountsSection() {
  const [showArchived, setShowArchived] = useState(false);
  const [archivedAccounts, setArchivedAccounts] = useState<{ id: string; name: string; parentName?: string }[]>([]);
  const [restoringId, setRestoringId] = useState<string | null>(null);

  async function loadArchivedAccounts() {
    try {
      const accounts = await invoke<{ id: string; name: string; parent_id?: string }[]>("get_archived_accounts");
      setArchivedAccounts(accounts.map(a => ({ id: a.id, name: a.name })));
    } catch {
      setArchivedAccounts([]);
    }
  }

  async function handleRestoreAccount(accountId: string) {
    setRestoringId(accountId);
    try {
      await invoke("restore_account", { accountId, restoreChildren: true });
      await loadArchivedAccounts();
    } catch (e) {
      console.error("Failed to restore account:", e);
      toast.error("Failed to restore account");
    } finally {
      setRestoringId(null);
    }
  }

  return (
    <div style={{ marginTop: 8 }}>
      <button
        onClick={() => {
          setShowArchived(!showArchived);
          if (!showArchived) loadArchivedAccounts();
        }}
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          fontWeight: 500,
          textTransform: "uppercase",
          letterSpacing: "0.06em",
          color: "var(--color-text-tertiary)",
          background: "none",
          border: "none",
          cursor: "pointer",
          padding: 0,
          display: "flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          style={{
            width: 14,
            height: 14,
            transform: showArchived ? "rotate(180deg)" : "none",
            transition: "transform 0.2s ease",
          }}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
        Archived Accounts
      </button>

      {showArchived && (
        <div style={{ marginTop: 16 }}>
          {archivedAccounts.length === 0 ? (
            <p style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
              fontStyle: "italic",
            }}>
              No archived accounts.
            </p>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
              {archivedAccounts.map((account, idx) => (
                <div
                  key={account.id}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    padding: "10px 0",
                    borderBottom: idx < archivedAccounts.length - 1 ? "1px solid var(--color-rule-light)" : "none",
                  }}
                >
                  <div>
                    <span style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 14,
                      color: "var(--color-text-primary)",
                    }}>
                      {account.name}
                    </span>
                    {account.parentName && (
                      <span style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        color: "var(--color-text-tertiary)",
                        marginLeft: 8,
                      }}>
                        ({account.parentName})
                      </span>
                    )}
                  </div>
                  <button
                    onClick={() => handleRestoreAccount(account.id)}
                    disabled={restoringId === account.id}
                    style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      fontWeight: 500,
                      textTransform: "uppercase",
                      letterSpacing: "0.06em",
                      color: "var(--color-garden-sage)",
                      background: "none",
                      border: "1px solid var(--color-garden-sage)",
                      borderRadius: 4,
                      padding: "4px 10px",
                      cursor: restoringId === account.id ? "default" : "pointer",
                      opacity: restoringId === account.id ? 0.5 : 1,
                    }}
                  >
                    {restoringId === account.id ? "Restoring..." : "Restore"}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// DiagnosticsSection (default export)
// ═══════════════════════════════════════════════════════════════════════════

export default function DiagnosticsSection() {
  const [config, setConfig] = useState<Config | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  const [rolePresetsEnabled, setRolePresetsEnabled] = useState(false);

  useEffect(() => {
    invoke<Config>("get_config")
      .then(setConfig)
      .catch((err) => console.error("get_config (diagnostics) failed:", err));
    invoke<FeatureFlags>("get_feature_flags")
      .then((flags) => setRolePresetsEnabled(flags.role_presets_enabled))
      .catch(() => setRolePresetsEnabled(false));
  }, []);

  async function handleRunWorkflow(workflow: string) {
    setRunning(workflow);
    try {
      await invoke<string>("run_workflow", { workflow });
      toast.success(`${workflow} workflow completed`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Workflow failed");
    } finally {
      setRunning(null);
    }
  }

  return (
    <div>
      <DeveloperToggle config={config} setConfig={setConfig} />
      {rolePresetsEnabled && (
        <>
          <hr style={styles.thinRule} />
          <EntityModeSelector
            currentMode={config?.entityMode ?? "account"}
            onModeChange={(mode) => setConfig(config ? { ...config, entityMode: mode } : null)}
          />
        </>
      )}
      <hr style={styles.thinRule} />
      <SchedulesSection config={config} running={running} onRun={handleRunWorkflow} />
      <hr style={styles.thinRule} />
      <ManualRunSection running={running} onRun={handleRunWorkflow} />
      <hr style={styles.thinRule} />
      <MeetingBackfillCard />
      <hr style={styles.thinRule} />
      <AiUsageCard />
      <hr style={styles.thinRule} />
      <DatabaseStorageCard />
      <hr style={styles.thinRule} />
      <ArchivedAccountsSection />
    </div>
  );
}
