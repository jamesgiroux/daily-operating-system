/**
 * BookOfBusinessPage — Slide-deck portfolio review for leadership.
 * Full-viewport slides with scroll-snap, editorial typography, one idea per screen.
 * Follows the same pattern as RiskBriefingPage and WeeklyImpactPage.
 */
import { useState, useEffect, useMemo, useCallback, useRef } from "react";
import { useNavigate } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { FileText, AlertTriangle, Search, Layers, MessageSquare, Check } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import type { AccountListItem } from "@/types";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { useRevealObserver } from "@/hooks/useRevealObserver";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { GeneratingProgress } from "@/components/editorial/GeneratingProgress";
import { CoverSlide } from "@/components/book-of-business/CoverSlide";
import { AttentionSlide } from "@/components/book-of-business/AttentionSlide";
import { SpotlightSlide } from "@/components/book-of-business/SpotlightSlide";
import { ValueThemesSlide } from "@/components/book-of-business/ValueThemesSlide";
import { AskSlide } from "@/components/book-of-business/AskSlide";
import type {
  ReportRow,
  BookOfBusinessContent,
  BookRiskItem,
  BookOpportunityItem,
  AccountSnapshotRow,
  AccountDeepDive,
  ValueDeliveredRow,
  BookTheme,
  LeadershipAsk,
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
    periodLabel: (raw.periodLabel as string) ?? "",
    executiveSummary: ((raw.executiveSummary ?? raw.executiveNarrative) as string) ?? "",
    totalAccounts: (raw.totalAccounts as number) ?? 0,
    totalArr: (raw.totalArr as number | null) ?? null,
    atRiskArr: (raw.atRiskArr as number | null) ?? null,
    upcomingRenewals: (raw.upcomingRenewals as number) ?? 0,
    upcomingRenewalsArr: (raw.upcomingRenewalsArr as number | null) ?? null,
    hasLeadershipAsks: (raw.hasLeadershipAsks as boolean) ?? false,
    topRisks: toArr<BookRiskItem>(raw.topRisks ?? raw.risks),
    topOpportunities: toArr<BookOpportunityItem>(raw.topOpportunities ?? raw.opportunities),
    accountSnapshot: toArr<AccountSnapshotRow>(raw.accountSnapshot ?? raw.snapshot),
    deepDives: toArr<AccountDeepDive>(raw.deepDives),
    valueDelivered: toArr<ValueDeliveredRow>(raw.valueDelivered),
    keyThemes: toArr<BookTheme>(raw.keyThemes ?? raw.themes),
    leadershipAsks: toArr<LeadershipAsk>(raw.leadershipAsks),
  };
}

// =============================================================================
// Generating progress config
// =============================================================================

const ANALYSIS_PHASES = [
  { key: "gathering", label: "Gathering portfolio data", detail: "Reading account health, meeting history, and renewal context" },
  { key: "analyzing", label: "Analyzing accounts", detail: "Assessing health, risks, and opportunities across the book" },
  { key: "themes", label: "Identifying themes", detail: "Finding patterns and cross-account trends" },
  { key: "synthesizing", label: "Synthesizing review", detail: "Building the portfolio narrative and leadership asks" },
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
        const active = topLevel.filter((a) => !a.archived);
        const parents = active.filter((a) => a.isParent && a.childCount > 0);
        const childLists = await Promise.all(
          parents.map((p) =>
            invoke<AccountListItem[]>("get_child_accounts_list", { parentId: p.id })
              .then((children) => children.filter((c) => !c.archived))
              .catch(() => [] as AccountListItem[]),
          ),
        );
        const allChildren = childLists.flat();
        setAccounts([...active, ...allChildren]);
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

  useRevealObserver(!loading && !!content);

  // Build dynamic slide/chapter list based on content
  const slideIds = useMemo(() => {
    if (!content) return [];
    const ids: string[] = ["cover", "attention"];
    content.deepDives.forEach((_, i) => ids.push(`spotlight-${i + 1}`));
    if (content.valueDelivered.length > 0 || content.keyThemes.length > 0) {
      ids.push("value-themes");
    }
    if (content.leadershipAsks.length > 0) {
      ids.push("the-ask");
    }
    return ids;
  }, [content]);

  const chapters = useMemo(() => {
    if (!content) return undefined;
    const ch: { id: string; label: string; icon: React.ReactNode }[] = [
      { id: "cover", label: "Cover", icon: <FileText size={18} strokeWidth={1.5} /> },
      { id: "attention", label: "Attention", icon: <AlertTriangle size={18} strokeWidth={1.5} /> },
    ];
    content.deepDives.forEach((dive, i) => {
      ch.push({
        id: `spotlight-${i + 1}`,
        label: dive.accountName.length > 16 ? dive.accountName.slice(0, 14) + "..." : dive.accountName,
        icon: <Search size={18} strokeWidth={1.5} />,
      });
    });
    if (content.valueDelivered.length > 0 || content.keyThemes.length > 0) {
      ch.push({ id: "value-themes", label: "Value & Themes", icon: <Layers size={18} strokeWidth={1.5} /> });
    }
    if (content.leadershipAsks.length > 0) {
      ch.push({ id: "the-ask", label: "The Ask", icon: <MessageSquare size={18} strokeWidth={1.5} /> });
    }
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

  // Return to spotlight picker (for regeneration), pre-populated with current spotlights
  const handleRegenerate = useCallback(() => {
    if (content) {
      setSelectedSpotlights(new Set(content.deepDives.map((d) => d.accountId)));
    }
    setContent(null);
    window.scrollTo({ top: 0, behavior: "instant" });
  }, [content]);

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

    // Group accounts: parents first (with children indented), then standalone
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
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-primary)",
              flex: 1,
            }}
          >
            {acct.name}
          </span>
          {acct.arr != null && acct.arr > 0 && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
              }}
            >
              ${(acct.arr / 1000).toFixed(0)}k
            </span>
          )}
          {acct.renewalDate && (
            <span
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                color: "var(--color-text-tertiary)",
                textTransform: "uppercase",
              }}
            >
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
          alignItems: accounts.length > 0 ? "center" : "center",
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
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              color: "var(--color-text-tertiary)",
              marginBottom: 16,
            }}
          >
            {selectedSpotlights.size} account{selectedSpotlights.size !== 1 ? "s" : ""} selected for
            spotlight
          </p>
        )}

        {error && <p className={slides.emptyError}>{error}</p>}
        <Button onClick={handleGenerate} disabled={generating || !userId}>
          Generate Portfolio Review
        </Button>
      </div>
    );
  }

  // ── Generating state ─────────────────────────────────────────────────────
  if (generating) {
    return (
      <GeneratingProgress
        title="Building Portfolio Review"
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
      </section>

      {/* Slide 2: What Needs Attention — risks & opportunities */}
      <div className="editorial-reveal">
        <AttentionSlide content={c} onUpdate={updateContent} />
      </div>

      {/* Slides 3-N: Account Spotlights — one per deep dive */}
      {c.deepDives.map((dive, i) => (
        <div key={dive.accountId} className="editorial-reveal">
          <SpotlightSlide
            dive={dive}
            index={i + 1}
            total={c.deepDives.length}
            content={c}
            onUpdate={updateContent}
          />
        </div>
      ))}

      {/* Value Delivered + Themes */}
      {(c.valueDelivered.length > 0 || c.keyThemes.length > 0) && (
        <div className="editorial-reveal">
          <ValueThemesSlide content={c} onUpdate={updateContent} />
        </div>
      )}

      {/* The Ask — leadership asks (conditional) */}
      {c.leadershipAsks.length > 0 && (
        <div className="editorial-reveal">
          <AskSlide content={c} onUpdate={updateContent} />
        </div>
      )}

      {/* Finis marker */}
      <div className="editorial-reveal">
        <FinisMarker enrichedAt={report?.generatedAt?.split("T")[0]} />
      </div>
    </div>
  );
}
