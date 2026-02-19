/**
 * RiskBriefingPage — 6-slide executive risk briefing for an account.
 * Uses the magazine shell with terracotta atmosphere.
 * Keyboard navigation: arrow keys for next/prev, number keys 1-6 for direct jump.
 * Scroll-snap settles on slide boundaries.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useParams, useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
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
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { RiskCover } from "@/components/risk-briefing/RiskCover";
import { BottomLineSlide } from "@/components/risk-briefing/BottomLineSlide";
import { WhatHappenedSlide } from "@/components/risk-briefing/WhatHappenedSlide";
import { StakesSlide } from "@/components/risk-briefing/StakesSlide";
import { ThePlanSlide } from "@/components/risk-briefing/ThePlanSlide";
import { TheAskSlide } from "@/components/risk-briefing/TheAskSlide";
import type {
  RiskBriefing,
  RiskBottomLine,
  RiskWhatHappened,
  RiskStakes,
  RiskThePlan,
  RiskTheAsk,
} from "@/types";

const SLIDES = [
  { id: "cover", label: "Cover", icon: <AlignLeft size={18} strokeWidth={1.5} /> },
  { id: "bottom-line", label: "Bottom Line", icon: <Crosshair size={18} strokeWidth={1.5} /> },
  { id: "what-happened", label: "What Happened", icon: <BookOpen size={18} strokeWidth={1.5} /> },
  { id: "stakes", label: "The Stakes", icon: <TrendingDown size={18} strokeWidth={1.5} /> },
  { id: "the-plan", label: "The Plan", icon: <Target size={18} strokeWidth={1.5} /> },
  { id: "the-ask", label: "The Ask", icon: <Hand size={18} strokeWidth={1.5} /> },
];

export default function RiskBriefingPage() {
  const { accountId } = useParams({ strict: false });
  const navigate = useNavigate();
  const [briefing, setBriefing] = useState<RiskBriefing | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Debounced save — persists edited briefing to disk
  const debouncedSave = useCallback(
    (updated: RiskBriefing) => {
      if (!accountId) return;
      if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
      saveTimerRef.current = setTimeout(() => {
        invoke("save_risk_briefing", { accountId, briefing: updated })
          .then(() => {
            setSaveStatus("saved");
            if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
            fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
          })
          .catch((e) => console.error("Failed to save risk briefing:", e));
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

  // Load cached briefing on mount
  useEffect(() => {
    if (!accountId) return;
    setLoading(true);
    invoke<RiskBriefing>("get_risk_briefing", { accountId })
      .then((data) => {
        setBriefing(data);
        setError(null);
      })
      .catch(() => {
        setBriefing(null);
      })
      .finally(() => setLoading(false));
  }, [accountId]);

  // Generate handler
  const handleGenerate = useCallback(async () => {
    if (!accountId || generating) return;
    setBriefing(null);
    setGenerating(true);
    setGenSeconds(0);
    setError(null);
    window.scrollTo({ top: 0, behavior: "instant" });

    timerRef.current = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const data = await invoke<RiskBriefing>("generate_risk_briefing", { accountId });
      setBriefing(data);
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate risk briefing");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [accountId, generating]);

  // Register magazine shell (after handleGenerate so folioActions can reference it)
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Risk Briefing",
      atmosphereColor: "terracotta" as const,
      activePage: "accounts" as const,
      backLink: {
        label: "Back",
        onClick: () => window.history.length > 1 ? window.history.back() : navigate({ to: "/accounts/$accountId", params: { accountId: accountId! } }),
      },
      chapters: briefing ? SLIDES : undefined,
      folioStatusText: saveStatus === "saved" ? "✓ Saved" : undefined,
      folioActions: briefing ? (
        <button
          onClick={handleGenerate}
          disabled={generating}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase" as const,
            color: generating ? "var(--color-text-tertiary)" : "var(--color-spice-terracotta)",
            background: "none",
            border: `1px solid ${generating ? "var(--color-rule-light)" : "var(--color-spice-terracotta)"}`,
            borderRadius: 4,
            padding: "2px 10px",
            cursor: generating ? "not-allowed" : "pointer",
            opacity: generating ? 0.5 : 1,
          }}
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
      <div style={{ padding: "120px 120px 80px" }}>
        <Skeleton className="mb-4 h-4 w-24" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton className="mb-2 h-12 w-96" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton className="mb-8 h-5 w-full max-w-2xl" style={{ background: "var(--color-rule-light)" }} />
      </div>
    );
  }

  // Empty state
  if (!briefing && !generating) {
    return (
      <div
        style={{
          padding: "120px 120px 80px",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "60vh",
          textAlign: "center",
        }}
      >
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            textTransform: "uppercase",
            letterSpacing: "0.12em",
            color: "var(--color-spice-terracotta)",
            marginBottom: 24,
          }}
        >
          Risk Briefing
        </div>
        <h2
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 32,
            fontWeight: 400,
            color: "var(--color-text-primary)",
            margin: "0 0 16px",
          }}
        >
          No briefing generated yet
        </h2>
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            color: "var(--color-text-secondary)",
            maxWidth: 420,
            lineHeight: 1.6,
            marginBottom: 32,
          }}
        >
          Generate a 6-slide executive briefing. This will analyze all available intelligence, meeting history, and stakeholder data.
        </p>
        {error && (
          <p
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-spice-terracotta)",
              marginBottom: 16,
            }}
          >
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
  if (generating) {
    return (
      <GeneratingProgress
        title="Building Risk Briefing"
        accentColor="var(--color-spice-terracotta)"
        phases={ANALYSIS_PHASES}
        currentPhaseKey={ANALYSIS_PHASES[Math.min(Math.floor(genSeconds / 20), ANALYSIS_PHASES.length - 1)].key}
        quotes={EDITORIAL_QUOTES}
        elapsed={genSeconds}
      />
    );
  }

  // Render the 6-slide briefing with scroll-snap
  return (
    <div style={{ scrollSnapType: "y proximity" }}>
      {/* Slide 1: Cover */}
      <section id="cover" style={{ scrollMarginTop: 60 }}>
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
      </div>

      {/* Slide 3: What Happened */}
      <div className="editorial-reveal">
        <WhatHappenedSlide
          data={briefing!.whatHappened}
          onUpdate={(v: RiskWhatHappened) => updateSlide("whatHappened", v)}
        />
      </div>

      {/* Slide 4: The Stakes */}
      <div className="editorial-reveal">
        <StakesSlide
          data={briefing!.stakes}
          onUpdate={(v: RiskStakes) => updateSlide("stakes", v)}
        />
      </div>

      {/* Slide 5: The Plan */}
      <div className="editorial-reveal">
        <ThePlanSlide
          data={briefing!.thePlan}
          onUpdate={(v: RiskThePlan) => updateSlide("thePlan", v)}
        />
      </div>

      {/* Slide 6: The Ask */}
      <div className="editorial-reveal">
        <TheAskSlide
          data={briefing!.theAsk}
          onUpdate={(v: RiskTheAsk) => updateSlide("theAsk", v)}
        />
      </div>

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={briefing!.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}

// =============================================================================
// Generating Progress Splash
// =============================================================================

const ANALYSIS_PHASES = [
  { key: "gathering", label: "Gathering intelligence", detail: "Reading account data, meeting history, and stakeholder signals" },
  { key: "reading", label: "Reading the room", detail: "Analyzing stakeholder dynamics and relationship signals" },
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

