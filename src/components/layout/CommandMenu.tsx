import * as React from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import {
  CalendarDays,
  CheckSquare,
  Inbox,
  LayoutDashboard,
  Play,
  RefreshCw,
  Search,
} from "lucide-react";
import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
} from "@/components/ui/command";
import type { MeetingSearchResult } from "@/types";

interface CommandMenuProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CommandMenu({ open, onOpenChange }: CommandMenuProps) {
  const navigate = useNavigate();
  const [searchQuery, setSearchQuery] = React.useState("");
  const [meetingResults, setMeetingResults] = React.useState<
    MeetingSearchResult[]
  >([]);

  // Debounced meeting search
  React.useEffect(() => {
    if (!open) {
      setSearchQuery("");
      setMeetingResults([]);
      return;
    }
    if (searchQuery.trim().length < 2) {
      setMeetingResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const results = await invoke<MeetingSearchResult[]>("search_meetings", {
          query: searchQuery.trim(),
        });
        setMeetingResults(results);
      } catch {
        setMeetingResults([]);
      }
    }, 250);

    return () => clearTimeout(timer);
  }, [searchQuery, open]);

  const go = (path: string) => {
    onOpenChange(false);
    navigate({ to: path });
  };

  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Command Menu"
      description="Search for commands and navigate"
    >
      <CommandInput
        placeholder="Type a command or search meetings..."
        value={searchQuery}
        onValueChange={setSearchQuery}
      />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        {meetingResults.length > 0 && (
          <>
            <CommandGroup heading="Meetings">
              {meetingResults.map((m) => (
                <CommandItem
                  key={m.id}
                  value={`meeting-${m.id}-${m.title}`}
                  onSelect={() => go(`/meeting/${m.id}`)}
                >
                  <Search className="mr-2 size-4 shrink-0" />
                  <div className="flex min-w-0 flex-col">
                    <span className="truncate">{m.title}</span>
                    <span className="truncate text-xs text-muted-foreground">
                      {formatSearchDate(m.startTime)}
                      {m.accountName && ` \u00b7 ${m.accountName}`}
                    </span>
                  </div>
                </CommandItem>
              ))}
            </CommandGroup>
            <CommandSeparator />
          </>
        )}

        <CommandGroup heading="Navigate">
          <CommandItem onSelect={() => go("/")}>
            <LayoutDashboard className="mr-2 size-4" />
            <span>Overview</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/inbox")}>
            <Inbox className="mr-2 size-4" />
            <span>Inbox</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/week")}>
            <CalendarDays className="mr-2 size-4" />
            <span>Calendar</span>
          </CommandItem>
          <CommandItem onSelect={() => go("/actions")}>
            <CheckSquare className="mr-2 size-4" />
            <span>Actions</span>
          </CommandItem>
        </CommandGroup>

        <CommandSeparator />

        <CommandGroup heading="Quick Actions">
          <CommandItem onSelect={() => onOpenChange(false)}>
            <Play className="mr-2 size-4" />
            <span>Run Morning Briefing</span>
          </CommandItem>
          <CommandItem onSelect={() => onOpenChange(false)}>
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

function formatSearchDate(dateStr: string): string {
  try {
    const date = new Date(dateStr);
    return date.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr.split("T")[0] ?? dateStr;
  }
}
