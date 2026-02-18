import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Wrench, RotateCcw, Database, Shield, Inbox, Zap, Sun, Calendar, Sparkles, Brain, Undo2, Trash2, Eraser } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { toast } from "sonner";
import { Copy } from "lucide-react";

/** Toast with a copy button so messages can be pasted into CLI. */
function devToast(type: "success" | "error", message: string) {
  toast[type](message, {
    duration: type === "error" ? 8000 : 5000,
    action: {
      label: <Copy className="h-3 w-3" />,
      onClick: () => navigator.clipboard.writeText(message),
    },
  });
}

interface DevState {
  isDebugBuild: boolean;
  hasConfig: boolean;
  workspacePath: string | null;
  hasDatabase: boolean;
  actionCount: number;
  accountCount: number;
  projectCount: number;
  meetingCount: number;
  peopleCount: number;
  hasTodayData: boolean;
  googleAuthStatus: string;
  isDevDbMode: boolean;
  hasDevDbFile: boolean;
  hasDevWorkspace: boolean;
}

interface LatencyCommandRollup {
  command: string;
  sampleCount: number;
  p50Ms: number;
  p95Ms: number;
  maxMs: number;
  budgetMs: number;
  budgetViolations: number;
  degradedCount: number;
  lastRecordedAt?: string;
}

interface LatencyRollupsPayload {
  generatedAt: string;
  commands: LatencyCommandRollup[];
}

export function DevToolsPanel() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    // Gate on dev build + config.developerMode
    if (!import.meta.env.DEV) return;
    invoke<{ developerMode?: boolean }>("get_config")
      .then((cfg) => setEnabled(cfg.developerMode === true))
      .catch(() => {}); // No config yet — stay hidden
  }, []);

  if (!import.meta.env.DEV || !enabled) return null;

  return <DevToolsPanelInner />;
}

function DevToolsPanelInner() {
  const [open, setOpen] = useState(false);
  const [devState, setDevState] = useState<DevState | null>(null);
  const [rollups, setRollups] = useState<LatencyRollupsPayload | null>(null);
  const [loading, setLoading] = useState<string | null>(null);

  const refreshState = useCallback(async () => {
    try {
      const [state, latency] = await Promise.all([
        invoke<DevState>("dev_get_state"),
        invoke<LatencyRollupsPayload>("get_latency_rollups"),
      ]);
      setDevState(state);
      setRollups(latency);
    } catch {
      // Silently fail — devtools not critical
    }
  }, []);

  useEffect(() => {
    if (open) {
      refreshState();
    }
  }, [open, refreshState]);

  async function applyScenario(scenario: string) {
    setLoading(scenario);
    try {
      const result = await invoke<string>("dev_apply_scenario", { scenario });
      devToast("success", result);
      // Brief delay to let the toast show before reload
      setTimeout(() => window.location.reload(), 500);
    } catch (err) {
      devToast("error", typeof err === "string" ? err : "Scenario failed");
      setLoading(null);
    }
  }

  async function runCommand(key: string, command: string, reload = false) {
    setLoading(key);
    try {
      const result = await invoke<string>(command);
      devToast("success", result);
      if (reload) {
        setTimeout(() => window.location.reload(), 500);
      } else {
        await refreshState();
      }
    } catch (err) {
      devToast("error", typeof err === "string" ? err : "Command failed");
    } finally {
      setLoading(null);
    }
  }

  return (
    <>
      {/* Floating wrench button */}
      <button
        onClick={() => setOpen(true)}
        className="fixed bottom-4 right-4 z-50 flex h-10 w-10 items-center justify-center rounded-full bg-muted/80 text-muted-foreground shadow-md backdrop-blur-sm transition-colors hover:bg-muted hover:text-foreground"
        title="Dev Tools"
      >
        <Wrench className="h-4 w-4" />
      </button>

      <Sheet open={open} onOpenChange={setOpen}>
        <SheetContent side="right" className="w-[380px] overflow-y-auto">
          <SheetHeader>
            <div className="flex items-center gap-2">
              <SheetTitle>Dev Tools</SheetTitle>
              <Badge variant="destructive" className="text-[10px] px-1.5 py-0">
                DEBUG
              </Badge>
            </div>
            <SheetDescription>
              Switch between app scenarios for testing
            </SheetDescription>
          </SheetHeader>

          <div className="space-y-6 px-4 pb-6">
            {/* Dev DB Mode Indicator + Return to Live */}
            {devState?.isDevDbMode && (
              <section className="rounded-md border border-amber-500/30 bg-amber-50 p-3 dark:bg-amber-950/20">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-amber-700 dark:text-amber-400">
                      Dev DB Active
                    </p>
                    <p className="text-xs text-amber-600/80 dark:text-amber-500/80">
                      Using isolated dailyos-dev.db
                    </p>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    className="border-amber-500/50 text-amber-700 hover:bg-amber-100 dark:text-amber-400 dark:hover:bg-amber-900/30"
                    disabled={loading !== null}
                    onClick={() => runCommand("restore_live", "dev_restore_live", true)}
                  >
                    <Undo2 className="mr-1.5 h-3 w-3" />
                    {loading === "restore_live" ? "Restoring..." : "Return to Live"}
                  </Button>
                </div>
              </section>
            )}

            {/* Current State */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Current State
              </h3>
              <div className="space-y-2 text-sm">
                <StateRow
                  label="Config"
                  ok={devState?.hasConfig ?? false}
                  detail={devState?.workspacePath ?? "none"}
                />
                <StateRow
                  label="Database"
                  ok={devState?.hasDatabase ?? false}
                  detail={
                    devState
                      ? `${devState.actionCount} actions, ${devState.accountCount} accounts, ${devState.projectCount} projects, ${devState.meetingCount} meetings`
                      : "—"
                  }
                />
                <StateRow
                  label="Today data"
                  ok={devState?.hasTodayData ?? false}
                />
                <StateRow
                  label="Google"
                  ok={devState?.googleAuthStatus?.startsWith("authenticated") ?? false}
                  detail={devState?.googleAuthStatus ?? "unknown"}
                />
              </div>
            </section>

            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Latency Rollups
              </h3>
              <div className="space-y-2">
                {rollups?.commands?.length ? (
                  rollups.commands.slice(0, 8).map((r) => (
                    <div key={r.command} className="rounded-md border px-3 py-2 text-xs">
                      <div className="flex items-center justify-between gap-2">
                        <code className="truncate">{r.command}</code>
                        <span className="text-muted-foreground">{r.sampleCount} samples</span>
                      </div>
                      <div className="mt-1 text-muted-foreground">
                        p50 {r.p50Ms}ms · p95 {r.p95Ms}ms · max {r.maxMs}ms · budget {r.budgetMs}ms
                      </div>
                      <div className="text-muted-foreground">
                        violations {r.budgetViolations} · degraded {r.degradedCount}
                      </div>
                    </div>
                  ))
                ) : (
                  <p className="text-xs text-muted-foreground">No latency samples yet.</p>
                )}
              </div>
            </section>

            {/* Scenarios */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Scenarios
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={RotateCcw}
                  label="Reset to First Run"
                  description="Clears everything, shows onboarding"
                  variant="destructive"
                  loading={loading === "reset"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("reset")}
                />
                <ScenarioButton
                  icon={Database}
                  label="Full Mock Data"
                  description="Dashboard with 5 meetings, emails, actions"
                  variant="default"
                  loading={loading === "mock_full"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("mock_full")}
                />
                <ScenarioButton
                  icon={Brain}
                  label="Enriched Mock (Intelligence)"
                  description="Full mock + decisions, delegations, portfolio alerts, skip-today"
                  variant="default"
                  loading={loading === "mock_enriched"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("mock_enriched")}
                />
                <ScenarioButton
                  icon={Shield}
                  label="Mock — No Google"
                  description="Same data, no Google auth"
                  variant="outline"
                  loading={loading === "mock_no_auth"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("mock_no_auth")}
                />
                <ScenarioButton
                  icon={Inbox}
                  label="Empty State"
                  description="Config + workspace, no briefing data"
                  variant="outline"
                  loading={loading === "mock_empty"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("mock_empty")}
                />
                <ScenarioButton
                  icon={Zap}
                  label="Simulate Briefing"
                  description="Full mock + workspace markdown + directive JSONs"
                  variant="default"
                  loading={loading === "simulate_briefing"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("simulate_briefing")}
                />
              </div>
            </section>

            {/* Pipeline Testing */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Daily Briefing
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Sun}
                  label="Today — Mechanical"
                  description="Deliver schedule, actions, preps, emails (no AI)"
                  variant="outline"
                  loading={loading === "today_mechanical"}
                  disabled={loading !== null}
                  onClick={() => runCommand("today_mechanical", "dev_run_today_mechanical")}
                />
                <ScenarioButton
                  icon={Sparkles}
                  label="Today — Full + AI"
                  description="Mechanical + email/prep/briefing enrichment via Claude"
                  variant="outline"
                  loading={loading === "today_full"}
                  disabled={loading !== null}
                  onClick={() => runCommand("today_full", "dev_run_today_full")}
                />
              </div>
            </section>

            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Weekly Prep
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Calendar}
                  label="Week — Mechanical"
                  description="Deliver week-overview.json from directive"
                  variant="outline"
                  loading={loading === "week_mechanical"}
                  disabled={loading !== null}
                  onClick={() => runCommand("week_mechanical", "dev_run_week_mechanical")}
                />
                <ScenarioButton
                  icon={Sparkles}
                  label="Week — Full + AI"
                  description="Claude /week enrichment + week-overview delivery"
                  variant="outline"
                  loading={loading === "week_full"}
                  disabled={loading !== null}
                  onClick={() => runCommand("week_full", "dev_run_week_full")}
                />
              </div>
            </section>

            {/* Cleanup — visible when dev artifacts or mock data may exist */}
            {(devState?.hasDevDbFile || devState?.hasDevWorkspace || !devState?.isDevDbMode) && (
              <section>
                <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                  Cleanup
                </h3>
                <div className="space-y-2">
                  {/* Purge mock data from current (live) DB */}
                  {!devState?.isDevDbMode && (
                    <ScenarioButton
                      icon={Eraser}
                      label="Purge Mock Data from Live DB"
                      description="Remove known mock accounts, meetings, actions, people"
                      variant="destructive"
                      loading={loading === "purge_mock"}
                      disabled={loading !== null}
                      onClick={() => runCommand("purge_mock", "dev_purge_mock_data")}
                    />
                  )}
                  {/* Clean dev artifact files from disk */}
                  {(devState?.hasDevDbFile || devState?.hasDevWorkspace) && (
                    <ScenarioButton
                      icon={Trash2}
                      label="Clean Dev Artifacts"
                      description={[
                        devState?.hasDevDbFile && "dailyos-dev.db",
                        devState?.hasDevWorkspace && "DailyOS-dev/",
                      ].filter(Boolean).join(" + ")}
                      variant="outline"
                      loading={loading === "clean_artifacts"}
                      disabled={loading !== null}
                      onClick={async () => {
                        setLoading("clean_artifacts");
                        try {
                          const result = await invoke<string>("dev_clean_artifacts", {
                            includeWorkspace: true,
                          });
                          devToast("success", result);
                          await refreshState();
                        } catch (err) {
                          devToast("error", typeof err === "string" ? err : "Cleanup failed");
                        } finally {
                          setLoading(null);
                        }
                      }}
                    />
                  )}
                </div>
              </section>
            )}

            {/* Workspace info */}
            <section className="border-t pt-4">
              <p className="text-xs text-muted-foreground">
                Dev workspace: <code className="text-[11px]">~/Documents/DailyOS-dev/</code>
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                Mock scenarios never touch your real workspace.
              </p>
            </section>
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}

function StateRow({
  label,
  ok,
  detail,
}: {
  label: string;
  ok: boolean;
  detail?: string;
}) {
  return (
    <div className="flex items-start gap-2">
      <span className="mt-0.5 text-xs">{ok ? "✓" : "✗"}</span>
      <div className="min-w-0 flex-1">
        <span className="font-medium">{label}</span>
        {detail && (
          <p className="truncate text-xs text-muted-foreground">{detail}</p>
        )}
      </div>
    </div>
  );
}

function ScenarioButton({
  icon: Icon,
  label,
  description,
  variant,
  loading,
  disabled,
  onClick,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  description: string;
  variant: "default" | "destructive" | "outline";
  loading: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <Button
      variant={variant}
      className="h-auto w-full justify-start gap-3 px-3 py-2.5"
      disabled={disabled}
      onClick={onClick}
    >
      <Icon className="h-4 w-4 shrink-0" />
      <div className="text-left">
        <div className="text-sm font-medium">
          {loading ? "Applying..." : label}
        </div>
        <div className="text-xs font-normal opacity-70">{description}</div>
      </div>
    </Button>
  );
}
