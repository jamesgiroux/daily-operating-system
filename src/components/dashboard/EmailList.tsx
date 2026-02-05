import { Mail, ChevronRight } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { Email } from "@/types";

interface EmailListProps {
  emails: Email[];
  maxVisible?: number;
}

export function EmailList({ emails, maxVisible = 4 }: EmailListProps) {
  const highPriorityCount = emails.filter((e) => e.priority === "high").length;
  const visibleEmails = emails.slice(0, maxVisible);
  const hasMore = emails.length > maxVisible;

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="text-base font-medium">
            Emails - Needs Attention
          </CardTitle>
          {highPriorityCount > 0 && (
            <span className="font-mono text-sm font-light text-muted-foreground">
              {highPriorityCount} high priority
            </span>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {emails.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-6 text-center">
            <Mail className="mb-2 size-8 text-muted-foreground/50" />
            <p className="text-sm text-muted-foreground">No emails needing attention</p>
          </div>
        ) : (
          <div className="space-y-1">
            {visibleEmails.map((email) => (
              <EmailItem key={email.id} email={email} />
            ))}

            {hasMore && (
              <button className="flex w-full items-center justify-center gap-1 py-3 text-sm text-primary hover:text-primary/80 transition-colors">
                View all emails
                <ChevronRight className="size-4" />
              </button>
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
      {/* Priority indicator */}
      <div className="mt-1.5 shrink-0">
        <div
          className={`size-2 rounded-full ${
            email.priority === "high" ? "bg-primary" : "bg-muted-foreground/40"
          }`}
        />
      </div>

      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="font-medium truncate">{email.sender}</span>
          <span className="text-xs text-muted-foreground truncate">
            &lt;{email.senderEmail}&gt;
          </span>
        </div>
        {email.subject && (
          <p className="mt-0.5 text-sm text-muted-foreground truncate">
            {email.subject}
          </p>
        )}
      </div>
    </div>
  );
}
