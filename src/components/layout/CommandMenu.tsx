import * as React from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import {
  Building2,
  CalendarDays,
  CheckSquare,
  FolderKanban,
  Inbox,
  LayoutDashboard,
  Mail,
  Play,
  RefreshCw,
  Settings,
  UserCircle,
  Users,
} from "lucide-react";
import { toast } from "sonner";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import type { GlobalSearchResult } from "@/types";

interface CommandMenuProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const ENTITY_TYPE_ICON: Record<string, React.ElementType> = {
  account: Building2,
  person: Users,
  project: FolderKanban,
  meeting: CalendarDays,
  action: CheckSquare,
  email: Mail,
};

const ENTITY_TYPE_LABEL: Record<string, string> = {
  account: "Accounts",
  person: "People",
  project: "Projects",
  meeting: "Meetings",
  action: "Actions",
  email: "Emails",
};

export function CommandMenu({ open, onOpenChange }: CommandMenuProps) {
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = React.useState("");
  const [searchResults, setSearchResults] = React.useState<
    GlobalSearchResult[]
  >([]);

  // Debounced global search
  React.useEffect(() => {
    if (!open) {
      setSearchQuery("");
      setSearchResults([]);
      return;
    }
    if (searchQuery.trim().length < 2) {
      setSearchResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const results = await invoke<GlobalSearchResult[]>("search_global", {
          query: searchQuery.trim(),
        });
        setSearchResults(results);
      } catch {
        setSearchResults([]);
      }
    }, 250);

    return () => clearTimeout(timer);
  }, [searchQuery, open]);

  const go = (path: string) => {
    onOpenChange(false);
    navigate({ to: path });
  };

  // Group results by entity type
  const groupedResults = React.useMemo(() => {
    const groups: Record<string, GlobalSearchResult[]> = {};
    for (const result of searchResults) {
      if (!groups[result.entityType]) {
        groups[result.entityType] = [];
      }
      groups[result.entityType].push(result);
    }
    return groups;
  }, [searchResults]);

  const hasResults = searchResults.length > 0;

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Command Menu"
      description="Search for commands and navigate"
    >
      <CommandInput
        placeholder="Search everything..."
        value={searchQuery}
        onValueChange={setSearchQuery}
      />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        {hasResults && (
          <>
            {Object.entries(groupedResults).map(([entityType, results]) => {
              const Icon = ENTITY_TYPE_ICON[entityType] ?? CalendarDays;
              const heading = ENTITY_TYPE_LABEL[entityType] ?? entityType;
              return (
                <CommandGroup key={entityType} heading={heading}>
                  {results.map((r) => (
                    <CommandItem
                      key={`${r.entityType}-${r.entityId}`}
                      value={`${r.entityType}-${r.entityId}-${r.name}`}
                      onSelect={() => go(r.route)}
                    >
                      <Icon className="mr-2 size-4 shrink-0" />
                      <div className="flex min-w-0 flex-col">
                        <span className="truncate">{r.name}</span>
                        {r.secondaryText && (
                          <span className="truncate text-xs text-muted-foreground">
                            {r.secondaryText}
                          </span>
                        )}
                      </div>
                    </CommandItem>
                  ))}
                </CommandGroup>
              );
            })}
            <CommandSeparator />
          </>
        )}

        <CommandGroup heading="Navigate">
          <CommandItem onSelect={() => go("/")}>
            <LayoutDashboard className="mr-2 size-4" />
            <span>Today</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/week")}>
            <CalendarDays className="mr-2 size-4" />
            <span>This Week</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/emails")}>
            <Mail className="mr-2 size-4" />
            <span>Mail</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/actions")}>
            <CheckSquare className="mr-2 size-4" />
            <span>Actions</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/me")}>
            <UserCircle className="mr-2 size-4" />
            <span>Me</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/people")}>
            <Users className="mr-2 size-4" />
            <span>People</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/accounts")}>
            <Building2 className="mr-2 size-4" />
            <span>Accounts</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/projects")}>
            <FolderKanban className="mr-2 size-4" />
            <span>Projects</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/inbox")}>
            <Inbox className="mr-2 size-4" />
            <span>Inbox</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/settings")}>
            <Settings className="mr-2 size-4" />
            <span>Settings</span>
          </CommandItem>
        </CommandGroup>

        <CommandSeparator />

        <CommandGroup heading="Quick Actions">
          <CommandItem onSelect={() => {
            onOpenChange(false);
            invoke("run_workflow", { workflow: "today" })
              .then(() => toast.success("Morning briefing started"))
              .catch(() => toast.error("Failed to start briefing"));
          }}>
            <Play className="mr-2 size-4" />
            <span>Run Morning Briefing</span>
          </CommandItem>
          <CommandItem onSelect={() => {
            onOpenChange(false);
            window.location.reload();
          }}>
            <RefreshCw className="mr-2 size-4" />
            <span>Refresh Dashboard</span>
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}

export function useCommandMenu() {
  const [open, setOpen] = React.useState(false);

  React.useEffect(() => {
    const down = (e: KeyboardEvent) => {
      if (e.key === "k" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        setOpen((open) => !open);
      }
    };

    document.addEventListener("keydown", down);
    return () => document.removeEventListener("keydown", down);
  }, []);

  return { open, setOpen };
}
