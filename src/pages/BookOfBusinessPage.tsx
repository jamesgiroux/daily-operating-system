/**
 * BookOfBusinessPage — Template-aligned slide-deck portfolio review for leadership.
 * 14 slides matching the BoB review template. Full-viewport slides with scroll-snap.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  FileText, AlertTriangle, Shield, TrendingUp, Target,
  Calendar, MessageSquare, Layers, Check,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { AccountListItem } from "@/types";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { useIntelligenceFeedback } from "@/hooks/useIntelligenceFeedback";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { CoverSlide } from "@/components/book-of-business/CoverSlide";
import { HealthOverviewSlide } from "@/components/book-of-business/HealthOverviewSlide";
import { RiskTableSlide } from "@/components/book-of-business/RiskTableSlide";
import { RetentionDeepDiveSlide } from "@/components/book-of-business/RetentionDeepDiveSlide";
import { SaveMotionsSlide } from "@/components/book-of-business/SaveMotionsSlide";
import { ExpansionSlide } from "@/components/book-of-business/ExpansionSlide";
import { YearEndSlide } from "@/components/book-of-business/YearEndSlide";
import { LeadershipAsksSlide } from "@/components/book-of-business/LeadershipAsksSlide";
import { AccountFocusSlide } from "@/components/book-of-business/AccountFocusSlide";
import { QuarterlyFocusSlide } from "@/components/book-of-business/QuarterlyFocusSlide";
import { ThemesSlide } from "@/components/book-of-business/ThemesSlide";
import type {
  ReportRow,
  BookOfBusinessContent,
} from "@/types/reports";
import slides from "./report-slides.module.css";

// =============================================================================
// Normalization — guards against cached reports with old schema
// =============================================================================

function toArr<T>(v: unknown): T[] {
  return Array.isArray(v) ? (v as T[]) : [];
}

function normalizeBookOfBusiness(raw: Record<string, unknown>): BookOfBusinessContent {
  return {
    // Slide 1: Executive Summary
    periodLabel: (raw.periodLabel as string) ?? "",
    executiveSummary: ((raw.executiveSummary ?? raw.executiveNarrative) as string) ?? "",
    totalAccounts: (raw.totalAccounts as number) ?? 0,
    totalArr: (raw.totalArr as number) ?? 0,
    atRiskArr: (raw.atRiskArr as number) ?? 0,
    committedExpansion: (raw.committedExpansion as number) ?? 0,
    projectedChurn: (raw.projectedChurn as number) ?? 0,
    topRisksSummary: toArr<string>(raw.topRisksSummary),
    topOpportunitiesSummary: toArr<string>(raw.topOpportunitiesSummary),
    biggestRisk: (raw.biggestRisk as BookOfBusinessContent["biggestRisk"]) ?? null,
    biggestUpside: (raw.biggestUpside as BookOfBusinessContent["biggestUpside"]) ?? null,
    eltHelpRequired: (raw.eltHelpRequired as boolean) ?? false,
    // Slide 2: Health Overview
    healthOverview: (raw.healthOverview as BookOfBusinessContent["healthOverview"]) ?? {
      healthyCount: 0, healthyArr: 0, mediumCount: 0, mediumArr: 0,
      highRiskCount: 0, highRiskArr: 0, secureArr: 0,
      renewals90d: 0, renewals90dArr: 0, renewals180d: 0, renewals180dArr: 0,
    },
    // Slide 3: Risk Table
    riskAccounts: toArr(raw.riskAccounts),
    // Slide 4: Retention Deep Dives
    retentionRiskDeepDives: toArr(raw.retentionRiskDeepDives),
    // Slide 5: Save Motions
    saveMotions: toArr(raw.saveMotions),
    // Slide 6: Expansion
    expansionAccounts: toArr(raw.expansionAccounts),
    // Slide 7: Expansion Readiness
    expansionReadiness: toArr(raw.expansionReadiness),
    // Slide 8: Year-End Outlook
    yearEndOutlook: (raw.yearEndOutlook as BookOfBusinessContent["yearEndOutlook"]) ?? {
      startingArr: 0, atRiskArr: 0, committedExpansion: 0, expectedChurn: 0, projectedEoyArr: 0,
    },
    // Slide 9: Landing Scenarios
    landingScenarios: (raw.landingScenarios as BookOfBusinessContent["landingScenarios"]) ?? {
      best: { keyAssumptions: "", attrition: "", expansion: "", notes: "" },
      expected: { keyAssumptions: "", attrition: "", expansion: "", notes: "" },
      worst: { keyAssumptions: "", attrition: "", expansion: "", notes: "" },
    },
    // Slide 10+14: Leadership Asks
    leadershipAsks: toArr(raw.leadershipAsks),
    // Slide 11: Account Focus
    accountFocus: toArr(raw.accountFocus),
    // Slide 12: Quarterly Focus
    quarterlyFocus: (raw.quarterlyFocus as BookOfBusinessContent["quarterlyFocus"]) ?? {
      retention: [], expansion: [], execution: [],
    },
    // Slide 13: Key Themes
    keyThemes: toArr(raw.keyThemes ?? raw.themes),
    // Account snapshot
    accountSnapshot: toArr(raw.accountSnapshot ?? raw.snapshot),
  };
}

// =============================================================================
// Progress config
// =============================================================================

const BOB_PHASES = [
  { key: "gathering", label: "Gathering portfolio data", detail: "Reading account health, meeting history, and renewal context" },
  { key: "glean", label: "Fetching enterprise insights", detail: "Querying Glean for cross-system context" },
  { key: "healthOverview", label: "Computing health overview", detail: "Calculating risk tiers and ARR weights" },
  { key: "riskAccounts", label: "Building risk table", detail: "Identifying at-risk accounts and drivers" },
  { key: "expansionAccounts", label: "Mapping expansion", detail: "Finding growth opportunities" },
  { key: "yearEndOutlook", label: "Projecting year-end", detail: "Computing ARR outlook" },
  { key: "synthesis", label: "Generating analysis", detail: "AI synthesizing narrative sections" },
];

const EDITORIAL_QUOTES = [
  "A portfolio is a story told across many accounts.",
  "The best reviews surface what the individual reports hide.",
  "Patterns across accounts reveal more than any single health score.",
  "Your book is a living document, not a static spreadsheet.",
];

// =============================================================================
// Page component
// =============================================================================

export default function BookOfBusinessPage() {
  const navigate = useNavigate();

  const [userId, setUserId] = useState<string | null>(null);
  const [report, setReport] = useState<ReportRow | null>(null);
  const [content, setContent] = useState<BookOfBusinessContent | null>(null);
  const [loading, setLoading] = useState(true);
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [genSeconds, setGenSeconds] = useState(0);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saved">("idle");
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [selectedSpotlights, setSelectedSpotlights] = useState<Set<string>>(new Set());
  const [completedSections, setCompletedSections] = useState<Set<string>>(new Set());
  const [currentPhaseKey, setCurrentPhaseKey] = useState<string>("gathering");

  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const saveTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const fadeTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Fetch user entity id on mount
  useEffect(() => {
    invoke<{ id: string | number }>("get_user_entity")
      .then((u) => setUserId(String(u.id)))
      .catch((err) => console.error("get_user_entity failed:", err));
  }, []);

  // Fetch accounts for spotlight picker (parents + all children)
  useEffect(() => {
    (async () => {
      try {
        const topLevel = await invoke<AccountListItem[]>("get_accounts_list");
        const customers = topLevel.filter((a) => !a.archived && a.accountType === "customer");
        const parents = customers.filter((a) => a.isParent && a.childCount > 0);
        const childLists = await Promise.all(
          parents.map((p) =>
            invoke<AccountListItem[]>("get_child_accounts_list", { parentId: p.id })
              .then((children) => children.filter((c) => !c.archived))
              .catch(() => [] as AccountListItem[]),
          ),
        );
        const allChildren = childLists.flat();
        setAccounts([...customers, ...allChildren]);
      } catch (err) {
        console.error("Failed to fetch accounts for picker:", err);
      }
    })();
  }, []);

  // Debounced save
  const debouncedSave = useCallback((updated: BookOfBusinessContent) => {
    if (!userId) return;
    if (saveTimerRef.current) clearTimeout(saveTimerRef.current);
    saveTimerRef.current = setTimeout(() => {
      invoke("save_report", {
        entityId: userId,
        entityType: "user",
        reportType: "book_of_business",
        contentJson: JSON.stringify(updated),
      })
        .then(() => {
          setSaveStatus("saved");
          if (fadeTimerRef.current) clearTimeout(fadeTimerRef.current);
          fadeTimerRef.current = setTimeout(() => setSaveStatus("idle"), 2000);
        })
        .catch((e) => console.error("Failed to save book of business report:", e));
    }, 500);
  }, [userId]);

  const updateContent = useCallback(
    (updated: BookOfBusinessContent) => {
      setContent(updated);
      debouncedSave(updated);
    },
    [debouncedSave],
  );

  const feedback = useIntelligenceFeedback(userId ?? undefined, "user");

  useRevealObserver(!loading && !!content);

  // Build dynamic slide/chapter list based on content
  const slideIds = useMemo(() => {
    if (!content) return [];
    const ids: string[] = [
      "cover",
      "health-overview",
      "risk-table",
    ];
    if (content.retentionRiskDeepDives.length > 0) ids.push("retention-deep-dives");
    if (content.saveMotions.length > 0) ids.push("save-motions");
    if (content.expansionAccounts.length > 0 || content.expansionReadiness.length > 0) ids.push("expansion");
    ids.push("year-end");
    if (content.leadershipAsks.length > 0) ids.push("the-ask");
    if (content.accountFocus.length > 0) ids.push("account-focus");
    if (content.quarterlyFocus.retention.length > 0 || content.quarterlyFocus.expansion.length > 0 || content.quarterlyFocus.execution.length > 0) ids.push("quarterly-focus");
    if (content.keyThemes.length > 0) ids.push("themes");
    return ids;
  }, [content]);

  const chapters = useMemo(() => {
    if (!content) return undefined;
    const ch: { id: string; label: string; icon: React.ReactNode }[] = [
      { id: "cover", label: "Cover", icon: <FileText size={18} strokeWidth={1.5} /> },
      { id: "health-overview", label: "Health", icon: <Shield size={18} strokeWidth={1.5} /> },
      { id: "risk-table", label: "Risks", icon: <AlertTriangle size={18} strokeWidth={1.5} /> },
    ];
    if (content.retentionRiskDeepDives.length > 0)
      ch.push({ id: "retention-deep-dives", label: "Deep Dives", icon: <AlertTriangle size={18} strokeWidth={1.5} /> });
    if (content.saveMotions.length > 0)
      ch.push({ id: "save-motions", label: "Save Motions", icon: <Shield size={18} strokeWidth={1.5} /> });
    if (content.expansionAccounts.length > 0 || content.expansionReadiness.length > 0)
      ch.push({ id: "expansion", label: "Expansion", icon: <TrendingUp size={18} strokeWidth={1.5} /> });
    ch.push({ id: "year-end", label: "Year-End", icon: <Calendar size={18} strokeWidth={1.5} /> });
    if (content.leadershipAsks.length > 0)
      ch.push({ id: "the-ask", label: "The Ask", icon: <MessageSquare size={18} strokeWidth={1.5} /> });
    if (content.accountFocus.length > 0)
      ch.push({ id: "account-focus", label: "Focus", icon: <Target size={18} strokeWidth={1.5} /> });
    if (content.quarterlyFocus.retention.length > 0 || content.quarterlyFocus.expansion.length > 0 || content.quarterlyFocus.execution.length > 0)
      ch.push({ id: "quarterly-focus", label: "Q→Q", icon: <Layers size={18} strokeWidth={1.5} /> });
    if (content.keyThemes.length > 0)
      ch.push({ id: "themes", label: "Themes", icon: <Layers size={18} strokeWidth={1.5} /> });
    return ch;
  }, [content]);

  // Load cached report once userId is available
  useEffect(() => {
    if (!userId) return;
    setLoading(true);
    invoke<ReportRow>("get_report", {
      entityId: userId,
      entityType: "user",
      reportType: "book_of_business",
    })
      .then((data) => {
        setReport(data);
        try {
          setContent(normalizeBookOfBusiness(JSON.parse(data.contentJson)));
        } catch (e) {
          console.error("Failed to parse book of business content:", e);
          setContent(null);
        }
        setError(null);
      })
      .catch((err) => {
        console.error("get_report (book_of_business) failed:", err);
        setReport(null);
        setContent(null);
      })
      .finally(() => setLoading(false));
  }, [userId]);

  // Toggle spotlight selection
  const toggleSpotlight = useCallback((accountId: string) => {
    setSelectedSpotlights((prev) => {
      const next = new Set(prev);
      if (next.has(accountId)) next.delete(accountId);
      else next.add(accountId);
      return next;
    });
  }, []);

  // Listen for progressive section completion events
  useEffect(() => {
    if (!generating) return;

    let unlistenProgress: UnlistenFn | null = null;
    let unlistenContent: UnlistenFn | null = null;

    listen<{ sectionName: string; completed: number; total: number; wave: number }>("bob-section-progress", (event) => {
      const { sectionName } = event.payload;
      setCompletedSections((prev) => new Set([...prev, sectionName]));
      setCurrentPhaseKey(sectionName);
    }).then((fn) => {
      unlistenProgress = fn;
    });

    listen<Record<string, unknown>>("bob-section-content", (event) => {
      setContent(normalizeBookOfBusiness(event.payload));
    }).then((fn) => {
      unlistenContent = fn;
    });

    return () => {
      if (unlistenProgress) unlistenProgress();
      if (unlistenContent) unlistenContent();
    };
  }, [generating]);

  // Generate handler
  const handleGenerate = useCallback(async () => {
    if (!userId || generating) return;
    setContent(null);
    setReport(null);
    setGenerating(true);
    setGenSeconds(0);
    setError(null);
    setCompletedSections(new Set());
    setCurrentPhaseKey("gathering");
    window.scrollTo({ top: 0, behavior: "instant" });

    timerRef.current = setInterval(() => setGenSeconds((s) => s + 1), 1000);

    try {
      const spotlightIds = selectedSpotlights.size > 0 ? [...selectedSpotlights] : undefined;
      const data = await invoke<ReportRow>("generate_report", {
        entityId: userId,
        entityType: "user",
        reportType: "book_of_business",
        spotlightAccountIds: spotlightIds,
      });
      setReport(data);
      setContent(normalizeBookOfBusiness(JSON.parse(data.contentJson)));
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to generate portfolio review");
    } finally {
      setGenerating(false);
      if (timerRef.current) clearInterval(timerRef.current);
    }
  }, [userId, generating, selectedSpotlights]);

  // Return to spotlight picker (for regeneration)
  const handleRegenerate = useCallback(() => {
    setContent(null);
    window.scrollTo({ top: 0, behavior: "instant" });
  }, []);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Book of Business",
      atmosphereColor: "turmeric" as const,
      activePage: "me" as const,
      backLink: {
        label: "Back",
        onClick: () =>
          window.history.length > 1
            ? window.history.back()
            : navigate({ to: "/me" }),
      },
      chapters,
      folioStatusText: saveStatus === "saved" ? "\u2713 Saved" : undefined,
      folioActions: content ? (
        <button
          onClick={handleRegenerate}
          disabled={generating}
          className={`${slides.folioAction} ${generating ? slides.folioActionDisabled : ""}`}
          style={{ "--report-accent": "var(--color-spice-turmeric)" } as React.CSSProperties}
        >
          {generating ? "Generating..." : "Regenerate"}
        </button>
      ) : undefined,
    }),
    [navigate, content, chapters, saveStatus, handleRegenerate, generating],
  );
  useRegisterMagazineShell(shellConfig);

  // Keyboard navigation
  useEffect(() => {
    if (!content || slideIds.length === 0) return;

    function handleKeyDown(e: KeyboardEvent) {
      const tag = (e.target as HTMLElement)?.tagName;
      if (tag === "INPUT" || tag === "TEXTAREA") return;

      const num = parseInt(e.key);
      if (num >= 1 && num <= slideIds.length) {
        document.getElementById(slideIds[num - 1])?.scrollIntoView({ behavior: "smooth" });
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

      for (let i = slideIds.length - 1; i >= 0; i--) {
        const el = document.getElementById(slideIds[i]);
        if (el && el.offsetTop <= scrollY) {
          currentIndex = i;
          break;
        }
      }

      const nextIndex = Math.max(0, Math.min(slideIds.length - 1, currentIndex + direction));
      document.getElementById(slideIds[nextIndex])?.scrollIntoView({ behavior: "smooth" });
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [content, slideIds]);

  // ── Loading ──────────────────────────────────────────────────────────────
  if (loading || (!userId && !error)) {
    return (
      <div className={slides.loadingSkeleton}>
        <Skeleton className={`mb-4 h-4 w-24 ${slides.skeletonBg}`} />
        <Skeleton className={`mb-2 h-12 w-96 ${slides.skeletonBg}`} />
        <Skeleton className={`mb-8 h-5 w-full max-w-2xl ${slides.skeletonBg}`} />
      </div>
    );
  }

  // ── Empty state with spotlight picker ─────────────────────────────────
  if (!content && !generating) {
    const healthDotColor: Record<string, string> = {
      green: "var(--color-garden-sage)",
      yellow: "var(--color-spice-saffron)",
      red: "var(--color-spice-terracotta)",
    };

    const parentAccounts = accounts.filter((a) => a.isParent);
    const childrenOf = new Map<string, AccountListItem[]>();
    for (const a of accounts) {
      if (a.parentId) {
        const siblings = childrenOf.get(a.parentId) ?? [];
        siblings.push(a);
        childrenOf.set(a.parentId, siblings);
      }
    }
    const standaloneAccounts = accounts.filter((a) => !a.isParent && !a.parentId);

    const renderAccountRow = (acct: AccountListItem, indent = false) => {
      const selected = selectedSpotlights.has(acct.id);
      return (
        <button
          key={acct.id}
          onClick={() => toggleSpotlight(acct.id)}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 12,
            width: "100%",
            padding: "10px 12px",
            paddingLeft: indent ? 36 : 12,
            background: selected ? "var(--color-cream-hover)" : "transparent",
            border: "none",
            borderBottom: "1px solid var(--color-rule-light)",
            cursor: "pointer",
            textAlign: "left",
            transition: "background 0.1s",
          }}
        >
          <span
            style={{
              width: 20,
              height: 20,
              borderRadius: 4,
              border: selected
                ? "2px solid var(--color-spice-turmeric)"
                : "2px solid var(--color-rule-light)",
              background: selected ? "var(--color-spice-turmeric)" : "transparent",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              flexShrink: 0,
              transition: "all 0.1s",
            }}
          >
            {selected && <Check size={12} strokeWidth={3} color="white" />}
          </span>
          {acct.health && (
            <span
              style={{
                width: 8,
                height: 8,
                borderRadius: "50%",
                background: healthDotColor[acct.health] ?? "var(--color-text-tertiary)",
                flexShrink: 0,
              }}
            />
          )}
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)", flex: 1 }}>
            {acct.name}
          </span>
          {acct.arr != null && acct.arr > 0 && (
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
              ${(acct.arr / 1000).toFixed(0)}k
            </span>
          )}
          {acct.renewalDate && (
            <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--color-text-tertiary)", textTransform: "uppercase" }}>
              {acct.renewalDate}
            </span>
          )}
        </button>
      );
    };

    return (
      <div
        className={slides.emptyState}
        style={{
          "--report-accent": "var(--color-spice-turmeric)",
        } as React.CSSProperties}
      >
        <div className={slides.emptyOverline}>Book of Business</div>
        <h2 className={slides.emptyTitle}>
          {report ? "Select accounts to spotlight" : "No portfolio review yet"}
        </h2>
        <p className={slides.emptyDescription}>
          {accounts.length > 0
            ? "Choose which accounts get their own spotlight slide. All accounts contribute to the overall analysis."
            : "Generate a leadership-ready portfolio review. Health trends, risks, account spotlights, and cross-portfolio themes — all in one presentation."}
        </p>

        {accounts.length > 0 && (
          <div
            style={{
              width: "100%",
              maxWidth: 520,
              maxHeight: 400,
              overflowY: "auto",
              border: "1px solid var(--color-rule-light)",
              borderRadius: 8,
              marginBottom: 24,
            }}
          >
            {parentAccounts.map((parent) => (
              <div key={parent.id}>
                {renderAccountRow(parent)}
                {(childrenOf.get(parent.id) ?? []).map((child) =>
                  renderAccountRow(child, true),
                )}
              </div>
            ))}
            {standaloneAccounts.map((acct) => renderAccountRow(acct))}
          </div>
        )}

        {selectedSpotlights.size > 0 && (
          <p style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", marginBottom: 16 }}>
            {selectedSpotlights.size} account{selectedSpotlights.size !== 1 ? "s" : ""} selected for spotlight
          </p>
        )}

        {error && <p className={slides.emptyError}>{error}</p>}
        <Button onClick={handleGenerate} disabled={generating || !userId}>
          Generate Portfolio Review
        </Button>
      </div>
    );
  }

  // ── Generating state ──────────────────────────────────────────────────
  if (generating && !content) {
    const showGlean = completedSections.size === 0 && genSeconds >= 3 && genSeconds < 15;
    const activeKey = completedSections.size > 0
      ? currentPhaseKey
      : (showGlean ? "glean" : "gathering");

    const accentColor = "var(--color-spice-turmeric)";
    const formatElapsed = (secs: number) => {
      const m = Math.floor(secs / 60);
      const s = secs % 60;
      return m > 0 ? `${m}m ${s}s` : `${s}s`;
    };

    return (
      <div style={{ display: "grid", gridTemplateColumns: "100px 32px 1fr", paddingTop: 80, paddingBottom: 96 }}>
        <div style={{ paddingTop: 6 }}>
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 600, letterSpacing: "0.1em", textTransform: "uppercase", color: accentColor }}>
            {formatElapsed(genSeconds)}
          </div>
        </div>
        <div />
        <div style={{ maxWidth: 520 }}>
          <div style={{ borderTop: "1px solid var(--color-rule-heavy)", marginBottom: 32 }} />
          <div style={{ fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 600, textTransform: "uppercase", letterSpacing: "0.1em", color: accentColor, marginBottom: 32 }}>
            Building Portfolio Review
          </div>

          <div style={{ marginBottom: 56 }}>
            {BOB_PHASES.map((phase, i) => {
              const isComplete = completedSections.has(phase.key) || phase.key === "gathering";
              const isCurrent = phase.key === activeKey;
              const isPending = !isComplete && !isCurrent;

              return (
                <div key={phase.key} style={{ display: "flex", gap: 16, alignItems: "flex-start", padding: "10px 0", borderBottom: i < BOB_PHASES.length - 1 ? "1px solid var(--color-rule-light)" : "none", opacity: isPending ? 0.3 : 1, transition: "opacity 0.5s ease" }}>
                  <div style={{ width: 20, height: 20, borderRadius: "50%", border: `2px solid ${isComplete ? "var(--color-garden-sage)" : isCurrent ? accentColor : "var(--color-rule-light)"}`, background: isComplete ? "var(--color-garden-sage)" : "transparent", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, marginTop: 2, transition: "all 0.3s ease" }}>
                    {isComplete && (
                      <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                        <path d="M2 6l3 3 5-5" stroke="white" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                      </svg>
                    )}
                    {isCurrent && !isComplete && (
                      <div style={{ width: 6, height: 6, borderRadius: "50%", background: accentColor, animation: "generating-pulse 1.5s ease infinite" }} />
                    )}
                  </div>
                  <div style={{ paddingTop: 1 }}>
                    <div style={{ fontFamily: "var(--font-sans)", fontSize: 14, fontWeight: isCurrent ? 600 : 400, color: isCurrent ? "var(--color-text-primary)" : isComplete ? "var(--color-text-secondary)" : "var(--color-text-tertiary)", transition: "all 0.3s ease" }}>
                      {phase.label}
                    </div>
                    {isCurrent && !isComplete && (
                      <div style={{ fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-text-tertiary)", marginTop: 3 }}>
                        {phase.detail}
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>

          <div>
            <div style={{ borderTop: "1px solid var(--color-rule-light)", marginBottom: 20 }} />
            <p style={{ fontFamily: "var(--font-serif)", fontSize: 16, fontStyle: "italic", fontWeight: 300, color: "var(--color-text-tertiary)", lineHeight: 1.6, margin: 0 }}>
              {EDITORIAL_QUOTES[Math.floor(genSeconds / 8) % EDITORIAL_QUOTES.length]}
            </p>
            <div style={{ borderTop: "1px solid var(--color-rule-light)", marginTop: 20 }} />
          </div>

          <div style={{ marginTop: 20, fontFamily: "var(--font-sans)", fontSize: 12, color: "var(--color-text-tertiary)", opacity: 0.6 }}>
            This runs in the background — feel free to navigate away
          </div>
        </div>

        <style>{`
          @keyframes generating-pulse {
            0%, 100% { opacity: 1; transform: scale(1); }
            50% { opacity: 0.5; transform: scale(0.8); }
          }
        `}</style>
      </div>
    );
  }

  // ── Render slides ────────────────────────────────────────────────────────
  const c = content!;

  return (
    <div className={slides.slideContainer}>
      {/* Slide 1: Cover — vitals, exec summary */}
      <section id="cover" className={slides.slideSection}>
        <CoverSlide
          content={c}
          isStale={report?.isStale}
          onRegenerate={handleRegenerate}
          generating={generating}
          onUpdate={updateContent}
        />
        <IntelligenceFeedback
          value={feedback.getFeedback("executive_summary")}
          onFeedback={(type) => feedback.submitFeedback("executive_summary", type)}
        />
      </section>

      {/* Slide 2: Health Overview */}
      <div className="editorial-reveal">
        <HealthOverviewSlide content={c} onUpdate={updateContent} />
      </div>

      {/* Slide 3: Risk Table */}
      <div className="editorial-reveal">
        <RiskTableSlide content={c} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("risk_table")}
          onFeedback={(type) => feedback.submitFeedback("risk_table", type)}
        />
      </div>

      {/* Slide 4: Retention Deep Dives */}
      {c.retentionRiskDeepDives.length > 0 && (
        <div className="editorial-reveal">
          <RetentionDeepDiveSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("retention_deep_dives")}
            onFeedback={(type) => feedback.submitFeedback("retention_deep_dives", type)}
          />
        </div>
      )}

      {/* Slide 5: Save Motions */}
      {c.saveMotions.length > 0 && (
        <div className="editorial-reveal">
          <SaveMotionsSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("save_motions")}
            onFeedback={(type) => feedback.submitFeedback("save_motions", type)}
          />
        </div>
      )}

      {/* Slide 6+7: Expansion */}
      {(c.expansionAccounts.length > 0 || c.expansionReadiness.length > 0) && (
        <div className="editorial-reveal">
          <ExpansionSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("expansion")}
            onFeedback={(type) => feedback.submitFeedback("expansion", type)}
          />
        </div>
      )}

      {/* Slide 8+9: Year-End */}
      <div className="editorial-reveal">
        <YearEndSlide content={c} onUpdate={updateContent} />
        <IntelligenceFeedback
          value={feedback.getFeedback("year_end")}
          onFeedback={(type) => feedback.submitFeedback("year_end", type)}
        />
      </div>

      {/* Slide 10+14: Leadership Asks */}
      {c.leadershipAsks.length > 0 && (
        <div className="editorial-reveal">
          <LeadershipAsksSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("leadership_asks")}
            onFeedback={(type) => feedback.submitFeedback("leadership_asks", type)}
          />
        </div>
      )}

      {/* Slide 11: Account Focus */}
      {c.accountFocus.length > 0 && (
        <div className="editorial-reveal">
          <AccountFocusSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("account_focus")}
            onFeedback={(type) => feedback.submitFeedback("account_focus", type)}
          />
        </div>
      )}

      {/* Slide 12: Quarterly Focus */}
      {(c.quarterlyFocus.retention.length > 0 || c.quarterlyFocus.expansion.length > 0 || c.quarterlyFocus.execution.length > 0) && (
        <div className="editorial-reveal">
          <QuarterlyFocusSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("quarterly_focus")}
            onFeedback={(type) => feedback.submitFeedback("quarterly_focus", type)}
          />
        </div>
      )}

      {/* Slide 13: Key Themes */}
      {c.keyThemes.length > 0 && (
        <div className="editorial-reveal">
          <ThemesSlide content={c} onUpdate={updateContent} />
          <IntelligenceFeedback
            value={feedback.getFeedback("key_themes")}
            onFeedback={(type) => feedback.submitFeedback("key_themes", type)}
          />
        </div>
      )}

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
