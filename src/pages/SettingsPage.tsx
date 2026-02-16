import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { open } from "@tauri-apps/plugin-dialog";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { TabFilter } from "@/components/ui/tab-filter";
import { ModeToggle } from "@/components/mode-toggle";
import { cn } from "@/lib/utils";
import {
  AlertCircle,
  ArrowDownToLine,
  Building2,
  Check,
  FolderKanban,
  FolderOpen,
  Globe,
  Layers,
  Play,
  RefreshCw,
  Clock,
  Settings,
  CheckCircle,
  Mail,
  MessageSquare,
  LogOut,
  Loader2,
  ToggleRight,
  User,
  Activity,
  Cpu,
  X,
} from "lucide-react";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { usePersonality, type Personality } from "@/hooks/usePersonality";
import { toast } from "sonner";
import type {
  PostMeetingCaptureConfig,
  FeatureDefinition,
  EntityMode,
  AiModelConfig,
  SettingsTabId,
  HygieneStatusView,
} from "@/types";

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

const allSettingsTabs: { key: SettingsTabId; label: string }[] = [
  { key: "profile", label: "Profile" },
  { key: "integrations", label: "Integrations" },
  { key: "workflows", label: "Workflows" },
  { key: "intelligence", label: "Intelligence" },
  { key: "hygiene", label: "Intelligence Hygiene" },
  { key: "diagnostics", label: "Diagnostics" },
];

const settingsTabs = import.meta.env.DEV
  ? allSettingsTabs
  : allSettingsTabs.filter((t) => t.key !== "diagnostics");

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

export default function SettingsPage() {
  const search = useSearch({ from: "/settings" });
  const navigate = useNavigate();
  const activeTab = parseSettingsTab(search.tab);
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

  function setActiveTab(tab: SettingsTabId) {
    navigate({
      to: "/settings",
      search: (prev: Record<string, unknown>) => ({
        ...prev,
        tab,
      }),
    });
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
              Create a config file at <code className="rounded bg-muted px-1">~/.dailyos/config.json</code> with your workspace path.
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

          <div className="mb-6 overflow-x-auto pb-1">
            <TabFilter
              tabs={settingsTabs}
              active={activeTab}
              onChange={setActiveTab}
              className="w-max min-w-full"
            />
          </div>

          <div className="space-y-6">
            {activeTab === "profile" && (
              <>
                <UpdateCard />
                <UserProfileCard />
                <UserDomainsCard />
                <EntityModeCard
                  currentMode={config?.entityMode ?? "account"}
                  onModeChange={(mode) => setConfig(config ? { ...config, entityMode: mode } : null)}
                />
                <WorkspaceCard
                  workspacePath={config?.workspacePath ?? ""}
                  onPathChange={(path) => setConfig(config ? { ...config, workspacePath: path } : null)}
                />
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
                <PersonalityCard />
              </>
            )}

            {activeTab === "integrations" && (
              <>
                <GoogleAccountCard />
                <ClaudeDesktopCard />
              </>
            )}

            {activeTab === "workflows" && (
              <>
                <CaptureSettingsCard />
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
                  <CardContent className="flex flex-wrap gap-3">
                    <Button
                      onClick={() => handleRunWorkflow("today")}
                      disabled={running !== null}
                    >
                      {running === "today" ? (
                        <RefreshCw className="mr-2 size-4 animate-spin" />
                      ) : (
                        <Play className="mr-2 size-4" />
                      )}
                      Run Daily Briefing
                    </Button>
                    <Button
                      variant="outline"
                      onClick={() => handleRunWorkflow("week")}
                      disabled={running !== null}
                    >
                      {running === "week" ? (
                        <RefreshCw className="mr-2 size-4 animate-spin" />
                      ) : (
                        <Play className="mr-2 size-4" />
                      )}
                      Run Weekly Briefing
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
              </>
            )}

            {activeTab === "intelligence" && (
              <>
                <FeaturesCard />
                <AiModelsCard />
              </>
            )}

            {activeTab === "hygiene" && <IntelligenceHygieneCard />}

            {activeTab === "diagnostics" && (
              <>
                <Card>
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <ToggleRight className="size-4" />
                      Developer
                    </CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="flex items-center justify-between">
                      <div>
                        <span className="text-sm">Developer Tools</span>
                        <p className="text-xs text-muted-foreground">
                          Show the devtools panel (wrench icon)
                        </p>
                      </div>
                      <Button
                        variant={config?.developerMode ? "default" : "outline"}
                        size="sm"
                        onClick={async () => {
                          const next = !config?.developerMode;
                          try {
                            const updated = await invoke<Config>("set_developer_mode", { enabled: next });
                            setConfig(updated);
                            toast.success(next ? "Developer tools enabled" : "Developer tools disabled");
                          } catch (e) {
                            toast.error(String(e));
                          }
                        }}
                      >
                        {config?.developerMode ? "On" : "Off"}
                      </Button>
                    </div>
                  </CardContent>
                </Card>
                <MeetingBackfillCard />
              </>
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ArrowDownToLine className="size-4" />
          Updates
        </CardTitle>
        <CardDescription>
          {appVersion ? `DailyOS v${appVersion}` : "DailyOS"}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {state.phase === "idle" || state.phase === "error" ? (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {state.phase === "error"
                ? "Update check failed"
                : "Check for new versions"}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={handleCheck}
            >
              <RefreshCw className="mr-1.5 size-3.5" />
              Check for Updates
            </Button>
          </div>
        ) : state.phase === "checking" ? (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              Checking for updates...
            </span>
            <Button variant="outline" size="sm" disabled>
              <Loader2 className="mr-1.5 size-3.5 animate-spin" />
              Checking
            </Button>
          </div>
        ) : state.phase === "available" ? (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm font-medium">
                  v{state.update.version} available
                </p>
                {state.update.body && (
                  <p className="mt-1 text-xs text-muted-foreground line-clamp-2">
                    {state.update.body}
                  </p>
                )}
              </div>
              <Button size="sm" onClick={handleInstall}>
                <ArrowDownToLine className="mr-1.5 size-3.5" />
                Install & Restart
              </Button>
            </div>
          </div>
        ) : state.phase === "installing" ? (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              Installing update...
            </span>
            <Button size="sm" disabled>
              <Loader2 className="mr-1.5 size-3.5 animate-spin" />
              Installing
            </Button>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}

function ClaudeDesktopCard() {
  const [configuring, setConfiguring] = useState(false);
  const [result, setResult] = useState<{
    success: boolean;
    message: string;
    configPath?: string;
    binaryPath?: string;
  } | null>(null);

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <MessageSquare className="size-4" />
          Claude Desktop
        </CardTitle>
        <CardDescription>
          Connect Claude Desktop to query your workspace via MCP
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {result && (
          <div
            className={cn(
              "flex items-center gap-2 rounded-md border px-3 py-2",
              result.success
                ? "border-sage/40 bg-sage/10"
                : "border-destructive/40 bg-destructive/10"
            )}
          >
            {result.success ? (
              <CheckCircle className="size-3.5 text-sage" />
            ) : (
              <AlertCircle className="size-3.5 text-destructive" />
            )}
            <span className="text-xs">{result.message}</span>
          </div>
        )}
        <Button
          variant="outline"
          size="sm"
          onClick={handleConfigure}
          disabled={configuring}
        >
          {configuring ? (
            <Loader2 className="mr-2 size-3.5 animate-spin" />
          ) : (
            <Settings className="mr-2 size-3.5" />
          )}
          {result?.success ? "Reconfigure" : "Connect to Claude Desktop"}
        </Button>
        <p className="text-xs text-muted-foreground">
          Adds DailyOS as an MCP server in Claude Desktop. After connecting,
          Claude can query your briefing, accounts, projects, and meeting
          history.
        </p>
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Mail className="size-4" />
          Google Account
        </CardTitle>
        <CardDescription>
          {status.status === "authenticated"
            ? "Calendar and meeting features active"
            : "Connect Google for calendar awareness and meeting features"}
        </CardDescription>
      </CardHeader>
      <CardContent>
        {error && (
          <div className="mb-3 flex items-center justify-between rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2">
            <span className="text-xs text-destructive">{error}</span>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-xs text-destructive"
              onClick={clearError}
            >
              Dismiss
            </Button>
          </div>
        )}

        {status.status === "authenticated" ? (
          <div className="flex items-center justify-between">
            <div>
              <div className="flex items-center gap-2">
                <span className="relative flex size-2">
                  <span className="absolute inline-flex size-full rounded-full bg-success opacity-75" />
                  <span className="relative inline-flex size-2 rounded-full bg-success" />
                </span>
                <span className="text-sm">{email}</span>
              </div>
              {justConnected && (
                <p className="mt-1 text-xs text-success">Connected successfully.</p>
              )}
            </div>
            <Button
              variant="ghost"
              size="sm"
              className="text-muted-foreground"
              onClick={disconnect}
              disabled={loading || phase === "authorizing"}
            >
              {loading ? (
                <Loader2 className="mr-1.5 size-3.5 animate-spin" />
              ) : (
                <LogOut className="mr-1.5 size-3.5" />
              )}
              Disconnect
            </Button>
          </div>
        ) : status.status === "tokenexpired" ? (
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="inline-flex size-2 rounded-full bg-destructive" />
              <span className="text-sm text-muted-foreground">
                Session expired
              </span>
            </div>
            <Button size="sm" onClick={connect} disabled={loading}>
              {loading ? (
                <Loader2 className="mr-1.5 size-3.5 animate-spin" />
              ) : (
                <RefreshCw className="mr-1.5 size-3.5" />
              )}
              {phase === "authorizing" ? "Waiting for authorization..." : "Reconnect"}
            </Button>
          </div>
        ) : (
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Not connected</span>
            <Button size="sm" onClick={connect} disabled={loading}>
              {loading && <Loader2 className="mr-1.5 size-3.5 animate-spin" />}
              {phase === "authorizing" ? "Waiting for authorization..." : "Connect"}
            </Button>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <MessageSquare className="size-4" />
          Personality
        </CardTitle>
        <CardDescription>
          Sets the tone for empty states, loading messages, and notifications
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-3">
          {PERSONALITY_OPTIONS.map((option) => (
            <button
              key={option.value}
              onClick={() => handleChange(option.value)}
              className={cn(
                "flex flex-col items-start gap-1 rounded-lg border p-3 text-left transition-colors",
                personality === option.value
                  ? "border-primary bg-primary/5"
                  : "border-border hover:bg-muted/50",
              )}
            >
              <div className="flex items-center gap-2">
                <span className="text-sm font-medium">{option.label}</span>
                {personality === option.value && (
                  <Check className="size-3.5 text-primary" />
                )}
              </div>
              <span className="text-xs text-muted-foreground">
                {option.description}
              </span>
              <span className="mt-1 text-xs italic text-muted-foreground/70">
                "{option.example}"
              </span>
            </button>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

function UserProfileCard() {
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [title, setTitle] = useState("");
  const [focus, setFocus] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<{
      userName?: string;
      userCompany?: string;
      userTitle?: string;
      userFocus?: string;
    }>("get_config")
      .then((config) => {
        setName(config.userName ?? "");
        setCompany(config.userCompany ?? "");
        setTitle(config.userTitle ?? "");
        setFocus(config.userFocus ?? "");
      })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      await invoke("set_user_profile", {
        name: name.trim() || null,
        company: company.trim() || null,
        title: title.trim() || null,
        focus: focus.trim() || null,
        domain: null, // domain is managed by UserDomainsCard
      });
      toast.success("Profile updated");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update profile");
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-24" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-10 w-full" />
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <User className="size-4" />
          About You
        </CardTitle>
        <CardDescription>
          Helps DailyOS personalize your briefings and meeting prep
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-1.5">
            <label htmlFor="profile-name" className="text-sm font-medium">Name</label>
            <Input
              id="profile-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Jamie"
            />
          </div>
          <div className="space-y-1.5">
            <label htmlFor="profile-company" className="text-sm font-medium">Company</label>
            <Input
              id="profile-company"
              value={company}
              onChange={(e) => setCompany(e.target.value)}
              placeholder="e.g. Acme Inc."
            />
          </div>
          <div className="space-y-1.5">
            <label htmlFor="profile-title" className="text-sm font-medium">Title</label>
            <Input
              id="profile-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="e.g. Customer Success Manager"
            />
          </div>
          <div className="space-y-1.5">
            <label htmlFor="profile-focus" className="text-sm font-medium">Current focus</label>
            <Input
              id="profile-focus"
              value={focus}
              onChange={(e) => setFocus(e.target.value)}
              placeholder="e.g. Driving Q2 renewals"
            />
          </div>
        </div>
        <div className="flex justify-end">
          <Button size="sm" onClick={handleSave} disabled={saving}>
            {saving ? <Loader2 className="mr-2 size-4 animate-spin" /> : null}
            Save
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Globe className="size-4" />
          Your Domains
        </CardTitle>
        <CardDescription>
          Your organization's email domains — used to distinguish internal vs external meetings
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div
          className={cn(
            "flex flex-wrap items-center gap-1.5 rounded-md border bg-background px-3 py-2 text-sm",
            "focus-within:ring-1 focus-within:ring-ring",
          )}
        >
          {domains.map((d) => (
            <span
              key={d}
              className="inline-flex items-center gap-1 rounded-md bg-muted px-2 py-0.5 font-mono text-xs"
            >
              {d}
              <button
                type="button"
                onClick={() => removeDomain(d)}
                className="text-muted-foreground hover:text-foreground"
                disabled={saving}
              >
                <X className="size-3" />
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
            className="min-w-[120px] flex-1 bg-transparent font-mono text-sm outline-none placeholder:text-muted-foreground/50"
            disabled={loading}
          />
          {saving && <Loader2 className="size-3.5 animate-spin text-muted-foreground" />}
        </div>
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <MessageSquare className="size-4" />
          Post-Meeting Capture
        </CardTitle>
        <CardDescription>
          Prompt for quick outcomes after customer meetings
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm">
              {captureConfig?.enabled ? "Enabled" : "Disabled"}
            </p>
            <p className="text-xs text-muted-foreground">
              {captureConfig?.enabled
                ? "Prompts appear after customer meetings end"
                : "Post-meeting prompts are turned off"}
            </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={toggleCapture}
            disabled={!captureConfig}
          >
            {captureConfig?.enabled ? "Disable" : "Enable"}
          </Button>
        </div>

        {captureConfig?.enabled && (
          <div className="flex items-center justify-between rounded-md border p-3">
            <div>
              <p className="text-sm font-medium">Prompt delay</p>
              <p className="text-xs text-muted-foreground">
                Wait before showing the prompt
              </p>
            </div>
            <div className="flex gap-1">
              {[2, 5, 10].map((mins) => (
                <Button
                  key={mins}
                  variant={captureConfig.delayMinutes === mins ? "default" : "outline"}
                  size="sm"
                  className="text-xs"
                  onClick={() => updateDelay(mins)}
                >
                  {mins}m
                </Button>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <ToggleRight className="size-4" />
          Features
        </CardTitle>
        <CardDescription>
          Enable or disable pipeline operations
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {features.map((feature) => (
          <div key={feature.key} className="flex items-center justify-between rounded-md border p-3">
            <div>
              <div className="flex items-center gap-2">
                <p className="text-sm font-medium">{feature.label}</p>
                {feature.csOnly && (
                  <Badge variant="outline" className="text-[10px]">CS</Badge>
                )}
              </div>
              <p className="text-xs text-muted-foreground">
                {feature.description}
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="text-xs"
              onClick={() => toggleFeature(feature.key, feature.enabled)}
            >
              {feature.enabled ? "Enabled" : "Disabled"}
            </Button>
          </div>
        ))}
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Cpu className="size-4" />
          AI Models
        </CardTitle>
        <CardDescription>
          Choose which Claude model handles each type of operation
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {(["synthesis", "extraction", "mechanical"] as const).map((tier) => {
          const info = tierDescriptions[tier];
          const current = aiModels?.[tier] ?? "sonnet";
          return (
            <div key={tier} className="flex items-center justify-between rounded-md border p-3">
              <div>
                <p className="text-sm font-medium">{info.label}</p>
                <p className="text-xs text-muted-foreground">{info.description}</p>
              </div>
              <div className="flex gap-1">
                {modelOptions.map((model) => (
                  <Button
                    key={model}
                    variant={current === model ? "default" : "outline"}
                    size="sm"
                    className="text-xs"
                    onClick={() => handleModelChange(tier, model)}
                    disabled={!aiModels}
                  >
                    {model}
                  </Button>
                ))}
              </div>
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}

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
    <div className="flex items-center justify-between rounded-md border p-3">
      <div>
        <div className="flex items-center gap-2">
          <span className="font-medium">{label}</span>
          <Badge variant={schedule.enabled ? "default" : "secondary"}>
            {schedule.enabled ? "Enabled" : "Disabled"}
          </Badge>
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          {cronToHumanTime(schedule.cron)}{" "}
          <span className="text-muted-foreground/60">({schedule.timezone})</span>
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

// =============================================================================
// Entity Mode Card
// =============================================================================

const entityModeOptions: { id: EntityMode; title: string; description: string; icon: typeof Building2 }[] = [
  {
    id: "account",
    title: "Account-based",
    description: "External relationships — customers, clients, partners",
    icon: Building2,
  },
  {
    id: "project",
    title: "Project-based",
    description: "Internal efforts — features, campaigns, initiatives",
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
      toast.success("Entity mode updated — reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update entity mode");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Layers className="size-4" />
          Work Mode
        </CardTitle>
        <CardDescription>
          How you organize your work — shapes workspace structure and sidebar
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2">
        {entityModeOptions.map((option) => {
          const Icon = option.icon;
          const isSelected = currentMode === option.id;
          return (
            <button
              key={option.id}
              type="button"
              className={cn(
                "flex w-full items-center gap-3 rounded-md border p-3 text-left transition-colors",
                isSelected
                  ? "border-primary bg-primary/5"
                  : "hover:bg-muted/50",
                saving && !isSelected && "pointer-events-none opacity-50",
              )}
              onClick={() => handleSelect(option.id)}
              disabled={saving}
            >
              <div className="flex size-8 items-center justify-center rounded-md bg-muted">
                <Icon className="size-4" />
              </div>
              <div className="flex-1">
                <p className="text-sm font-medium">{option.title}</p>
                <p className="text-xs text-muted-foreground">{option.description}</p>
              </div>
              {isSelected && (
                <div className="flex size-5 items-center justify-center rounded-full bg-primary text-primary-foreground">
                  <Check className="size-3" />
                </div>
              )}
            </button>
          );
        })}
      </CardContent>
    </Card>
  );
}

// =============================================================================
// Workspace Card
// =============================================================================

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
      toast.success("Workspace updated — reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set workspace");
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <FolderOpen className="size-4" />
          Workspace
        </CardTitle>
        <CardDescription>
          The directory where DailyOS stores briefings, actions, and files
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-between">
          <code className="rounded bg-muted px-3 py-1.5 font-mono text-sm">
            {workspacePath || "Not configured"}
          </code>
          <Button
            variant="outline"
            size="sm"
            onClick={handleChooseWorkspace}
            disabled={saving}
          >
            {saving ? (
              <Loader2 className="mr-1.5 size-3.5 animate-spin" />
            ) : (
              <FolderOpen className="mr-1.5 size-3.5" />
            )}
            Change
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

// =============================================================================
// Intelligence Hygiene Card (I213)
// =============================================================================

function formatTime(iso?: string): string {
  if (!iso) return "—";
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
  const [loading, setLoading] = useState(true);
  const [runningNow, setRunningNow] = useState(false);
  const [hygieneConfig, setHygieneConfig] = useState<HygieneConfig>({
    hygieneScanIntervalHours: 4,
    hygieneAiBudget: 10,
    hygienePreMeetingHours: 12,
  });
  const [showAllFixes, setShowAllFixes] = useState(false);

  async function loadStatus() {
    try {
      const result = await invoke<HygieneStatusView>("get_intelligence_hygiene_status");
      setStatus(result);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to load hygiene status");
    } finally {
      setLoading(false);
    }
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

  function handleGapAction(route?: string) {
    if (!route) {
      runScanNow();
      return;
    }

    if (route.startsWith("/people")) {
      const parsed = new URL(route, "http://localhost");
      const relationship = parsed.searchParams.get("relationship") ?? undefined;
      const hygiene = parsed.searchParams.get("hygiene") ?? undefined;
      navigate({
        to: "/people",
        search: {
          relationship: relationship as "all" | "external" | "internal" | "unknown" | undefined,
          hygiene: hygiene as "unnamed" | "duplicates" | undefined,
        },
      });
      return;
    }

    runScanNow();
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <Activity className="size-4" />
          Intelligence Hygiene
        </CardTitle>
        <CardDescription>
          Proactive intelligence maintenance with clear next actions
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {loading ? (
          <div className="space-y-3">
            <Skeleton className="h-6 w-48" />
            <Skeleton className="h-20 w-full" />
          </div>
        ) : status ? (
          <>
            <div className="grid gap-3 sm:grid-cols-2">
              <div className="rounded-md border p-3">
                <p className="text-xs uppercase tracking-wide text-muted-foreground">Status</p>
                <div className="mt-1 flex items-center justify-between">
                  <p className="text-sm font-medium">{status.statusLabel}</p>
                  <Badge variant={status.totalGaps === 0 ? "secondary" : "outline"}>
                    {status.totalGaps} gaps
                  </Badge>
                </div>
                <p className="mt-2 text-xs text-muted-foreground">
                  Last scan: {formatTime(status.lastScanTime)}
                  {status.scanDurationMs != null && (
                    <span className="ml-1 text-muted-foreground/60">
                      ({status.scanDurationMs < 1000
                        ? `${status.scanDurationMs}ms`
                        : `${(status.scanDurationMs / 1000).toFixed(1)}s`})
                    </span>
                  )}
                </p>
                <p className="text-xs text-muted-foreground">
                  Next scan: {formatTime(status.nextScanTime)}
                </p>
              </div>
              <div className="rounded-md border p-3">
                <p className="text-xs uppercase tracking-wide text-muted-foreground">AI Budget</p>
                <p className="mt-1 text-sm font-medium">
                  {status.budget.usedToday} / {status.budget.dailyLimit} used today
                </p>
                <p className="mt-2 text-xs text-muted-foreground">
                  Queued for next budget window: {status.budget.queuedForNextBudget}
                </p>
              </div>
            </div>

            <div className="rounded-md border p-3">
              <div className="mb-2 flex items-center justify-between">
                <p className="text-sm font-medium">Fixes Applied</p>
                <Badge variant={status.totalFixes > 0 ? "default" : "secondary"}>
                  {status.totalFixes}
                </Badge>
              </div>
              {status.fixDetails && status.fixDetails.length > 0 ? (
                <div className="space-y-1">
                  {(showAllFixes ? status.fixDetails : status.fixDetails.slice(0, 5)).map((detail, i) => (
                    <p key={i} className="text-xs text-muted-foreground">
                      &bull; {detail.description}
                    </p>
                  ))}
                  {status.fixDetails.length > 5 && !showAllFixes && (
                    <button
                      className="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
                      onClick={() => setShowAllFixes(true)}
                    >
                      &hellip; and {status.fixDetails.length - 5} more
                    </button>
                  )}
                  {showAllFixes && status.fixDetails.length > 5 && (
                    <button
                      className="text-xs text-muted-foreground underline underline-offset-2 hover:text-foreground"
                      onClick={() => setShowAllFixes(false)}
                    >
                      Show less
                    </button>
                  )}
                </div>
              ) : status.fixes.length > 0 ? (
                <div className="space-y-1">
                  {status.fixes.map((fix) => (
                    <div key={fix.key} className="flex items-center justify-between text-xs text-muted-foreground">
                      <span>{fix.label}</span>
                      <span>{fix.count}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-xs text-muted-foreground">No fixes were applied in the most recent scan.</p>
              )}
            </div>

            <div className="rounded-md border p-3">
              <div className="mb-2 flex items-center justify-between">
                <p className="text-sm font-medium">Gaps Detected</p>
                <Badge variant={status.gaps.length > 0 ? "outline" : "secondary"}>
                  {status.gaps.length}
                </Badge>
              </div>
              {status.gaps.length > 0 ? (
                <div className="space-y-2">
                  {status.gaps.map((gap) => (
                    <div key={gap.key} className="rounded-md border p-2.5">
                      <div className="flex items-start justify-between gap-3">
                        <div>
                          <p className="text-sm font-medium">
                            {gap.label} <span className="text-muted-foreground">({gap.count})</span>
                          </p>
                          <p className="text-xs text-muted-foreground">{gap.description}</p>
                        </div>
                        <Badge variant="outline" className="text-[10px] uppercase">
                          {gap.impact}
                        </Badge>
                      </div>
                      <div className="mt-2">
                        <Button
                          variant="outline"
                          size="sm"
                          className="text-xs"
                          onClick={() => handleGapAction(gap.action.route)}
                        >
                          {gap.action.label}
                        </Button>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-xs text-muted-foreground">
                  No open hygiene gaps. The system will continue scanning automatically.
                </p>
              )}
            </div>

            <div className="rounded-md border p-3">
              <p className="mb-3 text-sm font-medium">Configuration</p>
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm">Scan Interval</p>
                    <p className="text-xs text-muted-foreground">How often hygiene runs</p>
                  </div>
                  <div className="flex gap-1">
                    {scanIntervalOptions.map((v) => (
                      <Button
                        key={v}
                        variant={hygieneConfig.hygieneScanIntervalHours === v ? "default" : "outline"}
                        size="sm"
                        className="text-xs"
                        onClick={() => handleHygieneConfigChange("scanIntervalHours", v)}
                      >
                        {v}hr
                      </Button>
                    ))}
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm">Daily AI Budget</p>
                    <p className="text-xs text-muted-foreground">Max AI enrichments per day</p>
                  </div>
                  <div className="flex gap-1">
                    {aiBudgetOptions.map((v) => (
                      <Button
                        key={v}
                        variant={hygieneConfig.hygieneAiBudget === v ? "default" : "outline"}
                        size="sm"
                        className="text-xs"
                        onClick={() => handleHygieneConfigChange("aiBudget", v)}
                      >
                        {v}
                      </Button>
                    ))}
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-sm">Pre-Meeting Window</p>
                    <p className="text-xs text-muted-foreground">Refresh intel before meetings</p>
                  </div>
                  <div className="flex gap-1">
                    {preMeetingOptions.map((v) => (
                      <Button
                        key={v}
                        variant={hygieneConfig.hygienePreMeetingHours === v ? "default" : "outline"}
                        size="sm"
                        className="text-xs"
                        onClick={() => handleHygieneConfigChange("preMeetingHours", v)}
                      >
                        {v}hr
                      </Button>
                    ))}
                  </div>
                </div>
              </div>
            </div>

            <div className="flex items-center gap-2">
              <Button onClick={runScanNow} disabled={runningNow || status.isRunning}>
                {(runningNow || status.isRunning) && <Loader2 className="mr-2 size-4 animate-spin" />}
                Run Hygiene Scan Now
              </Button>
              <Button variant="ghost" size="sm" onClick={loadStatus}>
                Refresh
              </Button>
            </div>
          </>
        ) : (
          <p className="text-sm text-muted-foreground">
            No scan completed yet — runs automatically after startup.
          </p>
        )}
      </CardContent>
    </Card>
  );
}

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
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <RefreshCw className="size-4" />
          Historical Meeting Backfill
        </CardTitle>
        <CardDescription>
          Import historical meeting files from your workspace into the database
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <p className="text-sm text-muted-foreground">
            Scans account and project directories for meeting files (transcripts, notes, summaries)
            and creates database records + entity links for any meetings not already in the system.
          </p>
          <p className="text-sm text-muted-foreground">
            Looks for files in: <code className="text-xs">02-Meetings/</code>, <code className="text-xs">03-Call-Transcripts/</code>,{" "}
            <code className="text-xs">Call-Transcripts/</code>, <code className="text-xs">Meeting-Notes/</code>
          </p>
        </div>

        {result && (
          <div className="rounded-lg bg-muted p-3 space-y-2">
            <div className="flex items-center gap-2">
              {result.errors.length === 0 ? (
                <CheckCircle className="size-4 text-green-600" />
              ) : (
                <AlertCircle className="size-4 text-yellow-600" />
              )}
              <span className="text-sm font-medium">
                Created {result.created} meetings, skipped {result.skipped}
              </span>
            </div>
            
            {result.errors.length > 0 && (
              <div className="space-y-1">
                <p className="text-xs font-medium text-destructive">Errors:</p>
                <ScrollArea className="h-32">
                  <div className="space-y-1">
                    {result.errors.map((err, i) => (
                      <p key={i} className="text-xs text-muted-foreground font-mono">{err}</p>
                    ))}
                  </div>
                </ScrollArea>
              </div>
            )}
          </div>
        )}

        <Button 
          onClick={runBackfill} 
          disabled={isRunning}
          className="w-full"
        >
          {isRunning ? (
            <>
              <Loader2 className="mr-2 size-4 animate-spin" />
              Scanning directories...
            </>
          ) : (
            <>
              <RefreshCw className="mr-2 size-4" />
              Run Backfill
            </>
          )}
        </Button>
      </CardContent>
    </Card>
  );
}
