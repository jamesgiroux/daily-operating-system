/**
 * MonthlyWrappedPage — Spotify Wrapped-style celebration of the user's month.
 * 10 full-viewport slides, scroll-snap, CSS animations, animated count-up numbers.
 *
 * This is NOT a report — it's a celebration. One idea per screen.
 *
 * Keyboard navigation: 1-9 for slides 1-9, 0 for slide 10, arrow keys for prev/next.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import type { ReportRow } from "@/types/reports";

import type { MonthlyWrappedContent } from "./types";
import { SLIDES, ANALYSIS_PHASES, EDITORIAL_QUOTES, normalizeMonthlyWrapped } from "./constants";
import {
  SplashSlide,
  VolumeSlide,
  TopAccountsSlide,
  MomentsSlide,
  HiddenPatternSlide,
  PersonalitySlide,
  PrioritySlide,
  TopWinSlide,
  CarryForwardSlide,
  CloseSlide,
} from "./slides";
import styles from "./monthly-wrapped.module.css";
import "./animations.css";

// =============================================================================
// Page component
// =============================================================================

export default function MonthlyWrappedPage() {
  const navigate = useNavigate();

  const [userId, setUserId] = useState<string | null>(null);
  const [content, setContent] = useState<MonthlyWrappedContent | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Load user entity ID first
  useEffect(() => {
    invoke<{ id: string | number }>("get_user_entity")
      .then((u) => setUserId(String(u.id)))
      .catch((err) => {
        console.error("get_user_entity failed:", err); // Expected: background init on mount
        setLoading(false);
      });
  }, []);

  // Load cached report once userId is known
  useEffect(() => {
    if (!userId) return;
    setLoading(true);
    invoke<ReportRow>("get_report", {
      entityId: userId,
      entityType: "user",
      reportType: "monthly_wrapped",
    })
      .then((data) => {
        try {
          setContent(normalizeMonthlyWrapped(JSON.parse(data.contentJson)));
        } catch (e) {
          console.error("Failed to parse monthly_wrapped content:", e); // Expected: corrupted report JSON
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (monthly_wrapped) failed:", err);
        toast.error("Failed to load report");
        setContent(null);
      })
      .finally(() => setLoading(false));
  }, [userId]);

  useRevealObserver(!loading && !!content);

  // Debounced save
  const debouncedSave = useCallback(
    (updated: MonthlyWrappedContent) => {
      if (!userId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_report", {
          entityId: userId,
          entityType: "user",
          reportType: "monthly_wrapped",
          contentJson: JSON.stringify(updated),
        })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => {
            console.error("Failed to save monthly wrapped:", e);
            toast.error("Failed to save");
          });
      }, 500);
    },
    [userId],
  );

  // Generate handler
  const handleGenerate = useCallback(async () => {
    if (!userId || generating) return;
    setContent(null);
    setGenerating(true);
    setGenSeconds(0);
    setError(null);
    window.scrollTo({ top: 0, behavior: "instant" });

    timerRef.current = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const data = await invoke<ReportRow>("generate_report", {
        entityId: userId,
        entityType: "user",
        reportType: "monthly_wrapped",
      });
      const parsed = normalizeMonthlyWrapped(JSON.parse(data.contentJson));
      setContent(parsed);
      debouncedSave(parsed);
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate Monthly Wrapped");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [userId, generating, debouncedSave]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Monthly Wrapped",
      atmosphereColor: "eucalyptus" as const,
      activePage: "me" as const,
      chapters: content
        ? SLIDES.map((s) => ({
            id: s.id,
            icon: s.icon,
            label: s.id === "splash" ? (content.monthLabel ?? s.label) : s.label,
          }))
        : undefined,
      folioStatusText: saveStatus === "saved" ? "Saved" : undefined,
      folioActions: content ? (
        <button
          onClick={handleGenerate}
          disabled={generating}
          className={`${styles.folioAction} ${generating ? styles.folioActionDisabled : styles.folioActionActive}`}
        >
          {generating ? "Generating..." : "Regenerate"}
        </button>
      ) : undefined,
    }),
    [content, saveStatus, handleGenerate, generating],
  );
  useRegisterMagazineShell(shellConfig);

  // Keyboard navigation: 1-9 for slides 1-9, 0 for slide 10, arrows
  useEffect(() => {
    if (!content) return;

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
      document.getElementById(SLIDES[nextIndex].id)?.scrollIntoView({ behavior: "smooth" });
    }

    function handleKeyDown(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      // 1-9 → slides 0-8, 0 → slide 9
      if (e.key >= "1" && e.key <= "9") {
        const idx = parseInt(e.key) - 1;
        document.getElementById(SLIDES[idx].id)?.scrollIntoView({ behavior: "smooth" });
        return;
      }
      if (e.key === "0") {
        document.getElementById(SLIDES[9].id)?.scrollIntoView({ behavior: "smooth" });
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

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [content]);

  // Loading state
  if (loading) {
    return (
      <div className={styles.loadingSkeleton}>
        <Skeleton className={`mb-4 h-4 w-24 ${styles.skeletonBar}`} />
        <Skeleton className={`mb-2 h-12 w-96 ${styles.skeletonBar}`} />
        <Skeleton className={`mb-8 h-5 w-full max-w-2xl ${styles.skeletonBar}`} />
      </div>
    );
  }

  // Empty state
  if (!content && !generating) {
    const priorMonth = new Date();
    priorMonth.setMonth(priorMonth.getMonth() - 1);
    const monthGuess = priorMonth.toLocaleString("default", { month: "long", year: "numeric" });
    return (
      <div className={styles.emptyState}>
        <div className={styles.emptyOverline}>Monthly Wrapped</div>
        <h2 className={styles.emptyTitle}>
          Your {monthGuess} Wrapped hasn&apos;t been generated yet
        </h2>
        <p className={styles.emptyDescription}>
          Generate it to see how your last full month looked — personality type, biggest moments, hidden
          patterns, and more.
        </p>
        {error && (
          <p className={styles.emptyError}>{error}</p>
        )}
        <Button onClick={handleGenerate} disabled={generating || !userId}>
          Generate Monthly Wrapped
        </Button>
      </div>
    );
  }

  // Generating state
  if (generating) {
    return (
      <GeneratingProgress
        title="Wrapping Your Month"
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

  // Render all 10 slides
  return (
    <div className={styles.viewportBreakout}>
      <div className={styles.slideContainer}>
        <SplashSlide content={content!} />
        <VolumeSlide content={content!} />
        <TopAccountsSlide content={content!} />
        <MomentsSlide content={content!} />
        <HiddenPatternSlide content={content!} />
        <PersonalitySlide content={content!} />
        <PrioritySlide
          content={content!}
          onNavigateToMe={() => navigate({ to: "/me" })}
        />
        <TopWinSlide content={content!} />
        <CarryForwardSlide content={content!} />
        <CloseSlide content={content!} />
      </div>
    </div>
  );
}
