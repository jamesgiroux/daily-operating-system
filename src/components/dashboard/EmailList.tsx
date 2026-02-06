import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { Archive, ChevronRight, Mail, RefreshCw } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { Email } from "@/types";

interface EmailListProps {
  emails: Email[];
  maxVisible?: number;
}

export function EmailList({ emails, maxVisible = 3 }: EmailListProps) {
  const [scanning, setScanning] = useState(false);

  const highPriority = emails.filter((e) => e.priority === "high");
  const normalPriority = emails.filter((e) => e.priority !== "high");
  const visibleEmails = highPriority.slice(0, maxVisible);
  const hiddenCount = highPriority.length - visibleEmails.length;

  const handleScanEmails = useCallback(async () => {
    setScanning(true);
    try {
      await invoke("run_workflow", { workflow: "email_scan" });
    } catch {
      // Workflow may not be registered yet — silent for now
    } finally {
      // Keep spinner for a few seconds so user sees feedback
      setTimeout(() => setScanning(false), 3000);
    }
  }, []);

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base font-medium">
            Emails
          </CardTitle>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 gap-1.5 text-xs text-muted-foreground"
            onClick={handleScanEmails}
            disabled={scanning}
          >
            <RefreshCw className={cn("size-3", scanning && "animate-spin")} />
            {scanning ? "Scanning..." : "Scan"}
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {highPriority.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-6 text-center">
            <Mail className="mb-2 size-8 text-muted-foreground/50" />
            <p className="text-sm text-muted-foreground">
              {emails.length === 0
                ? "No emails scanned yet"
                : "Nothing needs attention"}
            </p>
            {emails.length === 0 && (
              <Button
                variant="ghost"
                size="sm"
                className="mt-2 h-7 text-xs"
                onClick={handleScanEmails}
                disabled={scanning}
              >
                {scanning ? "Scanning..." : "Run email scan"}
              </Button>
            )}
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
                +{hiddenCount} more high priority
                <ChevronRight className="size-3" />
              </Link>
            )}

            {normalPriority.length > 0 && (
              <Link
                to="/emails"
                className="flex items-center justify-center gap-1.5 pt-2 text-xs text-muted-foreground hover:text-foreground transition-colors"
              >
                <Archive className="size-3" />
                {normalPriority.length} lower priority reviewed
                <ChevronRight className="size-3" />
              </Link>
            )}
          </div>
        )}
      </CardContent>
    </Card>
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
        {email.snippet && (
          <p className="mt-0.5 text-xs text-muted-foreground/60 truncate">
            {email.snippet}
          </p>
        )}
      </div>
    </div>
  );
}
