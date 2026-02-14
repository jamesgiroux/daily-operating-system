import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { ArrowRight, CalendarDays, Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { OnboardingPrimingContext } from "@/types";

interface PrimeBriefingProps {
  onComplete: () => void;
}

export function PrimeBriefing({ onComplete }: PrimeBriefingProps) {
  const [loading, setLoading] = useState(true);
  const [running, setRunning] = useState(false);
  const [context, setContext] = useState<OnboardingPrimingContext | null>(null);
  const [runMessage, setRunMessage] = useState<string | null>(null);
  const navigate = useNavigate();

  useEffect(() => {
    async function load() {
      try {
        const payload = await invoke<OnboardingPrimingContext>("get_onboarding_priming_context");
        setContext(payload);
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  const cards = useMemo(() => context?.cards ?? [], [context]);

  async function handlePreviewRun() {
    setRunning(true);
    setRunMessage(null);
    try {
      const message = await invoke<string>("run_workflow", { workflow: "today" });
      setRunMessage(message);
    } catch (error) {
      setRunMessage(String(error));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">Prime Your First Briefing</h2>
        <p className="text-sm text-muted-foreground">
          {context?.prompt ?? "Load your calendar context, then run a full today workflow preview."}
        </p>
      </div>

      {loading ? (
        <div className="h-32" />
      ) : (
        <div className="space-y-3">
          {cards.length === 0 ? (
            <div className="rounded-lg border border-dashed p-4 text-sm text-muted-foreground">
              No upcoming events found yet. You can still generate a preview briefing now.
            </div>
          ) : (
            cards.map((card) => (
              <div key={card.id} className="rounded-lg border p-3">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="text-xs text-muted-foreground">{card.dayLabel}</div>
                    <div className="font-medium">{card.title}</div>
                    <div className="mt-1 text-xs text-muted-foreground">{card.suggestedAction}</div>
                    {card.suggestedEntityName && (
                      <div className="mt-1 text-xs text-muted-foreground">
                        Entity: {card.suggestedEntityName}
                      </div>
                    )}
                  </div>
                  {card.suggestedEntityId && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() =>
                        navigate({
                          to: "/inbox",
                          search: { entityId: card.suggestedEntityId },
                        })
                      }
                    >
                      Open Inbox
                    </Button>
                  )}
                </div>
              </div>
            ))
          )}
        </div>
      )}

      <div className="rounded-lg border bg-muted/30 p-4 text-sm text-muted-foreground">
        <div className="mb-2 flex items-center gap-2 text-foreground">
          <CalendarDays className="size-4" />
          <span>Full Workflow Preview</span>
        </div>
        Run <code>today</code> now to generate a complete preview briefing immediately.
      </div>

      {runMessage && (
        <div className="rounded-md border px-3 py-2 text-xs text-muted-foreground">{runMessage}</div>
      )}

      <div className="flex justify-between">
        <Button variant="outline" onClick={handlePreviewRun} disabled={running}>
          {running ? <Loader2 className="mr-2 size-4 animate-spin" /> : <Sparkles className="mr-2 size-4" />}
          {running ? "Running Preview..." : "Generate Preview Briefing"}
        </Button>
        <Button onClick={onComplete}>
          Go to Dashboard
          <ArrowRight className="ml-2 size-4" />
        </Button>
      </div>
    </div>
  );
}
