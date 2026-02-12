import { useEffect, useState } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Archive, ChevronRight, Loader2, Mail, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import type { Email } from "@/types";

interface EmailListProps {
  emails: Email[];
  maxVisible?: number;
}

export function EmailList({ emails, maxVisible = 3 }: EmailListProps) {
  const [refreshing, setRefreshing] = useState(false);
  const actionable = emails.filter((e) => e.priority === "high" || e.priority === "medium");
  const lowPriority = emails.filter((e) => e.priority === "low");
  const visibleEmails = actionable.slice(0, maxVisible);
  const hiddenCount = actionable.length - visibleEmails.length;

  useEffect(() => {
    const unlisten = listen<string>("email-enrichment-warning", (event) => {
      toast.warning(event.payload, { duration: 6000 });
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  async function handleRefresh() {
    setRefreshing(true);
    try {
      await invoke("refresh_emails");
      toast.success("Emails refreshed");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Email refresh failed");
    } finally {
      setRefreshing(false);
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
