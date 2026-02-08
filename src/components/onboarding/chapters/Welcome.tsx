import { Zap, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";

interface WelcomeProps {
  onNext: () => void;
}

export function Welcome({ onNext }: WelcomeProps) {
  return (
    <div className="space-y-8 text-center">
      <div className="mx-auto flex size-16 items-center justify-center rounded-2xl bg-primary text-primary-foreground">
        <Zap className="size-8" />
      </div>

      <div className="space-y-3">
        <h1 className="text-3xl font-semibold tracking-tight">
          Open the app. Your day is ready.
        </h1>
        <p className="text-lg text-muted-foreground">
          DailyOS prepares your day while you sleep — meeting prep,
          email triage, actions due, and a morning summary. You open it,
          read, and get to work.
        </p>
      </div>

      <div className="mx-auto max-w-sm rounded-lg border bg-muted/30 p-5 space-y-2">
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider">What it looks like</p>
        <div className="space-y-1.5 text-sm">
          <div className="flex items-baseline gap-3">
            <span className="font-mono text-xs text-muted-foreground shrink-0">6:00 AM</span>
            <span className="text-muted-foreground">Your briefing generates automatically</span>
          </div>
          <div className="flex items-baseline gap-3">
            <span className="font-mono text-xs text-muted-foreground shrink-0">8:00 AM</span>
            <span className="text-muted-foreground">You open the app. Everything's there.</span>
          </div>
          <div className="flex items-baseline gap-3">
            <span className="font-mono text-xs text-primary shrink-0">8:15 AM</span>
            <span className="text-foreground">You're prepared. Close the app. Do your work.</span>
          </div>
        </div>
      </div>

      <p className="text-sm text-muted-foreground leading-relaxed">
        No setup to maintain. No inbox to clear.
        Skip a day, skip a week — it picks up where you are.
      </p>

      <Button size="lg" onClick={onNext}>
        Let's get started
        <ArrowRight className="ml-2 size-4" />
      </Button>
    </div>
  );
}
