import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Building2,
  FolderKanban,
  Layers,
  FolderOpen,
  Mail,
  Zap,
  Check,
  ChevronLeft,
  Loader2,
  ArrowRight,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useGoogleAuth } from "@/hooks/useGoogleAuth";
import { useWorkflow } from "@/hooks/useWorkflow";
import type { EntityMode } from "@/types";

interface OnboardingWizardProps {
  onComplete: () => void;
}

const STEPS = ["welcome", "entity-mode", "workspace", "google", "generate"] as const;
type Step = (typeof STEPS)[number];

interface EntityModeOption {
  id: EntityMode;
  title: string;
  description: string;
  icon: typeof Building2;
}

const entityModeOptions: EntityModeOption[] = [
  {
    id: "account",
    title: "Account-based",
    description: "I manage external relationships — customers, clients, partners",
    icon: Building2,
  },
  {
    id: "project",
    title: "Project-based",
    description: "I manage internal efforts — features, campaigns, initiatives",
    icon: FolderKanban,
  },
  {
    id: "both",
    title: "Both",
    description: "I manage relationships and initiatives",
    icon: Layers,
  },
];

export function OnboardingWizard({ onComplete }: OnboardingWizardProps) {
  const [step, setStep] = useState<Step>("welcome");
  const [entityMode, setEntityMode] = useState<EntityMode | null>(null);
  const [workspacePath, setWorkspacePath] = useState<string>("");
  const [saving, setSaving] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [generated, setGenerated] = useState(false);

  const [homePath, setHomePath] = useState<string>("");

  const { status: authStatus, connect: connectGoogle, loading: authLoading } = useGoogleAuth();
  const { runNow } = useWorkflow();

  useEffect(() => {
    homeDir().then(setHomePath).catch(() => {});
  }, []);

  const stepIndex = STEPS.indexOf(step);
  const defaultWorkspacePath = homePath ? `${homePath}Documents/DailyOS` : "";
  const defaultWorkspaceDisplay = "~/Documents/DailyOS";

  function goBack() {
    if (stepIndex > 1 && stepIndex < 4) {
      setStep(STEPS[stepIndex - 1]);
    }
  }

  async function handleEntityModeSelect(mode: EntityMode) {
    setEntityMode(mode);
    setSaving(true);
    try {
      await invoke("set_entity_mode", { mode });
      setStep("workspace");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set entity mode");
      setEntityMode(null);
    } finally {
      setSaving(false);
    }
  }

  async function handleWorkspacePath(path: string) {
    setSaving(true);
    try {
      await invoke("set_workspace_path", { path });
      setWorkspacePath(path);
      setStep("google");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set workspace");
    } finally {
      setSaving(false);
    }
  }

  async function handleChooseWorkspace() {
    const selected = await open({
      directory: true,
      title: "Choose workspace directory",
    });
    if (selected) {
      await handleWorkspacePath(selected);
    }
  }

  async function handleGenerate() {
    setGenerating(true);
    try {
      await runNow();
      setGenerated(true);
      // Brief pause to show success state
      setTimeout(() => onComplete(), 1500);
    } catch {
      toast.error("Briefing generation failed — you can try again from the dashboard");
      onComplete();
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-4">
      <div className="w-full max-w-lg space-y-6">
        {/* Progress dots */}
        <div className="flex justify-center gap-2">
          {STEPS.map((s, i) => (
            <div
              key={s}
              className={cn(
                "size-2 rounded-full transition-colors",
                i <= stepIndex ? "bg-primary" : "bg-muted",
              )}
            />
          ))}
        </div>

        {/* Step content */}
        {step === "welcome" && (
          <div className="space-y-6 text-center">
            <div className="mx-auto flex size-16 items-center justify-center rounded-2xl bg-primary text-primary-foreground">
              <Zap className="size-8" />
            </div>
            <div className="space-y-2">
              <h1 className="text-3xl font-semibold tracking-tight">Welcome to DailyOS</h1>
              <p className="text-muted-foreground">
                Open the app. Your day is ready.
              </p>
            </div>
            <p className="text-sm text-muted-foreground leading-relaxed">
              DailyOS prepares your day before you start it — meetings prepped,
              emails triaged, actions surfaced. No setup rituals, no maintenance debt.
            </p>
            <Button size="lg" onClick={() => setStep("entity-mode")}>
              Get Started
              <ArrowRight className="ml-2 size-4" />
            </Button>
          </div>
        )}

        {step === "entity-mode" && (
          <div className="space-y-4">
            <div className="space-y-1">
              <h2 className="text-xl font-semibold tracking-tight">
                How do you organize your work?
              </h2>
              <p className="text-sm text-muted-foreground">
                This shapes how DailyOS structures your workspace
              </p>
            </div>
            <div className="grid gap-3">
              {entityModeOptions.map((option) => {
                const Icon = option.icon;
                const isSelected = entityMode === option.id;
                return (
                  <Card
                    key={option.id}
                    className={cn(
                      "cursor-pointer transition-all hover:-translate-y-0.5 hover:shadow-lg",
                      isSelected && "border-primary ring-1 ring-primary",
                      saving && !isSelected && "pointer-events-none opacity-50",
                    )}
                    onClick={() => !saving && handleEntityModeSelect(option.id)}
                  >
                    <CardHeader className="pb-3">
                      <div className="flex items-start justify-between">
                        <div className="flex items-center gap-3">
                          <div className="flex size-10 items-center justify-center rounded-lg bg-muted">
                            <Icon className="size-5" />
                          </div>
                          <div>
                            <CardTitle className="text-base">{option.title}</CardTitle>
                            <CardDescription>{option.description}</CardDescription>
                          </div>
                        </div>
                        {isSelected && (
                          <div className="flex size-6 items-center justify-center rounded-full bg-primary text-primary-foreground">
                            <Check className="size-4" />
                          </div>
                        )}
                      </div>
                    </CardHeader>
                  </Card>
                );
              })}
            </div>
          </div>
        )}

        {step === "workspace" && (
          <div className="space-y-6">
            <div className="space-y-1">
              <h2 className="text-xl font-semibold tracking-tight">
                Where should your workspace live?
              </h2>
              <p className="text-sm text-muted-foreground">
                DailyOS stores briefings, actions, and files here
              </p>
            </div>

            <div className="space-y-3">
              <Button
                className="w-full justify-between"
                onClick={() => defaultWorkspacePath && handleWorkspacePath(defaultWorkspacePath)}
                disabled={saving || !defaultWorkspacePath}
              >
                <div className="flex items-center gap-2">
                  <FolderOpen className="size-4" />
                  <span>Use default location</span>
                </div>
                <code className="text-xs opacity-70">{defaultWorkspaceDisplay}</code>
              </Button>

              <div className="relative">
                <div className="absolute inset-0 flex items-center">
                  <div className="w-full border-t" />
                </div>
                <div className="relative flex justify-center text-xs">
                  <span className="bg-background px-2 text-muted-foreground">or</span>
                </div>
              </div>

              <Button
                variant="outline"
                className="w-full"
                onClick={handleChooseWorkspace}
                disabled={saving}
              >
                {saving ? (
                  <Loader2 className="mr-2 size-4 animate-spin" />
                ) : (
                  <FolderOpen className="mr-2 size-4" />
                )}
                Choose a different folder
              </Button>
            </div>

            <Button variant="ghost" size="sm" onClick={goBack}>
              <ChevronLeft className="mr-1 size-4" />
              Back
            </Button>
          </div>
        )}

        {step === "google" && (
          <div className="space-y-6">
            <div className="space-y-1">
              <h2 className="text-xl font-semibold tracking-tight">
                Connect Google Calendar & Gmail
              </h2>
              <p className="text-sm text-muted-foreground">
                DailyOS reads your calendar to prep meetings and triages your email.
                Everything stays on your machine.
              </p>
            </div>

            {authStatus.status === "authenticated" ? (
              <div className="flex items-center gap-3 rounded-lg border bg-muted/30 p-4">
                <div className="flex size-8 items-center justify-center rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400">
                  <Check className="size-4" />
                </div>
                <div>
                  <p className="text-sm font-medium">Connected</p>
                  <p className="text-xs text-muted-foreground">
                    {authStatus.status === "authenticated" ? authStatus.email : ""}
                  </p>
                </div>
              </div>
            ) : (
              <Button
                size="lg"
                className="w-full"
                onClick={connectGoogle}
                disabled={authLoading}
              >
                {authLoading ? (
                  <Loader2 className="mr-2 size-4 animate-spin" />
                ) : (
                  <Mail className="mr-2 size-4" />
                )}
                Connect Google Account
              </Button>
            )}

            <div className="flex items-center justify-between">
              <Button variant="ghost" size="sm" onClick={goBack}>
                <ChevronLeft className="mr-1 size-4" />
                Back
              </Button>

              <Button onClick={() => setStep("generate")}>
                {authStatus.status === "authenticated" ? "Continue" : "Skip for now"}
                <ArrowRight className="ml-2 size-4" />
              </Button>
            </div>
          </div>
        )}

        {step === "generate" && (
          <div className="space-y-6 text-center">
            <div className="space-y-2">
              <h2 className="text-xl font-semibold tracking-tight">
                Ready to go
              </h2>
              <p className="text-sm text-muted-foreground">
                Generate your first briefing to see DailyOS in action
              </p>
            </div>

            <div className="rounded-lg border bg-muted/30 p-4 text-left text-sm">
              <div className="space-y-2">
                <div className="flex items-center gap-2">
                  <Check className="size-4 text-green-600" />
                  <span>
                    {entityMode === "account"
                      ? "Account-based"
                      : entityMode === "project"
                        ? "Project-based"
                        : "Both modes"}{" "}
                    workspace
                  </span>
                </div>
                <div className="flex items-center gap-2">
                  <Check className="size-4 text-green-600" />
                  <span className="truncate">{workspacePath || defaultWorkspaceDisplay}</span>
                </div>
                <div className="flex items-center gap-2">
                  {authStatus.status === "authenticated" ? (
                    <>
                      <Check className="size-4 text-green-600" />
                      <span>Google connected</span>
                    </>
                  ) : (
                    <>
                      <span className="size-4 text-center text-muted-foreground">—</span>
                      <span className="text-muted-foreground">Google not connected</span>
                    </>
                  )}
                </div>
              </div>
            </div>

            {generated ? (
              <div className="flex items-center justify-center gap-2 text-green-600">
                <Check className="size-5" />
                <span className="font-medium">Briefing ready</span>
              </div>
            ) : (
              <Button
                size="lg"
                onClick={handleGenerate}
                disabled={generating}
              >
                {generating ? (
                  <>
                    <Loader2 className="mr-2 size-4 animate-spin" />
                    Generating briefing...
                  </>
                ) : (
                  <>
                    <Zap className="mr-2 size-4" />
                    Generate First Briefing
                  </>
                )}
              </Button>
            )}

            {!generating && !generated && (
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground"
                onClick={onComplete}
              >
                Skip — I'll generate later
              </Button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
