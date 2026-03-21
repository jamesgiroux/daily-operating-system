/**
 * AccountHealthPage — 5-slide editorial magazine account health review.
 * Uses the magazine shell with turmeric atmosphere.
 * Keyboard navigation: arrow keys for next/prev, number keys 1-5 for direct jump.
 * Scroll-snap settles on slide boundaries.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Activity, Users, BarChart2, Star, ArrowRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { AccountHealthCover } from "@/components/account-health/AccountHealthCover";
import { PartnershipSlide } from "@/components/account-health/PartnershipSlide";
import { WhereWeStandSlide } from "@/components/account-health/WhereWeStandSlide";
import { ValueDeliveredSlide } from "@/components/account-health/ValueDeliveredSlide";
import { WhatAheadSlide } from "@/components/account-health/WhatAheadSlide";
import type { AccountHealthContent, AccountHealthSignal, AccountHealthRisk } from "@/components/account-health/types";
import type { ReportRow } from "@/types/reports";
import slides from "./report-slides.module.css";

// Normalize DB data to the current schema — guards against old cached reports
// whose JSON predates field additions (I397 schema evolution).
function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

function normalizeAccountHealth(raw: Record<string, unknown>): AccountHealthContent {
  return {
    overallAssessment: (raw.overallAssessment as string) ?? "",
    healthScoreNarrative: (raw.healthScoreNarrative as string) ?? null,
    relationshipSummary: (raw.relationshipSummary as string) ?? "",
    engagementCadence: (raw.engagementCadence as string) ?? "",
    customerQuote: (raw.customerQuote as string) ?? null,
    whatIsWorking: toArr<string>(raw.whatIsWorking),
    whatIsStruggling: toArr<string>(raw.whatIsStruggling),
    expansionSignals: toArr<string>(raw.expansionSignals),
    valueDelivered: toArr<AccountHealthSignal>(raw.valueDelivered),
    risks: toArr<AccountHealthRisk>(raw.risks),
    renewalContext: (raw.renewalContext as string) ?? null,
    recommendedActions: toArr<string>(raw.recommendedActions),
    csmName: (raw.csmName as string) ?? null,
  };
}

const SLIDES = [
  { id: "cover", label: "Cover", icon: <Activity size={18} strokeWidth={1.5} /> },
  { id: "partnership", label: "The Partnership", icon: <Users size={18} strokeWidth={1.5} /> },
  { id: "where-we-stand", label: "Where We Stand", icon: <BarChart2 size={18} strokeWidth={1.5} /> },
  { id: "value-delivered", label: "Value Delivered", icon: <Star size={18} strokeWidth={1.5} /> },
  { id: "what-ahead", label: "What's Ahead", icon: <ArrowRight size={18} strokeWidth={1.5} /> },
];

const ANALYSIS_PHASES = [
  {
    key: "gathering",
    label: "Gathering account data",
    detail: "Reading meeting history, stakeholder records, and recent activity",
  },
  {
    key: "assessing",
    label: "Assessing relationship health",
    detail: "Analyzing engagement patterns and risk indicators",
  },
  {
    key: "building",
    label: "Synthesizing insights",
    detail: "Identifying what's working, what's struggling, and expansion opportunities",
  },
  {
    key: "finalizing",
    label: "Finalizing review",
    detail: "Assembling recommended actions and renewal context",
  },
];

const EDITORIAL_QUOTES = [
  "A healthy customer relationship is built on honest conversation.",
  "Value delivered is only valuable when it's visible.",
  "The best account reviews reveal what the data already knew.",
  "Trust is the most renewable resource in a customer relationship.",
];

export default function AccountHealthPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();

  const [report, setReport] = useState<ReportRow | null>(null);
  const [content, setContent] = useState<AccountHealthContent | null>(null);
  const [accountName, setAccountName] = useState("");
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced save — persists edited content to the report row
  const debouncedSave = useCallback(
    (updated: AccountHealthContent) => {
      if (!accountId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_report", {
          entityId: accountId,
          entityType: "account",
          reportType: "account_health",
          contentJson: JSON.stringify(updated),
        })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => {
            console.error("Failed to save account health report:", e);
            toast.error("Failed to save");
          });
      }, 500);
    },
    [accountId],
  );

  const updateContent = useCallback(
    (updated: AccountHealthContent) => {
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
      reportType: "account_health",
    })
      .then((data) => {
        setReport(data);
        try {
          setContent(normalizeAccountHealth(JSON.parse(data.contentJson)));
        } catch (e) {
          console.error("Failed to parse account health content:", e); // Expected: corrupted report JSON
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (account_health) failed:", err);
        toast.error("Failed to load health report");
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
      .catch((err) => console.error("get_account_detail failed:", err)); // Expected: background data fetch on mount
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
        reportType: "account_health",
      });
      setReport(data);
      setContent(normalizeAccountHealth(JSON.parse(data.contentJson)));
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate account health review");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [accountId, generating]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Account Review",
      atmosphereColor: "turmeric" as const,
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
          style={{ "--report-accent": "var(--color-spice-turmeric)" } as React.CSSProperties}
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
        style={{ "--report-accent": "var(--color-spice-turmeric)" } as React.CSSProperties}
      >
        <div className={slides.emptyOverline}>
          Account Review
        </div>
        <h2 className={slides.emptyTitle}>
          No review generated yet
        </h2>
        <p className={slides.emptyDescription}>
          Generate a 5-slide account health review. This will analyze meeting history, stakeholder
          data, recent activity, and relationship context to build a complete account picture.
        </p>
        {error && (
          <p className={slides.emptyError}>
            {error}
          </p>
        )}
        <Button onClick={handleGenerate} disabled={generating}>
          Generate Account Review
        </Button>
      </div>
    );
  }

  // Generating state
  if (generating) {
    return (
      <GeneratingProgress
        title="Building Account Review"
        accentColor="var(--color-spice-turmeric)"
        phases={ANALYSIS_PHASES}
        currentPhaseKey={
          ANALYSIS_PHASES[Math.min(Math.floor(genSeconds / 20), ANALYSIS_PHASES.length - 1)].key
        }
        quotes={EDITORIAL_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  // Render the 5-slide review with scroll-snap
  return (
    <div className={slides.slideContainer}>
      {/* Slide 1: Cover */}
      <section id="cover" className={slides.slideSection}>
        <AccountHealthCover
          accountName={accountName}
          content={content!}
          onUpdate={updateContent}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("overall_assessment")}
          onFeedback={(type) => feedback.submitFeedback("overall_assessment", type)}
        />
      </section>

      {/* Slide 2: The Partnership */}
      <div className="editorial-reveal">
        <PartnershipSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("partnership")}
          onFeedback={(type) => feedback.submitFeedback("partnership", type)}
        />
      </div>

      {/* Slide 3: Where We Stand */}
      <div className="editorial-reveal">
        <WhereWeStandSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("where_we_stand")}
          onFeedback={(type) => feedback.submitFeedback("where_we_stand", type)}
        />
      </div>

      {/* Slide 4: Value Delivered */}
      <div className="editorial-reveal">
        <ValueDeliveredSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("value_delivered")}
          onFeedback={(type) => feedback.submitFeedback("value_delivered", type)}
        />
      </div>

      {/* Slide 5: What's Ahead */}
      <div className="editorial-reveal">
        <WhatAheadSlide content={content!} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("whats_ahead")}
          onFeedback={(type) => feedback.submitFeedback("whats_ahead", type)}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
