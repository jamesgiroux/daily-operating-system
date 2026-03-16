/**
 * SwotPage — 5-slide editorial magazine SWOT analysis for an account.
 * Uses the magazine shell with sage atmosphere.
 * Keyboard navigation: arrow keys for next/prev, number keys 1-5 for direct jump.
 * Scroll-snap settles on slide boundaries.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { Compass, TrendingUp, AlertTriangle, Lightbulb, LayoutGrid } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { SwotCover } from "@/components/swot/SwotCover";
import { QuadrantSlide } from "@/components/swot/QuadrantSlide";
import type { SwotContent, SwotItem, ReportRow } from "@/types/reports";
import slides from "./report-slides.module.css";

// =============================================================================
// Normalization — guards against schema changes between report versions
// =============================================================================

function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

function normalizeSwot(raw: Record<string, unknown>): SwotContent {
  return {
    strengths: toArr<SwotItem>(raw.strengths),
    weaknesses: toArr<SwotItem>(raw.weaknesses),
    opportunities: toArr<SwotItem>(raw.opportunities),
    threats: toArr<SwotItem>(raw.threats),
    summary: (raw.summary as string) ?? null,
  };
}

// =============================================================================
// Slide manifest
// =============================================================================

const SLIDES = [
  { id: "cover", label: "Cover", icon: <LayoutGrid size={18} strokeWidth={1.5} /> },
  { id: "strengths", label: "Strengths", icon: <TrendingUp size={18} strokeWidth={1.5} /> },
  { id: "weaknesses", label: "Weaknesses", icon: <AlertTriangle size={18} strokeWidth={1.5} /> },
  { id: "opportunities", label: "Opportunities", icon: <Lightbulb size={18} strokeWidth={1.5} /> },
  { id: "threats", label: "Threats", icon: <Compass size={18} strokeWidth={1.5} /> },
];

const ANALYSIS_PHASES = [
  {
    key: "gathering",
    label: "Gathering account data",
    detail: "Reading meeting history, stakeholder data, and recent activity",
  },
  {
    key: "analyzing",
    label: "Analyzing internal factors",
    detail: "Identifying strengths and areas for improvement",
  },
  {
    key: "scanning",
    label: "Scanning external landscape",
    detail: "Mapping opportunities and competitive threats",
  },
  {
    key: "finalizing",
    label: "Finalizing analysis",
    detail: "Validating items against source data",
  },
];

const EDITORIAL_QUOTES = [
  "Know yourself and know your customer.",
  "Honest analysis is the beginning of good strategy.",
  "The best SWOT doesn't just describe — it decides.",
  "Strengths are leverage. Weaknesses are invitations.",
];

// =============================================================================
// Page component
// =============================================================================

export default function SwotPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();

  const [report, setReport] = useState<ReportRow | null>(null);
  const [content, setContent] = useState<SwotContent | null>(null);
  const [accountName, setAccountName] = useState("");
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [currentPhaseKey, setCurrentPhaseKey] = useState<string>("gathering");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced save — persists edited content to the report row
  const debouncedSave = useCallback(
    (updated: SwotContent) => {
      if (!accountId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_report", {
          entityId: accountId,
          entityType: "account",
          reportType: "swot",
          contentJson: JSON.stringify(updated),
        })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => {
            console.error("Failed to save SWOT report:", e);
            toast.error("Failed to save");
          });
      }, 500);
    },
    [accountId],
  );

  const updateContent = useCallback(
    (updated: SwotContent) => {
      setContent(updated);
      debouncedSave(updated);
    },
    [debouncedSave],
  );

  const feedback = useIntelligenceFeedback(accountId, "account");

  useRevealObserver(!loading && !!content);

  // Load cached report on mount
  useEffect(() => {
    if (!accountId) return;
    setLoading(true);
    invoke<ReportRow>("get_report", {
      entityId: accountId,
      entityType: "account",
      reportType: "swot",
    })
      .then((data) => {
        setReport(data);
        try {
          setContent(normalizeSwot(JSON.parse(data.contentJson)));
        } catch (e) {
          console.error("Failed to parse SWOT content:", e);
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (swot) failed:", err);
        setReport(null);
        setContent(null);
      })
      .finally(() => setLoading(false));
  }, [accountId]);

  // Fetch account name separately
  useEffect(() => {
    if (!accountId) return;
    invoke<{ name: string }>("get_account_detail", { accountId })
      .then((acct) => setAccountName(acct.name))
      .catch((err) => console.error("get_account_detail failed:", err));
  }, [accountId]);

  // Generate handler
  const handleGenerate = useCallback(async () => {
    if (!accountId || generating) return;
    setContent(null);
    setReport(null);
    setGenerating(true);
    setGenSeconds(0);
    setError(null);
    window.scrollTo({ top: 0, behavior: "instant" });

    timerRef.current = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const data = await invoke<ReportRow>("generate_report", {
        entityId: accountId,
        entityType: "account",
        reportType: "swot",
      });
      setReport(data);
      setContent(normalizeSwot(JSON.parse(data.contentJson)));
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate SWOT analysis");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [accountId, generating]);

  useEffect(() => {
    if (!generating) return;
    let unlistenContent: UnlistenFn | null = null;
    listen<{ entityId: string; content: Record<string, unknown> }>("swot-content", (event) => {
      if (!accountId || event.payload.entityId !== accountId) return;
      setContent(normalizeSwot(event.payload.content));
    }).then((fn) => {
      unlistenContent = fn;
    });
    return () => {
      if (unlistenContent) unlistenContent();
    };
  }, [generating]);

  const handleSwotProgress = useCallback((payload: {
    entityId: string;
    completed: number;
    total: number;
    sectionName: string;
  }) => {
    if (!accountId || payload.entityId !== accountId) return;
    const phaseMap: Record<string, string> = {
      strengths: "analyzing",
      weaknesses: "analyzing",
      opportunities: "scanning",
      threats: "scanning",
      summary: "finalizing",
    };
    setCurrentPhaseKey(phaseMap[payload.sectionName] ?? "gathering");
  }, [accountId]);

  useTauriEvent("swot-progress", handleSwotProgress);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "SWOT Analysis",
      atmosphereColor: "olive" as const,
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
      chapters: content ? SLIDES : undefined,
      folioStatusText: saveStatus === "saved" ? "\u2713 Saved" : undefined,
      folioActions: content ? (
        <button
          onClick={handleGenerate}
          disabled={generating}
          className={`${slides.folioAction} ${generating ? slides.folioActionDisabled : ""}`}
          style={{ "--report-accent": "var(--color-garden-sage)" } as React.CSSProperties}
        >
          {generating ? "Generating..." : "Regenerate"}
        </button>
      ) : undefined,
    }),
    [navigate, accountId, content, saveStatus, handleGenerate, generating],
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
  if (!content && !generating) {
    return (
      <div
        className={slides.emptyState}
        style={{ "--report-accent": "var(--color-garden-sage)" } as React.CSSProperties}
      >
        <div className={slides.emptyOverline}>
          SWOT Analysis
        </div>
        <h2 className={slides.emptyTitle}>
          No analysis generated yet
        </h2>
        <p className={slides.emptyDescription}>
          Generate a 5-slide SWOT analysis. This will analyze meeting history, stakeholder data,
          recent activity, and relationship context to map strengths, weaknesses, opportunities, and threats.
        </p>
        {error && (
          <p className={slides.emptyError}>
            {error}
          </p>
        )}
        <Button onClick={handleGenerate} disabled={generating}>
          Generate SWOT Analysis
        </Button>
      </div>
    );
  }

  // Generating state
  if (generating && !content) {
    return (
      <GeneratingProgress
        title="Building SWOT Analysis"
        accentColor="var(--color-garden-sage)"
        phases={ANALYSIS_PHASES}
        currentPhaseKey={currentPhaseKey || ANALYSIS_PHASES[Math.min(Math.floor(genSeconds / 20), ANALYSIS_PHASES.length - 1)].key}
        quotes={EDITORIAL_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  // Render the 5-slide analysis with scroll-snap
  return (
    <div className={slides.slideContainer}>
      {/* Slide 1: Cover */}
      <section id="cover" className={slides.slideSection}>
        <SwotCover
          accountName={accountName}
          content={content!}
          onUpdate={updateContent}
          generatedAt={report?.generatedAt}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("summary")}
          onFeedback={(type) => feedback.submitFeedback("summary", type)}
        />
      </section>

      {/* Slide 2: Strengths */}
      <div className="editorial-reveal">
        <QuadrantSlide
          id="strengths"
          overline="Strengths"
          accentColor="var(--color-garden-sage)"
          items={content!.strengths}
          onUpdate={(items) => updateContent({ ...content!, strengths: items })}
          emptyLabel="No strengths identified."
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("strengths")}
          onFeedback={(type) => feedback.submitFeedback("strengths", type)}
        />
      </div>

      {/* Slide 3: Weaknesses */}
      <div className="editorial-reveal">
        <QuadrantSlide
          id="weaknesses"
          overline="Weaknesses"
          accentColor="var(--color-spice-turmeric)"
          items={content!.weaknesses}
          onUpdate={(items) => updateContent({ ...content!, weaknesses: items })}
          emptyLabel="No weaknesses identified."
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("weaknesses")}
          onFeedback={(type) => feedback.submitFeedback("weaknesses", type)}
        />
      </div>

      {/* Slide 4: Opportunities */}
      <div className="editorial-reveal">
        <QuadrantSlide
          id="opportunities"
          overline="Opportunities"
          accentColor="var(--color-garden-larkspur)"
          items={content!.opportunities}
          onUpdate={(items) => updateContent({ ...content!, opportunities: items })}
          emptyLabel="No opportunities identified."
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("opportunities")}
          onFeedback={(type) => feedback.submitFeedback("opportunities", type)}
        />
      </div>

      {/* Slide 5: Threats */}
      <div className="editorial-reveal">
        <QuadrantSlide
          id="threats"
          overline="Threats"
          accentColor="var(--color-spice-terracotta)"
          items={content!.threats}
          onUpdate={(items) => updateContent({ ...content!, threats: items })}
          emptyLabel="No threats identified."
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("threats")}
          onFeedback={(type) => feedback.submitFeedback("threats", type)}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
