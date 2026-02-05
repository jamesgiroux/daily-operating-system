import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { ModeToggle } from "@/components/mode-toggle";
import { cn } from "@/lib/utils";
import {
  AlertCircle,
  FolderOpen,
  Play,
  RefreshCw,
  Clock,
  Settings,
  CheckCircle,
} from "lucide-react";

interface Config {
  workspacePath: string;
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

export default function SettingsPage() {
  const [config, setConfig] = useState<Config | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState<string | null>(null);
  const [runResult, setRunResult] = useState<{ workflow: string; success: boolean; message: string } | null>(null);

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

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-32" />
          <Skeleton className="h-4 w-48" />
        </div>
        <div className="space-y-4">
          <Skeleton className="h-32" />
          <Skeleton className="h-48" />
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <Card className="border-destructive">
          <CardContent className="pt-6">
            <div className="flex items-center gap-2 text-destructive">
              <AlertCircle className="size-5" />
              <p>{error}</p>
            </div>
            <p className="mt-2 text-sm text-muted-foreground">
              Create a config file at <code className="rounded bg-muted px-1">~/.daybreak/config.json</code> with your workspace path.
            </p>
            <Button
              variant="outline"
              size="sm"
              className="mt-4"
              onClick={handleReloadConfig}
            >
              <RefreshCw className="mr-2 size-4" />
              Retry
            </Button>
          </CardContent>
        </Card>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
            <p className="text-sm text-muted-foreground">
              Configure your workspace and schedules
            </p>
          </div>

          <div className="space-y-6">
            {/* Workspace */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <FolderOpen className="size-4" />
                  Workspace
                </CardTitle>
                <CardDescription>
                  The directory where your DailyOS workspace lives
                </CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-between">
                  <code className="rounded bg-muted px-3 py-1.5 font-mono text-sm">
                    {config?.workspacePath || "Not configured"}
                  </code>
                  <Button variant="ghost" size="sm" onClick={handleReloadConfig}>
                    <RefreshCw className="size-4" />
                  </Button>
                </div>
                <p className="mt-2 text-xs text-muted-foreground">
                  Edit <code className="rounded bg-muted px-1">~/.daybreak/config.json</code> to change
                </p>
              </CardContent>
            </Card>

            {/* Theme */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <Settings className="size-4" />
                  Appearance
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex items-center justify-between">
                  <span className="text-sm">Theme</span>
                  <ModeToggle />
                </div>
              </CardContent>
            </Card>

            {/* Schedules */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <Clock className="size-4" />
                  Schedules
                </CardTitle>
                <CardDescription>
                  Automated workflow execution times
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {config?.schedules && (
                  <>
                    <ScheduleRow
                      label="Morning Briefing"
                      schedule={config.schedules.today}
                      running={running === "today"}
                      onRun={() => handleRunWorkflow("today")}
                    />
                    <ScheduleRow
                      label="Nightly Archive"
                      schedule={config.schedules.archive}
                      running={running === "archive"}
                      onRun={() => handleRunWorkflow("archive")}
                    />
                  </>
                )}
              </CardContent>
            </Card>

            {/* Run result */}
            {runResult && (
              <Card className={cn(runResult.success ? "border-success" : "border-destructive")}>
                <CardContent className="pt-6">
                  <div className="flex items-center gap-2">
                    {runResult.success ? (
                      <CheckCircle className="size-5 text-success" />
                    ) : (
                      <AlertCircle className="size-5 text-destructive" />
                    )}
                    <p className={cn("text-sm", runResult.success ? "text-success" : "text-destructive")}>
                      {runResult.message}
                    </p>
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Manual run section */}
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <Play className="size-4" />
                  Manual Run
                </CardTitle>
                <CardDescription>
                  Trigger workflows manually without waiting for schedule
                </CardDescription>
              </CardHeader>
              <CardContent className="flex gap-3">
                <Button
                  onClick={() => handleRunWorkflow("today")}
                  disabled={running !== null}
                >
                  {running === "today" ? (
                    <RefreshCw className="mr-2 size-4 animate-spin" />
                  ) : (
                    <Play className="mr-2 size-4" />
                  )}
                  Run /today
                </Button>
                <Button
                  variant="outline"
                  onClick={() => handleRunWorkflow("archive")}
                  disabled={running !== null}
                >
                  {running === "archive" ? (
                    <RefreshCw className="mr-2 size-4 animate-spin" />
                  ) : (
                    <Play className="mr-2 size-4" />
                  )}
                  Run Archive
                </Button>
              </CardContent>
            </Card>
          </div>
        </div>
      </ScrollArea>
    </main>
  );
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
    <div className="flex items-center justify-between rounded-md border p-3">
      <div>
        <div className="flex items-center gap-2">
          <span className="font-medium">{label}</span>
          <Badge variant={schedule.enabled ? "default" : "secondary"}>
            {schedule.enabled ? "Enabled" : "Disabled"}
          </Badge>
        </div>
        <p className="mt-1 font-mono text-xs text-muted-foreground">
          {schedule.cron} ({schedule.timezone})
        </p>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={onRun}
        disabled={running}
      >
        {running ? (
          <RefreshCw className="size-4 animate-spin" />
        ) : (
          <Play className="size-4" />
        )}
      </Button>
    </div>
  );
}
