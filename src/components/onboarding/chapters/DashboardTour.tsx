import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Target, ChevronRight, Loader2, ArrowRight, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TourHighlight } from "@/components/onboarding/TourHighlight";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { DashboardData, DataFreshness } from "@/types";

interface DashboardTourProps {
  onNext: () => void;
  onSkipTour: () => void;
}

const TOUR_STOPS = [
  {
    key: "focus",
    title: "Today's focus",
    body: "AI picks your focus based on today's priorities. Click to see full detail.",
  },
  {
    key: "schedule",
    title: "Your schedule, front and center",
    body: "Meetings appear immediately. Highlighted ones have full prep ready. Click 'View Prep' to see context, talking points, and risks.",
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

  useEffect(() => {
    let cancelled = false;
    async function setup() {
      try {
        const result = await invoke<{
          status: string;
          data?: DashboardData;
          freshness?: DataFreshness;
        }>("get_dashboard_data");
        if (!cancelled && result.status === "success" && result.data) {
          setData(result.data);
        }
      } catch (err) {
        console.error("Failed to load dashboard data:", err);
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
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", gap: 16, padding: "64px 0" }}>
        <Loader2 size={32} className="animate-spin" style={{ color: "var(--color-spice-turmeric)" }} />
        <p style={{ fontSize: 14, color: "var(--color-text-tertiary)" }}>
          Loading your briefing...
        </p>
      </div>
    );
  }

  if (!data) {
    return (
      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 16, padding: "32px 0" }}>
        <p style={{ fontSize: 14, color: "var(--color-text-tertiary)" }}>
          No briefing data yet. You can explore the dashboard after setup.
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
      <div style={{ display: "flex", flexDirection: "column", gap: 24, paddingBottom: 32 }}>
        <ChapterHeading
          title="Anatomy of your day"
          epigraph="This is what a real briefing looks like. Let's walk through each section."
        />

        {/* Single-column layout matching the actual dashboard */}
        <div style={{ display: "flex", flexDirection: "column", gap: 32 }}>
          <div>
            <h1
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 24,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              {new Date().toLocaleDateString("en-US", {
                weekday: "long",
                month: "long",
                day: "numeric",
                year: "numeric",
              })}
            </h1>
          </div>

          {data.overview.focus && (
            <TourHighlight ref={setStopRef(0)} active={stop.key === "focus"}>
              <div
                style={{
                  borderLeft: "3px solid var(--color-garden-sage)",
                  paddingLeft: 16,
                  paddingTop: 12,
                  paddingBottom: 12,
                }}
              >
                <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    <Target size={18} style={{ color: "var(--color-garden-sage)" }} />
                    <span style={{ fontSize: 13, fontWeight: 600, color: "var(--color-garden-sage)" }}>Focus</span>
                  </div>
                  <ChevronRight size={16} style={{ color: "var(--color-text-tertiary)" }} />
                </div>
                <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-secondary)", lineHeight: 1.5, margin: 0 }}>
                  {data.overview.focus}
                </p>
              </div>
            </TourHighlight>
          )}

          <TourHighlight ref={setStopRef(1)} active={stop.key === "schedule"}>
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
                {data.meetings.filter(m => m.overlayStatus !== "cancelled").length} meeting{data.meetings.filter(m => m.overlayStatus !== "cancelled").length !== 1 ? "s" : ""} today
              </div>
              {data.meetings.map((m) => (
                <div
                  key={m.id}
                  style={{
                    borderTop: "1px solid var(--color-rule-light)",
                    paddingTop: 12,
                  }}
                >
                  <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)", margin: 0 }}>{m.title}</p>
                  <p style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-tertiary)", margin: "4px 0 0" }}>{m.time}</p>
                </div>
              ))}
            </div>
          </TourHighlight>

          <TourHighlight ref={setStopRef(2)} active={stop.key === "actions"}>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
                {data.actions.length} action{data.actions.length !== 1 ? "s" : ""}
              </div>
              {data.actions.slice(0, 5).map((a) => (
                <div key={a.id} style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 8 }}>
                  <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)", margin: 0 }}>{a.title}</p>
                  <p style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-tertiary)", margin: "2px 0 0" }}>{a.priority}</p>
                </div>
              ))}
            </div>
          </TourHighlight>

          <TourHighlight ref={setStopRef(3)} active={stop.key === "emails"}>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              <div style={{ fontSize: 12, color: "var(--color-text-tertiary)" }}>
                {emails.length} email{emails.length !== 1 ? "s" : ""}
              </div>
              {emails.slice(0, 5).map((e, i) => (
                <div key={i} style={{ borderTop: "1px solid var(--color-rule-light)", paddingTop: 8 }}>
                  <p style={{ fontSize: 14, fontWeight: 500, color: "var(--color-text-primary)", margin: 0 }}>{e.subject}</p>
                  <p style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-tertiary)", margin: "2px 0 0" }}>{e.sender}</p>
                </div>
              ))}
            </div>
          </TourHighlight>
        </div>
      </div>

      {/* Floating tour card — editorial styling */}
      <div
        style={{
          position: "fixed",
          bottom: 24,
          right: 24,
          zIndex: 50,
          width: 320,
          background: "var(--color-paper-warm-white)",
          border: "1px solid var(--color-rule-heavy)",
          borderRadius: "var(--radius-editorial-lg)",
          padding: 20,
          boxShadow: "var(--shadow-md)",
        }}
      >
        <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
          {/* Progress indicator */}
          <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, color: "var(--color-text-tertiary)" }}>
            <span>{currentStop + 1} of {TOUR_STOPS.length}</span>
            <div style={{ marginLeft: "auto", display: "flex", gap: 6 }}>
              {TOUR_STOPS.map((_, i) => (
                <button
                  key={i}
                  style={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    border: "none",
                    cursor: "pointer",
                    background:
                      i === currentStop
                        ? "var(--color-spice-turmeric)"
                        : i < currentStop
                          ? "rgba(201, 162, 39, 0.4)"
                          : "var(--color-rule-light)",
                    transition: "background 0.15s ease",
                    padding: 0,
                  }}
                  onClick={() => setCurrentStop(i)}
                />
              ))}
            </div>
          </div>

          {/* Annotation */}
          <div>
            <h4 style={{ fontSize: 14, fontWeight: 600, color: "var(--color-text-primary)", margin: 0 }}>
              {stop.title}
            </h4>
            <p style={{ fontSize: 13, color: "var(--color-text-secondary)", marginTop: 4, marginBottom: 0, lineHeight: 1.5 }}>
              {stop.body}
            </p>
          </div>

          {/* Navigation */}
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", paddingTop: 4 }}>
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
            style={{
              width: "100%",
              textAlign: "center",
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              letterSpacing: "0.04em",
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              cursor: "pointer",
              transition: "color 0.15s ease",
            }}
            onClick={onSkipTour}
          >
            Skip tour
          </button>
        </div>
      </div>
    </>
  );
}
