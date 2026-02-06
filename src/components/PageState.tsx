import { AlertCircle, Coffee, RefreshCw } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import type { LucideIcon } from "lucide-react";

interface PageEmptyProps {
  icon?: LucideIcon;
  title: string;
  message?: string;
  footnote?: string;
}

interface PageErrorProps {
  message: string;
  onRetry?: () => void;
}

/**
 * Centered empty state for pages with no data yet.
 * Positive framing — "nothing here" should feel like "you're good."
 */
export function PageEmpty({
  icon: Icon = Coffee,
  title,
  message,
  footnote,
}: PageEmptyProps) {
  return (
    <ScrollArea className="flex-1">
      <div className="flex h-full min-h-[60vh] items-center justify-center p-6">
        <div className="space-y-4 text-center">
          <div className="mx-auto flex size-16 items-center justify-center rounded-full bg-primary/10">
            <Icon className="size-8 text-primary" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-semibold">{title}</h2>
            {message && (
              <p className="mx-auto max-w-md text-muted-foreground">
                {message}
              </p>
            )}
          </div>
          {footnote && (
            <p className="pt-2 text-sm text-muted-foreground">{footnote}</p>
          )}
        </div>
      </div>
    </ScrollArea>
  );
}

/**
 * Centered error state for pages that failed to load.
 * Clear message + retry action when available.
 */
export function PageError({ message, onRetry }: PageErrorProps) {
  return (
    <ScrollArea className="flex-1">
      <div className="flex h-full min-h-[60vh] items-center justify-center p-6">
        <div className="space-y-4 text-center">
          <div className="mx-auto flex size-16 items-center justify-center rounded-full bg-destructive/10">
            <AlertCircle className="size-8 text-destructive" />
          </div>
          <div className="space-y-2">
            <h2 className="text-xl font-semibold">Something went wrong</h2>
            <p className="mx-auto max-w-md text-muted-foreground">{message}</p>
          </div>
          {onRetry && (
            <Button onClick={onRetry} variant="outline" className="gap-2">
              <RefreshCw className="size-4" />
              Try again
            </Button>
          )}
        </div>
      </div>
    </ScrollArea>
  );
}
