import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
} from "@/components/ui/sidebar";
import {
  CalendarDays,
  CheckSquare,
  Inbox,
  LayoutDashboard,
  Settings,
  Zap,
} from "lucide-react";

const navItems = {
  today: [
    {
      title: "Overview",
      icon: LayoutDashboard,
      isActive: true,
    },
  ],
  view: [
    {
      title: "Inbox",
      icon: Inbox,
      badge: 7,
    },
    {
      title: "Calendar",
      icon: CalendarDays,
    },
  ],
  actions: [
    {
      title: "Actions",
      icon: CheckSquare,
      badge: 3,
    },
  ],
};

export function AppSidebar() {
  return (
    <Sidebar collapsible="icon">
      <SidebarHeader>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton
              size="lg"
              className="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
            >
              <div className="flex aspect-square size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground group-data-[collapsible=icon]:size-8">
                <Zap className="size-4" />
              </div>
              <div className="grid flex-1 text-left text-sm leading-tight">
                <span className="truncate font-semibold">DailyOS</span>
                <span className="truncate text-xs text-muted-foreground">
                  Your day, ready
                </span>
              </div>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Today</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.today.map((item) => (
                <SidebarMenuItem key={item.title}>
                  <SidebarMenuButton isActive={item.isActive} tooltip={item.title}>
                    <item.icon />
                    <span>{item.title}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>View</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.view.map((item) => (
                <SidebarMenuItem key={item.title}>
                  <SidebarMenuButton tooltip={item.title}>
                    <item.icon />
                    <span>{item.title}</span>
                    {item.badge && (
                      <span className="ml-auto text-xs text-muted-foreground">
                        {item.badge}
                      </span>
                    )}
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>Actions</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {navItems.actions.map((item) => (
                <SidebarMenuItem key={item.title}>
                  <SidebarMenuButton tooltip={item.title}>
                    <item.icon />
                    <span>{item.title}</span>
                    {item.badge && (
                      <span className="ml-auto flex size-5 items-center justify-center rounded-full bg-primary/15 text-xs font-medium text-primary">
                        {item.badge}
                      </span>
                    )}
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Settings">
              <Settings />
              <span>Settings</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
        </SidebarMenu>
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}
