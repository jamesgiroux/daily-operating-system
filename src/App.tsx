import { ThemeProvider } from "@/components/theme-provider";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";
import { AppSidebar } from "@/components/layout/AppSidebar";
import { CommandMenu, useCommandMenu } from "@/components/layout/CommandMenu";
import { Header } from "@/components/dashboard/Header";
import { Dashboard } from "@/components/dashboard/Dashboard";
import { DashboardSkeleton } from "@/components/dashboard/DashboardSkeleton";
import { DashboardEmpty } from "@/components/dashboard/DashboardEmpty";
import { DashboardError } from "@/components/dashboard/DashboardError";
import { useDashboardData } from "@/hooks/useDashboardData";

function DashboardContent() {
  const { state, refresh } = useDashboardData();

  switch (state.status) {
    case "loading":
      return <DashboardSkeleton />;
    case "empty":
      return <DashboardEmpty message={state.message} />;
    case "error":
      return <DashboardError message={state.message} onRetry={refresh} />;
    case "success":
      return <Dashboard data={state.data} />;
  }
}

function App() {
  const { open: commandOpen, setOpen: setCommandOpen } = useCommandMenu();

  return (
    <ThemeProvider defaultTheme="system" storageKey="dailyos-theme">
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <Header onCommandMenuOpen={() => setCommandOpen(true)} />
          <DashboardContent />
        </SidebarInset>
        <CommandMenu open={commandOpen} onOpenChange={setCommandOpen} />
      </SidebarProvider>
    </ThemeProvider>
  );
}

export default App;
