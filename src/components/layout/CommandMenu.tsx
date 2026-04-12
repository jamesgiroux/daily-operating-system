import * as React from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import {
  Building2,
  CalendarDays,
  CheckSquare,
  FolderKanban,
  Inbox,
  LayoutDashboard,
  Mail,
  Play,
  Plus,
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

/** Extract entity context from the current route for auto-linking actions. */
function useEntityContext(): {
  accountId?: string;
  projectId?: string;
  personId?: string;
} {
  const routerState = useRouterState();
  const params = routerState.matches[routerState.matches.length - 1]?.params as
    | Record<string, string>
    | undefined;
  if (!params) return {};
  return {
    accountId: params.accountId,
    projectId: params.projectId,
    personId: params.personId,
  };
}

export function CommandMenu({ open, onOpenChange }: CommandMenuProps) {
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = React.useState("");
  const [searchResults, setSearchResults] = React.useState<
    GlobalSearchResult[]
  >([]);
  const [addActionMode, setAddActionMode] = React.useState(false);
  const [actionTitle, setActionTitle] = React.useState("");
  const actionInputRef = React.useRef<HTMLInputElement>(null);
  const entityContext = useEntityContext();

  // Reset state when dialog closes
  React.useEffect(() => {
    if (!open) {
      setAddActionMode(false);
      setActionTitle("");
    }
  }, [open]);

  // Focus the action title input when entering add-action mode
  React.useEffect(() => {
    if (addActionMode) {
      // Small delay to let the input render
      const t = setTimeout(() => actionInputRef.current?.focus(), 50);
      return () => clearTimeout(t);
    }
  }, [addActionMode]);

  const handleCreateAction = React.useCallback(async () => {
    const trimmed = actionTitle.trim();
    if (!trimmed) return;
    try {
      await invoke("create_action", {
        request: {
          title: trimmed,
          priority: "P2",
          accountId: entityContext.accountId,
          projectId: entityContext.projectId,
          personId: entityContext.personId,
        },
      });
      toast.success("Action created");
      onOpenChange(false);
    } catch (err) {
      toast.error(
        err instanceof Error ? err.message : "Failed to create action"
      );
    }
  }, [actionTitle, entityContext, onOpenChange]);

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

  // Inline add-action mode: replace the command list with a title input
  if (addActionMode) {
    return (
      <CommandDialog
        open={open}
        onOpenChange={onOpenChange}
        title="Add Action"
        description="Create a new action item"
      >
        <div className="flex items-center border-b px-3">
          <Plus className="mr-2 size-4 shrink-0 opacity-50" />
          <input
            ref={actionInputRef}
            className="flex h-11 w-full rounded-md bg-transparent py-3 text-sm outline-none placeholder:text-muted-foreground"
            placeholder="Action title..."
            value={actionTitle}
            onChange={(e) => setActionTitle(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                handleCreateAction();
              } else if (e.key === "Escape") {
                e.preventDefault();
                setAddActionMode(false);
                setActionTitle("");
              }
            }}
          />
        </div>
        <div className="px-3 py-2 text-xs text-muted-foreground">
          {entityContext.accountId
            ? "Linked to current account"
            : entityContext.projectId
              ? "Linked to current project"
              : entityContext.personId
                ? "Linked to current person"
                : "No entity link"}
          {" \u00b7 "}Enter to create, Escape to cancel
        </div>
      </CommandDialog>
    );
  }

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
          <CommandItem
            value="add action task todo"
            onSelect={() => setAddActionMode(true)}
          >
            <Plus className="mr-2 size-4" />
            <span>Add Action</span>
          </CommandItem>
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
