import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Target, ChevronRight, Loader2, ArrowRight, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TourHighlight } from "@/components/onboarding/TourHighlight";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { DashboardData, DataFreshness } from "@/types";
import styles from "../onboarding.module.css";

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
    body: "Meetings appear immediately. Highlighted ones have a full briefing ready. Click 'View Briefing' to see context, talking points, and risks.",
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
      <div className={styles.tourLoadingContainer}>
        <Loader2 size={32} className={`animate-spin ${styles.accentColor}`} />
        <p className={`${styles.bodyText} ${styles.tertiaryText}`}>
          Loading your briefing...
        </p>
      </div>
    );
  }

  if (!data) {
    return (
      <div className={styles.tourEmptyContainer}>
        <p className={`${styles.bodyText} ${styles.tertiaryText}`}>
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
      <div className={`${styles.flexCol} ${styles.gap24} ${styles.pb32}`}>
        <ChapterHeading
          title="Anatomy of your day"
          epigraph="This is what a real briefing looks like. Let's walk through each section."
        />

        {/* Single-column layout matching the actual dashboard */}
        <div className={`${styles.flexCol} ${styles.gap32}`}>
          <div>
            <h1 className={styles.dateHeadline}>
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
              <div className={styles.focusStrip}>
                <div className={`${styles.flexBetween} ${styles.mb8}`}>
                  <div className={`${styles.flexRow} ${styles.gap8}`}>
                    <Target size={18} className={styles.sageColor} />
                    <span className={styles.focusLabel}>Focus</span>
                  </div>
                  <ChevronRight size={16} className={styles.tertiaryText} />
                </div>
                <p className={styles.focusText}>
                  {data.overview.focus}
                </p>
              </div>
            </TourHighlight>
          )}

          <TourHighlight ref={setStopRef(1)} active={stop.key === "schedule"}>
            <div className={`${styles.flexCol} ${styles.gap12}`}>
              <div className={styles.tourCountLabel}>
                {data.meetings.filter(m => m.overlayStatus !== "cancelled").length} meeting{data.meetings.filter(m => m.overlayStatus !== "cancelled").length !== 1 ? "s" : ""} today
              </div>
              {data.meetings.map((m) => (
                <div key={m.id} className={styles.tourMeetingCard}>
                  <p className={styles.tourItemTitle}>{m.title}</p>
                  <p className={styles.tourItemMeta}>{m.time}</p>
                </div>
              ))}
            </div>
          </TourHighlight>

          <TourHighlight ref={setStopRef(2)} active={stop.key === "actions"}>
            <div className={`${styles.flexCol} ${styles.gap8}`}>
              <div className={styles.tourCountLabel}>
                {data.actions.length} action{data.actions.length !== 1 ? "s" : ""}
              </div>
              {data.actions.slice(0, 5).map((a) => (
                <div key={a.id} className={styles.tourMeetingCard}>
                  <p className={styles.tourItemTitle}>{a.title}</p>
                  <p className={styles.tourItemMeta}>{a.priority}</p>
                </div>
              ))}
            </div>
          </TourHighlight>

          <TourHighlight ref={setStopRef(3)} active={stop.key === "emails"}>
            <div className={`${styles.flexCol} ${styles.gap8}`}>
              <div className={styles.tourCountLabel}>
                {emails.length} email{emails.length !== 1 ? "s" : ""}
              </div>
              {emails.slice(0, 5).map((e, i) => (
                <div key={i} className={styles.tourMeetingCard}>
                  <p className={styles.tourItemTitle}>{e.subject}</p>
                  <p className={styles.tourItemMeta}>{e.sender}</p>
                </div>
              ))}
            </div>
          </TourHighlight>
        </div>
      </div>

      {/* Floating tour card — editorial styling */}
      <div className={styles.tourCard}>
        <div className={`${styles.flexCol} ${styles.gap12}`}>
          {/* Progress indicator */}
          <div className={styles.tourProgress}>
            <span>{currentStop + 1} of {TOUR_STOPS.length}</span>
            <div className={styles.tourDots}>
              {TOUR_STOPS.map((_, i) => (
                <button
                  key={i}
                  className={`${styles.tourDot} ${
                    i === currentStop
                      ? styles.tourDotActive
                      : i < currentStop
                        ? styles.tourDotVisited
                        : styles.tourDotPending
                  }`}
                  onClick={() => setCurrentStop(i)}
                />
              ))}
            </div>
          </div>

          {/* Annotation */}
          <div>
            <h4 className={styles.tourTitle}>
              {stop.title}
            </h4>
            <p className={styles.tourBody}>
              {stop.body}
            </p>
          </div>

          {/* Navigation */}
          <div className={styles.tourNav}>
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
            className={styles.tourSkipButton}
            onClick={onSkipTour}
          >
            Skip tour
          </button>
        </div>
      </div>
    </>
  );
}
