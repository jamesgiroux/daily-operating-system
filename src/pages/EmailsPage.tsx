import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import { PageError, SectionEmpty } from "@/components/PageState";
import {
  ArrowLeft,
  Archive,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  Loader2,
  Mail,
} from "lucide-react";
import type { Email } from "@/types";

interface EmailsApiResult {
  status: "success" | "not_found" | "error";
  data?: Email[];
  message?: string;
}

export default function EmailsPage() {
  const { personality } = usePersonality();
  const [emails, setEmails] = useState<Email[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [archivedExpanded, setArchivedExpanded] = useState(false);
  const [archiving, setArchiving] = useState(false);
  const [confirmArchive, setConfirmArchive] = useState(false);

  const loadEmails = useCallback(async () => {
    try {
      const result = await invoke<EmailsApiResult>("get_all_emails");
      if (result.status === "success" && result.data) {
        setEmails(result.data);
      } else if (result.status === "not_found") {
        setEmails([]);
      } else if (result.status === "error") {
        setError(result.message || "Failed to load emails");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadEmails();
  }, [loadEmails]);

  const highPriority = emails.filter((e) => e.priority === "high");
  const mediumPriority = emails.filter((e) => e.priority === "medium");
  const lowPriority = emails.filter((e) => e.priority === "low");

  async function handleArchiveLow() {
    setArchiving(true);
    try {
      await invoke<number>("archive_low_priority_emails");
      // Optimistic removal from local state
      setEmails((prev) => prev.filter((e) => e.priority !== "low"));
      setConfirmArchive(false);
      setArchivedExpanded(false);

    } catch (err) {
      console.error("Archive failed:", err);
    } finally {
      setArchiving(false);
    }
  }

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-3">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-20 w-full" />
          ))}
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} onRetry={loadEmails} />
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <PageHeader />

          {emails.length === 0 ? (
            <SectionEmpty icon={Mail} {...getPersonalityCopy("emails-empty", personality)} />
          ) : highPriority.length === 0 && mediumPriority.length === 0 && lowPriority.length === 0 ? (
            <SectionEmpty icon={CheckCircle2} {...getPersonalityCopy("emails-clear", personality)} />
          ) : (
            <div className="space-y-6">
              {/* High priority — Needs Attention */}
              {highPriority.length > 0 && (
                <section>
                  <h2 className="mb-3 text-sm font-medium text-muted-foreground uppercase tracking-wider">
                    Needs Attention ({highPriority.length})
                  </h2>
                  <div className="space-y-2">
                    {highPriority.map((email) => (
                      <EmailRow key={email.id} email={email} variant="high" />
                    ))}
                  </div>
                </section>
              )}

              {/* Medium priority — Worth a Look */}
              {mediumPriority.length > 0 && (
                <section>
                  <h2 className="mb-3 text-sm font-medium text-muted-foreground uppercase tracking-wider">
                    Worth a Look ({mediumPriority.length})
                  </h2>
                  <div className="space-y-1">
                    {mediumPriority.map((email) => (
                      <EmailRow key={email.id} email={email} variant="medium" />
                    ))}
                  </div>
                </section>
              )}

              {/* Low priority — FYI */}
              {lowPriority.length > 0 && (
                <section>
                  <div className="mb-3 flex items-center gap-2">
                    <button
                      onClick={() => setArchivedExpanded(!archivedExpanded)}
                      className="flex items-center gap-2 text-sm font-medium text-muted-foreground uppercase tracking-wider hover:text-foreground transition-colors"
                    >
                      <Archive className="size-3.5" />
                      FYI ({lowPriority.length})
                      {archivedExpanded ? (
                        <ChevronDown className="size-3.5" />
                      ) : (
                        <ChevronRight className="size-3.5" />
                      )}
                    </button>

                    <div className="ml-auto flex items-center gap-2">
                      {confirmArchive ? (
                        <>
                          <span className="text-xs text-muted-foreground">
                            Archive {lowPriority.length} in Gmail?
                          </span>
                          <Button
                            variant="default"
                            size="sm"
                            className="h-7 text-xs"
                            onClick={handleArchiveLow}
                            disabled={archiving}
                          >
                            {archiving ? (
                              <Loader2 className="mr-1.5 size-3 animate-spin" />
                            ) : (
                              <Archive className="mr-1.5 size-3" />
                            )}
                            {archiving ? "Archiving..." : "Confirm"}
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 text-xs"
                            onClick={() => setConfirmArchive(false)}
                            disabled={archiving}
                          >
                            Cancel
                          </Button>
                        </>
                      ) : (
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 text-xs text-muted-foreground"
                          onClick={() => setConfirmArchive(true)}
                        >
                          <Archive className="mr-1.5 size-3" />
                          Archive all
                        </Button>
                      )}
                    </div>
                  </div>

                  {!archivedExpanded && !confirmArchive && (
                    <p className="text-sm text-muted-foreground">
                      {lowPriority.length} emails reviewed and deprioritized.
                      Click to expand.
                    </p>
                  )}

                  {archivedExpanded && (
                    <div className="space-y-1">
                      {lowPriority.map((email) => (
                        <EmailRow key={email.id} email={email} variant="low" />
                      ))}
                    </div>
                  )}
                </section>
              )}
            </div>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

function PageHeader() {
  return (
    <div className="mb-6">
      <div className="flex items-center gap-3">
        <Link
          to="/"
          className="text-muted-foreground hover:text-foreground transition-colors"
        >
          <ArrowLeft className="size-5" />
        </Link>
        <h1 className="text-2xl font-semibold tracking-tight">Emails</h1>
      </div>
      <p className="mt-1 ml-8 text-sm text-muted-foreground">
        AI-triaged email intelligence
      </p>
    </div>
  );
}

function EmailRow({
  email,
  variant,
}: {
  email: Email;
  variant: "high" | "medium" | "low";
}) {
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg px-4 py-3 transition-colors",
        variant === "high"
          ? "bg-card border hover:shadow-sm"
          : variant === "medium"
            ? "bg-card/50 border border-border/50 hover:shadow-sm"
            : "hover:bg-muted/50"
      )}
    >
      <div className="mt-1.5 shrink-0">
        <div
          className={cn(
            "size-2 rounded-full",
            variant === "high"
              ? "bg-primary"
              : variant === "medium"
                ? "bg-primary/40"
                : "bg-muted-foreground/30"
          )}
        />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className={cn("truncate", variant === "high" ? "font-medium" : "text-sm")}>
            {email.sender}
          </span>
          {email.senderEmail && variant !== "low" && (
            <span className="shrink-0 text-xs text-muted-foreground">
              {email.senderEmail}
            </span>
          )}
        </div>
        {email.subject && (
          <p
            className={cn(
              "mt-0.5 truncate",
              variant === "low"
                ? "text-xs text-muted-foreground/70"
                : "text-sm text-muted-foreground"
            )}
          >
            {email.subject}
          </p>
        )}

        {/* AI enrichment context — shown for high and medium */}
        {email.summary && variant !== "low" && (
          <p className="mt-1 text-xs text-muted-foreground/80 line-clamp-2">
            {email.summary}
          </p>
        )}
        {email.recommendedAction && variant === "high" && (
          <p className="mt-1 text-xs font-medium text-primary/80">
            → {email.recommendedAction}
          </p>
        )}
        {email.conversationArc && variant === "high" && (
          <p className="mt-0.5 text-xs text-muted-foreground/50 italic">
            {email.conversationArc}
          </p>
        )}

        {/* Fallback to snippet when no enrichment */}
        {!email.summary && email.snippet && variant !== "low" && (
          <p className="mt-0.5 text-xs text-muted-foreground/60 line-clamp-2">
            {email.snippet}
          </p>
        )}
      </div>
    </div>
  );
}
