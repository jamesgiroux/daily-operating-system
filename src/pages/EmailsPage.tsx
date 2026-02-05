import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import type { EmailDetail, EmailSummaryData } from "@/types";
import { cn } from "@/lib/utils";
import { AlertCircle, Mail, ArrowRight, CheckCircle2 } from "lucide-react";

interface EmailsResult {
  status: "success" | "not_found" | "error";
  data?: EmailSummaryData;
  message?: string;
}

export default function EmailsPage() {
  const [data, setData] = useState<EmailSummaryData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    async function loadEmails() {
      try {
        const result = await invoke<EmailsResult>("get_all_emails");
        if (result.status === "success" && result.data) {
          setData(result.data);
        } else if (result.status === "not_found") {
          setData(null);
        } else if (result.status === "error") {
          setError(result.message || "Failed to load emails");
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : "Unknown error");
      } finally {
        setLoading(false);
      }
    }
    loadEmails();
  }, []);

  if (loading) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-32 w-full" />
          ))}
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
          </CardContent>
        </Card>
      </main>
    );
  }

  if (!data || (data.highPriority.length === 0 && (!data.mediumPriority || data.mediumPriority.length === 0))) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6">
          <h1 className="text-2xl font-semibold tracking-tight">Emails</h1>
          <p className="text-sm text-muted-foreground">
            Emails needing attention with context
          </p>
        </div>
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-12 text-center">
            <CheckCircle2 className="mb-4 size-12 text-success" />
            <p className="text-lg font-medium">Inbox Zero!</p>
            <p className="text-sm text-muted-foreground">
              No emails need your attention right now.
            </p>
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
            <h1 className="text-2xl font-semibold tracking-tight">Emails</h1>
            <p className="text-sm text-muted-foreground">
              Emails needing attention with context and recommended actions
            </p>
          </div>

          {/* Stats summary */}
          <div className="mb-6 flex gap-4">
            <div className="flex items-center gap-2">
              <Badge variant="destructive">{data.stats.highCount}</Badge>
              <span className="text-sm text-muted-foreground">High Priority</span>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="secondary">{data.stats.mediumCount}</Badge>
              <span className="text-sm text-muted-foreground">Medium</span>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline">{data.stats.lowCount}</Badge>
              <span className="text-sm text-muted-foreground">Low</span>
            </div>
          </div>

          {/* High priority emails */}
          {data.highPriority.length > 0 && (
            <section className="mb-8">
              <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold text-destructive">
                <AlertCircle className="size-5" />
                High Priority
              </h2>
              <div className="space-y-4">
                {data.highPriority.map((email) => (
                  <EmailCard key={email.id} email={email} />
                ))}
              </div>
            </section>
          )}

          {/* Medium priority emails */}
          {data.mediumPriority && data.mediumPriority.length > 0 && (
            <section>
              <h2 className="mb-4 flex items-center gap-2 text-lg font-semibold">
                <Mail className="size-5" />
                Notable
              </h2>
              <div className="space-y-4">
                {data.mediumPriority.map((email) => (
                  <EmailCard key={email.id} email={email} />
                ))}
              </div>
            </section>
          )}
        </div>
      </ScrollArea>
    </main>
  );
}

function EmailCard({ email }: { email: EmailDetail }) {
  return (
    <Card
      className={cn(
        "transition-all hover:-translate-y-0.5 hover:shadow-md",
        email.priority === "high" && "border-l-4 border-l-destructive"
      )}
    >
      <CardContent className="p-5">
        <div className="space-y-3">
          {/* Header */}
          <div className="flex items-start justify-between gap-4">
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <span className="font-medium">{email.sender}</span>
                {email.emailType && (
                  <Badge variant="outline" className="text-xs">
                    {email.emailType}
                  </Badge>
                )}
              </div>
              <p className="text-sm text-muted-foreground">{email.senderEmail}</p>
            </div>
            {email.received && (
              <span className="text-xs text-muted-foreground">{email.received}</span>
            )}
          </div>

          {/* Subject */}
          <p className="font-medium">{email.subject}</p>

          {/* Summary */}
          {email.summary && (
            <p className="text-sm text-muted-foreground">{email.summary}</p>
          )}

          {/* Conversation arc */}
          {email.conversationArc && (
            <div className="rounded-md bg-muted/50 p-3">
              <p className="text-xs font-medium text-muted-foreground mb-1">
                Conversation Arc:
              </p>
              <p className="text-sm">{email.conversationArc}</p>
            </div>
          )}

          {/* Recommended action */}
          {email.recommendedAction && (
            <div className="flex items-start gap-2 rounded-md bg-primary/5 p-3">
              <ArrowRight className="mt-0.5 size-4 text-primary" />
              <div className="flex-1">
                <p className="text-sm font-medium text-primary">
                  Recommended Action:
                </p>
                <p className="text-sm">{email.recommendedAction}</p>
                {email.actionOwner && (
                  <p className="mt-1 text-xs text-muted-foreground">
                    Owner: {email.actionOwner}
                    {email.actionPriority && ` • ${email.actionPriority}`}
                  </p>
                )}
              </div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
