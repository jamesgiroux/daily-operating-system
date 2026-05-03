import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import clsx from "clsx";
import {
  Loader2,
  Play,
  RefreshCw,
  HardDrive,
} from "lucide-react";
import type { EntityMode } from "@/types";
import { styles } from "@/components/settings/styles";
import ds from "./DiagnosticsSection.module.css";

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
          <span className={ds.devToolLabel}>
            Developer Tools
          </span>
          <p className={ds.devToolDescription}>
            {config?.developerMode
              ? "Active — using isolated database and workspace"
              : "Switches to an isolated sandbox (separate database, workspace, and auth)"}
          </p>
        </div>
        <button
          className={config?.developerMode ? ds.btnPrimary : ds.btnGhost}
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
        <div className={ds.scheduleHeader}>
          <span className={ds.scheduleLabel}>
            {label}
          </span>
          <div className={ds.scheduleStatusGroup}>
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
        <p className={ds.scheduleCron}>
          {cronToHumanTime(schedule.cron)}{" "}
          <span className={ds.scheduleTimezone}>({schedule.timezone})</span>
        </p>
      </div>
      <button
        className={ds.scheduleRunBtn}
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
      <p className={ds.manualRunDescription}>
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
      <p className={ds.manualRunDescription}>
        Trigger workflows manually without waiting for schedule
      </p>
      <div className={ds.manualRunRow}>
        <button
          className={clsx(ds.btnPrimary, running !== null && ds.btnDisabledOpacity)}
          onClick={() => onRun("today")}
          disabled={running !== null}
        >
          {running === "today" ? (
            <span className={ds.manualRunSpinner}>
              <Loader2 size={12} className="animate-spin" /> Running...
            </span>
          ) : (
            "Run Daily Briefing"
          )}
        </button>
        <button
          className={clsx(ds.btnGhost, running !== null && ds.btnDisabledOpacity)}
          onClick={() => onRun("week")}
          disabled={running !== null}
        >
          {running === "week" ? (
            <span className={ds.manualRunSpinner}>
              <Loader2 size={12} className="animate-spin" /> Running...
            </span>
          ) : (
            "Run Weekly Briefing"
          )}
        </button>
        <button
          className={clsx(ds.btnGhost, running !== null && ds.btnDisabledOpacity)}
          onClick={() => onRun("archive")}
          disabled={running !== null}
        >
          {running === "archive" ? (
            <span className={ds.manualRunSpinner}>
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
      <p className={ds.backfillDescription}>
        Import historical meeting files from your workspace into the database.
        Scans account and project directories for meeting files (transcripts, notes, summaries)
        and creates database records + entity links for any meetings not already in the system.
      </p>
      <p className={ds.backfillPathDescription}>
        Looks for files in: <code className={ds.backfillCode}>02-Meetings/</code>,{" "}
        <code className={ds.backfillCode}>03-Call-Transcripts/</code>,{" "}
        <code className={ds.backfillCode}>Call-Transcripts/</code>,{" "}
        <code className={ds.backfillCode}>Meeting-Notes/</code>
      </p>

      {result && (
        <div className={ds.backfillResultRow}>
          <div className={ds.backfillResultHeader}>
            <div
              style={styles.statusDot(
                result.errors.length === 0 ? "var(--color-garden-sage)" : "var(--color-spice-turmeric)"
              )}
            />
            <span className={ds.backfillResultLabel}>
              Created {result.created} meetings, skipped {result.skipped}
            </span>
          </div>

          {result.errors.length > 0 && (
            <div className={ds.backfillErrorSection}>
              <p className={ds.backfillErrorLabel}>
                Errors:
              </p>
              <div className={ds.backfillErrorList}>
                {result.errors.map((err, i) => (
                  <p
                    key={i}
                    className={ds.backfillErrorItem}
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
        className={clsx(ds.btnPrimary, ds.backfillRunBtn, isRunning && ds.btnDisabledOpacity)}
        onClick={runBackfill}
        disabled={isRunning}
      >
        {isRunning ? (
          <span className={ds.manualRunSpinner}>
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
// DatabaseStorageCard
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
  /** Configured daily token budget (50k / 100k / 250k). */
  dailyTokenBudget: number;
  /** Tokens consumed today (local day). */
  tokensUsedToday: number;
  /** Tokens remaining before next local-day reset. */
  tokensRemaining: number;
  /** True when budget is exhausted and new AI calls are blocked. */
  budgetExhausted: boolean;
  /** Local YYYY-MM-DD key — resets at local midnight. */
  budgetResetDate: string;
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

// ═══════════════════════════════════════════════════════════════════════════
// Feedback & Learning Card
// ═══════════════════════════════════════════════════════════════════════════

interface FeedbackDiagnostics {
  eventCount: number;
  suppressionCount: number;
  lastFeedback: string | null;
}

function FeedbackLearningCard() {
  const [diag, setDiag] = useState<FeedbackDiagnostics | null>(null);

  useEffect(() => {
    invoke<FeedbackDiagnostics>("get_feedback_diagnostics")
      .then(setDiag)
      .catch((err) => console.warn("get_feedback_diagnostics failed:", err));
  }, []);

  if (!diag) return null;

  const lastDate = diag.lastFeedback
    ? new Date(diag.lastFeedback + "Z").toLocaleDateString(undefined, {
        month: "short",
        day: "numeric",
        hour: "numeric",
        minute: "2-digit",
      })
    : null;

  return (
    <div className={ds.feedbackCard}>
      <p style={styles.subsectionLabel}>Feedback &amp; Learning</p>
      <div className={ds.feedbackStatGrid}>
        <div className={ds.feedbackStatItem}>
          <div style={styles.monoLabel}>Feedback Events</div>
          <div className={ds.feedbackStatNumber}>{diag.eventCount}</div>
          <p className={ds.feedbackStatDescription}>corrections recorded</p>
        </div>
        <div className={ds.feedbackStatItem}>
          <div style={styles.monoLabel}>Suppressions</div>
          <div className={ds.feedbackStatNumber}>{diag.suppressionCount}</div>
          <p className={ds.feedbackStatDescription}>active item suppressions</p>
        </div>
        {lastDate && (
          <div className={ds.feedbackStatItem}>
            <div style={styles.monoLabel}>Last Feedback</div>
            <p className={ds.feedbackStatDescription} style={{ marginTop: 8 }}>
              {lastDate}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

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
    <div className={ds.dbCard}>
      <div className={ds.dbHeader}>
        <HardDrive
          size={14}
          className={isDanger ? ds.dbIconDanger : isWarning ? ds.dbIconWarning : ds.dbIconDefault}
        />
        <span className={ds.dbLabel}>
          Database Storage
        </span>
        <span className={isDanger ? ds.dbSizeDanger : isWarning ? ds.dbSizeWarning : ds.dbSizeDefault}>
          {report.fileSizeDisplay}
        </span>
      </div>

      {isDanger && (
        <p className={ds.dbDangerMessage}>
          Database exceeds 500 MB. Old data is automatically purged daily.
        </p>
      )}

      <button
        onClick={() => setExpanded(!expanded)}
        className={ds.dbExpandBtn}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={expanded ? ds.dbChevronExpanded : ds.dbChevron}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
        Table details
      </button>

      {expanded && (
        <div className={ds.dbTableDetails}>
          {report.tableCounts.map((tc) => (
            <div
              key={tc.tableName}
              className={ds.dbTableRow}
            >
              <span className={ds.dbTableName}>
                {TABLE_LABELS[tc.tableName] ?? tc.tableName}
              </span>
              <span className={ds.dbTableCount}>
                {tc.rowCount.toLocaleString()}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function formatBudgetTier(budget: number): string {
  if (budget >= 1_000_000) return `${(budget / 1_000_000).toFixed(1)}M`;
  if (budget >= 1_000) return `${Math.round(budget / 1_000)}k`;
  return budget.toLocaleString();
}

function AiUsageCard() {
  const [usage, setUsage] = useState<AiUsageDiagnostics | null>(null);

  useEffect(() => {
    invoke<AiUsageDiagnostics>("get_ai_usage_diagnostics")
      .then(setUsage)
      .catch((err) => console.warn("get_ai_usage_diagnostics failed:", err));
  }, []);

  if (!usage) return null;

  const usedPct = usage.dailyTokenBudget > 0
    ? Math.min(100, (usage.tokensUsedToday / usage.dailyTokenBudget) * 100)
    : 0;

  return (
    <div className={ds.aiCard}>
      <p style={styles.subsectionLabel}>AI Usage</p>
      <div className={ds.aiStatGrid}>
        <div className={ds.aiStatCard}>
          <div style={styles.monoLabel}>Used Today</div>
          <div className={ds.aiStatNumber}>
            {usage.tokensUsedToday.toLocaleString()}
          </div>
          <p className={ds.aiStatDescription}>
            est. tokens across {usage.today.callCount} calls
          </p>
        </div>
        <div className={ds.aiStatCard}>
          <div style={styles.monoLabel}>Remaining</div>
          <div className={ds.aiStatNumber} style={usage.budgetExhausted ? { color: "var(--color-spice-terracotta)" } : undefined}>
            {usage.budgetExhausted ? "Exhausted" : usage.tokensRemaining.toLocaleString()}
          </div>
          <p className={ds.aiStatDescription}>
            of {formatBudgetTier(usage.dailyTokenBudget)} daily budget
          </p>
        </div>
        <div className={ds.aiStatCard}>
          <div style={styles.monoLabel}>Status</div>
          <div className={ds.aiStatNumber} style={{ fontSize: 14, paddingTop: 4 }}>
            {usage.budgetExhausted ? "Blocked" : "Active"}
          </div>
          <p className={ds.aiStatDescription}>
            resets at local midnight
          </p>
        </div>
      </div>

      {/* Budget progress bar */}
      <div style={{ marginBottom: 16 }}>
        <div style={{
          height: 4,
          background: "var(--color-rule-light)",
          borderRadius: 2,
          overflow: "hidden",
        }}>
          <div style={{
            height: "100%",
            width: `${usedPct}%`,
            background: usage.budgetExhausted
              ? "var(--color-spice-terracotta)"
              : usedPct > 80
                ? "var(--color-spice-turmeric)"
                : "var(--color-garden-sage)",
            borderRadius: 2,
            transition: "width 0.3s ease",
          }} />
        </div>
        <p className={ds.aiStatDescription} style={{ marginTop: 4 }}>
          {usedPct.toFixed(0)}% of daily budget used
          {usage.budgetExhausted && " — new AI calls are blocked until midnight"}
        </p>
      </div>

      <div className={ds.aiSectionBlock}>
        <div className={ds.aiSectionLabel}>Background Guard</div>
        <div className={ds.aiGuardCard}>
          <div className={ds.aiGuardStatus}>
            {usage.budgetExhausted ? "Budget Exhausted" : usage.backgroundPause.paused ? "Paused" : "Running"}
          </div>
          <p className={ds.aiGuardDescription}>
            {usage.budgetExhausted
              ? "Daily AI budget exhausted. All AI calls blocked until local midnight."
              : usage.backgroundPause.paused
                ? usage.backgroundPause.reason ?? "Background AI is temporarily paused"
                : `${usage.backgroundPause.rolling4hTokens.toLocaleString()} tokens in the last 4 hours`}
          </p>
          {!usage.budgetExhausted && (
            <p className={ds.aiGuardDescription}>
              Timeout rate: {(usage.backgroundPause.timeoutRateLast20 * 100).toFixed(0)}% across recent background calls
            </p>
          )}
        </div>
      </div>

      <div className={ds.aiSectionBlock}>
        <div className={ds.aiSectionLabel}>Top Operations</div>
        {usage.operationCounts.length > 0 ? (
          usage.operationCounts.map((entry) => (
            <div
              key={entry.label}
              className={ds.aiCallSiteRow}
            >
              <span className={ds.aiCallSiteLabel}>{entry.label}</span>
              <span className={ds.aiCallSiteCount}>{entry.count}</span>
            </div>
          ))
        ) : (
          <p className={ds.aiNoData}>No AI calls recorded today.</p>
        )}
      </div>

      <div className={ds.aiSectionBlock}>
        <div className={ds.aiSectionLabel}>Models</div>
        {usage.modelCounts.length > 0 ? (
          usage.modelCounts.map((entry) => (
            <div
              key={entry.label}
              className={ds.aiCallSiteRow}
            >
              <span className={ds.aiCallSiteLabel}>{entry.label}</span>
              <span className={ds.aiCallSiteCount}>{entry.count}</span>
            </div>
          ))
        ) : (
          <p className={ds.aiNoData}>No model usage recorded today.</p>
        )}
      </div>

      <div className={ds.aiSectionBlock}>
        <div className={ds.aiSectionLabel}>7-Day Trend</div>
        {usage.trend.length > 0 ? (
          usage.trend.map((point) => (
            <div
              key={point.date}
              className={ds.aiTrendRow}
            >
              <span className={ds.aiTrendDate}>
                {point.date}
              </span>
              <span className={ds.aiTrendSummary}>
                {point.callCount} calls · {point.estimatedTotalTokens.toLocaleString()} tokens · {(point.totalDurationMs / 1000).toFixed(1)}s
              </span>
              <span className={ds.aiTrendTokens}>
                in {point.estimatedPromptTokens.toLocaleString()} / out {point.estimatedOutputTokens.toLocaleString()}
              </span>
            </div>
          ))
        ) : (
          <p className={ds.aiNoData}>No usage recorded in the last 7 days.</p>
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
    <div className={ds.archivedCard}>
      <button
        onClick={() => {
          setShowArchived(!showArchived);
          if (!showArchived) loadArchivedAccounts();
        }}
        className={ds.archivedToggleBtn}
      >
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={showArchived ? ds.archivedChevronExpanded : ds.archivedChevron}
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
        Archived Accounts
      </button>

      {showArchived && (
        <div className={ds.archivedList}>
          {archivedAccounts.length === 0 ? (
            <p className={ds.archivedEmpty}>
              No archived accounts.
            </p>
          ) : (
            <div className={ds.archivedAccountsList}>
              {archivedAccounts.map((account, idx) => (
                <div
                  key={account.id}
                  className={idx < archivedAccounts.length - 1 ? ds.archivedAccountRowBorder : ds.archivedAccountRow}
                >
                  <div>
                    <span className={ds.archivedAccountName}>
                      {account.name}
                    </span>
                    {account.parentName && (
                      <span className={ds.archivedAccountParent}>
                        ({account.parentName})
                      </span>
                    )}
                  </div>
                  <button
                    onClick={() => handleRestoreAccount(account.id)}
                    disabled={restoringId === account.id}
                    className={ds.archivedRestoreBtn}
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

  useEffect(() => {
    invoke<Config>("get_config")
      .then(setConfig)
      .catch((err) => console.error("get_config (diagnostics) failed:", err));
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
      <hr style={styles.thinRule} />
      <SchedulesSection config={config} running={running} onRun={handleRunWorkflow} />
      <hr style={styles.thinRule} />
      <ManualRunSection running={running} onRun={handleRunWorkflow} />
      <hr style={styles.thinRule} />
      <MeetingBackfillCard />
      <hr style={styles.thinRule} />
      <AiUsageCard />
      <hr style={styles.thinRule} />
      <FeedbackLearningCard />
      <hr style={styles.thinRule} />
      <DatabaseStorageCard />
      <hr style={styles.thinRule} />
      <ArchivedAccountsSection />
    </div>
  );
}
