import { useState } from "react";
import {
  AlertTriangle,
  ChevronDown,
  Clock,
  Lightbulb,
  Scale,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { useExecutiveIntelligence } from "@/hooks/useExecutiveIntelligence";
import type {
  DecisionSignal,
  DelegationSignal,
  PortfolioAlert,
  SkipSignal,
} from "@/types";

export function IntelligenceCard() {
  const { data, totalSignals } = useExecutiveIntelligence();

  // Don't render if no data or no signals
  if (!data || totalSignals === 0) return null;

  return (
    <Card className="animate-fade-in-up opacity-0">
      <CardContent className="pt-5 pb-4">
        {/* Compact badge row */}
        <div className="flex flex-wrap items-center gap-2">
          {data.signalCounts.decisions > 0 && (
            <SignalBadge
              icon={<Scale className="size-3" />}
              count={data.signalCounts.decisions}
              label="decision"
              className="bg-primary/15 text-primary border-primary/20"
            />
          )}
          {data.signalCounts.delegations > 0 && (
            <SignalBadge
              icon={<Clock className="size-3" />}
              count={data.signalCounts.delegations}
              label="stale delegation"
              className="bg-destructive/15 text-destructive border-destructive/20"
            />
          )}
          {data.signalCounts.portfolioAlerts > 0 && (
            <SignalBadge
              icon={<AlertTriangle className="size-3" />}
              count={data.signalCounts.portfolioAlerts}
              label="portfolio alert"
              className="bg-destructive/15 text-destructive border-destructive/20"
            />
          )}
          {data.signalCounts.skipToday > 0 && (
            <SignalBadge
              icon={<Lightbulb className="size-3" />}
              count={data.signalCounts.skipToday}
              label="skip today"
              className="bg-success/15 text-success border-success/20"
            />
          )}
        </div>

        {/* Expandable sections */}
        <div className="mt-3 space-y-1">
          {data.decisions.length > 0 && (
            <SignalSection title="Decisions Due" icon={<Scale className="size-4 text-primary" />}>
              {data.decisions.map((d) => (
                <DecisionRow key={d.actionId} signal={d} />
              ))}
            </SignalSection>
          )}

          {data.delegations.length > 0 && (
            <SignalSection title="Stale Delegations" icon={<Clock className="size-4 text-destructive" />}>
              <div className="space-y-1">
                {data.delegations.map((d) => (
                  <DelegationRow key={d.actionId} signal={d} />
                ))}
              </div>
            </SignalSection>
          )}

          {data.portfolioAlerts.length > 0 && (
            <SignalSection title="Portfolio Alerts" icon={<AlertTriangle className="size-4 text-destructive" />}>
              {data.portfolioAlerts.map((a, i) => (
                <PortfolioRow key={`${a.accountId}-${i}`} alert={a} />
              ))}
            </SignalSection>
          )}

          {data.skipToday.length > 0 && (
            <SignalSection title="Skip Today" icon={<Lightbulb className="size-4 text-success" />}>
              {data.skipToday.map((s, i) => (
                <SkipRow key={i} signal={s} />
              ))}
            </SignalSection>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

// ─────────────────────────────────────────────────────────────────────
// Sub-components
// ─────────────────────────────────────────────────────────────────────

function SignalBadge({
  icon,
  count,
  label,
  className,
}: {
  icon: React.ReactNode;
  count: number;
  label: string;
  className: string;
}) {
  const plural = count === 1 ? label : `${label}s`;
  return (
    <Badge variant="outline" className={className}>
      {icon}
      <span>
        {count} {plural}
      </span>
    </Badge>
  );
}

function SignalSection({
  title,
  icon,
  children,
}: {
  title: string;
  icon: React.ReactNode;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(false);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger className="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm font-medium hover:bg-muted/50 transition-colors">
        {icon}
        <span className="flex-1 text-left">{title}</span>
        <ChevronDown
          className={`size-4 text-muted-foreground transition-transform ${open ? "rotate-180" : ""}`}
        />
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="px-2 pb-2 pt-1 space-y-1.5">{children}</div>
      </CollapsibleContent>
    </Collapsible>
  );
}

function DecisionRow({ signal }: { signal: DecisionSignal }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <Badge variant="outline" className="shrink-0 font-mono text-[10px]">
        {signal.priority}
      </Badge>
      <span className="flex-1 truncate">{signal.title}</span>
      {signal.dueDate && (
        <span className="shrink-0 font-mono text-xs text-muted-foreground">
          {signal.dueDate}
        </span>
      )}
      {signal.account && (
        <span className="shrink-0 text-xs text-muted-foreground">
          {signal.account}
        </span>
      )}
    </div>
  );
}

function DelegationRow({ signal }: { signal: DelegationSignal }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="flex-1 truncate">{signal.title}</span>
      {signal.waitingOn && (
        <span className="shrink-0 text-xs text-muted-foreground">
          on {signal.waitingOn}
        </span>
      )}
      <span className="shrink-0 font-mono text-xs text-destructive">
        {signal.daysStale}d
      </span>
    </div>
  );
}

function PortfolioRow({ alert }: { alert: PortfolioAlert }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="font-medium">{alert.accountName}</span>
      <span className="flex-1 text-xs text-muted-foreground truncate">
        {alert.detail}
      </span>
    </div>
  );
}

function SkipRow({ signal }: { signal: SkipSignal }) {
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className="flex-1">{signal.item}</span>
      <span className="shrink-0 text-xs text-muted-foreground italic">
        {signal.reason}
      </span>
    </div>
  );
}
