import { Check, ArrowRight, Inbox } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { GoogleAuthStatus } from "@/types";

interface ReadyProps {
  entityMode: string;
  workspacePath: string;
  googleStatus: GoogleAuthStatus;
  onComplete: () => void;
}

function entityModeLabel(mode: string): string {
  switch (mode) {
    case "account": return "Account-based";
    case "project": return "Project-based";
    case "both": return "Both modes";
    default: return mode;
  }
}

export function Ready({ entityMode, workspacePath, googleStatus, onComplete }: ReadyProps) {
  const isGoogleConnected = googleStatus.status === "authenticated";
  const displayPath = workspacePath.replace(/^\/Users\/[^/]+/, "~");

  return (
    <div className="space-y-6 text-center">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">
          Tomorrow morning, your day will be ready.
        </h2>
      </div>

      {/* Config summary */}
      <div className="rounded-lg border bg-muted/30 p-4 text-left text-sm">
        <div className="space-y-2.5">
          <div className="flex items-center gap-2">
            <Check className="size-4 text-green-600 shrink-0" />
            <span>{entityModeLabel(entityMode)} workspace</span>
          </div>
          <div className="flex items-center gap-2">
            <Check className="size-4 text-green-600 shrink-0" />
            <span className="truncate">{displayPath}</span>
          </div>
          <div className="flex items-center gap-2">
            {isGoogleConnected ? (
              <>
                <Check className="size-4 text-green-600 shrink-0" />
                <span>Google connected</span>
              </>
            ) : (
              <>
                <span className="size-4 text-center text-muted-foreground shrink-0">—</span>
                <span className="text-muted-foreground">Google not connected</span>
              </>
            )}
          </div>
        </div>
      </div>

      {/* What happens next */}
      <div className="space-y-3 text-left text-sm text-muted-foreground">
        <p>
          Your first real briefing generates at{" "}
          <span className="font-medium text-foreground">6:00 AM</span> tomorrow.
          Your meetings will have context from the accounts and projects you just added.
          Each day, the system learns more — prep gets richer, patterns sharpen, nothing falls through.
        </p>

        {!isGoogleConnected && (
          <p>
            Connect Google anytime from Settings to unlock calendar prep and email triage.
          </p>
        )}
      </div>

      {/* Inbox reminder */}
      <div className="flex items-start gap-3 rounded-lg border bg-muted/30 p-4 text-left text-sm">
        <Inbox className="mt-0.5 size-4 shrink-0 text-primary" />
        <p className="text-muted-foreground">
          Drop transcripts, notes, or documents into your{" "}
          <code className="rounded bg-muted px-1 text-xs">_inbox/</code> folder anytime.
          DailyOS processes them automatically.
        </p>
      </div>

      <Button size="lg" onClick={onComplete}>
        Go to Dashboard
        <ArrowRight className="ml-2 size-4" />
      </Button>
    </div>
  );
}
