/**
 * WeeklyImpactPage — 5-slide editorial magazine weekly impact report.
 * Uses the magazine shell with eucalyptus atmosphere.
 * Keyboard navigation: arrow keys for next/prev, number keys 1-5 for direct jump.
 * Scroll-snap settles on slide boundaries.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Calendar, Target, CheckSquare, Eye, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { CoverSlide } from "@/components/weekly-impact/CoverSlide";
import { PrioritiesMovedSlide } from "@/components/weekly-impact/PrioritiesMovedSlide";
import { TheWorkSlide } from "@/components/weekly-impact/TheWorkSlide";
import { WatchSlide } from "@/components/weekly-impact/WatchSlide";
import { IntoNextWeekSlide } from "@/components/weekly-impact/IntoNextWeekSlide";
import type { WeeklyImpactContent, WeeklyImpactMove, WeeklyImpactItem } from "@/types/reports";
import type { ReportRow } from "@/types/reports";
import slides from "./report-slides.module.css";

// =============================================================================
// Normalization — guards against cached reports with old schema
// =============================================================================

function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

function normalizeWeeklyImpact(raw: Record<string, unknown>): WeeklyImpactContent {
  return {
    weekLabel: (raw.weekLabel as string) ?? "",
    totalMeetings: (raw.totalMeetings as number) ?? 0,
    totalActionsClosed: (raw.totalActionsClosed as number) ?? 0,
    headline: (raw.headline as string) ?? "",
    prioritiesMoved: toArr<WeeklyImpactMove>(raw.prioritiesMoved),
    wins: toArr<WeeklyImpactItem>(raw.wins),
    whatYouDid: (raw.whatYouDid as string) ?? "",
    watch: toArr<WeeklyImpactItem>(raw.watch),
    intoNextWeek: toArr<string>(raw.intoNextWeek),
  };
}

// =============================================================================
// Slide registry
// =============================================================================

const SLIDES = [
  { id: "cover", label: "Cover", icon: <Calendar size={18} strokeWidth={1.8} /> },
  { id: "priorities", label: "Priorities", icon: <Target size={18} strokeWidth={1.8} /> },
  { id: "the-work", label: "The Work", icon: <CheckSquare size={18} strokeWidth={1.8} /> },
  { id: "watch", label: "Watch", icon: <Eye size={18} strokeWidth={1.8} /> },
  { id: "next-week", label: "Next Week", icon: <ArrowRight size={18} strokeWidth={1.8} /> },
];

// =============================================================================
// Generating progress config
// =============================================================================

const ANALYSIS_PHASES = [
  { key: "gathering", label: "Gathering this week's data", detail: "Reading meetings, actions, and activity from the past 7 days" },
  { key: "priorities", label: "Checking priority movement", detail: "Finding what actually moved forward this week" },
  { key: "patterns", label: "Spotting patterns", detail: "Looking for wins and things worth watching" },
  { key: "finalizing", label: "Finalizing", detail: "Building your weekly view" },
];

const EDITORIAL_QUOTES = [
  "Small weeks compound into big quarters.",
  "A good week is one you'd be willing to repeat.",
  "Progress isn't always obvious from inside it.",
  "The work is the record.",
];

// =============================================================================
// Page component
// =============================================================================

export default function WeeklyImpactPage() {
  const navigate = useNavigate();

  const [userId, setUserId] = useState<string | null>(null);
  const [report, setReport] = useState<ReportRow | null>(null);
  const [content, setContent] = useState<WeeklyImpactContent | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Fetch user entity id on mount
  useEffect(() => {
    invoke<{ id: string | number }>("get_user_entity")
      .then((u) => setUserId(String(u.id)))
      .catch((err) => console.error("get_user_entity failed:", err));
  }, []);

  // Debounced save — persists edited content to the report row
  const debouncedSave = useCallback(
    (updated: WeeklyImpactContent) => {
      if (!userId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_report", {
          entityId: userId,
          entityType: "user",
          reportType: "weekly_impact",
          contentJson: JSON.stringify(updated),
        })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => {
            console.error("Failed to save weekly impact report:", e);
            toast.error("Failed to save");
          });
      }, 500);
    },
    [userId],
  );

  const updateContent = useCallback(
    (updated: WeeklyImpactContent) => {
      setContent(updated);
      debouncedSave(updated);
    },
    [debouncedSave],
  );

  const feedback = useIntelligenceFeedback(userId ?? undefined, "user");

  useRevealObserver(!loading && !!content);

  // Load cached report once userId is available
  useEffect(() => {
    if (!userId) return;
    setLoading(true);
    invoke<ReportRow>("get_report", {
      entityId: userId,
      entityType: "user",
      reportType: "weekly_impact",
    })
      .then((data) => {
        setReport(data);
        try {
          setContent(normalizeWeeklyImpact(JSON.parse(data.contentJson)));
        } catch (e) {
          console.error("Failed to parse weekly impact content:", e);
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (weekly_impact) failed:", err);
        setReport(null);
        setContent(null);
      })
      .finally(() => setLoading(false));
  }, [userId]);

  // Generate handler
  const handleGenerate = useCallback(async () => {
    if (!userId || generating) return;
    setContent(null);
    setReport(null);
    setGenerating(true);
    setGenSeconds(0);
    setError(null);
    window.scrollTo({ top: 0, behavior: "instant" });

    timerRef.current = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const data = await invoke<ReportRow>("generate_report", {
        entityId: userId,
        entityType: "user",
        reportType: "weekly_impact",
      });
      setReport(data);
      setContent(normalizeWeeklyImpact(JSON.parse(data.contentJson)));
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate weekly impact report");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [userId, generating]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Weekly Impact",
      atmosphereColor: "eucalyptus" as const,
      activePage: "me" as const,
      backLink: {
        label: "Back",
        onClick: () =>
          window.history.length > 1 ? window.history.back() : navigate({ to: "/me" }),
      },
      chapters: content ? SLIDES : undefined,
      folioStatusText: saveStatus === "saved" ? "\u2713 Saved" : undefined,
      folioActions: content ? (
        <button
          onClick={handleGenerate}
          disabled={generating}
          className={`${slides.folioAction} ${generating ? slides.folioActionDisabled : ""}`}
          style={{ "--report-accent": "var(--color-garden-eucalyptus)" } as React.CSSProperties}
        >
          {generating ? "Generating..." : "Regenerate"}
        </button>
      ) : undefined,
    }),
    [navigate, content, saveStatus, handleGenerate, generating],
  );
  useRegisterMagazineShell(shellConfig);

  // Keyboard navigation: 1-5 jump to slides, arrows navigate
  useEffect(() => {
    if (!content) return;

    function handleKeyDown(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const num = parseInt(e.key);
      if (num >= 1 && num <= 5) {
        const slide = SLIDES[num - 1];
        if (slide) {
          document.getElementById(slide.id)?.scrollIntoView({ behavior: "smooth" });
        }
        return;
      }

      if (e.key === "ArrowDown" || e.key === "ArrowRight") {
        e.preventDefault();
        scrollToNextSlide(1);
      } else if (e.key === "ArrowUp" || e.key === "ArrowLeft") {
        e.preventDefault();
        scrollToNextSlide(-1);
      }
    }

    function scrollToNextSlide(direction: 1 | -1) {
      const scrollY = window.scrollY + 100;
      let currentIndex = 0;

      for (let i = SLIDES.length - 1; i >= 0; i--) {
        const el = document.getElementById(SLIDES[i].id);
        if (el && el.offsetTop <= scrollY) {
          currentIndex = i;
          break;
        }
      }

      const nextIndex = Math.max(0, Math.min(SLIDES.length - 1, currentIndex + direction));
      const nextEl = document.getElementById(SLIDES[nextIndex].id);
      nextEl?.scrollIntoView({ behavior: "smooth" });
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [content]);

  // Loading state — wait for userId + report fetch
  if (loading || (!userId && !error)) {
    return (
      <div className={slides.loadingSkeleton}>
        <Skeleton className={`mb-4 h-4 w-24 ${slides.skeletonBg}`} />
        <Skeleton className={`mb-2 h-12 w-96 ${slides.skeletonBg}`} />
        <Skeleton className={`mb-8 h-5 w-full max-w-2xl ${slides.skeletonBg}`} />
      </div>
    );
  }

  // Empty state
  if (!content && !generating) {
    return (
      <div
        className={slides.emptyState}
        style={{ "--report-accent": "var(--color-garden-eucalyptus)" } as React.CSSProperties}
      >
        <div className={slides.emptyOverline}>
          Weekly Impact
        </div>
        <h2 className={slides.emptyTitle}>
          No weekly impact report yet
        </h2>
        <p className={slides.emptyDescription}>
          No weekly impact report yet for last week. Generate to see how your week looked.
        </p>
        {error && (
          <p className={slides.emptyError}>
            {error}
          </p>
        )}
        <Button onClick={handleGenerate} disabled={generating || !userId}>
          Generate Weekly Impact
        </Button>
      </div>
    );
  }

  // Generating state
  if (generating) {
    return (
      <GeneratingProgress
        title="Building Weekly Impact"
        accentColor="var(--color-garden-eucalyptus)"
        phases={ANALYSIS_PHASES}
        currentPhaseKey={
          ANALYSIS_PHASES[Math.min(Math.floor(genSeconds / 15), ANALYSIS_PHASES.length - 1)].key
        }
        quotes={EDITORIAL_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  // Render the 5-slide report with scroll-snap
  return (
    <div className={slides.slideContainer}>
      {/* Slide 1: Cover */}
      <section id="cover" className={slides.slideSection}>
        <CoverSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("headline")}
          onFeedback={(type) => feedback.submitFeedback("headline", type)}
        />
      </section>

      {/* Slide 2: Priorities Moved */}
      <div className="editorial-reveal">
        <PrioritiesMovedSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("priorities_moved")}
          onFeedback={(type) => feedback.submitFeedback("priorities_moved", type)}
        />
      </div>

      {/* Slide 3: The Work */}
      <div className="editorial-reveal">
        <TheWorkSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("the_work")}
          onFeedback={(type) => feedback.submitFeedback("the_work", type)}
        />
      </div>

      {/* Slide 4: Watch */}
      <div className="editorial-reveal">
        <WatchSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("watch")}
          onFeedback={(type) => feedback.submitFeedback("watch", type)}
        />
      </div>

      {/* Slide 5: Into Next Week */}
      <div className="editorial-reveal">
        <IntoNextWeekSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("into_next_week")}
          onFeedback={(type) => feedback.submitFeedback("into_next_week", type)}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
