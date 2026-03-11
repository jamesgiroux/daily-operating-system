import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Wrench, RotateCcw, Database, Shield, Zap, Sun, Calendar, Sparkles, Undo2, Trash2, UserX, KeyRound, AlertTriangle } from "lucide-react";
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

/** Toast with a copy button so messages can be pasted into CLI. */
function devToast(type: "success" | "error", message: string) {
  toast[type](message, {
    duration: type === "error" ? 8000 : 5000,
    action: {
      label: "Copy",
      onClick: (e) => {
        e.preventDefault(); // Prevent toast dismissal
        navigator.clipboard.writeText(message).catch(() => {});
      },
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

export function DevToolsPanel() {
  const [enabled, setEnabled] = useState(false);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
    // Show wrench when EITHER config.developerMode is on OR dev sandbox is active.
    // Check both: config may not exist after "Reset to First Run", but dev_get_state
    // still reports isDevDbMode correctly — so we don't lose the escape hatch.
    Promise.all([
      invoke<{ developerMode?: boolean }>("get_config")
        .then((cfg) => cfg.developerMode === true)
        .catch(() => false),
      invoke<{ isDevDbMode?: boolean }>("dev_get_state")
        .then((s) => s.isDevDbMode === true)
        .catch(() => false),
    ]).then(([configEnabled, sandboxActive]) => {
      setEnabled(configEnabled || sandboxActive);
    });
  }, []);

  if (!import.meta.env.DEV || !enabled) return null;

  return <DevToolsPanelInner />;
}

function DevToolsPanelInner() {
  const [open, setOpen] = useState(false);
  const [devState, setDevState] = useState<DevState | null>(null);
  const [loading, setLoading] = useState<string | null>(null);

  const refreshState = useCallback(async () => {
    try {
      const state = await invoke<DevState>("dev_get_state");
      setDevState(state);
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

  async function runOnboarding(key: string, scenario: string) {
    setLoading(key);
    try {
      const result = await invoke<string>("dev_onboarding_scenario", { scenario });
      devToast("success", result);
      setTimeout(() => window.location.reload(), 500);
    } catch (err) {
      devToast("error", typeof err === "string" ? err : "Onboarding scenario failed");
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

      <Sheet open={open} onOpenChange={setOpen} modal={false}>
        <SheetContent side="right" className="w-[380px] overflow-y-auto" showOverlay={false}>
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
                      Sandbox Active
                    </p>
                    <p className="text-xs text-amber-600/80 dark:text-amber-500/80">
                      Fully isolated — changes won't affect your real data
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
                <div className="mt-2 space-y-1 border-t border-amber-500/20 pt-2">
                  <p className="text-[11px] text-amber-600/70 dark:text-amber-500/60">
                    Database: <code>dailyos-dev.db</code>
                  </p>
                  <p className="text-[11px] text-amber-600/70 dark:text-amber-500/60">
                    Workspace: <code>~/Documents/DailyOS-dev/</code>
                  </p>
                  <p className="text-[11px] text-amber-600/70 dark:text-amber-500/60">
                    Google Auth: <code>in-memory only</code>
                  </p>
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
                      ? `${devState.accountCount} accounts, ${devState.peopleCount} people, ${devState.meetingCount} meetings, ${devState.actionCount} actions`
                      : "—"
                  }
                />
                <StateRow
                  label="Google"
                  ok={devState?.googleAuthStatus?.startsWith("authenticated") ?? false}
                  detail={devState?.googleAuthStatus ?? "unknown"}
                />
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
                  description="DB + intelligence + signals + health scores (all 6 dimensions)"
                  variant="default"
                  loading={loading === "full"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("full")}
                />
                <ScenarioButton
                  icon={Shield}
                  label="No Connectors"
                  description="Full DB data, no Google auth"
                  variant="outline"
                  loading={loading === "no_connectors"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("no_connectors")}
                />
                <ScenarioButton
                  icon={Zap}
                  label="Pipeline Test"
                  description="Full data + directive fixtures for delivery testing"
                  variant="default"
                  loading={loading === "pipeline"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("pipeline")}
                />
              </div>
            </section>

            {/* Onboarding Scenarios */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Onboarding
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Sparkles}
                  label="Happy Path"
                  description="Both auth mocked as ready"
                  variant="default"
                  loading={loading === "onb_auth_ready"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_auth_ready", "auth_ready")}
                />
                <ScenarioButton
                  icon={RotateCcw}
                  label="Fresh (Real Auth)"
                  description="Real first-run with real auth checks"
                  variant="outline"
                  loading={loading === "onb_fresh"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_fresh", "fresh")}
                />
                <ScenarioButton
                  icon={UserX}
                  label="Claude Not Installed"
                  description="Shows install instructions"
                  variant="outline"
                  loading={loading === "onb_no_claude"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_no_claude", "no_claude")}
                />
                <ScenarioButton
                  icon={KeyRound}
                  label="Claude Not Authenticated"
                  description="Claude found but not logged in"
                  variant="outline"
                  loading={loading === "onb_claude_unauthed"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_claude_unauthed", "claude_unauthed")}
                />
                <ScenarioButton
                  icon={Shield}
                  label="No Google"
                  description="Claude ready, Google not connected"
                  variant="outline"
                  loading={loading === "onb_no_google"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_no_google", "no_google")}
                />
                <ScenarioButton
                  icon={Calendar}
                  label="Google Token Expired"
                  description="Claude ready, Google token expired"
                  variant="outline"
                  loading={loading === "onb_google_expired"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_google_expired", "google_expired")}
                />
                <ScenarioButton
                  icon={AlertTriangle}
                  label="Nothing Works"
                  description="Both auth unavailable"
                  variant="destructive"
                  loading={loading === "onb_nothing_works"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_nothing_works", "nothing_works")}
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


            {/* Cleanup — visible when dev artifacts exist */}
            {(devState?.hasDevDbFile || devState?.hasDevWorkspace) && (
              <section>
                <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                  Cleanup
                </h3>
                <div className="space-y-2">
                  {/* Clean dev artifact files from disk */}
                  {(devState?.hasDevDbFile || devState?.hasDevWorkspace) && (
                    <ScenarioButton
                      icon={Trash2}
                      label="Reset Dev Environment"
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
