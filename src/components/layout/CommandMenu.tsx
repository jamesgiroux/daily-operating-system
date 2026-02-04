import * as React from "react";
import {
  CalendarDays,
  CheckSquare,
  Inbox,
  LayoutDashboard,
  Play,
  RefreshCw,
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

interface CommandMenuProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CommandMenu({ open, onOpenChange }: CommandMenuProps) {
  return (
    <CommandDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Command Menu"
      description="Search for commands and navigate"
    >
      <CommandInput placeholder="Type a command or search..." />
      <CommandList>
        <CommandEmpty>No results found.</CommandEmpty>

        <CommandGroup heading="Navigate">
          <CommandItem onSelect={() => onOpenChange(false)}>
            <LayoutDashboard className="mr-2 size-4" />
            <span>Overview</span>
          </CommandItem>
          <CommandItem onSelect={() => onOpenChange(false)}>
            <Inbox className="mr-2 size-4" />
            <span>Inbox</span>
          </CommandItem>
          <CommandItem onSelect={() => onOpenChange(false)}>
            <CalendarDays className="mr-2 size-4" />
            <span>Calendar</span>
          </CommandItem>
          <CommandItem onSelect={() => onOpenChange(false)}>
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
