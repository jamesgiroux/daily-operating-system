import { useState, useEffect, useRef } from "react";
import {
  createRouter,
  createRootRoute,
  createRoute,
  Outlet,
  useRouterState,
  useNavigate,
} from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { ThemeProvider } from "@/components/theme-provider";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";
import { CommandMenu, useCommandMenu } from "@/components/layout/CommandMenu";
import { Header } from "@/components/dashboard/Header";
import { OnboardingFlow } from "@/components/onboarding/OnboardingFlow";

// Lazy load pages for code splitting
import { DailyBriefing } from "@/components/dashboard/DailyBriefing";
import { DashboardSkeleton } from "@/components/dashboard/DashboardSkeleton";
import { DashboardEmpty } from "@/components/dashboard/DashboardEmpty";
import { DashboardError } from "@/components/dashboard/DashboardError";
import { useDashboardData } from "@/hooks/useDashboardData";
import { useWorkflow } from "@/hooks/useWorkflow";

// Page components
import AccountsPage from "@/pages/AccountsPage";
import AccountDetailPage from "@/pages/AccountDetailPage";
import ActionDetailPage from "@/pages/ActionDetailPage";
import ActionsPage from "@/pages/ActionsPage";
import InboxPage from "@/pages/InboxPage";
import MeetingDetailPage from "@/pages/MeetingDetailPage";
import MeetingHistoryDetailPage from "@/pages/MeetingHistoryDetailPage";
import EmailsPage from "@/pages/EmailsPage";
import HistoryPage from "@/pages/HistoryPage";
import PeoplePage from "@/pages/PeoplePage";
import PersonDetailEditorial from "@/pages/PersonDetailEditorial";
import ProjectsPage from "@/pages/ProjectsPage";
import ProjectDetailEditorial from "@/pages/ProjectDetailEditorial";
import RiskBriefingPage from "@/pages/RiskBriefingPage";
import ReportPage from "@/pages/ReportPage";
import AccountHealthPage from "@/pages/AccountHealthPage";
import EbrQbrPage from "@/pages/EbrQbrPage";
import SwotPage from "@/pages/SwotPage";
import WeeklyImpactPage from "@/pages/WeeklyImpactPage";
import MonthlyWrappedPage from "@/pages/monthly-wrapped/MonthlyWrappedPage";
import BookOfBusinessPage from "@/pages/BookOfBusinessPage";
import SettingsPage from "@/pages/SettingsPage";
import MePage from "@/pages/MePage";
import WeekPage from "@/pages/WeekPage";


// Magazine shell
import MagazinePageLayout from "@/components/layout/MagazinePageLayout";
import { MagazineShellContext, useMagazineShellProvider } from "@/hooks/useMagazineShell";

// Global overlays
import { PostMeetingPrompt } from "@/components/PostMeetingPrompt";
import { Toaster } from "@/components/ui/sonner";
import { DevToolsPanelStandalone } from "@/components/devtools/DevToolsPanel";
import { useNotifications } from "@/hooks/useNotifications";
import { useBackgroundStatus } from "@/hooks/useBackgroundStatus";
import type { CalendarEvent, GoogleAuthStatus } from "@/types";
import { PersonalityProvider } from "@/hooks/usePersonality";
import { UpdateBanner } from "@/components/notifications/UpdateBanner";
import { WhatsNewModal, useWhatsNewAutoShow } from "@/components/notifications/WhatsNewModal";
import { ICloudWarningModal } from "@/components/ICloudWarningModal";
import { LockOverlay } from "@/components/LockOverlay";
import { useAppLock } from "@/hooks/useAppLock";
import { EncryptionRecovery, useEncryptionStatus } from "@/components/EncryptionRecovery";
import { DatabaseRecovery } from "@/components/DatabaseRecovery";
import { AppStateCtx, useAppStateProvider } from "@/hooks/useAppState";
import { ClaudeStatusCtx, useClaudeStatusProvider } from "@/hooks/useClaudeStatus";
import { useDatabaseRecoveryStatus } from "@/hooks/useDatabaseRecoveryStatus";
import { TourTips } from "@/components/tour/TourTips";
import { resolveStartupGate } from "@/routerStartupGate";
import { StartupBriefingScreen } from "@/components/startup/StartupBriefingScreen";

const settingsTabs = new Set([
  "you",
  "connectors",
  "system",
  "diagnostics",
  // Legacy tab IDs for backwards compatibility
  "profile",
  "role",
  "integrations",
  "workflows",
  "intelligence",
  "hygiene",
]);
const peopleRelationshipTabs = new Set(["all", "external", "internal", "unknown"]);
const peopleHygieneFilters = new Set(["unnamed", "duplicates"]);

// Route IDs that use the magazine shell instead of the sidebar shell.
// Add new editorial routes here as they're built.
const MAGAZINE_ROUTE_IDS = new Set(["/", "/week", "/actions", "/actions/$actionId", "/accounts", "/projects", "/people", "/accounts/$accountId", "/accounts/$accountId/reports/risk_briefing", "/accounts/$accountId/reports/$reportType", "/accounts/$accountId/reports/account_health", "/accounts/$accountId/reports/ebr_qbr", "/accounts/$accountId/reports/swot", "/me/reports/weekly_impact", "/me/reports/monthly_wrapped", "/me/reports/book_of_business", "/me/reports/$reportType", "/projects/$projectId", "/people/$personId", "/emails", "/inbox", "/history", "/settings", "/me", "/meeting/$meetingId", "/meeting/history/$meetingId"]);

const WELCOME_MIN_MS = 1500;
const WELCOME_MAX_MS = 5000;
const CALENDAR_SETTLE_GRACE_MS = 450;

// Root layout that wraps all pages
function RootLayout() {
  const { open: commandOpen, setOpen: setCommandOpen } = useCommandMenu();
  useNotifications();
  const backgroundWork = useBackgroundStatus();
  const navigate = useNavigate();
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  const [checkingConfig, setCheckingConfig] = useState(true);
  const [welcomeVisible, setWelcomeVisible] = useState(true);
  const [welcomeFading, setWelcomeFading] = useState(false);
  const mountTime = useRef(Date.now());
  const [startupNeedsCalendarSettle, setStartupNeedsCalendarSettle] = useState<boolean | null>(null);
  const [startupCalendarSettled, setStartupCalendarSettled] = useState(false);
  const [whatsNewOpen, setWhatsNewOpen] = useState(false);
  const { autoShowOpen, dismissAutoShow } = useWhatsNewAutoShow();
  const { isLocked, setIsLocked } = useAppLock();
  const encryptionKeyMissing = useEncryptionStatus();
  const { status: dbRecoveryStatus } = useDatabaseRecoveryStatus();
  const appStateCtx = useAppStateProvider();
  const claudeStatusCtx = useClaudeStatusProvider();

  // Magazine shell context — pages register their config, layout consumes it
  const magazineShell = useMagazineShellProvider();

  const routerState = useRouterState();
  const deepestRouteId = routerState.matches[routerState.matches.length - 1]?.routeId ?? "";
  const useMagazineShell = MAGAZINE_ROUTE_IDS.has(deepestRouteId);

  useEffect(() => {
    async function checkConfig() {
      try {
        const config = await invoke<{
          workspacePath?: string;
          textScalePercent?: number;
        }>("get_config");
        // Apply persisted text scale (DOS-45)
        const scale = config.textScalePercent ?? 100;
        if (scale !== 100) {
          document.documentElement.style.zoom = `${scale / 100}`;
        }
        if (!config.workspacePath) {
          // No workspace path — but check if wizard was already completed
          // (workspace creation may have failed silently)
          const appState = await invoke<{ wizardCompletedAt?: string | null }>("get_app_state").catch(() => null);
          if (!appState?.wizardCompletedAt) {
            setNeedsOnboarding(true);
          }
        }
      } catch {
        setNeedsOnboarding(true);
      } finally {
        setCheckingConfig(false);
      }
    }
    checkConfig();
  }, []);

  // Allow Settings "Resume setup" to re-enter onboarding
  useEffect(() => {
    if (appStateCtx.forceOnboarding) {
      setNeedsOnboarding(true);
    }
  }, [appStateCtx.forceOnboarding]);

  function handleOnboardingComplete() {
    setNeedsOnboarding(false);
    window.location.reload();
  }

  // Navigation handler for FloatingNavIsland
  function handleNavNavigate(page: string) {
    const routes: Record<string, string> = {
      today: "/",
      week: "/week",
      emails: "/emails",
      dropbox: "/inbox",
      actions: "/actions",
      me: "/me",
      people: "/people",
      accounts: "/accounts",
      projects: "/projects",
      settings: "/settings",
    };
    const path = routes[page];
    if (path) navigate({ to: path });
  }

  const startupGate = resolveStartupGate({
    checkingConfig,
    encryptionKeyMissing,
    dbRecoveryRequired: dbRecoveryStatus.required,
    isLocked,
    needsOnboarding,
  });

  const showWelcomeShellOnly = startupGate === "checking";
  const showWelcomeOverlay = startupGate === "app" && welcomeVisible;

  useEffect(() => {
    let cancelled = false;

    if (startupGate !== "app") {
      setStartupNeedsCalendarSettle(null);
      setStartupCalendarSettled(false);
      return;
    }

    async function inspectStartupStability() {
      try {
        const auth = await invoke<GoogleAuthStatus>("get_google_auth_status");
        if (cancelled) return;
        if (auth.status !== "authenticated") {
          setStartupNeedsCalendarSettle(false);
          setStartupCalendarSettled(true);
          return;
        }

        setStartupNeedsCalendarSettle(true);
        const events = await invoke<CalendarEvent[]>("get_calendar_events").catch(() => []);
        if (cancelled) return;

        if (events.length > 0) {
          setStartupCalendarSettled(true);
        }
      } catch {
        if (!cancelled) {
          setStartupNeedsCalendarSettle(false);
          setStartupCalendarSettled(true);
        }
      }
    }

    inspectStartupStability();

    return () => {
      cancelled = true;
    };
  }, [startupGate]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    let settleTimer: ReturnType<typeof setTimeout> | null = null;

    if (startupGate !== "app") {
      return;
    }

    listen("calendar-updated", () => {
      if (settleTimer) clearTimeout(settleTimer);
      settleTimer = setTimeout(() => {
        setStartupCalendarSettled(true);
      }, CALENDAR_SETTLE_GRACE_MS);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (settleTimer) clearTimeout(settleTimer);
      unlisten?.();
    };
  }, [startupGate]);

  // Cross-entity contamination guard: when the enrichment pipeline detects
  // foreign-entity tokens in an account's narrative output, the write is
  // rejected and the prior intelligence row stays in place. The backend
  // emits this event so the user sees a non-blocking notification with
  // enough detail to decide whether to investigate the false positive or
  // accept the rejection.
  useEffect(() => {
    if (startupGate !== "app") return;
    let unlisten: UnlistenFn | undefined;

    listen<{
      entity_id: string;
      entity_type: string;
      hits: Array<{
        foreignToken: string;
        kind: "domain" | "infrastructure_id" | "company_name";
        sourceAccountId: string | null;
      }>;
      rejected: boolean;
    }>("enrichment-contamination-rejected", (event) => {
      const { entity_id, hits, rejected } = event.payload;
      if (!rejected) return; // shadow mode — no toast
      const summary = hits
        .map((h) => {
          const target = h.sourceAccountId ? ` (from ${h.sourceAccountId})` : "";
          return `${h.foreignToken}${target} [${h.kind.replace("_", " ")}]`;
        })
        .join(", ");
      toast.warning(`Refresh skipped for ${entity_id}`, {
        description: `Foreign reference detected: ${summary}. Prior intelligence kept.`,
        duration: 8000,
      });
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      unlisten?.();
    };
  }, [startupGate]);

  // I599: Dismiss the pre-React HTML welcome screen once the React overlay is active.
  useEffect(() => {
    if (!showWelcomeShellOnly && !showWelcomeOverlay) return;
    const htmlWelcome = document.getElementById("welcome-screen");
    if (htmlWelcome) htmlWelcome.remove();
  }, [showWelcomeOverlay, showWelcomeShellOnly]);

  // I614: DB size warning removed from user-facing UI.
  // Auto-purge handles large databases silently. Size info
  // is available in Settings > Diagnostics for developers.

  useEffect(() => {
    if (startupGate !== "checking" && startupGate !== "app") {
      setWelcomeVisible(false);
      setWelcomeFading(false);
    }
  }, [startupGate]);

  // Welcome screen: hold briefly, then fade while the app continues mounting underneath it.
  useEffect(() => {
    if (startupGate !== "app" || !welcomeVisible) {
      return;
    }
    const elapsed = Date.now() - mountTime.current;
    const minRemaining = Math.max(0, WELCOME_MIN_MS - elapsed);
    const maxRemaining = Math.max(0, WELCOME_MAX_MS - elapsed);
    const calendarReady =
      startupNeedsCalendarSettle === false ||
      (startupNeedsCalendarSettle === true && startupCalendarSettled);
    const remaining = calendarReady ? minRemaining : maxRemaining;
    const fadeTimer = setTimeout(() => {
      const nowElapsed = Date.now() - mountTime.current;
      const canFade =
        nowElapsed >= WELCOME_MIN_MS &&
        (
          startupNeedsCalendarSettle !== true ||
          startupCalendarSettled ||
          nowElapsed >= WELCOME_MAX_MS
        );
      if (!canFade) return;
      setWelcomeFading(true);
      setTimeout(() => setWelcomeVisible(false), 300);
    }, remaining);
    return () => clearTimeout(fadeTimer);
  }, [startupGate, startupCalendarSettled, startupNeedsCalendarSettle, welcomeVisible]);

  if (showWelcomeShellOnly) {
    return (
      <ThemeProvider>
        <StartupBriefingScreen />
        <DevToolsPanelStandalone />
      </ThemeProvider>
    );
  }

  if (startupGate === "encryption-recovery") {
    return (
      <ThemeProvider>
        <EncryptionRecovery />
      </ThemeProvider>
    );
  }

  if (startupGate === "database-recovery") {
    return (
      <ThemeProvider>
        <DatabaseRecovery status={dbRecoveryStatus} />
      </ThemeProvider>
    );
  }

  if (startupGate === "lock") {
    return (
      <ThemeProvider>
        <LockOverlay onUnlock={() => setIsLocked(false)} />
      </ThemeProvider>
    );
  }

  if (startupGate === "onboarding") {
    return (
      <ThemeProvider>
        <OnboardingFlow onComplete={handleOnboardingComplete} />
        <Toaster position="bottom-right" />
        <DevToolsPanelStandalone />
      </ThemeProvider>
    );
  }

  const handleWhatsNewClose = () => {
    setWhatsNewOpen(false);
    if (autoShowOpen) dismissAutoShow();
  };

  // Magazine shell for editorial pages (account detail, future editorial pages)
  if (useMagazineShell) {
    return (
      <ThemeProvider>
        <PersonalityProvider>
          <AppStateCtx.Provider value={appStateCtx}>
          <ClaudeStatusCtx.Provider value={claudeStatusCtx}>
            <MagazineShellContext.Provider value={magazineShell}>
              <MagazinePageLayout
                onFolioSearch={() => setCommandOpen(true)}
                onNavigate={handleNavNavigate}
                onNavHome={() => navigate({ to: "/" })}
                onWhatsNew={() => setWhatsNewOpen(true)}
                backgroundWork={backgroundWork}
              >
                <Outlet />
              </MagazinePageLayout>
            </MagazineShellContext.Provider>
            <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
            <PostMeetingPrompt />
            <WhatsNewModal open={whatsNewOpen || autoShowOpen} onClose={handleWhatsNewClose} />
            <ICloudWarningModal />
            <TourTips />
            <Toaster position="bottom-right" />
            {showWelcomeOverlay && <StartupBriefingScreen fading={welcomeFading} />}
          </ClaudeStatusCtx.Provider>
          </AppStateCtx.Provider>
        </PersonalityProvider>
      </ThemeProvider>
    );
  }

  // Standard sidebar shell for all other pages
  return (
    <ThemeProvider>
      <PersonalityProvider>
        <AppStateCtx.Provider value={appStateCtx}>
        <ClaudeStatusCtx.Provider value={claudeStatusCtx}>
          <SidebarProvider defaultOpen={false}>
            <SidebarInset>
              <UpdateBanner onWhatsNew={() => setWhatsNewOpen(true)} />
              <Header onCommandMenuOpen={() => setCommandOpen(true)} backgroundWork={backgroundWork} />
              <Outlet />
            </SidebarInset>
            <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
          </SidebarProvider>
          <PostMeetingPrompt />
          <WhatsNewModal open={whatsNewOpen || autoShowOpen} onClose={handleWhatsNewClose} />
          <ICloudWarningModal />
          <TourTips />
          <Toaster position="bottom-right" />
          <DevToolsPanelStandalone />
          {showWelcomeOverlay && <StartupBriefingScreen fading={welcomeFading} />}
        </ClaudeStatusCtx.Provider>
        </AppStateCtx.Provider>
      </PersonalityProvider>
    </ThemeProvider>
  );
}

// Dashboard page content
function DashboardPage() {
  const { state, refresh } = useDashboardData();
  const { runNow, isRunning, status } = useWorkflow();

  switch (state.status) {
    case "loading":
      return <DashboardSkeleton />;
    case "empty":
      return <DashboardEmpty message={state.message} onGenerate={runNow} isRunning={isRunning} workflowStatus={status} googleAuth={state.googleAuth} />;
    case "error":
      return <DashboardError message={state.message} onRetry={refresh} />;
    case "success":
      return <DailyBriefing data={state.data} freshness={state.freshness} onRunBriefing={runNow} isRunning={isRunning} workflowStatus={status} onRefresh={refresh} />;
  }
}

// Create root route
const rootRoute = createRootRoute({
  component: RootLayout,
});

// Create child routes
const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DashboardPage,
});

const actionsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/actions",
  component: ActionsPage,
  validateSearch: (search: Record<string, unknown>) => ({
    search: (search.search as string) || undefined,
  }),
});

const actionDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/actions/$actionId",
  component: ActionDetailPage,
});

const accountsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts",
  component: AccountsPage,
});

const accountDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId",
  component: AccountDetailPage,
});

const riskBriefingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/reports/risk_briefing",
  component: RiskBriefingPage,
});

const swotRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/reports/swot",
  component: SwotPage,
});

const accountReportRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/reports/$reportType",
  component: ReportPage,
});

const accountHealthRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/reports/account_health",
  component: AccountHealthPage,
});

const ebrQbrRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/reports/ebr_qbr",
  component: EbrQbrPage,
});

const weeklyImpactRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/me/reports/weekly_impact",
  component: WeeklyImpactPage,
});

const monthlyWrappedRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/me/reports/monthly_wrapped",
  component: MonthlyWrappedPage,
});

const bookOfBusinessRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/me/reports/book_of_business",
  component: BookOfBusinessPage,
});

const meReportRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/me/reports/$reportType",
  component: ReportPage,
});

const inboxRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/inbox",
  component: InboxPage,
  validateSearch: (search: Record<string, unknown>) => ({
    entityId: typeof search.entityId === "string" ? search.entityId : undefined,
  }),
});

const emailsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/emails",
  component: EmailsPage,
});

const meetingDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/meeting/$meetingId",
  component: MeetingDetailPage,
});

const meetingHistoryDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/meeting/history/$meetingId",
  component: MeetingHistoryDetailPage,
});

const projectsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/projects",
  component: ProjectsPage,
});

const projectDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/projects/$projectId",
  component: ProjectDetailEditorial,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
  validateSearch: (search: Record<string, unknown>) => {
    const validated: { tab?: string } = {};
    if (typeof search.tab === "string" && settingsTabs.has(search.tab)) {
      validated.tab = search.tab;
    }
    return validated;
  },
});

const meRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/me",
  component: MePage,
});

const weekRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/week",
  component: WeekPage,
});

const historyRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/history",
  component: HistoryPage,
});

const peopleRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/people",
  component: PeoplePage,
  validateSearch: (search: Record<string, unknown>) => {
    const validated: { relationship?: string; hygiene?: string } = {};
    if (
      typeof search.relationship === "string" &&
      peopleRelationshipTabs.has(search.relationship)
    ) {
      validated.relationship = search.relationship;
    }
    if (
      typeof search.hygiene === "string" &&
      peopleHygieneFilters.has(search.hygiene)
    ) {
      validated.hygiene = search.hygiene;
    }
    return validated;
  },
});

const personDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/people/$personId",
  component: PersonDetailEditorial,
});

// Create route tree
const routeTree = rootRoute.addChildren([
  indexRoute,
  accountsRoute,
  accountDetailRoute,
  riskBriefingRoute,
  accountHealthRoute,
  ebrQbrRoute,
  swotRoute,
  accountReportRoute,
  weeklyImpactRoute,
  monthlyWrappedRoute,
  bookOfBusinessRoute,
  meReportRoute,
  actionDetailRoute,
  actionsRoute,
  emailsRoute,
  historyRoute,
  inboxRoute,
  meRoute,
  meetingHistoryDetailRoute,
  meetingDetailRoute,
  peopleRoute,
  personDetailRoute,
  projectsRoute,
  projectDetailRoute,
  settingsRoute,
  weekRoute,
]);

// Create router
export const router = createRouter({ routeTree });

// Register router types for type safety
declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
