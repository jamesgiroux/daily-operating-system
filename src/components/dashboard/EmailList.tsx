import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { AlertCircle, Archive, ChevronRight, Loader2, Mail, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import type { Email, EmailSyncStatus } from "@/types";

interface EmailListProps {
  emails: Email[];
  emailSync?: EmailSyncStatus;
  maxVisible?: number;
}

export function EmailList({ emails, emailSync, maxVisible = 3 }: EmailListProps) {
  const [refreshing, setRefreshing] = useState(false);
  const [syncStatus, setSyncStatus] = useState<EmailSyncStatus | null>(emailSync ?? null);
  const actionable = emails.filter((e) => e.priority === "high" || e.priority === "medium");
  const lowPriority = emails.filter((e) => e.priority === "low");
  const visibleEmails = actionable.slice(0, maxVisible);
  const hiddenCount = actionable.length - visibleEmails.length;
  const showSyncBanner = Boolean(syncStatus && syncStatus.state !== "ok");

  useEffect(() => {
    if (emailSync) {
      setSyncStatus(emailSync);
    }
  }, [emailSync]);

  useEffect(() => {
    const unlistenSync = listen<EmailSyncStatus>("email-sync-status", (event) => {
      setSyncStatus(event.payload);
    });
    const unlistenError = listen<string>("email-error", (event) => {
      setSyncStatus({
        state: "error",
        stage: "deliver",
        code: "legacy_email_error",
        message: event.payload,
        usingLastKnownGood: emails.length > 0,
        canRetry: true,
        lastAttemptAt: new Date().toISOString(),
      });
    });
    const unlistenWarning = listen<string>("email-enrichment-warning", (event) => {
      setSyncStatus({
        state: "warning",
        stage: "enrich",
        code: "legacy_email_enrichment_warning",
        message: event.payload,
        usingLastKnownGood: true,
        canRetry: true,
        lastAttemptAt: new Date().toISOString(),
      });
    });
    return () => {
      unlistenSync.then((fn) => fn());
      unlistenError.then((fn) => fn());
      unlistenWarning.then((fn) => fn());
    };
  }, [emails.length]);

  async function handleRefresh() {
    setRefreshing(true);
    // Track whether a backend event updates status during the refresh.
    // If it does, that event is authoritative — don't overwrite it.
    let eventFiredDuringRefresh = false;
    const unlistenRefreshWarning = listen<string>("email-enrichment-warning", () => {
      eventFiredDuringRefresh = true;
    });
    const unlistenRefreshSync = listen<EmailSyncStatus>("email-sync-status", () => {
      eventFiredDuringRefresh = true;
    });
    try {
      await invoke("refresh_emails");
      if (!eventFiredDuringRefresh) {
        setSyncStatus(null);
        toast.success("Emails refreshed");
      }
    } catch (err) {
      setSyncStatus({
        state: "error",
        stage: "refresh",
        code: "manual_refresh_failed",
        message: normalizeErrorMessage(err),
        usingLastKnownGood: emails.length > 0,
        canRetry: true,
        lastAttemptAt: new Date().toISOString(),
      });
    } finally {
      setRefreshing(false);
      unlistenRefreshWarning.then((fn) => fn());
      unlistenRefreshSync.then((fn) => fn());
    }
  }

  return (
    <section>
      <div className="flex items-center justify-between mb-3">
        <h3 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Emails
        </h3>
        <Button
          variant="ghost"
          size="icon"
          className="size-7"
          onClick={handleRefresh}
          disabled={refreshing}
          title="Refresh emails"
        >
          {refreshing ? (
            <Loader2 className="size-3.5 animate-spin" />
          ) : (
            <RefreshCw className="size-3.5" />
          )}
        </Button>
      </div>
      {showSyncBanner && syncStatus && (
        <div className="mb-3 rounded-lg border border-border/70 bg-muted/30 px-3 py-2.5">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="flex items-center gap-1.5">
                <AlertCircle className="size-3.5 text-foreground" />
                <p className="text-xs font-semibold text-foreground">
                  {syncStatus.state === "error" ? "Email Sync Issue" : "Email Enrichment Limited"}
                </p>
              </div>
              <p className="mt-1 text-xs leading-relaxed text-foreground">
                {syncStatus.message || defaultSyncMessage(syncStatus)}
              </p>
            </div>
            {syncStatus.canRetry !== false && (
              <Button
                variant="outline"
                size="sm"
                className="h-7 shrink-0 text-xs"
                onClick={handleRefresh}
                disabled={refreshing}
              >
                {refreshing ? (
                  <Loader2 className="mr-1 size-3 animate-spin" />
                ) : null}
                Retry
              </Button>
            )}
          </div>
        </div>
      )}
      {actionable.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-6 text-center">
          <Mail className="mb-2 size-8 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">
            {emails.length === 0
              ? "No email data yet"
              : "Nothing needs attention"}
          </p>
        </div>
      ) : (
        <div className="space-y-1">
          {visibleEmails.map((email) => (
            <EmailItem key={email.id} email={email} />
          ))}

          {hiddenCount > 0 && (
            <Link
              to="/emails"
              className="flex items-center justify-center gap-1 py-2 text-xs text-primary hover:text-primary/80 transition-colors"
            >
              +{hiddenCount} more
              <ChevronRight className="size-3" />
            </Link>
          )}
        </div>
      )}

      {lowPriority.length > 0 && (
        <Link
          to="/emails"
          className="flex items-center justify-center gap-1.5 pt-2 text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <Archive className="size-3" />
          {lowPriority.length} lower priority reviewed
          <ChevronRight className="size-3" />
        </Link>
      )}
    </section>
  );
}

function EmailItem({ email }: { email: Email }) {
  return (
    <div className="flex items-start gap-3 rounded-lg p-3 transition-colors hover:bg-muted/50">
      <div className="mt-1.5 shrink-0">
        <div className="size-2 rounded-full bg-primary" />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="font-medium truncate">{email.sender}</span>
        </div>
        {email.subject && (
          <p className="mt-0.5 text-sm text-muted-foreground truncate">
            {email.subject}
          </p>
        )}
        {email.recommendedAction ? (
          <p className="mt-0.5 text-xs font-medium text-primary/80 truncate">
            → {email.recommendedAction}
          </p>
        ) : email.summary ? (
          <p className="mt-0.5 text-xs text-muted-foreground/60 truncate">
            {email.summary}
          </p>
        ) : email.snippet ? (
          <p className="mt-0.5 text-xs text-muted-foreground/60 truncate">
            {email.snippet}
          </p>
        ) : null}
      </div>
    </div>
  );
}

function normalizeErrorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return "Email refresh failed";
}

function defaultSyncMessage(status: EmailSyncStatus): string {
  if (status.stage === "enrich") {
    return "Email summaries are unavailable right now. Core email triage data is still available.";
  }
  if (status.stage === "fetch" || status.stage === "deliver" || status.stage === "refresh") {
    return "Email sync failed. Retry to restore the latest inbox triage.";
  }
  return "Email sync is currently degraded.";
}
