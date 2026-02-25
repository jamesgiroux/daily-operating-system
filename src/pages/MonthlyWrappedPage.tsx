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
import {
  BookOpen,
  BarChart2,
  Star,
  Layers,
  Eye,
  Sparkles,
  Target,
  Trophy,
  ArrowRight,
  Moon,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { AnimatedNumber } from "@/components/monthly-wrapped/AnimatedNumber";
import type { ReportRow } from "@/types/reports";

// =============================================================================
// Types — new Spotify Wrapped-style schema
// =============================================================================

interface WrappedPersonality {
  typeName: string;
  description: string;
  keySignal: string;
  rarityLabel: string;
}

interface WrappedMoment {
  label: string;
  headline: string;
  subtext?: string | null;
}

interface MonthlyWrappedContent {
  monthLabel: string;
  totalConversations: number;
  totalEntitiesTouched: number;
  totalPeopleMet: number;
  signalsCaptured: number;
  topEntityName: string;
  topEntityTouches: number;
  moments: WrappedMoment[];
  hiddenPattern: string;
  personality: WrappedPersonality;
  priorityAlignmentPct: number | null;
  priorityAlignmentLabel: string | null;
  topWin: string;
  carryForward: string;
  wordOne: string;
  wordTwo: string;
  wordThree: string;
}

// =============================================================================
// Normalization — guards against schema drift in cached JSON
// =============================================================================

function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

function normalizeMonthlyWrapped(raw: Record<string, unknown>): MonthlyWrappedContent {
  const p = (raw.personality ?? {}) as Record<string, unknown>;
  return {
    monthLabel: (raw.monthLabel as string) ?? "",
    totalConversations: (raw.totalConversations as number) ?? 0,
    totalEntitiesTouched: (raw.totalEntitiesTouched as number) ?? 0,
    totalPeopleMet: (raw.totalPeopleMet as number) ?? 0,
    signalsCaptured: (raw.signalsCaptured as number) ?? 0,
    topEntityName: (raw.topEntityName as string) ?? "",
    topEntityTouches: (raw.topEntityTouches as number) ?? 0,
    moments: toArr<WrappedMoment>(raw.moments),
    hiddenPattern: (raw.hiddenPattern as string) ?? "",
    personality: {
      typeName: (p.typeName as string) ?? "",
      description: (p.description as string) ?? "",
      keySignal: (p.keySignal as string) ?? "",
      rarityLabel: (p.rarityLabel as string) ?? "",
    },
    priorityAlignmentPct: (raw.priorityAlignmentPct as number) ?? null,
    priorityAlignmentLabel: (raw.priorityAlignmentLabel as string) ?? null,
    topWin: (raw.topWin as string) ?? "",
    carryForward: (raw.carryForward as string) ?? "",
    wordOne: (raw.wordOne as string) ?? "",
    wordTwo: (raw.wordTwo as string) ?? "",
    wordThree: (raw.wordThree as string) ?? "",
  };
}

// =============================================================================
// Slide registry
// =============================================================================

const SLIDES = [
  { id: "splash",          label: "Cover",              icon: <BookOpen size={18} strokeWidth={1.5} /> },
  { id: "volume",          label: "You Showed Up",      icon: <BarChart2 size={18} strokeWidth={1.5} /> },
  { id: "top-entity",      label: "Top Account",        icon: <Star size={18} strokeWidth={1.5} /> },
  { id: "moments",         label: "The Moments",        icon: <Layers size={18} strokeWidth={1.5} /> },
  { id: "hidden-pattern",  label: "You Missed This",    icon: <Eye size={18} strokeWidth={1.5} /> },
  { id: "personality",     label: "Your Type",          icon: <Sparkles size={18} strokeWidth={1.5} /> },
  { id: "priority",        label: "Priority Check",     icon: <Target size={18} strokeWidth={1.5} /> },
  { id: "top-win",         label: "Your Win",           icon: <Trophy size={18} strokeWidth={1.5} /> },
  { id: "carry-forward",   label: "Carry Forward",      icon: <ArrowRight size={18} strokeWidth={1.5} /> },
  { id: "close",           label: "See You Next Month", icon: <Moon size={18} strokeWidth={1.5} /> },
];

const ANALYSIS_PHASES = [
  {
    key: "counting",
    label: "Counting up your month",
    detail: "Tallying conversations, signals, and relationship touches",
  },
  {
    key: "moments",
    label: "Finding your moments",
    detail: "Surfacing firsts, peaks, and memorable interactions",
  },
  {
    key: "pattern",
    label: "Reading the pattern",
    detail: "Looking for what you might have missed",
  },
  {
    key: "type",
    label: "Assigning your type",
    detail: "This one is personal",
  },
  {
    key: "wrapping",
    label: "Wrapping it up",
    detail: "Almost done",
  },
];

const EDITORIAL_QUOTES = [
  "Every conversation leaves a trace.",
  "The months you show up are the ones that compound.",
  "Your relationships are your record.",
  "Consistency is invisible until it isn't.",
];

// =============================================================================
// Slide-active hook — fires once when a slide enters viewport
// =============================================================================

function useSlideActive(id: string) {
  const [active, setActive] = useState(false);
  useEffect(() => {
    const el = document.getElementById(id);
    if (!el) return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) setActive(true);
      },
      { threshold: 0.3 },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [id]);
  return active;
}

// =============================================================================
// Helper — next month label
// =============================================================================

function nextMonthName(monthLabel: string): string {
  const months = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
  ];
  const found = months.find((m) => monthLabel.toLowerCase().startsWith(m.toLowerCase()));
  if (!found) return "next month";
  const idx = months.indexOf(found);
  return months[(idx + 1) % 12];
}

// =============================================================================
// Slide components
// =============================================================================

function SplashSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("splash");

  const year = content.monthLabel
    ? content.monthLabel.split(" ").find((p) => /^\d{4}$/.test(p)) ?? ""
    : "";
  const month = content.monthLabel
    ? content.monthLabel.replace(year, "").trim()
    : content.monthLabel;

  return (
    <section
      id="splash"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-desk-ink)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Background texture — subtle grain effect */}
      <div
        aria-hidden
        style={{
          position: "absolute",
          inset: 0,
          background:
            "radial-gradient(ellipse at 20% 80%, rgba(107, 168, 164, 0.08) 0%, transparent 60%), " +
            "radial-gradient(ellipse at 80% 20%, rgba(201, 162, 39, 0.06) 0%, transparent 50%)",
          pointerEvents: "none",
        }}
      />

      <div style={{ position: "relative", zIndex: 1 }}>
        {/* Overline */}
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.14em",
            textTransform: "uppercase",
            color: "rgba(255,255,255,0.4)",
            marginBottom: 24,
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
            animationDelay: "0ms",
          }}
        >
          Monthly Wrapped
        </div>

        {/* Month name */}
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: "clamp(64px, 9vw, 120px)",
            fontWeight: 300,
            lineHeight: 0.95,
            color: "#ffffff",
            margin: "0 0 20px",
            letterSpacing: "-0.02em",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.55s ease forwards" : "none",
            animationDelay: "200ms",
          }}
        >
          {month}
        </h1>

        {/* Year */}
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: "clamp(28px, 3.5vw, 48px)",
            fontWeight: 700,
            color: "var(--color-spice-turmeric)",
            letterSpacing: "0.04em",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.55s ease forwards" : "none",
            animationDelay: "400ms",
          }}
        >
          {year}
        </div>
      </div>

      <WrappedKeyframes />
    </section>
  );
}

function VolumeSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("volume");

  return (
    <section
      id="volume"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-garden-eucalyptus)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "rgba(255,255,255,0.7)",
          marginBottom: 32,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        You showed up.
      </div>

      {/* Big animated number */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: "clamp(72px, 10vw, 96px)",
          fontWeight: 700,
          lineHeight: 1,
          color: "#ffffff",
          marginBottom: 8,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedFadeIn 0.4s ease forwards" : "none",
          animationDelay: "100ms",
        }}
      >
        <AnimatedNumber value={content.totalConversations} duration={1500} />
      </div>

      {/* Label */}
      <div
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 28,
          fontWeight: 300,
          color: "rgba(255,255,255,0.9)",
          marginBottom: 32,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "200ms",
        }}
      >
        conversations
      </div>

      {/* Sub-stats row */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 13,
          color: "rgba(255,255,255,0.65)",
          letterSpacing: "0.04em",
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "400ms",
        }}
      >
        <AnimatedNumber value={content.totalEntitiesTouched} /> accounts
        &nbsp;&middot;&nbsp;
        <AnimatedNumber value={content.totalPeopleMet} /> people
        &nbsp;&middot;&nbsp;
        <AnimatedNumber value={content.signalsCaptured} /> signals
      </div>
    </section>
  );
}

function TopEntitySlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("top-entity");

  return (
    <section
      id="top-entity"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-spice-turmeric)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "rgba(42,43,61,0.55)",
          marginBottom: 32,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        Your most active account.
      </div>

      {/* Entity name */}
      <h2
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: "clamp(48px, 7vw, 80px)",
          fontWeight: 300,
          lineHeight: 1.05,
          color: "var(--color-desk-ink)",
          margin: "0 0 24px",
          letterSpacing: "-0.01em",
          opacity: active ? 1 : 0,
          animation: active ? "wrappedScaleReveal 0.6s ease forwards" : "none",
          animationDelay: "150ms",
        }}
      >
        {content.topEntityName}
      </h2>

      {/* Touch count */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 16,
          color: "rgba(42,43,61,0.65)",
          letterSpacing: "0.02em",
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "350ms",
        }}
      >
        <AnimatedNumber value={content.topEntityTouches} /> touchpoints this month
      </div>
    </section>
  );
}

function MomentsSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("moments");

  return (
    <section
      id="moments"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-paper-warm-white)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 700,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "var(--color-spice-turmeric)",
          marginBottom: 40,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        The moments.
      </div>

      {/* Moments list */}
      <div style={{ maxWidth: 680 }}>
        {content.moments.map((moment, i) => (
          <div
            key={i}
            style={{
              paddingBottom: i < content.moments.length - 1 ? 28 : 0,
              marginBottom: i < content.moments.length - 1 ? 28 : 0,
              borderBottom:
                i < content.moments.length - 1
                  ? "1px solid var(--color-rule-heavy)"
                  : "none",
              opacity: active ? 1 : 0,
              animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
              animationDelay: `${i * 200}ms`,
            }}
          >
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 700,
                letterSpacing: "0.14em",
                textTransform: "uppercase",
                color: "var(--color-spice-turmeric)",
                marginBottom: 6,
              }}
            >
              {moment.label}
            </div>
            <div
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 22,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                lineHeight: 1.35,
                marginBottom: moment.subtext ? 6 : 0,
              }}
            >
              {moment.headline}
            </div>
            {moment.subtext && (
              <div
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 14,
                  color: "var(--color-text-tertiary)",
                  lineHeight: 1.5,
                }}
              >
                {moment.subtext}
              </div>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}

function HiddenPatternSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("hidden-pattern");

  return (
    <section
      id="hidden-pattern"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-spice-terracotta)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "rgba(255,255,255,0.55)",
          marginBottom: 40,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        Something you might have missed.
      </div>

      {/* The pattern */}
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 28,
          fontStyle: "italic",
          fontWeight: 300,
          color: "#ffffff",
          lineHeight: 1.6,
          maxWidth: 700,
          margin: 0,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.6s ease forwards" : "none",
          animationDelay: "200ms",
        }}
      >
        {content.hiddenPattern}
      </p>
    </section>
  );
}

function PersonalitySlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("personality");

  return (
    <section
      id="personality"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-desk-espresso)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Subtle radial glow behind the type name */}
      <div
        aria-hidden
        style={{
          position: "absolute",
          top: "40%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          width: 600,
          height: 400,
          background:
            "radial-gradient(ellipse at center, rgba(222, 184, 65, 0.12) 0%, transparent 70%)",
          pointerEvents: "none",
        }}
      />

      <div style={{ position: "relative", zIndex: 1 }}>
        {/* Small label */}
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.14em",
            textTransform: "uppercase",
            color: "rgba(255,255,255,0.3)",
            marginBottom: 28,
            opacity: active ? 1 : 0,
            animation: active ? "wrappedFadeIn 0.4s ease forwards" : "none",
            animationDelay: "0ms",
          }}
        >
          Your type this month
        </div>

        {/* THE VIRAL MOMENT — type name */}
        <h2
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: "clamp(48px, 7vw, 80px)",
            fontWeight: 300,
            lineHeight: 1.05,
            color: "var(--color-spice-saffron)",
            margin: "0 0 24px",
            letterSpacing: "-0.01em",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedScaleReveal 0.6s ease forwards" : "none",
            animationDelay: "150ms",
          }}
        >
          {content.personality.typeName}
        </h2>

        {/* Description */}
        <p
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 18,
            fontWeight: 300,
            color: "rgba(255,255,255,0.88)",
            lineHeight: 1.6,
            maxWidth: 600,
            margin: "0 0 16px",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
            animationDelay: "400ms",
          }}
        >
          {content.personality.description}
        </p>

        {/* Key signal */}
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            color: "rgba(255,255,255,0.45)",
            marginBottom: 24,
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
            animationDelay: "550ms",
          }}
        >
          {content.personality.keySignal}
        </div>

        {/* Rarity label */}
        <div
          style={{
            borderTop: "1px solid rgba(255,255,255,0.12)",
            paddingTop: 16,
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.1em",
            textTransform: "uppercase",
            color: "var(--color-spice-turmeric)",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
            animationDelay: "700ms",
          }}
        >
          {content.personality.rarityLabel}
        </div>
      </div>
    </section>
  );
}

function PrioritySlide({
  content,
  onNavigateToMe,
}: {
  content: MonthlyWrappedContent;
  onNavigateToMe: () => void;
}) {
  const active = useSlideActive("priority");
  const hasPriority = content.priorityAlignmentPct !== null;

  // Badge style based on alignment label
  const labelStyle = (): React.CSSProperties => {
    const label = (content.priorityAlignmentLabel ?? "").toLowerCase();
    if (label.includes("on track") || label.includes("strong")) {
      return {
        background: "rgba(126, 170, 123, 0.25)",
        color: "#c8e6c5",
        border: "1px solid rgba(126, 170, 123, 0.4)",
      };
    }
    if (label.includes("worth") || label.includes("look")) {
      return {
        background: "rgba(201, 162, 39, 0.25)",
        color: "#ffe082",
        border: "1px solid rgba(201, 162, 39, 0.4)",
      };
    }
    return {
      background: "rgba(255,255,255,0.1)",
      color: "rgba(255,255,255,0.75)",
      border: "1px solid rgba(255,255,255,0.2)",
    };
  };

  return (
    <section
      id="priority"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-garden-sage)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "rgba(255,255,255,0.65)",
          marginBottom: 40,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        Were you spending time where it matters?
      </div>

      {hasPriority ? (
        <>
          {/* Big percentage */}
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: "clamp(72px, 10vw, 96px)",
              fontWeight: 700,
              lineHeight: 1,
              color: "#ffffff",
              marginBottom: 8,
              opacity: active ? 1 : 0,
              animation: active ? "wrappedFadeIn 0.4s ease forwards" : "none",
              animationDelay: "100ms",
            }}
          >
            <AnimatedNumber value={content.priorityAlignmentPct!} duration={1200} />%
          </div>

          {/* Description */}
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 22,
              fontWeight: 300,
              color: "rgba(255,255,255,0.9)",
              marginBottom: 24,
              opacity: active ? 1 : 0,
              animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
              animationDelay: "300ms",
            }}
          >
            of relationship energy on priority accounts
          </div>

          {/* Alignment badge */}
          {content.priorityAlignmentLabel && (
            <div
              style={{
                display: "inline-flex",
                alignItems: "center",
                padding: "6px 14px",
                borderRadius: 4,
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 700,
                letterSpacing: "0.1em",
                textTransform: "uppercase",
                opacity: active ? 1 : 0,
                animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
                animationDelay: "500ms",
                ...labelStyle(),
              }}
            >
              {content.priorityAlignmentLabel}
            </div>
          )}
        </>
      ) : (
        /* No priorities set */
        <div
          style={{
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
            animationDelay: "100ms",
          }}
        >
          <p
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 24,
              fontWeight: 300,
              color: "rgba(255,255,255,0.9)",
              lineHeight: 1.5,
              maxWidth: 520,
              marginBottom: 28,
            }}
          >
            Set priorities on your profile to track alignment month over month.
          </p>
          <button
            onClick={onNavigateToMe}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 700,
              letterSpacing: "0.1em",
              textTransform: "uppercase",
              color: "rgba(255,255,255,0.9)",
              background: "none",
              border: "1px solid rgba(255,255,255,0.4)",
              borderRadius: 4,
              padding: "8px 20px",
              cursor: "pointer",
            }}
          >
            Go to /me
          </button>
        </div>
      )}
    </section>
  );
}

function TopWinSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("top-win");

  return (
    <section
      id="top-win"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-spice-saffron)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "rgba(42,43,61,0.5)",
          marginBottom: 40,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        Your biggest win.
      </div>

      {/* Win text */}
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 28,
          fontStyle: "italic",
          fontWeight: 300,
          color: "var(--color-desk-ink)",
          lineHeight: 1.6,
          maxWidth: 700,
          margin: "0 0 32px",
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.55s ease forwards" : "none",
          animationDelay: "200ms",
        }}
      >
        {content.topWin}
      </p>

      {/* Flourish */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 20,
          color: "var(--color-spice-turmeric)",
          opacity: active ? 1 : 0,
          animation: active ? "wrappedFadeIn 0.5s ease forwards" : "none",
          animationDelay: "600ms",
        }}
      >
        &#10022;
      </div>
    </section>
  );
}

function CarryForwardSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("carry-forward");

  const pillColors = [
    { border: "var(--color-spice-turmeric)", color: "var(--color-text-primary)" },
    { border: "var(--color-garden-larkspur)", color: "var(--color-text-primary)" },
    { border: "var(--color-garden-eucalyptus)", color: "var(--color-text-primary)" },
  ];
  const words = [content.wordOne, content.wordTwo, content.wordThree].filter(Boolean);

  return (
    <section
      id="carry-forward"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-paper-linen)",
      }}
    >
      {/* Overline */}
      <div
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          fontWeight: 600,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
          marginBottom: 40,
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
          animationDelay: "0ms",
        }}
      >
        Into next month.
      </div>

      {/* Carry forward text */}
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 22,
          fontWeight: 300,
          color: "var(--color-text-primary)",
          lineHeight: 1.7,
          maxWidth: 640,
          margin: "0 0 56px",
          opacity: active ? 1 : 0,
          animation: active ? "wrappedSlideUp 0.55s ease forwards" : "none",
          animationDelay: "200ms",
        }}
      >
        {content.carryForward}
      </p>

      {/* Three words */}
      {words.length > 0 && (
        <div
          style={{
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
            animationDelay: "400ms",
          }}
        >
          <div
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 700,
              letterSpacing: "0.14em",
              textTransform: "uppercase",
              color: "var(--color-text-tertiary)",
              marginBottom: 16,
            }}
          >
            {content.monthLabel} in three words
          </div>
          <div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
            {words.map((word, i) => (
              <span
                key={i}
                style={{
                  fontFamily: "var(--font-serif)",
                  fontSize: 16,
                  fontWeight: 400,
                  color: pillColors[i % pillColors.length].color,
                  border: `1px solid ${pillColors[i % pillColors.length].border}`,
                  borderRadius: 4,
                  padding: "8px 16px",
                  opacity: active ? 1 : 0,
                  animation: active ? "wrappedSlideUp 0.5s ease forwards" : "none",
                  animationDelay: `${400 + i * 120}ms`,
                }}
              >
                {word}
              </span>
            ))}
          </div>
        </div>
      )}
    </section>
  );
}

function CloseSlide({ content }: { content: MonthlyWrappedContent }) {
  const active = useSlideActive("close");
  const next = nextMonthName(content.monthLabel);

  return (
    <section
      id="close"
      style={{
        minHeight: "100vh",
        scrollSnapAlign: "start",
        scrollMarginTop: 60,
        display: "flex",
        flexDirection: "column",
        justifyContent: "center",
        padding: "120px 120px 80px",
        background: "var(--color-desk-ink)",
        position: "relative",
        overflow: "hidden",
      }}
    >
      {/* Subtle background glow */}
      <div
        aria-hidden
        style={{
          position: "absolute",
          inset: 0,
          background:
            "radial-gradient(ellipse at 80% 30%, rgba(107, 168, 164, 0.06) 0%, transparent 60%)",
          pointerEvents: "none",
        }}
      />

      <div style={{ position: "relative", zIndex: 1 }}>
        {/* Main headline */}
        <h2
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: "clamp(40px, 5.5vw, 72px)",
            fontWeight: 300,
            lineHeight: 1.1,
            color: "#ffffff",
            margin: "0 0 24px",
            letterSpacing: "-0.01em",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedSlideUp 0.6s ease forwards" : "none",
            animationDelay: "0ms",
          }}
        >
          See you in {next}.
        </h2>

        {/* Sub-label */}
        <div
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 14,
            color: "rgba(255,255,255,0.3)",
            letterSpacing: "0.04em",
            marginBottom: 48,
            opacity: active ? 1 : 0,
            animation: active ? "wrappedFadeIn 0.5s ease forwards" : "none",
            animationDelay: "300ms",
          }}
        >
          Your {content.monthLabel}.
        </div>

        {/* Export button */}
        <button
          onClick={() => window.print()}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 700,
            letterSpacing: "0.1em",
            textTransform: "uppercase",
            color: "rgba(255,255,255,0.6)",
            background: "none",
            border: "1px solid rgba(255,255,255,0.2)",
            borderRadius: 4,
            padding: "8px 20px",
            cursor: "pointer",
            opacity: active ? 1 : 0,
            animation: active ? "wrappedFadeIn 0.5s ease forwards" : "none",
            animationDelay: "600ms",
          }}
        >
          Save as PDF
        </button>
      </div>
    </section>
  );
}

// =============================================================================
// Shared keyframes — injected once at root level via a component
// =============================================================================

let keyframesInjected = false;

function WrappedKeyframes() {
  if (keyframesInjected) return null;
  keyframesInjected = true;
  return (
    <style>{`
      @keyframes wrappedSlideUp {
        from { opacity: 0; transform: translateY(32px); }
        to   { opacity: 1; transform: translateY(0); }
      }
      @keyframes wrappedFadeIn {
        from { opacity: 0; }
        to   { opacity: 1; }
      }
      @keyframes wrappedScaleReveal {
        from { opacity: 0; transform: scale(0.92); }
        to   { opacity: 1; transform: scale(1); }
      }
    `}</style>
  );
}

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
        console.error("get_user_entity failed:", err);
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
          console.error("Failed to parse monthly_wrapped content:", e);
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (monthly_wrapped) failed:", err);
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
          .catch((e) => console.error("Failed to save monthly wrapped:", e));
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
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase" as const,
            color: generating
              ? "var(--color-text-tertiary)"
              : "var(--color-garden-eucalyptus)",
            background: "none",
            border: `1px solid ${generating ? "var(--color-rule-light)" : "var(--color-garden-eucalyptus)"}`,
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
      <div style={{ padding: "120px 120px 80px" }}>
        <Skeleton className="mb-4 h-4 w-24" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton className="mb-2 h-12 w-96" style={{ background: "var(--color-rule-light)" }} />
        <Skeleton
          className="mb-8 h-5 w-full max-w-2xl"
          style={{ background: "var(--color-rule-light)" }}
        />
      </div>
    );
  }

  // Empty state
  if (!content && !generating) {
    const monthGuess = new Date().toLocaleString("default", { month: "long", year: "numeric" });
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
            color: "var(--color-garden-eucalyptus)",
            marginBottom: 24,
          }}
        >
          Monthly Wrapped
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
          Your {monthGuess} Wrapped isn&apos;t ready yet
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
          Generate it to see how your month looked — personality type, biggest moments, hidden
          patterns, and more.
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
    <div style={{ scrollSnapType: "y proximity" }}>
      <WrappedKeyframes />

      {/* Slide 1 — Splash */}
      <SplashSlide content={content!} />

      {/* Slide 2 — Volume */}
      <VolumeSlide content={content!} />

      {/* Slide 3 — Top Entity */}
      <TopEntitySlide content={content!} />

      {/* Slide 4 — Moments */}
      <MomentsSlide content={content!} />

      {/* Slide 5 — Hidden Pattern */}
      <HiddenPatternSlide content={content!} />

      {/* Slide 6 — Personality */}
      <PersonalitySlide content={content!} />

      {/* Slide 7 — Priority */}
      <PrioritySlide
        content={content!}
        onNavigateToMe={() => navigate({ to: "/me" })}
      />

      {/* Slide 8 — Top Win */}
      <TopWinSlide content={content!} />

      {/* Slide 9 — Carry Forward */}
      <CarryForwardSlide content={content!} />

      {/* Slide 10 — Close */}
      <CloseSlide content={content!} />
    </div>
  );
}
