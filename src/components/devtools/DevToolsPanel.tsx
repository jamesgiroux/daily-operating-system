import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Wrench, RotateCcw, Database, Shield, Inbox } from "lucide-react";
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

interface DevState {
  isDebugBuild: boolean;
  hasConfig: boolean;
  workspacePath: string | null;
  hasDatabase: boolean;
  actionCount: number;
  accountCount: number;
  meetingCount: number;
  hasTodayData: boolean;
  googleAuthStatus: string;
}

export function DevToolsPanel() {
  // Gate on dev mode — renders nothing in production builds
  if (!import.meta.env.DEV) return null;

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
      toast.success(result);
      // Brief delay to let the toast show before reload
      setTimeout(() => window.location.reload(), 500);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Scenario failed");
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

          <div className="mt-6 space-y-6">
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
                      ? `${devState.actionCount} actions, ${devState.accountCount} accounts, ${devState.meetingCount} meetings`
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
              </div>
            </section>

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
      <span className="mt-0.5 text-xs">{ok ? "\u2713" : "\u2717"}</span>
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
