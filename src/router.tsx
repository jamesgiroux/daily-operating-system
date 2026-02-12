import { useState, useEffect } from "react";
import {
  createRouter,
  createRootRoute,
  createRoute,
  Outlet,
} from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { ThemeProvider } from "@/components/theme-provider";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { CommandMenu, useCommandMenu } from "@/components/layout/CommandMenu";
import { Header } from "@/components/dashboard/Header";
import { OnboardingFlow } from "@/components/onboarding/OnboardingFlow";

// Lazy load pages for code splitting
import { Dashboard } from "@/components/dashboard/Dashboard";
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
import FocusPage from "@/pages/FocusPage";
import HistoryPage from "@/pages/HistoryPage";
import PeoplePage from "@/pages/PeoplePage";
import PersonDetailPage from "@/pages/PersonDetailPage";
import ProjectsPage from "@/pages/ProjectsPage";
import ProjectDetailPage from "@/pages/ProjectDetailPage";
import SettingsPage from "@/pages/SettingsPage";
import WeekPage from "@/pages/WeekPage";

// Global overlays
import { PostMeetingPrompt } from "@/components/PostMeetingPrompt";
import { Toaster } from "@/components/ui/sonner";
import { DevToolsPanel } from "@/components/devtools/DevToolsPanel";

// Root layout that wraps all pages
function RootLayout() {
  const { open: commandOpen, setOpen: setCommandOpen } = useCommandMenu();
  const [needsOnboarding, setNeedsOnboarding] = useState(false);
  const [checkingConfig, setCheckingConfig] = useState(true);

  useEffect(() => {
    async function checkConfig() {
      try {
        const config = await invoke<{ workspacePath?: string; entityMode?: string }>("get_config");
        // Show onboarding if config exists but workspace is missing/empty
        if (!config.workspacePath) {
          setNeedsOnboarding(true);
        }
      } catch {
        // No config at all — needs onboarding
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

  if (checkingConfig) {
    return (
      <ThemeProvider defaultTheme="system" storageKey="dailyos-theme">
        <div className="flex h-screen items-center justify-center bg-background" />
        <DevToolsPanel />
      </ThemeProvider>
    );
  }

  if (needsOnboarding) {
    return (
      <ThemeProvider defaultTheme="system" storageKey="dailyos-theme">
        <OnboardingFlow onComplete={handleOnboardingComplete} />
        <Toaster position="bottom-right" />
        <DevToolsPanel />
      </ThemeProvider>
    );
  }

  return (
    <ThemeProvider defaultTheme="system" storageKey="dailyos-theme">
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
    </ThemeProvider>
  );
}

// Dashboard page content
function DashboardPage() {
  const { state, refresh } = useDashboardData();
  const { runNow } = useWorkflow();

  switch (state.status) {
    case "loading":
      return <DashboardSkeleton />;
    case "empty":
      return <DashboardEmpty message={state.message} onGenerate={runNow} googleAuth={state.googleAuth} />;
    case "error":
      return <DashboardError message={state.message} onRetry={refresh} />;
    case "success":
      return <Dashboard data={state.data} freshness={state.freshness} />;
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

const inboxRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/inbox",
  component: InboxPage,
});

const emailsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/emails",
  component: EmailsPage,
});

const focusRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/focus",
  component: FocusPage,
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
  component: ProjectDetailPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/settings",
  component: SettingsPage,
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
});

const personDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/people/$personId",
  component: PersonDetailPage,
});

// Create route tree
const routeTree = rootRoute.addChildren([
  indexRoute,
  accountsRoute,
  accountDetailRoute,
  actionDetailRoute,
  actionsRoute,
  emailsRoute,
  focusRoute,
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
