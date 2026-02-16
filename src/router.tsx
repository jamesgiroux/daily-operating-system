import { useState, useEffect } from "react";
import {
  createRouter,
  createRootRoute,
  createRoute,
  Outlet,
  useRouterState,
  useNavigate,
} from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { ThemeProvider } from "@/components/theme-provider";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";
import { AppSidebar } from "@/components/layout/AppSidebar";
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
import AccountDetailEditorial from "@/pages/AccountDetailEditorial";
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
import SettingsPage from "@/pages/SettingsPage";
import WeekPage from "@/pages/WeekPage";


// Magazine shell
import MagazinePageLayout from "@/components/layout/MagazinePageLayout";
import { MagazineShellContext, useMagazineShellProvider } from "@/hooks/useMagazineShell";

// Global overlays
import { PostMeetingPrompt } from "@/components/PostMeetingPrompt";
import { Toaster } from "@/components/ui/sonner";
import { DevToolsPanel } from "@/components/devtools/DevToolsPanel";
import { useNotifications } from "@/hooks/useNotifications";
import { PersonalityProvider } from "@/hooks/usePersonality";

const settingsTabs = new Set([
  "profile",
  "integrations",
  "workflows",
  "intelligence",
  "hygiene",
  "diagnostics",
]);
const peopleRelationshipTabs = new Set(["all", "external", "internal", "unknown"]);
const peopleHygieneFilters = new Set(["unnamed", "duplicates"]);

// Route IDs that use the magazine shell instead of the sidebar shell.
// Add new editorial routes here as they're built.
const MAGAZINE_ROUTE_IDS = new Set(["/", "/week", "/actions", "/actions/$actionId", "/accounts", "/projects", "/people", "/accounts/$accountId", "/accounts/$accountId/risk-briefing", "/projects/$projectId", "/people/$personId", "/emails", "/inbox", "/history", "/settings", "/meeting/$meetingId", "/meeting/history/$meetingId"]);

// Root layout that wraps all pages
function RootLayout() {
  const { open: commandOpen, setOpen: setCommandOpen } = useCommandMenu();
  useNotifications();
  const navigate = useNavigate();
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  const [checkingConfig, setCheckingConfig] = useState(true);

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
        }>("get_config");
        if (!config.workspacePath) {
          setNeedsOnboarding(true);
        }
      } catch {
        setNeedsOnboarding(true);
      } finally {
        setCheckingConfig(false);
      }
    }
    checkConfig();
  }, []);

  function handleOnboardingComplete() {
    setNeedsOnboarding(false);
    window.location.reload();
  }

  // Navigation handler for FloatingNavIsland
  function handleNavNavigate(page: string) {
    const routes: Record<string, string> = {
      today: "/",
      week: "/week",
      inbox: "/inbox",
      actions: "/actions",
      people: "/people",
      accounts: "/accounts",
      projects: "/projects",
      settings: "/settings",
    };
    const path = routes[page];
    if (path) navigate({ to: path });
  }

  if (checkingConfig) {
    return (
      <ThemeProvider>
        <div className="flex h-screen items-center justify-center bg-background" />
        <DevToolsPanel />
      </ThemeProvider>
    );
  }

  if (needsOnboarding) {
    return (
      <ThemeProvider>
        <OnboardingFlow onComplete={handleOnboardingComplete} />
        <Toaster position="bottom-right" />
        <DevToolsPanel />
      </ThemeProvider>
    );
  }

  // Magazine shell for editorial pages (account detail, future editorial pages)
  if (useMagazineShell) {
    return (
      <ThemeProvider>
        <PersonalityProvider>
          <MagazineShellContext.Provider value={magazineShell}>
            <MagazinePageLayout
              onFolioSearch={() => setCommandOpen(true)}
              onNavigate={handleNavNavigate}
              onNavHome={() => navigate({ to: "/" })}
            >
              <Outlet />
            </MagazinePageLayout>
          </MagazineShellContext.Provider>
          <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
          <PostMeetingPrompt />
          <Toaster position="bottom-right" />
          <DevToolsPanel />
        </PersonalityProvider>
      </ThemeProvider>
    );
  }

  // Standard sidebar shell for all other pages
  return (
    <ThemeProvider>
      <PersonalityProvider>
        <SidebarProvider defaultOpen={false}>
          <AppSidebar />
          <SidebarInset>
            <Header onCommandMenuOpen={() => setCommandOpen(true)} />
            <Outlet />
          </SidebarInset>
          <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
        </SidebarProvider>
        <PostMeetingPrompt />
        <Toaster position="bottom-right" />
        <DevToolsPanel />
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
      return <DashboardEmpty message={state.message} onGenerate={runNow} googleAuth={state.googleAuth} />;
    case "error":
      return <DashboardError message={state.message} onRetry={refresh} />;
    case "success":
      return <DailyBriefing data={state.data} freshness={state.freshness} onRunBriefing={runNow} isRunning={isRunning} workflowStatus={status} />;
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
  component: AccountDetailEditorial,
});

const riskBriefingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts/$accountId/risk-briefing",
  component: RiskBriefingPage,
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
  actionDetailRoute,
  actionsRoute,
  emailsRoute,
  historyRoute,
  inboxRoute,

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
