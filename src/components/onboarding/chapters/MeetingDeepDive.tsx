import { ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

interface MeetingDeepDiveProps {
  onNext: () => void;
}

export function MeetingDeepDive({ onNext }: MeetingDeepDiveProps) {
  return (
    <div className="space-y-6">
      <div className="space-y-2 text-center">
        <h2 className="text-2xl font-semibold tracking-tight">
          This is what prepared looks like
        </h2>
        <p className="text-sm text-muted-foreground">
          Every meeting gets this automatically. History, context, risks, talking
          points — compiled from your data, past meetings, and AI analysis.
        </p>
      </div>

      {/* Mock expanded meeting card */}
      <div className="rounded-lg border border-l-4 border-l-primary bg-card shadow-sm">
        <div className="p-5 space-y-1">
          <div className="flex items-center gap-2">
            <span className="font-mono text-sm text-muted-foreground">10:30 AM</span>
            <span className="text-muted-foreground/50">—</span>
            <span className="font-mono text-sm text-muted-foreground/70">11:30 AM</span>
          </div>
          <h3 className="font-semibold">Acme Corp Quarterly Sync</h3>
          <p className="text-sm text-primary">Acme Corp</p>
        </div>

        {/* Prep content */}
        <div className="border-t bg-muted/30 p-5 space-y-5">
          {/* Quick Context */}
          <div>
            <h4 className="text-sm font-medium mb-2">At a Glance</h4>
            <div className="flex flex-wrap gap-2">
              <Badge variant="secondary">Enterprise</Badge>
              <Badge variant="secondary">$1.2M ARR</Badge>
              <Badge variant="secondary" className="bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400">
                Health: Green
              </Badge>
              <Badge variant="secondary">Ring 1</Badge>
            </div>
          </div>

          {/* Attendees */}
          <div>
            <h4 className="text-sm font-medium mb-2">Attendees</h4>
            <div className="space-y-1 text-sm text-muted-foreground">
              <p><span className="font-medium text-foreground">Sarah Chen</span> — VP Engineering <span className="text-xs text-primary">(Decision-maker for expansion)</span></p>
              <p><span className="font-medium text-foreground">Marcus Rivera</span> — Director, Platform <span className="text-xs text-primary">(Day-to-day contact)</span></p>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-2">
            {/* Since Last Meeting */}
            <div>
              <h4 className="text-sm font-medium mb-2">Since Last Meeting</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-foreground" />
                  Phase 1 migration completed ahead of schedule
                </li>
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-foreground" />
                  NPS survey deployed — 3 detractors identified
                </li>
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-foreground" />
                  SOW for Phase 2 sent to legal
                </li>
              </ul>
            </div>

            {/* Talking Points */}
            <div>
              <h4 className="text-sm font-medium text-primary mb-2">Talking Points</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" />
                  Celebrate Phase 1 completion — set up case study
                </li>
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" />
                  Address NPS detractor concerns
                </li>
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" />
                  Phase 2 timeline and resource needs
                </li>
              </ul>
            </div>

            {/* Risks */}
            <div>
              <h4 className="text-sm font-medium text-destructive mb-2">Risks</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-destructive" />
                  Key engineer leaving in March — knowledge transfer at risk
                </li>
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-destructive" />
                  NPS trending down — 3 detractors need follow-up
                </li>
              </ul>
            </div>

            {/* Open Items */}
            <div>
              <h4 className="text-sm font-medium mb-2">Open Items</h4>
              <ul className="space-y-1 text-sm text-muted-foreground">
                <li className="flex items-start gap-2">
                  <span className="mt-1 text-xs text-destructive font-medium">OVERDUE</span>
                  Send updated SOW to legal team
                </li>
                <li className="flex items-start gap-2">
                  <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-muted-foreground" />
                  Follow up on NPS survey responses
                </li>
              </ul>
            </div>
          </div>
        </div>
      </div>

      {/* Post-meeting teaser */}
      <div className="rounded-lg border bg-muted/30 p-4 text-sm text-muted-foreground">
        <p>
          <span className="font-medium text-foreground">After a meeting:</span> attach a
          transcript or capture quick outcomes. They feed into the next prep — wins, risks,
          actions, and decisions carry forward automatically.
        </p>
      </div>

      <div className="flex justify-end">
        <Button onClick={onNext}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}
