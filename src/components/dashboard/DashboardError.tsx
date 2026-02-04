import { AlertCircle, RefreshCw } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";

interface DashboardErrorProps {
  message: string;
  onRetry: () => void;
}

export function DashboardError({ message, onRetry }: DashboardErrorProps) {
  return (
    <ScrollArea className="flex-1">
      <div className="flex h-full min-h-[60vh] items-center justify-center p-6">
        <div className="text-center space-y-4">
          <div className="mx-auto flex items-center justify-center w-16 h-16 rounded-full bg-destructive/10">
            <AlertCircle className="size-8 text-destructive" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-semibold">Something went wrong</h2>
            <p className="text-muted-foreground max-w-md">{message}</p>
          </div>
          <Button onClick={onRetry} variant="outline" className="gap-2">
            <RefreshCw className="size-4" />
            Try again
          </Button>
        </div>
      </div>
    </ScrollArea>
  );
}
