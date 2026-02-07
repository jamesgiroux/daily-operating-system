import { useState, useEffect } from "react";
import { Link, useRouterState } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuBadge,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";
import {
  Building2,
  Calendar,
  CheckSquare,
  FolderKanban,
  History,
  Inbox,
  LayoutDashboard,
  Settings,
  Zap,
} from "lucide-react";
import { useInboxCount } from "@/hooks/useInbox";
import type { EntityMode } from "@/types";
import type { LucideIcon } from "lucide-react";

interface NavItem {
  title: string;
  icon: LucideIcon;
  href: string;
  tooltip?: string;
}

const todayItems: NavItem[] = [
  { title: "Today", icon: LayoutDashboard, href: "/" },
  { title: "This Week", icon: Calendar, href: "/week" },
];

const workspaceItems: NavItem[] = [
  { title: "Actions", icon: CheckSquare, href: "/actions" },
  { title: "Inbox", icon: Inbox, href: "/inbox", tooltip: "Document Inbox" },
  { title: "History", icon: History, href: "/history", tooltip: "Processing History" },
];

const accountsItem: NavItem = { title: "Accounts", icon: Building2, href: "/accounts" };
const projectsItem: NavItem = { title: "Projects", icon: FolderKanban, href: "/projects" };

function entityModeLabel(mode: EntityMode): string {
  switch (mode) {
    case "account": return "Account-based";
    case "project": return "Project-based";
    case "both": return "Accounts & Projects";
  }
}

function entityNavItems(mode: EntityMode): NavItem[] {
  switch (mode) {
    case "account": return [accountsItem];
    case "project": return [projectsItem];
    case "both": return [accountsItem, projectsItem];
  }
}

export function AppSidebar() {
  const routerState = useRouterState();
  const currentPath = routerState.location.pathname;
  const [entityMode, setEntityMode] = useState<EntityMode>("account");
  const inboxCount = useInboxCount();

  useEffect(() => {
    async function loadEntityMode() {
      try {
        const config = await invoke<{ entityMode?: string }>("get_config");
        if (config.entityMode === "project" || config.entityMode === "both" || config.entityMode === "account") {
          setEntityMode(config.entityMode);
        }
      } catch {
        // Config not loaded yet — default to account
      }
    }
    loadEntityMode();
  }, []);

  const allWorkspaceItems = [...workspaceItems, ...entityNavItems(entityMode)];

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
              tooltip="DailyOS"
              asChild
            >
              <Link to="/">
                <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground group-data-[collapsible=icon]:size-4 group-data-[collapsible=icon]:rounded-sm">
                  <Zap className="size-4 group-data-[collapsible=icon]:size-3" />
                </div>
                <div className="grid flex-1 text-left text-sm leading-tight">
                  <span className="truncate font-semibold">DailyOS</span>
                  <span className="truncate text-xs text-muted-foreground">
                    {entityModeLabel(entityMode)}
                  </span>
                </div>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Today</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {todayItems.map((item) => (
                <SidebarMenuItem key={item.title}>
                  <SidebarMenuButton
                    isActive={currentPath === item.href}
                    tooltip={item.title}
                    asChild
                  >
                    <Link to={item.href}>
                      <item.icon />
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Workspace</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {allWorkspaceItems.map((item) => (
                <SidebarMenuItem key={item.title}>
                  <SidebarMenuButton
                    isActive={currentPath === item.href}
                    tooltip={item.tooltip ?? item.title}
                    asChild
                  >
                    <Link to={item.href}>
                      <item.icon />
                      <span>{item.title}</span>
                    </Link>
                  </SidebarMenuButton>
                  {item.title === "Inbox" && inboxCount > 0 && (
                    <SidebarMenuBadge>{inboxCount}</SidebarMenuBadge>
                  )}
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              isActive={currentPath === "/settings"}
              tooltip="Settings"
              asChild
            >
              <Link to="/settings">
                <Settings />
                <span>Settings</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
