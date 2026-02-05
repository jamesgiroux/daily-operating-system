import * as React from "react";
import { ChevronDown } from "lucide-react";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Badge } from "@/components/ui/badge";
import type { Meeting, MeetingType } from "@/types";
import { cn } from "@/lib/utils";

interface MeetingCardProps {
  meeting: Meeting;
}

const borderStyles: Record<MeetingType, string> = {
  customer: "border-l-4 border-l-primary",
  internal: "border-l-4 border-l-muted-foreground/50",
  personal: "border-l-4 border-l-success",
};

const badgeStyles: Record<MeetingType, string> = {
  customer: "bg-primary/15 text-primary hover:bg-primary/20",
  internal: "bg-muted text-muted-foreground hover:bg-muted",
  personal: "bg-success/15 text-success hover:bg-success/20",
};

const badgeLabels: Record<MeetingType, string> = {
  customer: "Customer",
  internal: "Internal",
  personal: "Personal",
};

export function MeetingCard({ meeting }: MeetingCardProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const hasPrep = meeting.prep && Object.keys(meeting.prep).length > 0;

  return (
    <div
      className={cn(
        "rounded-lg border bg-card shadow-sm transition-all duration-150",
        "hover:-translate-y-0.5 hover:shadow-md",
        borderStyles[meeting.type],
        meeting.isCurrent && "animate-pulse-gold ring-2 ring-primary/50"
      )}
    >
      <Collapsible open={isOpen} onOpenChange={setIsOpen}>
        <div className="flex items-start justify-between p-5">
          <div className="space-y-1">
            <div className="flex items-center gap-2">
              <span className="font-mono text-sm text-muted-foreground">
                {meeting.time}
              </span>
              {meeting.endTime && (
                <>
                  <span className="text-muted-foreground/50">—</span>
                  <span className="font-mono text-sm text-muted-foreground/70">
                    {meeting.endTime}
                  </span>
                </>
              )}
            </div>
            <h3 className="font-semibold">{meeting.title}</h3>
            {meeting.account && (
              <p className="text-sm text-primary">{meeting.account}</p>
            )}
          </div>

          <div className="flex items-center gap-2">
            <Badge className={badgeStyles[meeting.type]} variant="secondary">
              {badgeLabels[meeting.type]}
            </Badge>
            {hasPrep && (
              <CollapsibleTrigger asChild>
                <button
                  className={cn(
                    "rounded-md p-1.5 transition-colors hover:bg-muted",
                    isOpen && "bg-muted"
                  )}
                >
                  <ChevronDown
                    className={cn(
                      "size-4 text-muted-foreground transition-transform duration-200",
                      isOpen && "rotate-180"
                    )}
                  />
                </button>
              </CollapsibleTrigger>
            )}
          </div>
        </div>

        {hasPrep && (
          <CollapsibleContent>
            <div className="border-t bg-muted/30 p-5">
              <MeetingPrepContent prep={meeting.prep!} />
            </div>
          </CollapsibleContent>
        )}
      </Collapsible>
    </div>
  );
}

function MeetingPrepContent({ prep }: { prep: NonNullable<Meeting["prep"]> }) {
  return (
    <div className="space-y-4 text-sm">
      {prep.context && (
        <p className="text-muted-foreground">{prep.context}</p>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        {prep.metrics && prep.metrics.length > 0 && (
          <PrepSection title="Metrics" items={prep.metrics} color="text-foreground" />
        )}
        {prep.risks && prep.risks.length > 0 && (
          <PrepSection title="Risks" items={prep.risks} color="text-destructive" />
        )}
        {prep.wins && prep.wins.length > 0 && (
          <PrepSection title="Wins" items={prep.wins} color="text-success" />
        )}
        {prep.actions && prep.actions.length > 0 && (
          <PrepSection title="Actions" items={prep.actions} color="text-primary" />
        )}
      </div>
    </div>
  );
}

function PrepSection({
  title,
  items,
  color,
}: {
  title: string;
  items: string[];
  color: string;
}) {
  return (
    <div className="space-y-1.5">
      <h4 className={cn("font-medium", color)}>{title}</h4>
      <ul className="space-y-1">
        {items.map((item, i) => (
          <li key={i} className="flex items-start gap-2 text-muted-foreground">
            <span className={cn("mt-1.5 size-1.5 shrink-0 rounded-full", color.replace("text-", "bg-"))} />
            {item}
          </li>
        ))}
      </ul>
    </div>
  );
}
