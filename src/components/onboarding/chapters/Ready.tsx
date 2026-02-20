import { Check, ArrowRight, Inbox, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
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
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="Tomorrow morning, your day will be ready."
        epigraph="Everything is set. Here's what we configured."
      />

      {/* Config summary */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
          display: "flex",
          flexDirection: "column",
          gap: 12,
        }}
      >
        <ConfigRow
          done
          label={`${entityModeLabel(entityMode)} workspace`}
        />
        <ConfigRow done label={displayPath} />
        <ConfigRow
          done={isGoogleConnected}
          label={isGoogleConnected ? "Google connected" : "Google not connected"}
        />
        <ConfigRow
          done={claudeCodeInstalled}
          label={claudeCodeInstalled ? "Claude Code connected" : "Claude Code not connected"}
        />
      </div>

      {/* Inbox processing summary */}
      {hasInboxResults && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-light)",
            paddingTop: 20,
            display: "flex",
            alignItems: "flex-start",
            gap: 12,
          }}
        >
          <Sparkles size={16} style={{ flexShrink: 0, marginTop: 2, color: "var(--color-spice-turmeric)" }} />
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <p style={{ fontSize: 14, color: "var(--color-text-primary)", margin: 0 }}>
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
              <p style={{ fontSize: 12, color: "var(--color-text-tertiary)", margin: 0 }}>
                AI analysis is still running — check your Inbox page in a few minutes.
              </p>
            )}
          </div>
        </div>
      )}

      {/* What happens next */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
          fontSize: 14,
          lineHeight: 1.6,
          color: "var(--color-text-secondary)",
        }}
      >
        <p style={{ margin: 0 }}>
          Your first real briefing generates at{" "}
          <span style={{ fontWeight: 500, color: "var(--color-text-primary)" }}>6:00 AM</span> tomorrow.
          Your meetings will have context from the accounts and projects you just added.
          Each day, the system learns more — prep gets richer, patterns sharpen, nothing falls through.
        </p>

        {!isGoogleConnected && (
          <p style={{ marginTop: 12, marginBottom: 0 }}>
            Connect Google anytime from Settings to unlock calendar prep and email triage.
          </p>
        )}
      </div>

      {/* Inbox reminder — only show if they didn't already use inbox training */}
      {!hasInboxResults && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-light)",
            paddingTop: 20,
            display: "flex",
            alignItems: "flex-start",
            gap: 12,
          }}
        >
          <Inbox size={16} style={{ flexShrink: 0, marginTop: 2, color: "var(--color-text-tertiary)" }} />
          <p style={{ fontSize: 14, color: "var(--color-text-secondary)", margin: 0 }}>
            Drop transcripts, notes, or documents into your{" "}
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
              }}
            >
              _inbox/
            </span>{" "}
            folder anytime. DailyOS processes them automatically.
          </p>
        </div>
      )}

      <div style={{ display: "flex", justifyContent: "center", paddingTop: 8 }}>
        <Button size="lg" onClick={onComplete}>
          Go to Dashboard
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>

      <FinisMarker />
    </div>
  );
}

function ConfigRow({ done, label }: { done: boolean; label: string }) {
  return (
    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
      {done ? (
        <Check size={14} style={{ flexShrink: 0, color: "var(--color-garden-sage)" }} />
      ) : (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 14,
            color: "var(--color-text-tertiary)",
            flexShrink: 0,
            width: 14,
            textAlign: "center",
          }}
        >
          —
        </span>
      )}
      <span
        style={{
          fontSize: 14,
          color: done ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {label}
      </span>
    </div>
  );
}
