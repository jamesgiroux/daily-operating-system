/**
 * RiskBriefingPage — 6-slide executive risk briefing for an account.
 * Uses the magazine shell with terracotta atmosphere.
 * Keyboard navigation: arrow keys for next/prev, number keys 1-6 for direct jump.
 * Scroll-snap settles on slide boundaries.
 *
 * I600: Migrated to reports framework. Uses get_report for reading cached data
 * and generate_risk_briefing for generation (risk briefing has its own PTY pipeline).
 * Route: /accounts/$accountId/reports/risk_briefing
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import {
  AlignLeft,
  Crosshair,
  BookOpen,
  TrendingDown,
  Target,
  Hand,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { RiskCover } from "@/components/risk-briefing/RiskCover";
import { BottomLineSlide } from "@/components/risk-briefing/BottomLineSlide";
import { WhatHappenedSlide } from "@/components/risk-briefing/WhatHappenedSlide";
import { StakesSlide } from "@/components/risk-briefing/StakesSlide";
import { ThePlanSlide } from "@/components/risk-briefing/ThePlanSlide";
import { TheAskSlide } from "@/components/risk-briefing/TheAskSlide";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import type {
  RiskBriefing,
  RiskBottomLine,
  RiskWhatHappened,
  RiskStakes,
  RiskThePlan,
  RiskTheAsk,
} from "@/types";
import type { ReportRow } from "@/types/reports";
import slides from "./report-slides.module.css";

// =============================================================================
// Normalization — guards against schema changes between report versions
// =============================================================================

function normalizeRiskBriefing(raw: Record<string, unknown>): RiskBriefing {
  return {
    accountId: (raw.accountId as string) ?? "",
    generatedAt: (raw.generatedAt as string) ?? "",
    cover: (raw.cover as RiskBriefing["cover"]) ?? { accountName: "", date: "" },
    bottomLine: (raw.bottomLine as RiskBriefing["bottomLine"]) ?? { headline: "" },
    whatHappened: (raw.whatHappened as RiskBriefing["whatHappened"]) ?? { narrative: "" },
    stakes: (raw.stakes as RiskBriefing["stakes"]) ?? {},
    thePlan: (raw.thePlan as RiskBriefing["thePlan"]) ?? { strategy: "" },
    theAsk: (raw.theAsk as RiskBriefing["theAsk"]) ?? {},
  };
}

// =============================================================================
// Slide manifest
// =============================================================================

const SLIDES = [
  { id: "cover", label: "Cover", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "bottom-line", label: "Bottom Line", icon: <Crosshair size={18} strokeWidth={1.5} /> },
  { id: "what-happened", label: "What Happened", icon: <BookOpen size={18} strokeWidth={1.5} /> },
  { id: "stakes", label: "The Stakes", icon: <TrendingDown size={18} strokeWidth={1.5} /> },
  { id: "the-plan", label: "The Plan", icon: <Target size={18} strokeWidth={1.5} /> },
  { id: "the-ask", label: "The Ask", icon: <Hand size={18} strokeWidth={1.5} /> },
];

const ANALYSIS_PHASES = [
  { key: "gathering", label: "Gathering context", detail: "Reading account data, meeting history, and stakeholder updates" },
  { key: "reading", label: "Reading the room", detail: "Analyzing stakeholder dynamics and relationship patterns" },
  { key: "building", label: "Building the story", detail: "Synthesizing situation, complication, and decline arc" },
  { key: "mapping", label: "Mapping stakes", detail: "Assessing financial exposure and decision-maker landscape" },
  { key: "planning", label: "Developing the plan", detail: "Building recovery strategy and action steps" },
  { key: "finalizing", label: "Finalizing", detail: "Assembling executive briefing and resource asks" },
];

const EDITORIAL_QUOTES = [
  "The first step in solving a problem is recognizing there is one.",
  "Strategy without tactics is the slowest route to victory.",
  "In the middle of difficulty lies opportunity.",
  "The best way to predict the future is to create it.",
  "What gets measured gets managed.",
  "The most dangerous phrase is: we've always done it this way.",
];

// =============================================================================
// Page component
// =============================================================================

export default function RiskBriefingPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();

  const [report, setReport] = useState<ReportRow | null>(null);
  const [briefing, setBriefing] = useState<RiskBriefing | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [completedSections, setCompletedSections] = useState<Set<string>>(new Set());
  const [currentPhaseKey, setCurrentPhaseKey] = useState<string>("gathering");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Slide-level feedback (matches AccountHealthPage/SwotPage pattern)
  const feedback = useIntelligenceFeedback(accountId ?? undefined, "account");

  // Debounced save — persists edited briefing via reports framework
  const debouncedSave = useCallback(
    (updated: RiskBriefing) => {
      if (!accountId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_report", {
          entityId: accountId,
          entityType: "account",
          reportType: "risk_briefing",
          contentJson: JSON.stringify(updated),
        })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => {
            console.error("Failed to save risk briefing:", e);
            toast.error("Failed to save risk briefing");
          });
      }, 500);
    },
    [accountId],
  );

  // Slide update handlers — update local state + trigger save
  const updateSlide = useCallback(
    <K extends keyof RiskBriefing>(key: K, value: RiskBriefing[K]) => {
      setBriefing((prev) => {
        if (!prev) return prev;
        const updated = { ...prev, [key]: value };
        debouncedSave(updated);
        return updated;
      });
    },
    [debouncedSave],
  );

  useRevealObserver(!loading && !!briefing);

  // Load cached report on mount — uses reports framework (get_report)
  useEffect(() => {
    if (!accountId) return;
    setLoading(true);
    invoke<ReportRow | null>("get_report", {
      entityId: accountId,
      entityType: "account",
      reportType: "risk_briefing",
    })
      .then((data) => {
        if (data) {
          setReport(data);
          try {
            setBriefing(normalizeRiskBriefing(JSON.parse(data.contentJson)));
          } catch (e) {
            console.error("Failed to parse risk briefing content:", e); // Expected: corrupted report JSON
            setBriefing(null);
          }
        } else {
          setReport(null);
          setBriefing(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (risk_briefing) failed:", err); // Expected: background data fetch on mount
        setReport(null);
        setBriefing(null);
      })
      .finally(() => setLoading(false));
  }, [accountId]);

  // Generate handler — uses dedicated risk briefing pipeline
  const handleGenerate = useCallback(async () => {
    if (!accountId || generating) return;
    setBriefing(null);
    setReport(null);
    setGenerating(true);
    setGenSeconds(0);
    setError(null);
    setCompletedSections(new Set());
    setCurrentPhaseKey("gathering");
    window.scrollTo({ top: 0, behavior: "instant" });

    timerRef.current = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const data = await invoke<RiskBriefing>("generate_risk_briefing", { accountId });
      setBriefing(data);
      // Re-fetch the report row to get updated metadata (generatedAt, etc.)
      invoke<ReportRow | null>("get_report", {
        entityId: accountId,
        entityType: "account",
        reportType: "risk_briefing",
      }).then((row) => {
        if (row) setReport(row);
      }).catch(() => {});
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate risk briefing");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [accountId, generating]);

  useEffect(() => {
    if (!generating) return;

    let unlistenContent: UnlistenFn | null = null;

    listen<RiskBriefing>("risk-briefing-content", (event) => {
      if (event.payload.accountId !== accountId) return;
      setBriefing(event.payload);
    }).then((fn) => {
      unlistenContent = fn;
    });

    return () => {
      if (unlistenContent) unlistenContent();
    };
  }, [accountId, generating]);

  const handleRiskProgress = useCallback((payload: {
    accountId: string;
    sectionName: string;
    completed: number;
    total: number;
  }) => {
    if (!accountId || payload.accountId !== accountId) return;
    setCompletedSections((prev) => new Set([...prev, payload.sectionName]));
    setCurrentPhaseKey(payload.sectionName);
  }, [accountId]);

  useTauriEvent("risk-briefing-progress", handleRiskProgress);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Risk Briefing",
      atmosphereColor: "terracotta" as const,
      activePage: "accounts" as const,
      backLink: {
        label: "Back",
        onClick: () =>
          window.history.length > 1
            ? window.history.back()
            : navigate({
                to: "/accounts/$accountId",
                params: { accountId: accountId! },
              }),
      },
      chapters: briefing ? SLIDES : undefined,
      folioStatusText: saveStatus === "saved" ? "\u2713 Saved" : undefined,
      folioActions: briefing ? (
        <button
          onClick={handleGenerate}
          disabled={generating}
          className={`${slides.folioAction} ${generating ? slides.folioActionDisabled : ""}`}
          style={{ "--report-accent": "var(--color-spice-terracotta)" } as React.CSSProperties}
        >
          {generating ? "Generating..." : "Regenerate"}
        </button>
      ) : undefined,
    }),
    [navigate, accountId, briefing, saveStatus, handleGenerate, generating],
  );
  useRegisterMagazineShell(shellConfig);

  // Keyboard navigation: 1-6 jump to slides, arrows navigate
  useEffect(() => {
    if (!briefing) return;

    function handleKeyDown(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const num = parseInt(e.key);
      if (num >= 1 && num <= 6) {
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
  }, [briefing]);

  // Loading state
  if (loading) {
    return (
      <div className={slides.loadingSkeleton}>
        <Skeleton className={`mb-4 h-4 w-24 ${slides.skeletonBg}`} />
        <Skeleton className={`mb-2 h-12 w-96 ${slides.skeletonBg}`} />
        <Skeleton className={`mb-8 h-5 w-full max-w-2xl ${slides.skeletonBg}`} />
      </div>
    );
  }

  // Empty state
  if (!briefing && !generating) {
    return (
      <div
        className={slides.emptyState}
        style={{ "--report-accent": "var(--color-spice-terracotta)" } as React.CSSProperties}
      >
        <div className={slides.emptyOverline}>
          Risk Briefing
        </div>
        <h2 className={slides.emptyTitle}>
          No briefing generated yet
        </h2>
        <p className={slides.emptyDescription}>
          Generate a 6-slide executive briefing. This will analyze all available intelligence, meeting history, and stakeholder data.
        </p>
        {error && (
          <p className={slides.emptyError}>
            {error}
          </p>
        )}
        <Button onClick={handleGenerate} disabled={generating}>
          Generate Risk Briefing
        </Button>
      </div>
    );
  }

  // Generating state
  if (generating && !briefing) {
    const phaseMap: Record<string, string> = {
      bottomLine: "building",
      whatHappened: "reading",
      stakes: "mapping",
      thePlan: "planning",
      theAsk: "finalizing",
    };
    return (
      <GeneratingProgress
        title="Building Risk Briefing"
        accentColor="var(--color-spice-terracotta)"
        phases={ANALYSIS_PHASES}
        currentPhaseKey={
          completedSections.size > 0
            ? (phaseMap[currentPhaseKey] ?? "gathering")
            : ANALYSIS_PHASES[Math.min(Math.floor(genSeconds / 20), ANALYSIS_PHASES.length - 1)].key
        }
        quotes={EDITORIAL_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  // Render the 6-slide briefing with scroll-snap
  return (
    <div className={slides.slideContainer}>
      {/* Slide 1: Cover */}
      <section id="cover" className={slides.slideSection}>
        <RiskCover
          data={briefing!.cover}
          onUpdate={(v) => updateSlide("cover", v)}
        />
      </section>

      {/* Slide 2: Bottom Line */}
      <div className="editorial-reveal">
        <BottomLineSlide
          data={briefing!.bottomLine}
          onUpdate={(v: RiskBottomLine) => updateSlide("bottomLine", v)}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("bottom_line")}
          onFeedback={(type) => feedback.submitFeedback("bottom_line", type)}
        />
      </div>

      {/* Slide 3: What Happened */}
      <div className="editorial-reveal">
        <WhatHappenedSlide
          data={briefing!.whatHappened}
          onUpdate={(v: RiskWhatHappened) => updateSlide("whatHappened", v)}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("what_happened")}
          onFeedback={(type) => feedback.submitFeedback("what_happened", type)}
        />
      </div>

      {/* Slide 4: The Stakes */}
      <div className="editorial-reveal">
        <StakesSlide
          data={briefing!.stakes}
          onUpdate={(v: RiskStakes) => updateSlide("stakes", v)}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("stakes")}
          onFeedback={(type) => feedback.submitFeedback("stakes", type)}
        />
      </div>

      {/* Slide 5: The Plan */}
      <div className="editorial-reveal">
        <ThePlanSlide
          data={briefing!.thePlan}
          onUpdate={(v: RiskThePlan) => updateSlide("thePlan", v)}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("the_plan")}
          onFeedback={(type) => feedback.submitFeedback("the_plan", type)}
        />
      </div>

      {/* Slide 6: The Ask */}
      <div className="editorial-reveal">
        <TheAskSlide
          data={briefing!.theAsk}
          onUpdate={(v: RiskTheAsk) => updateSlide("theAsk", v)}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("the_ask")}
          onFeedback={(type) => feedback.submitFeedback("the_ask", type)}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
