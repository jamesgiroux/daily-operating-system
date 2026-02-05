import { ScrollArea } from "@/components/ui/scroll-area";
import { Overview } from "./Overview";
import { MeetingTimeline } from "./MeetingTimeline";
import { ActionList } from "./ActionList";
import { EmailList } from "./EmailList";
import type { DashboardData } from "@/types";

interface DashboardProps {
  data: DashboardData;
}

export function Dashboard({ data }: DashboardProps) {
  const emails = data.emails ?? [];

  return (
    <ScrollArea className="flex-1">
      <div className="p-8">
        <div className="mx-auto max-w-6xl space-y-8">
          {/* Overview section */}
          <Overview overview={data.overview} stats={data.stats} />

          {/* Main content grid */}
          <div className="grid gap-6 lg:grid-cols-[2fr_1fr]">
            {/* Meeting timeline - main column */}
            <MeetingTimeline meetings={data.meetings} />

            {/* Right sidebar: Emails + Actions */}
            <div className="space-y-6">
              <EmailList emails={emails} />
              <ActionList actions={data.actions} />
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}
