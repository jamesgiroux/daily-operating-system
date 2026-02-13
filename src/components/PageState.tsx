import { AlertCircle, Coffee, RefreshCw } from "lucide-react";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { LucideIcon } from "lucide-react";

interface PageEmptyProps {
  icon?: LucideIcon;
  title: string;
  message?: string;
  footnote?: string;
  action?: React.ReactNode;
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
  action,
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
          {action && <div className="pt-2">{action}</div>}
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

interface SectionEmptyProps {
  icon: LucideIcon;
  title: string;
  message?: string;
  action?: React.ReactNode;
}

/**
 * Card-wrapped empty state for list sections and tabs.
 * Used when a filtered/tabbed list has no items to show.
 */
export function SectionEmpty({
  icon: Icon,
  title,
  message,
  action,
}: SectionEmptyProps) {
  return (
    <Card>
      <CardContent className="flex flex-col items-center justify-center py-12 text-center">
        <Icon className="mb-4 size-12 text-muted-foreground/40" />
        <p className="text-lg font-medium">{title}</p>
        {message && (
          <p className="mt-1 text-sm text-muted-foreground">{message}</p>
        )}
        {action && <div className="mt-4">{action}</div>}
      </CardContent>
    </Card>
  );
}

interface InlineEmptyProps {
  icon: LucideIcon;
  message: string;
}

/**
 * Lightweight empty state for detail page sections.
 * No card wrapper — blends into surrounding content.
 */
export function InlineEmpty({ icon: Icon, message }: InlineEmptyProps) {
  return (
    <div className="flex flex-col items-center py-6 text-center">
      <Icon className="mb-2 size-8 text-muted-foreground/40" />
      <p className="text-sm text-muted-foreground">{message}</p>
    </div>
  );
}
