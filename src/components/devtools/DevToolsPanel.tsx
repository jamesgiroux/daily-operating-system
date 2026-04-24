import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Wrench, RotateCcw, Database, Shield, Zap, Sun, Sparkles, Undo2, Trash2, UserX, AlertTriangle, Star, Link2, Brain, Package } from "lucide-react";
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

function DevToolsPanelInner({
  open: controlledOpen,
  onOpenChange: controlledOnOpenChange,
  hideWrench,
}: {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  hideWrench?: boolean;
} = {}) {
  const [internalOpen, setInternalOpen] = useState(false);
  const open = controlledOpen ?? internalOpen;
  const setOpen = controlledOnOpenChange ?? setInternalOpen;
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

  async function applyScenarioAndNavigate(key: string, scenario: string, path: string) {
    setLoading(key);
    try {
      const result = await invoke<string>("dev_apply_scenario", { scenario });
      devToast("success", result);
      setTimeout(() => {
        window.location.href = path;
      }, 500);
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
      {/* Floating wrench button — hidden when opened via external trigger (badge/standalone) */}
      {!hideWrench && (
        <button
          onClick={() => setOpen(true)}
          className="fixed bottom-4 right-4 z-50 flex h-10 w-10 items-center justify-center rounded-full bg-muted/80 text-muted-foreground shadow-md backdrop-blur-sm transition-colors hover:bg-muted hover:text-foreground"
          title="Dev Tools"
        >
          <Wrench className="h-4 w-4" />
        </button>
      )}

      <Sheet open={open} onOpenChange={setOpen} modal={false}>
        <SheetContent side="right" className="w-[380px] overflow-y-auto pt-[var(--folio-height)]" showOverlay={false}>
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
            {/* Sandbox indicator + Return to Live */}
            {devState?.isDevDbMode && (
              <section className="rounded-md border border-amber-500/30 bg-amber-50 p-3 dark:bg-amber-950/20">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm font-medium text-amber-700 dark:text-amber-400">
                      Sandbox Active
                    </p>
                    <p className="text-xs text-amber-600/80 dark:text-amber-500/80">
                      Isolated — changes won't affect your real data
                    </p>
                  </div>
                  <Button
                    variant="outline"
                    size="sm"
                    className="border-amber-500/50 text-amber-700 hover:bg-amber-100 dark:text-amber-400 dark:hover:bg-amber-900/30"
                    disabled={loading !== null}
                    onClick={async () => {
                      setLoading("restore_live");
                      try {
                        const result = await invoke<string>("dev_restore_live");
                        devToast("success", result);
                        setTimeout(() => { window.location.href = "/"; }, 500);
                      } catch (err) {
                        devToast("error", typeof err === "string" ? err : "Restore failed");
                        setLoading(null);
                      }
                    }}
                  >
                    <Undo2 className="mr-1.5 h-3 w-3" />
                    {loading === "restore_live" ? "Restoring..." : "Return to Live"}
                  </Button>
                </div>
              </section>
            )}

            {/* State summary */}
            {devState && (
              <p className="text-xs text-muted-foreground">
                {devState.accountCount} accounts · {devState.peopleCount} people · {devState.meetingCount} meetings · {devState.actionCount} actions
              </p>
            )}

            {/* ═══════════ QUICK START ═══════════ */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Quick Start
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Star}
                  label="Golden Path"
                  description="Full data + Linear + Glean sources + all 6 health dimensions"
                  variant="default"
                  loading={loading === "golden"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("golden")}
                />
              </div>
            </section>

            {/* ═══════════ ONBOARDING ═══════════ */}
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
                  icon={UserX}
                  label="No Claude"
                  description="Claude not installed, Google ready"
                  variant="outline"
                  loading={loading === "onb_no_claude"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_no_claude", "no_claude")}
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
                  icon={AlertTriangle}
                  label="Nothing Works"
                  description="Both auth unavailable"
                  variant="destructive"
                  loading={loading === "onb_nothing_works"}
                  disabled={loading !== null}
                  onClick={() => runOnboarding("onb_nothing_works", "nothing_works")}
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
              </div>
            </section>

            {/* ═══════════ DAILY BRIEFING ═══════════ */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Daily Briefing
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Zap}
                  label="Full Day + AI"
                  description="Full data + directive fixtures + AI enrichment"
                  variant="outline"
                  loading={loading === "pipeline"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("pipeline")}
                />
                <ScenarioButton
                  icon={Sun}
                  label="Mechanical Only"
                  description="Schedule, actions, preps, emails (no AI)"
                  variant="outline"
                  loading={loading === "today_mechanical"}
                  disabled={loading !== null}
                  onClick={() => runCommand("today_mechanical", "dev_run_today_mechanical")}
                />
              </div>
            </section>

            {/* ═══════════ ACCOUNT STATES ═══════════ */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Account States
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Database}
                  label="Healthy Account"
                  description="Green health, active engagement → Acme Corp"
                  variant="outline"
                  loading={loading === "acct_healthy"}
                  disabled={loading !== null}
                  onClick={async () => {
                    await applyScenarioAndNavigate("acct_healthy", "full", "/accounts/mock-acme-corp");
                  }}
                />
                <ScenarioButton
                  icon={AlertTriangle}
                  label="At-Risk Account"
                  description="Yellow health, renewal in 45 days → Globex Industries"
                  variant="outline"
                  loading={loading === "acct_atrisk"}
                  disabled={loading !== null}
                  onClick={async () => {
                    await applyScenarioAndNavigate("acct_atrisk", "full", "/accounts/mock-globex-industries");
                  }}
                />
                <ScenarioButton
                  icon={Package}
                  label="New Account"
                  description="Onboarding lifecycle, sparse data → Initech"
                  variant="outline"
                  loading={loading === "acct_new"}
                  disabled={loading !== null}
                  onClick={async () => {
                    await applyScenarioAndNavigate("acct_new", "full", "/accounts/mock-initech");
                  }}
                />
                <ScenarioButton
                  icon={Link2}
                  label="Parent/Child"
                  description="Contoso hierarchy (parent + 2 children)"
                  variant="outline"
                  loading={loading === "acct_hierarchy"}
                  disabled={loading !== null}
                  onClick={async () => {
                    await applyScenarioAndNavigate("acct_hierarchy", "full", "/accounts/mock-contoso");
                  }}
                />
                <ScenarioButton
                  icon={Database}
                  label="Financial Services"
                  description="Globex Holdings — steady-state with expansion signals"
                  variant="outline"
                  loading={loading === "acct_globex_holdings"}
                  disabled={loading !== null}
                  onClick={async () => {
                    await applyScenarioAndNavigate("acct_globex_holdings", "full", "/accounts/mock-globex-holdings");
                  }}
                />
              </div>
            </section>

            {/* ═══════════ INTEGRATIONS ═══════════ */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Integrations
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Link2}
                  label="Linear Connected"
                  description="Mock issues, projects, entity links, push indicators"
                  variant="outline"
                  loading={loading === "linear_connected"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("linear_connected")}
                />
                <ScenarioButton
                  icon={Brain}
                  label="Glean-Enriched"
                  description="Gong summaries, Salesforce context, source attribution"
                  variant="outline"
                  loading={loading === "glean_enriched"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("glean_enriched")}
                />
                <ScenarioButton
                  icon={Shield}
                  label="Disconnected"
                  description="Full data, no external integrations"
                  variant="outline"
                  loading={loading === "no_connectors"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("no_connectors")}
                />
              </div>
            </section>

            {/* ═══════════ EDGE CASES ═══════════ */}
            <section>
              <h3 className="mb-3 text-sm font-medium text-muted-foreground">
                Edge Cases
              </h3>
              <div className="space-y-2">
                <ScenarioButton
                  icon={Database}
                  label="Empty Portfolio"
                  description="Post-onboarding, 0 accounts"
                  variant="outline"
                  loading={loading === "empty_portfolio"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("empty_portfolio")}
                />
                <ScenarioButton
                  icon={RotateCcw}
                  label="Reset to First Run"
                  description="Clears everything, shows onboarding wizard"
                  variant="destructive"
                  loading={loading === "reset"}
                  disabled={loading !== null}
                  onClick={() => applyScenario("reset")}
                />
              </div>
            </section>

            {/* Cleanup — visible when stale dev artifacts exist */}
            {(devState?.hasDevDbFile || devState?.hasDevWorkspace) && !devState?.isDevDbMode && (
              <section className="border-t pt-4">
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

/**
 * DevToolsPanelSheet — controlled sheet variant for use by FolioBar badge.
 * Opens the dev tools panel without the floating wrench button.
 */
export function DevToolsPanelSheet({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return <DevToolsPanelInner open={open} onOpenChange={onOpenChange} hideWrench />;
}

/**
 * DevToolsPanelStandalone — floating wrench button for non-magazine shells.
 * Gated: only renders when dev mode is actually enabled (config OR DB flag).
 */
export function DevToolsPanelStandalone() {
  const [enabled, setEnabled] = useState(false);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (!import.meta.env.DEV) return;
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

  return (
    <>
      <button
        onClick={() => setOpen(true)}
        className="fixed bottom-4 right-4 z-50 flex h-8 w-8 items-center justify-center rounded-full bg-muted/80 text-muted-foreground shadow-md backdrop-blur-sm transition-colors hover:bg-muted hover:text-foreground"
        title="Dev Tools"
        style={{ fontSize: 14 }}
      >
        &#9881;
      </button>
      <DevToolsPanelInner open={open} onOpenChange={setOpen} />
    </>
  );
}
