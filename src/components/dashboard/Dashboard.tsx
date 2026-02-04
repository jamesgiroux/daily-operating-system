import { ScrollArea } from "@/components/ui/scroll-area";
import { Overview } from "./Overview";
import { MeetingTimeline } from "./MeetingTimeline";
import { ActionList } from "./ActionList";
import type { DashboardData } from "@/types";

interface DashboardProps {
  data: DashboardData;
}

export function Dashboard({ data }: DashboardProps) {
  return (
    <ScrollArea className="flex-1">
      <div className="p-6">
        <div className="mx-auto max-w-6xl space-y-8">
          {/* Overview section */}
          <Overview overview={data.overview} stats={data.stats} />

          {/* Main content grid */}
          <div className="grid gap-6 lg:grid-cols-[2fr_1fr]">
            {/* Meeting timeline - main column */}
            <MeetingTimeline meetings={data.meetings} />

            {/* Actions sidebar */}
            <ActionList actions={data.actions} />
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}
