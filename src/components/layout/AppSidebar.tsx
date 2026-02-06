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
  Inbox,
  LayoutDashboard,
  Settings,
  Zap,
} from "lucide-react";
import { useInboxCount } from "@/hooks/useInbox";
import type { ProfileType } from "@/types";
import type { LucideIcon } from "lucide-react";

interface NavItem {
  title: string;
  icon: LucideIcon;
  href: string;
}

const todayItems: NavItem[] = [
  { title: "Today", icon: LayoutDashboard, href: "/" },
  { title: "This Week", icon: Calendar, href: "/week" },
];

const workspaceItems: NavItem[] = [
  { title: "Actions", icon: CheckSquare, href: "/actions" },
  { title: "Inbox", icon: Inbox, href: "/inbox" },
];

const profileNavItem: Record<ProfileType, NavItem> = {
  "customer-success": { title: "Accounts", icon: Building2, href: "/accounts" },
  general: { title: "Projects", icon: FolderKanban, href: "/projects" },
};

const profileLabel: Record<ProfileType, string> = {
  "customer-success": "Customer Success",
  general: "General",
};

export function AppSidebar() {
  const routerState = useRouterState();
  const currentPath = routerState.location.pathname;
  const [profile, setProfile] = useState<ProfileType>("general");
  const inboxCount = useInboxCount();

  useEffect(() => {
    async function loadProfile() {
      try {
        const config = await invoke<{ profile?: string }>("get_config");
        if (config.profile === "customer-success") {
          setProfile("customer-success");
        }
      } catch {
        // Config not loaded yet — default to general
      }
    }
    loadProfile();
  }, []);

  const profileItem = profileNavItem[profile];
  const allWorkspaceItems = [...workspaceItems, profileItem];

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
                    {profileLabel[profile]}
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
                    tooltip={item.title}
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
