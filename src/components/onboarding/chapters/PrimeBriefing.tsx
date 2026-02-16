import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { ArrowRight, CalendarDays, Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { FinisMarker } from "@/components/editorial/FinisMarker";
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
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="Prime Your First Briefing"
        epigraph={context?.prompt ?? "Load your calendar context, then run a full today workflow preview."}
      />

      {loading ? (
        <div style={{ height: 128 }} />
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {cards.length === 0 ? (
            <p
              style={{
                fontFamily: "var(--font-serif)",
                fontStyle: "italic",
                fontSize: 14,
                color: "var(--color-text-tertiary)",
                margin: 0,
              }}
            >
              No upcoming events found yet. You can still generate a preview briefing now.
            </p>
          ) : (
            cards.map((card) => (
              <div
                key={card.id}
                style={{
                  borderTop: "1px solid var(--color-rule-light)",
                  paddingTop: 12,
                }}
              >
                <div style={{ display: "flex", alignItems: "flex-start", justifyContent: "space-between", gap: 12 }}>
                  <div>
                    <div
                      style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 12,
                        color: "var(--color-text-tertiary)",
                      }}
                    >
                      {card.dayLabel}
                    </div>
                    <div
                      style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 14,
                        fontWeight: 500,
                        color: "var(--color-text-primary)",
                        marginTop: 2,
                      }}
                    >
                      {card.title}
                    </div>
                    <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginTop: 4 }}>
                      {card.suggestedAction}
                    </div>
                    {card.suggestedEntityName && (
                      <div style={{ fontSize: 12, color: "var(--color-text-tertiary)", marginTop: 2 }}>
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

      {/* Full Workflow Preview */}
      <div
        style={{
          borderTop: "1px solid var(--color-rule-light)",
          paddingTop: 20,
          fontSize: 14,
          color: "var(--color-text-secondary)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8, color: "var(--color-text-primary)" }}>
          <CalendarDays size={16} />
          <span style={{ fontWeight: 500 }}>Full Workflow Preview</span>
        </div>
        Run{" "}
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 12 }}>today</span>{" "}
        now to generate a complete preview briefing immediately.
      </div>

      {runMessage && (
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            borderTop: "1px solid var(--color-rule-light)",
            paddingTop: 12,
          }}
        >
          {runMessage}
        </div>
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

      {/* Editorial close — FinisMarker */}
      <FinisMarker />
    </div>
  );
}
