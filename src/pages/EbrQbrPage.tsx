/**
 * EbrQbrPage — 7-slide editorial magazine slide deck for customer-facing
 * executive business reviews (EBR/QBR).
 * Atmosphere: larkspur. Modeled on RiskBriefingPage.tsx structure.
 * Keyboard navigation: arrow keys for next/prev, number keys 1-7 for direct jump.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import {
  Building2,
  BookOpen,
  Star,
  BarChart2,
  Compass,
  ArrowRight,
  Target,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { EbrCover } from "@/components/ebr-qbr/EbrCover";
import { TheStorySlide } from "@/components/ebr-qbr/TheStorySlide";
import { ValueDeliveredEbrSlide } from "@/components/ebr-qbr/ValueDeliveredEbrSlide";
import { MetricsSlide } from "@/components/ebr-qbr/MetricsSlide";
import { NavigatedSlide } from "@/components/ebr-qbr/NavigatedSlide";
import { RoadmapSlide } from "@/components/ebr-qbr/RoadmapSlide";
import { NextStepsSlide } from "@/components/ebr-qbr/NextStepsSlide";
import type {
  EbrQbrContent,
  EbrQbrValueItem,
  EbrQbrMetric,
  EbrQbrRisk,
  EbrQbrAction,
  ReportRow,
} from "@/types/reports";
import type { AccountDetail } from "@/types";
import slides from "./report-slides.module.css";

// Normalize DB data to the current schema — guards against old cached reports
// whose JSON predates field additions (schema evolution).
function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

function normalizeEbrQbr(raw: Record<string, unknown>): EbrQbrContent {
  return {
    quarterLabel: (raw.quarterLabel as string) ?? "",
    executiveSummary: (raw.executiveSummary as string) ?? "",
    storyBullets: toArr<string>(raw.storyBullets),
    customerQuote: (raw.customerQuote as string) ?? null,
    valueDelivered: toArr<EbrQbrValueItem>(raw.valueDelivered),
    successMetrics: toArr<EbrQbrMetric>(raw.successMetrics),
    challengesAndResolutions: toArr<EbrQbrRisk>(raw.challengesAndResolutions),
    strategicRoadmap: (raw.strategicRoadmap as string) ?? "",
    nextSteps: toArr<EbrQbrAction>(raw.nextSteps),
  };
}

const SLIDES = [
  { id: "cover", label: "Cover", icon: <Building2 size={18} strokeWidth={1.5} /> },
  { id: "the-story", label: "The Story", icon: <BookOpen size={18} strokeWidth={1.5} /> },
  { id: "value-delivered", label: "Value Delivered", icon: <Star size={18} strokeWidth={1.5} /> },
  {
    id: "by-the-numbers",
    label: "By the Numbers",
    icon: <BarChart2 size={18} strokeWidth={1.5} />,
  },
  {
    id: "what-we-navigated",
    label: "What We Navigated",
    icon: <Compass size={18} strokeWidth={1.5} />,
  },
  { id: "whats-ahead", label: "What's Ahead", icon: <ArrowRight size={18} strokeWidth={1.5} /> },
  { id: "next-steps", label: "Next Steps", icon: <Target size={18} strokeWidth={1.5} /> },
];

const ANALYSIS_PHASES = [
  {
    key: "gathering",
    label: "Gathering quarter data",
    detail: "Reading meeting history, recent activity, and account context",
  },
  {
    key: "synthesizing",
    label: "Synthesizing value",
    detail: "Identifying outcomes delivered and strategic wins",
  },
  {
    key: "metrics",
    label: "Assembling metrics",
    detail: "Pulling success indicators and trend data",
  },
  {
    key: "roadmap",
    label: "Building the roadmap",
    detail: "Synthesizing strategic direction for next quarter",
  },
  {
    key: "finalizing",
    label: "Finalizing review",
    detail: "Assembling next steps and executive summary",
  },
];

const EDITORIAL_QUOTES = [
  "The best EBRs spend more time on the future than the past.",
  "Show value before asking for it.",
  "A customer who understands your roadmap is an invested partner.",
  "Strategic reviews are conversations, not presentations.",
  "Every EBR is an opportunity to reframe the relationship.",
];

export default function EbrQbrPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();

  const [report, setReport] = useState<ReportRow | null>(null);
  const [content, setContent] = useState<EbrQbrContent | null>(null);
  const [accountName, setAccountName] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced save — persists edited content to backend
  const debouncedSave = useCallback(
    (updated: EbrQbrContent) => {
      if (!accountId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_report", {
          entityId: accountId,
          entityType: "account",
          reportType: "ebr_qbr",
          contentJson: JSON.stringify(updated),
        })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => {
            console.error("Failed to save EBR/QBR report:", e);
            toast.error("Failed to save");
          });
      }, 500);
    },
    [accountId],
  );

  const updateContent = useCallback(
    (updated: EbrQbrContent) => {
      setContent(updated);
      debouncedSave(updated);
    },
    [debouncedSave],
  );

  const feedback = useIntelligenceFeedback(accountId, "account");

  useRevealObserver(!loading && !!content);

  // Load account name
  useEffect(() => {
    if (!accountId) return;
    invoke<AccountDetail>("get_account_detail", { accountId })
      .then((detail) => setAccountName(detail.name))
      .catch((e) => console.error("Failed to load account detail:", e)); // Expected: background data fetch on mount
  }, [accountId]);

  // Load cached report on mount
  useEffect(() => {
    if (!accountId) return;
    setLoading(true);
    invoke<ReportRow | null>("get_report", {
      entityId: accountId,
      entityType: "account",
      reportType: "ebr_qbr",
    })
      .then((data) => {
        if (data) {
          setReport(data);
          setContent(normalizeEbrQbr(JSON.parse(data.contentJson)));
        } else {
          setReport(null);
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (ebr_qbr) failed:", err);
        toast.error("Failed to load report");
        setReport(null);
        setContent(null);
      })
      .finally(() => setLoading(false));
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
        reportType: "ebr_qbr",
      });
      setReport(data);
      setContent(normalizeEbrQbr(JSON.parse(data.contentJson)));
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate EBR/QBR report");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [accountId, generating]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "EBR / QBR",
      atmosphereColor: "larkspur" as const,
      activePage: "accounts" as const,
      breadcrumbs: [
        { label: "Accounts", onClick: () => navigate({ to: "/accounts" }) },
        {
          label: "Account",
          onClick: () => navigate({ to: "/accounts/$accountId", params: { accountId: accountId! } }),
        },
        { label: "EBR / QBR" },
      ],
      chapters: content ? SLIDES : undefined,
      folioStatusText: saveStatus === "saved" ? "\u2713 Saved" : undefined,
      folioActions: content ? (
        <button
          onClick={handleGenerate}
          disabled={generating}
          className={`${slides.folioAction} ${generating ? slides.folioActionDisabled : ""}`}
          style={{ "--report-accent": "var(--color-garden-larkspur)" } as React.CSSProperties}
        >
          {generating ? "Generating..." : "Regenerate"}
        </button>
      ) : undefined,
    }),
    [navigate, accountId, content, saveStatus, handleGenerate, generating],
  );
  useRegisterMagazineShell(shellConfig);

  // Keyboard navigation: 1-7 jump to slides, arrows navigate
  useEffect(() => {
    if (!content) return;

    function handleKeyDown(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const num = parseInt(e.key);
      if (num >= 1 && num <= 7) {
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
        style={{ "--report-accent": "var(--color-garden-larkspur)" } as React.CSSProperties}
      >
        <div className={slides.emptyOverline}>
          EBR / QBR
        </div>
        <h2 className={slides.emptyTitle}>
          No review generated yet
        </h2>
        <p className={slides.emptyDescription}>
          Generate a 7-slide executive business review. This will synthesize all available
          context, meeting history, and success metrics for this account.
        </p>
        {error && (
          <p className={slides.emptyError}>
            {error}
          </p>
        )}
        <Button onClick={handleGenerate} disabled={generating}>
          Generate EBR / QBR
        </Button>
      </div>
    );
  }

  // Generating state
  if (generating) {
    return (
      <GeneratingProgress
        title="Building EBR / QBR"
        accentColor="var(--color-garden-larkspur)"
        phases={ANALYSIS_PHASES}
        currentPhaseKey={
          ANALYSIS_PHASES[Math.min(Math.floor(genSeconds / 20), ANALYSIS_PHASES.length - 1)].key
        }
        quotes={EDITORIAL_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  // Render the 7-slide review with scroll-snap
  return (
    <div className={slides.slideContainer}>
      {/* Slide 1: Cover */}
      <section id="cover" className={slides.slideSection}>
        <EbrCover
          accountName={accountName}
          content={content!}
          onUpdate={updateContent}
          generatedAt={report?.generatedAt}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("executive_summary")}
          onFeedback={(type) => feedback.submitFeedback("executive_summary", type)}
        />
      </section>

      {/* Slide 2: The Story */}
      <div className="editorial-reveal">
        <TheStorySlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("the_story")}
          onFeedback={(type) => feedback.submitFeedback("the_story", type)}
        />
      </div>

      {/* Slide 3: Value Delivered */}
      <div className="editorial-reveal">
        <ValueDeliveredEbrSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("value_delivered")}
          onFeedback={(type) => feedback.submitFeedback("value_delivered", type)}
        />
      </div>

      {/* Slide 4: By the Numbers */}
      <div className="editorial-reveal">
        <MetricsSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("metrics")}
          onFeedback={(type) => feedback.submitFeedback("metrics", type)}
        />
      </div>

      {/* Slide 5: What We Navigated */}
      <div className="editorial-reveal">
        <NavigatedSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("challenges")}
          onFeedback={(type) => feedback.submitFeedback("challenges", type)}
        />
      </div>

      {/* Slide 6: What's Ahead */}
      <div className="editorial-reveal">
        <RoadmapSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("roadmap")}
          onFeedback={(type) => feedback.submitFeedback("roadmap", type)}
        />
      </div>

      {/* Slide 7: Next Steps */}
      <div className="editorial-reveal">
        <NextStepsSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("next_steps")}
          onFeedback={(type) => feedback.submitFeedback("next_steps", type)}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
