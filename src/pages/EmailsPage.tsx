import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { PageError } from "@/components/PageState";
import {
  ArrowLeft,
  Archive,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  Mail,
  RefreshCw,
} from "lucide-react";
import type { Email } from "@/types";

interface EmailsApiResult {
  status: "success" | "not_found" | "error";
  data?: Email[];
  message?: string;
}

export default function EmailsPage() {
  const [emails, setEmails] = useState<Email[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [archivedExpanded, setArchivedExpanded] = useState(false);

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

  const handleScanEmails = useCallback(async () => {
    setScanning(true);
    try {
      await invoke("run_workflow", { workflow: "email_scan" });
      // Refresh after scan completes
      await loadEmails();
    } catch {
      // Workflow may not be registered yet
    } finally {
      setTimeout(() => setScanning(false), 3000);
    }
  }, [loadEmails]);

  const highPriority = emails.filter((e) => e.priority === "high");
  const normalPriority = emails.filter((e) => e.priority !== "high");

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
          <PageHeader
            scanning={scanning}
            onScan={handleScanEmails}
          />

          {emails.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                <Mail className="mb-4 size-12 text-muted-foreground/30" />
                <p className="text-lg font-medium">No emails scanned yet</p>
                <p className="mt-1 text-sm text-muted-foreground">
                  Run an email scan to pull, review, and prioritize your inbox.
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  className="mt-4 gap-1.5"
                  onClick={handleScanEmails}
                  disabled={scanning}
                >
                  <RefreshCw className={cn("size-3.5", scanning && "animate-spin")} />
                  {scanning ? "Scanning..." : "Run email scan"}
                </Button>
              </CardContent>
            </Card>
          ) : highPriority.length === 0 && normalPriority.length === 0 ? (
            <Card>
              <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                <CheckCircle2 className="mb-4 size-12 text-sage" />
                <p className="text-lg font-medium">All clear</p>
                <p className="mt-1 text-sm text-muted-foreground">
                  Nothing needs your attention right now.
                </p>
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-6">
              {/* High priority section */}
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

              {/* Archived / lower priority manifest */}
              {normalPriority.length > 0 && (
                <section>
                  <button
                    onClick={() => setArchivedExpanded(!archivedExpanded)}
                    className="mb-3 flex w-full items-center gap-2 text-sm font-medium text-muted-foreground uppercase tracking-wider hover:text-foreground transition-colors"
                  >
                    <Archive className="size-3.5" />
                    Lower Priority ({normalPriority.length})
                    {archivedExpanded ? (
                      <ChevronDown className="size-3.5" />
                    ) : (
                      <ChevronRight className="size-3.5" />
                    )}
                  </button>

                  {!archivedExpanded && (
                    <p className="text-sm text-muted-foreground">
                      {normalPriority.length} emails reviewed and deprioritized.
                      Click to expand.
                    </p>
                  )}

                  {archivedExpanded && (
                    <div className="space-y-1">
                      {normalPriority.map((email) => (
                        <EmailRow key={email.id} email={email} variant="normal" />
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

function PageHeader({
  scanning,
  onScan,
}: {
  scanning: boolean;
  onScan: () => void;
}) {
  return (
    <div className="mb-6 flex items-start justify-between">
      <div>
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
      <Button
        variant="outline"
        size="sm"
        className="gap-1.5"
        onClick={onScan}
        disabled={scanning}
      >
        <RefreshCw className={cn("size-3.5", scanning && "animate-spin")} />
        {scanning ? "Scanning..." : "Scan emails"}
      </Button>
    </div>
  );
}

function EmailRow({
  email,
  variant,
}: {
  email: Email;
  variant: "high" | "normal";
}) {
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg px-4 py-3 transition-colors",
        variant === "high"
          ? "bg-card border hover:shadow-sm"
          : "hover:bg-muted/50"
      )}
    >
      <div className="mt-1.5 shrink-0">
        <div
          className={cn(
            "size-2 rounded-full",
            variant === "high" ? "bg-primary" : "bg-muted-foreground/30"
          )}
        />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className={cn("truncate", variant === "high" ? "font-medium" : "text-sm")}>
            {email.sender}
          </span>
          {email.senderEmail && variant === "high" && (
            <span className="shrink-0 text-xs text-muted-foreground">
              {email.senderEmail}
            </span>
          )}
        </div>
        {email.subject && (
          <p
            className={cn(
              "mt-0.5 truncate",
              variant === "high"
                ? "text-sm text-muted-foreground"
                : "text-xs text-muted-foreground/70"
            )}
          >
            {email.subject}
          </p>
        )}
        {email.snippet && variant === "high" && (
          <p className="mt-0.5 text-xs text-muted-foreground/60 line-clamp-2">
            {email.snippet}
          </p>
        )}
      </div>
    </div>
  );
}
