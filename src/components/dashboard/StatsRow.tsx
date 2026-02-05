import {
  CalendarDays,
  CheckSquare,
  Inbox,
  Users,
} from "lucide-react";
import { Card, CardContent } from "@/components/ui/card";
import type { DayStats } from "@/types";
import { cn } from "@/lib/utils";

interface StatsRowProps {
  stats: DayStats;
}

const statConfig = [
  {
    key: "totalMeetings" as const,
    label: "Meetings",
    icon: CalendarDays,
    color: "text-foreground",
    bgColor: "bg-secondary/50",
  },
  {
    key: "customerMeetings" as const,
    label: "Customer",
    icon: Users,
    color: "text-primary",
    bgColor: "bg-primary/10",
  },
  {
    key: "actionsDue" as const,
    label: "Actions Due",
    icon: CheckSquare,
    color: "text-destructive",
    bgColor: "bg-destructive/10",
  },
  {
    key: "inboxCount" as const,
    label: "Inbox",
    icon: Inbox,
    color: "text-muted-foreground",
    bgColor: "bg-muted",
  },
];

export function StatsRow({ stats }: StatsRowProps) {
  return (
    <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
      {statConfig.map((stat, index) => (
        <Card
          key={stat.key}
          className={cn(
            "transition-all duration-150 hover:-translate-y-0.5 hover:shadow-md",
            "animate-fade-in-up opacity-0",
            `animate-delay-${index + 1}`
          )}
        >
          <CardContent className="flex items-center gap-4 p-5">
            <div className={cn("rounded-md p-2", stat.bgColor)}>
              <stat.icon className={cn("size-4", stat.color)} />
            </div>
            <div>
              <div className={cn("text-2xl font-bold", stat.color)}>
                {stats[stat.key]}
              </div>
              <div className="text-xs text-muted-foreground">{stat.label}</div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
