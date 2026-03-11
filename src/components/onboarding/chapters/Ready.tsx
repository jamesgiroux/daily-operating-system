import { Check, ArrowRight, Inbox, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import type { GoogleAuthStatus } from "@/types";
import type { InboxProcessingState } from "./InboxTraining";
import styles from "../onboarding.module.css";

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
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="Tomorrow morning, your day will be ready."
        epigraph="Everything is set. Here's what we configured."
      />

      {/* Config summary */}
      <div className={styles.configSummary}>
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
        <div className={styles.inboxSummary}>
          <Sparkles size={16} className={`${styles.flexShrink0} ${styles.mt2} ${styles.accentColor}`} />
          <div className={`${styles.flexCol} ${styles.gap4}`}>
            <p className={styles.resultText}>
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
              <p className={styles.backgroundHint}>
                AI analysis is still running — check your Inbox page in a few minutes.
              </p>
            )}
          </div>
        </div>
      )}

      {/* What happens next */}
      <div className={`${styles.ruleSection} ${styles.nextStepSection}`}>
        <p className={`${styles.secondaryText} ${styles.noMargin}`}>
          Your first real briefing generates at{" "}
          <span className={styles.accentText}>6:00 AM</span> tomorrow.
          Your meetings will have context from the accounts and projects you just added.
          Each day, the system learns more — prep gets richer, patterns sharpen, nothing falls through.
        </p>

        {!isGoogleConnected && (
          <p className={`${styles.secondaryText} ${styles.nextStepReminder}`}>
            Connect Google anytime from Settings to unlock calendar prep and email triage.
          </p>
        )}
      </div>

      {/* Inbox reminder — only show if they didn't already use inbox training */}
      {!hasInboxResults && (
        <div className={styles.inboxReminder}>
          <Inbox size={16} className={`${styles.flexShrink0} ${styles.mt2} ${styles.tertiaryText}`} />
          <p className={styles.inboxReminderText}>
            Drop transcripts, notes, or documents into your{" "}
            <span className={styles.inboxFolder}>
              _inbox/
            </span>{" "}
            folder anytime. DailyOS processes them automatically.
          </p>
        </div>
      )}

      <div className={styles.ctaCenter}>
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
    <div className={styles.configRow}>
      {done ? (
        <Check size={14} className={`${styles.flexShrink0} ${styles.sageColor}`} />
      ) : (
        <span className={styles.configDash}>
          —
        </span>
      )}
      <span className={`${styles.configLabel} ${done ? styles.configLabelDone : styles.configLabelPending}`}>
        {label}
      </span>
    </div>
  );
}
