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
import { ProfileSelector } from "@/components/ProfileSelector";

// Lazy load pages for code splitting
import { Dashboard } from "@/components/dashboard/Dashboard";
import { DashboardSkeleton } from "@/components/dashboard/DashboardSkeleton";
import { DashboardEmpty } from "@/components/dashboard/DashboardEmpty";
import { DashboardError } from "@/components/dashboard/DashboardError";
import { useDashboardData } from "@/hooks/useDashboardData";
import { useWorkflow } from "@/hooks/useWorkflow";

// Page components
import AccountsPage from "@/pages/AccountsPage";
import ActionsPage from "@/pages/ActionsPage";
import InboxPage from "@/pages/InboxPage";
import MeetingDetailPage from "@/pages/MeetingDetailPage";
import EmailsPage from "@/pages/EmailsPage";
import FocusPage from "@/pages/FocusPage";
import ProjectsPage from "@/pages/ProjectsPage";
import SettingsPage from "@/pages/SettingsPage";
import WeekPage from "@/pages/WeekPage";

// Global overlays
import { PostMeetingPrompt } from "@/components/PostMeetingPrompt";
import { WeekPlanningWizard } from "@/components/WeeklyPlanning/WeekPlanningWizard";
import { Toaster } from "@/components/ui/sonner";

import type { ProfileType } from "@/types";

// Root layout that wraps all pages
function RootLayout() {
  const { open: commandOpen, setOpen: setCommandOpen } = useCommandMenu();
  const [needsProfile, setNeedsProfile] = useState(false);

  useEffect(() => {
    async function checkProfile() {
      try {
        const config = await invoke<{ profile?: string }>("get_config");
        // Show selector if profile is missing, empty, or not set
        if (!config.profile) {
          setNeedsProfile(true);
        }
      } catch {
        // Config not loaded — don't show selector (bigger problem)
      }
    }
    checkProfile();
  }, []);

  function handleProfileSet(_profile: ProfileType) {
    setNeedsProfile(false);
    // Reload page to pick up new profile across all components
    window.location.reload();
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
      <ProfileSelector open={needsProfile} onProfileSet={handleProfileSet} />
      <PostMeetingPrompt />
      <WeekPlanningWizard />
      <Toaster position="bottom-right" />
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
      return <DashboardEmpty message={state.message} onGenerate={runNow} />;
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
});

const accountsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/accounts",
  component: AccountsPage,
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
  path: "/meeting/$prepFile",
  component: MeetingDetailPage,
});

const projectsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/projects",
  component: ProjectsPage,
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

// Create route tree
const routeTree = rootRoute.addChildren([
  indexRoute,
  accountsRoute,
  actionsRoute,
  emailsRoute,
  focusRoute,
  inboxRoute,
  meetingDetailRoute,
  projectsRoute,
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
