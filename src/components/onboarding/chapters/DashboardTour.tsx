import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Target, ChevronRight, Loader2, ArrowRight, ArrowLeft } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { MeetingTimeline } from "@/components/dashboard/MeetingTimeline";
import { EmailList } from "@/components/dashboard/EmailList";
import { ActionList } from "@/components/dashboard/ActionList";
import { TourHighlight } from "@/components/onboarding/TourHighlight";
import type { DashboardData, DataFreshness } from "@/types";

interface DashboardTourProps {
  onNext: () => void;
  onSkipTour: () => void;
}

const TOUR_STOPS = [
  {
    key: "schedule",
    title: "Your schedule, front and center",
    body: "Meetings appear immediately. Highlighted ones have full prep ready. Click 'View Prep' to see context, talking points, and risks.",
  },
  {
    key: "focus",
    title: "Today's focus",
    body: "AI picks your focus based on today's priorities. Click to see full detail.",
  },
  {
    key: "actions",
    title: "Actions sourced from everywhere",
    body: "Meetings, emails, manual entry — sorted by priority and due date.",
  },
  {
    key: "emails",
    title: "Priority-sorted email triage",
    body: "AI reads each email, writes a summary, and recommends an action.",
  },
] as const;

export function DashboardTour({ onNext, onSkipTour }: DashboardTourProps) {
  const [loading, setLoading] = useState(true);
  const [data, setData] = useState<DashboardData | null>(null);
  const [currentStop, setCurrentStop] = useState<number>(0);
  const stopRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Auto-scroll to active section
  useEffect(() => {
    const el = stopRefs.current[currentStop];
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [currentStop]);

  const setStopRef = useCallback(
    (index: number) => (el: HTMLDivElement | null) => {
      stopRefs.current[index] = el;
    },
    [],
  );

  // Install demo data then load dashboard
  useEffect(() => {
    let cancelled = false;
    async function setup() {
      try {
        await invoke("install_demo_data");
        const result = await invoke<{
          status: string;
          data?: DashboardData;
          freshness?: DataFreshness;
        }>("get_dashboard_data");
        if (!cancelled && result.status === "success" && result.data) {
          setData(result.data);
        }
      } catch (err) {
        console.error("Failed to install demo data:", err);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    setup();
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center gap-4 py-16">
        <Loader2 className="size-8 animate-spin text-primary" />
        <p className="text-sm text-muted-foreground">
          Preparing your demo briefing...
        </p>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="space-y-4 text-center py-8">
        <p className="text-sm text-muted-foreground">
          Couldn't load demo data. You can explore the dashboard after setup.
        </p>
        <Button onClick={onNext}>
          Continue
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    );
  }

  const stop = TOUR_STOPS[currentStop];
  const emails = data.emails ?? [];

  return (
    <>
      <div className="space-y-6 pb-8">
        <div className="space-y-2 text-center">
          <h2 className="text-2xl font-semibold tracking-tight">
            Anatomy of your day
          </h2>
          <p className="text-sm text-muted-foreground">
            This is what a real briefing looks like. Let's walk through each
            section.
          </p>
        </div>

        {/* Two-column layout matching the actual dashboard */}
        <div className="grid gap-8 lg:grid-cols-[5fr_2fr]">
          {/* Left: Schedule */}
          <div className="min-w-0 space-y-6">
            <div className="space-y-1">
              <h1 className="text-2xl font-semibold tracking-tight">
                {new Date().toLocaleDateString("en-US", {
                  weekday: "long",
                  month: "long",
                  day: "numeric",
                  year: "numeric",
                })}
              </h1>
            </div>
            <TourHighlight ref={setStopRef(0)} active={stop.key === "schedule"}>
              <MeetingTimeline meetings={data.meetings} />
            </TourHighlight>
          </div>

          {/* Right: Context sidebar */}
          <div className="min-w-0 space-y-5">
            {data.overview.focus && (
              <TourHighlight ref={setStopRef(1)} active={stop.key === "focus"}>
                <div className="rounded-lg bg-primary/5 border border-primary/10 px-3.5 py-3">
                  <div className="flex items-start gap-2.5">
                    <Target className="size-4 shrink-0 text-primary mt-0.5" />
                    <div className="min-w-0 flex-1">
                      <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">Focus</span>
                      <p className="mt-1 text-sm font-medium text-primary leading-relaxed">{data.overview.focus}</p>
                    </div>
                    <ChevronRight className="size-4 shrink-0 text-muted-foreground mt-0.5" />
                  </div>
                </div>
              </TourHighlight>
            )}
            <TourHighlight ref={setStopRef(2)} active={stop.key === "actions"}>
              <ActionList actions={data.actions} />
            </TourHighlight>
            <TourHighlight ref={setStopRef(3)} active={stop.key === "emails"}>
              <EmailList emails={emails} />
            </TourHighlight>
          </div>
        </div>
      </div>

      {/* Floating tour card — always visible */}
      <div className="fixed bottom-6 right-6 z-50 w-80 rounded-xl border bg-card p-5 shadow-lg">
        <div className="space-y-3">
          {/* Progress indicator */}
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span>
              {currentStop + 1} of {TOUR_STOPS.length}
            </span>
            <div className="ml-auto flex gap-1.5">
              {TOUR_STOPS.map((_, i) => (
                <button
                  key={i}
                  className={cn(
                    "size-2 rounded-full transition-colors",
                    i === currentStop
                      ? "bg-primary"
                      : i < currentStop
                        ? "bg-primary/40"
                        : "bg-muted",
                  )}
                  onClick={() => setCurrentStop(i)}
                />
              ))}
            </div>
          </div>

          {/* Annotation */}
          <div>
            <h4 className="text-sm font-semibold">{stop.title}</h4>
            <p className="mt-1 text-sm text-muted-foreground">{stop.body}</p>
          </div>

          {/* Navigation */}
          <div className="flex items-center justify-between pt-1">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setCurrentStop((s) => Math.max(0, s - 1))}
              disabled={currentStop === 0}
            >
              <ArrowLeft className="mr-1 size-3" />
              Back
            </Button>

            {currentStop < TOUR_STOPS.length - 1 ? (
              <Button size="sm" onClick={() => setCurrentStop((s) => s + 1)}>
                Next
                <ArrowRight className="ml-1 size-3" />
              </Button>
            ) : (
              <Button size="sm" onClick={onNext}>
                Continue
                <ArrowRight className="ml-1 size-3" />
              </Button>
            )}
          </div>

          {/* Skip */}
          <button
            className="w-full text-center text-xs text-muted-foreground transition-colors hover:text-foreground"
            onClick={onSkipTour}
          >
            Skip tour
          </button>
        </div>
      </div>
    </>
  );
}
