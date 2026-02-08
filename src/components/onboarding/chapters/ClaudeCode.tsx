import { useState } from "react";
import {
  Check,
  ArrowRight,
  Terminal,
  Loader2,
  AlertCircle,
  Sparkles,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { invoke } from "@tauri-apps/api/core";

interface ClaudeCodeProps {
  onNext: (installed: boolean) => void;
}

interface ClaudeStatus {
  installed: boolean;
  authenticated: boolean;
}

export function ClaudeCode({ onNext }: ClaudeCodeProps) {
  const [status, setStatus] = useState<ClaudeStatus | null>(null);
  const [checking, setChecking] = useState(false);

  async function checkStatus() {
    setChecking(true);
    try {
      const result = await invoke<ClaudeStatus>("check_claude_status");
      setStatus(result);
    } catch {
      setStatus({ installed: false, authenticated: false });
    } finally {
      setChecking(false);
    }
  }

  // Auto-check on first render
  if (status === null && !checking) {
    checkStatus();
  }

  const isReady = status?.installed && status?.authenticated;

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">
          The AI engine behind your briefings
        </h2>
      </div>

      {/* What Claude Code does */}
      <div className="rounded-lg border bg-muted/30 p-4 space-y-3">
        <div className="flex items-center gap-2">
          <Sparkles className="size-4 text-primary" />
          <span className="text-sm font-medium">Claude Code powers DailyOS intelligence</span>
        </div>
        <p className="text-sm text-muted-foreground leading-relaxed">
          Claude Code generates your briefing narrative, enriches email summaries with
          recommended actions, and processes inbox files with AI classification. Without it,
          DailyOS still delivers your schedule, actions, and meeting preps — but AI summaries
          and enrichment won't be available.
        </p>
      </div>

      {/* Status display */}
      {checking && !status && (
        <div className="flex items-center gap-3 rounded-lg border bg-muted/30 p-4">
          <Loader2 className="size-5 animate-spin text-muted-foreground" />
          <span className="text-sm text-muted-foreground">Checking for Claude Code...</span>
        </div>
      )}

      {status && isReady && (
        <div className="flex items-center gap-3 rounded-lg border bg-muted/30 p-4">
          <div className="flex size-8 items-center justify-center rounded-full bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400">
            <Check className="size-4" />
          </div>
          <div>
            <p className="text-sm font-medium">Claude Code is ready</p>
            <p className="text-xs text-muted-foreground">Installed and authenticated</p>
          </div>
        </div>
      )}

      {status && status.installed && !status.authenticated && (
        <div className="space-y-3">
          <div className="flex items-center gap-3 rounded-lg border border-amber-200 bg-amber-50 p-4 dark:border-amber-800 dark:bg-amber-950/30">
            <AlertCircle className="size-5 shrink-0 text-amber-600 dark:text-amber-400" />
            <div>
              <p className="text-sm font-medium">Claude Code is installed but not signed in</p>
              <p className="text-xs text-muted-foreground mt-1">
                Open your terminal and run:
              </p>
              <code className="mt-2 block rounded bg-muted px-3 py-2 text-xs font-mono">
                claude login
              </code>
            </div>
          </div>
          <Button
            variant="outline"
            className="w-full"
            onClick={checkStatus}
            disabled={checking}
          >
            {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
            Re-check
          </Button>
        </div>
      )}

      {status && !status.installed && (
        <div className="space-y-3">
          <div className="flex items-center gap-3 rounded-lg border border-amber-200 bg-amber-50 p-4 dark:border-amber-800 dark:bg-amber-950/30">
            <Terminal className="size-5 shrink-0 text-amber-600 dark:text-amber-400" />
            <div>
              <p className="text-sm font-medium">Claude Code not found</p>
              <p className="text-xs text-muted-foreground mt-1">
                Install it from your terminal:
              </p>
              <code className="mt-2 block rounded bg-muted px-3 py-2 text-xs font-mono">
                npm install -g @anthropic-ai/claude-code
              </code>
              <p className="text-xs text-muted-foreground mt-2">
                Then run <code className="rounded bg-muted px-1">claude login</code> to sign in.
              </p>
            </div>
          </div>
          <Button
            variant="outline"
            className="w-full"
            onClick={checkStatus}
            disabled={checking}
          >
            {checking && <Loader2 className="mr-2 size-4 animate-spin" />}
            Re-check
          </Button>
        </div>
      )}

      {/* Continue / skip */}
      <div className="flex items-center justify-between">
        {!isReady && status && (
          <button
            className="text-xs text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => onNext(false)}
          >
            Skip — AI features will be limited
          </button>
        )}
        <div className="ml-auto">
          <Button onClick={() => onNext(isReady ?? false)} disabled={checking && !status}>
            Continue
            <ArrowRight className="ml-2 size-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
