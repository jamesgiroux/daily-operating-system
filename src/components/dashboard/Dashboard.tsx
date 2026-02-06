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
            <div className="min-w-0">
              <MeetingTimeline meetings={data.meetings} />
            </div>

            {/* Right sidebar: Emails + Actions */}
            <div className="min-w-0 space-y-6">
              <div className="animate-fade-in-up opacity-0 animate-delay-3">
                <EmailList emails={emails} />
              </div>
              <div className="animate-fade-in-up opacity-0 animate-delay-4">
                <ActionList actions={data.actions} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}
