import { useMemo } from "react";
import { CalendarCheck, AlertTriangle, Clock, Play } from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { Meeting, Action, MeetingType } from "@/types";

interface ReadinessStripProps {
  meetings: Meeting[];
  actions: Action[];
}

const EXTERNAL_TYPES: MeetingType[] = ["customer", "qbr", "partnership", "external"];

function parseTimeToday(timeStr: string): Date | null {
  const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
  if (!match) return null;
  let hours = parseInt(match[1], 10);
  const minutes = parseInt(match[2], 10);
  const period = match[3].toUpperCase();
  if (period === "PM" && hours !== 12) hours += 12;
  if (period === "AM" && hours === 12) hours = 0;
  const d = new Date();
  d.setHours(hours, minutes, 0, 0);
  return d;
}

function useReadinessSignals(meetings: Meeting[], actions: Action[]) {
  return useMemo(() => {
    const now = new Date();
    const active = meetings.filter((m) => m.overlayStatus !== "cancelled");
    const prepped = active.filter((m) => m.hasPrep);

    // Prep coverage
    const prepTotal = active.length;
    const prepCount = prepped.length;
    const prepFraction = prepTotal > 0 ? prepCount / prepTotal : 1;

    // Agendas needed (external meetings without prep)
    const agendasNeeded = active.filter(
      (m) => !m.hasPrep && EXTERNAL_TYPES.includes(m.type)
    ).length;

    // Overdue actions
    const overdueActions = actions.filter(
      (a) => a.status !== "completed" && a.isOverdue
    );
    const pendingActions = actions.filter((a) => a.status !== "completed");
    const overdueCount = overdueActions.length;

    // Next meeting
    let nextMeeting: { title: string; time: string } | null = null;
    let allDone = false;

    if (active.length > 0) {
      // Find first meeting whose start time is still in the future
      const upcoming = active
        .map((m) => ({ meeting: m, start: parseTimeToday(m.time) }))
        .filter((x) => x.start && x.start > now)
        .sort((a, b) => a.start!.getTime() - b.start!.getTime());

      if (upcoming.length > 0) {
        nextMeeting = {
          title: upcoming[0].meeting.title,
          time: upcoming[0].meeting.time,
        };
      } else {
        allDone = true;
      }
    }

    return {
      prepCount,
      prepTotal,
      prepFraction,
      agendasNeeded,
      overdueCount,
      pendingCount: pendingActions.length,
      nextMeeting,
      allDone,
    };
  }, [meetings, actions]);
}

interface SignalCardProps {
  icon: React.ElementType;
  value: string;
  label: string;
  iconColor: string;
  iconBg: string;
  index: number;
}

function SignalCard({ icon: Icon, value, label, iconColor, iconBg, index }: SignalCardProps) {
  return (
    <Card
      className={cn(
        "transition-all duration-150 hover:-translate-y-0.5 hover:shadow-md",
        "animate-fade-in-up opacity-0",
        `animate-delay-${index + 1}`
      )}
    >
      <CardContent className="flex items-center gap-4 p-5">
        <div className={cn("rounded-md p-2", iconBg)}>
          <Icon className={cn("size-4", iconColor)} />
        </div>
        <div className="min-w-0">
          <div className="text-sm font-semibold truncate">{value}</div>
          <div className="text-xs text-muted-foreground">{label}</div>
        </div>
      </CardContent>
    </Card>
  );
}

export function ReadinessStrip({ meetings, actions }: ReadinessStripProps) {
  const signals = useReadinessSignals(meetings, actions);

  // Prep coverage color
  const prepColor =
    signals.prepTotal === 0
      ? "text-muted-foreground"
      : signals.prepFraction >= 1
        ? "text-primary"
        : signals.prepFraction < 0.5
          ? "text-destructive"
          : "text-foreground";
  const prepBg =
    signals.prepTotal === 0
      ? "bg-muted"
      : signals.prepFraction >= 1
        ? "bg-primary/10"
        : signals.prepFraction < 0.5
          ? "bg-destructive/10"
          : "bg-secondary/50";

  // Agendas needed color
  const agendaColor = signals.agendasNeeded > 0 ? "text-amber-600" : "text-emerald-600";
  const agendaBg = signals.agendasNeeded > 0 ? "bg-amber-500/10" : "bg-emerald-500/10";

  // Overdue actions color
  const actionColor = signals.overdueCount > 0 ? "text-destructive" : "text-muted-foreground";
  const actionBg = signals.overdueCount > 0 ? "bg-destructive/10" : "bg-muted";

  // Prep value
  const prepValue =
    signals.prepTotal === 0 ? "No meetings" : `${signals.prepCount} of ${signals.prepTotal}`;

  // Agenda value
  const agendaValue =
    signals.agendasNeeded === 0 ? "All set" : `${signals.agendasNeeded} needed`;

  // Action value
  const actionValue =
    signals.overdueCount > 0
      ? `${signals.overdueCount} overdue`
      : `${signals.pendingCount} due today`;

  // Next meeting value
  const nextValue = signals.nextMeeting
    ? signals.nextMeeting.time
    : signals.allDone
      ? "All done"
      : "No meetings";
  const nextLabel = signals.nextMeeting
    ? signals.nextMeeting.title
    : signals.allDone
      ? "For today"
      : "Today";

  return (
    <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
      <SignalCard
        icon={CalendarCheck}
        value={prepValue}
        label="Prepped"
        iconColor={prepColor}
        iconBg={prepBg}
        index={0}
      />
      <SignalCard
        icon={AlertTriangle}
        value={agendaValue}
        label="Agendas"
        iconColor={agendaColor}
        iconBg={agendaBg}
        index={1}
      />
      <SignalCard
        icon={Clock}
        value={actionValue}
        label="Actions"
        iconColor={actionColor}
        iconBg={actionBg}
        index={2}
      />
      <SignalCard
        icon={Play}
        value={nextValue}
        label={nextLabel}
        iconColor="text-muted-foreground"
        iconBg="bg-muted"
        index={3}
      />
    </div>
  );
}
