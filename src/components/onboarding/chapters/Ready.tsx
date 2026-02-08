import { Check, ArrowRight, Inbox, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { GoogleAuthStatus } from "@/types";
import type { InboxProcessingState } from "./InboxTraining";

interface ReadyProps {
  entityMode: string;
  workspacePath: string;
  googleStatus: GoogleAuthStatus;
  claudeCodeInstalled: boolean;
  inboxProcessing?: InboxProcessingState;
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

export function Ready({
  entityMode,
  workspacePath,
  googleStatus,
  claudeCodeInstalled,
  inboxProcessing,
  onComplete,
}: ReadyProps) {
  const isGoogleConnected = googleStatus.status === "authenticated";
  const displayPath = workspacePath.replace(/^\/Users\/[^/]+/, "~");
  const hasInboxResults = inboxProcessing && inboxProcessing.results.length > 0;

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
          <div className="flex items-center gap-2">
            {claudeCodeInstalled ? (
              <>
                <Check className="size-4 text-green-600 shrink-0" />
                <span>Claude Code connected</span>
              </>
            ) : (
              <>
                <span className="size-4 text-center text-muted-foreground shrink-0">—</span>
                <span className="text-muted-foreground">Claude Code not connected</span>
              </>
            )}
          </div>
        </div>
      </div>

      {/* Inbox processing summary */}
      {hasInboxResults && (
        <div className="flex items-start gap-3 rounded-lg border bg-muted/30 p-4 text-left text-sm">
          <Sparkles className="mt-0.5 size-4 shrink-0 text-primary" />
          <div className="space-y-1">
            <p className="text-foreground">
              Processed {inboxProcessing.filesProcessed} file{inboxProcessing.filesProcessed !== 1 ? "s" : ""}
              {inboxProcessing.results
                .filter((r) => r.classification)
                .map((r) => r.classification!.replace(/_/g, " "))
                .length > 0 && (
                <>
                  {" "}— classified as{" "}
                  {[
                    ...new Set(
                      inboxProcessing.results
                        .filter((r) => r.classification)
                        .map((r) => r.classification!.replace(/_/g, " "))
                    ),
                  ].join(", ")}
                </>
              )}
            </p>
            {inboxProcessing.processingInProgress && (
              <p className="text-xs text-muted-foreground">
                AI enrichment is still running — check your Inbox page in a few minutes.
              </p>
            )}
          </div>
        </div>
      )}

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

      {/* Inbox reminder — only show if they didn't already use inbox training */}
      {!hasInboxResults && (
        <div className="flex items-start gap-3 rounded-lg border bg-muted/30 p-4 text-left text-sm">
          <Inbox className="mt-0.5 size-4 shrink-0 text-primary" />
          <p className="text-muted-foreground">
            Drop transcripts, notes, or documents into your{" "}
            <code className="rounded bg-muted px-1 text-xs">_inbox/</code> folder anytime.
            DailyOS processes them automatically.
          </p>
        </div>
      )}

      <Button size="lg" onClick={onComplete}>
        Go to Dashboard
        <ArrowRight className="ml-2 size-4" />
      </Button>
    </div>
  );
}
