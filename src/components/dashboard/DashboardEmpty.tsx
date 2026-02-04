import { Coffee, Sunrise } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";

interface DashboardEmptyProps {
  message: string;
}

export function DashboardEmpty({ message }: DashboardEmptyProps) {
  return (
    <ScrollArea className="flex-1">
      <div className="flex h-full min-h-[60vh] items-center justify-center p-6">
        <div className="text-center space-y-4">
          <div className="mx-auto flex items-center justify-center w-16 h-16 rounded-full bg-primary/10">
            <Sunrise className="size-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-semibold">No briefing yet</h2>
            <p className="text-muted-foreground max-w-md">{message}</p>
          </div>
          <div className="flex items-center justify-center gap-2 text-sm text-muted-foreground pt-4">
            <Coffee className="size-4" />
            <span>Grab a coffee — your day will be ready soon</span>
          </div>
        </div>
      </div>
    </ScrollArea>
  );
}
