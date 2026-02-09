import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { Building2, FolderKanban, X, ChevronsUpDown } from "lucide-react";

interface EntityOption {
  id: string;
  name: string;
  type: "account" | "project";
  parentName?: string;
}

interface EntityPickerProps {
  value: string | null;
  onChange: (id: string | null, name?: string) => void;
  entityType?: "account" | "project" | "all";
  placeholder?: string;
  /** When set, shows a non-removable chip (for pre-filled entity) */
  locked?: boolean;
  className?: string;
}

export function EntityPicker({
  value,
  onChange,
  entityType = "all",
  placeholder = "Link entity...",
  locked = false,
  className,
}: EntityPickerProps) {
  const [open, setOpen] = useState(false);
  const [entities, setEntities] = useState<EntityOption[]>([]);
  const [selectedName, setSelectedName] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    async function load() {
      const items: EntityOption[] = [];
      if (entityType !== "project") {
        try {
          const accounts = await invoke<
            { id: string; name: string; parentName?: string }[]
          >("get_accounts_for_picker");
          items.push(
            ...accounts.map((a) => ({
              id: a.id,
              name: a.name,
              type: "account" as const,
              parentName: a.parentName ?? undefined,
            }))
          );
        } catch {
          // accounts not available
        }
      }
      if (entityType !== "account") {
        try {
          const projects = await invoke<{ id: string; name: string }[]>(
            "get_projects_list"
          );
          items.push(
            ...projects.map((p) => ({
              id: p.id,
              name: p.name,
              type: "project" as const,
            }))
          );
        } catch {
          // projects not available
        }
      }
      setEntities(items);

      // Resolve name for current value
      if (value) {
        const match = items.find((e) => e.id === value);
        if (match) setSelectedName(match.name);
      }
    }
    load();
  }, [entityType, value]);

  // Close dropdown on outside click
  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  const parentAccounts = entities.filter(
    (e) => e.type === "account" && !e.parentName
  );
  const childAccounts = entities.filter(
    (e) => e.type === "account" && e.parentName
  );
  const projects = entities.filter((e) => e.type === "project");

  if (value && selectedName) {
    const entity = entities.find((e) => e.id === value);
    const Icon = entity?.type === "project" ? FolderKanban : Building2;
    return (
      <div className={cn("flex items-center gap-1.5", className)}>
        <span className="inline-flex items-center gap-1 rounded-md border bg-muted/50 px-2 py-0.5 text-xs">
          <Icon className="size-3 text-muted-foreground" />
          {selectedName}
          {!locked && (
            <button
              type="button"
              onClick={() => {
                onChange(null);
                setSelectedName(null);
              }}
              className="ml-0.5 text-muted-foreground hover:text-foreground"
            >
              <X className="size-3" />
            </button>
          )}
        </span>
      </div>
    );
  }

  return (
    <div ref={containerRef} className={cn("relative", className)}>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="h-7 gap-1 text-xs text-muted-foreground"
        onClick={() => setOpen(!open)}
      >
        <ChevronsUpDown className="size-3" />
        {placeholder}
      </Button>

      {open && (
        <div className="absolute top-8 left-0 z-50 w-64 rounded-md border bg-popover shadow-md">
          <Command>
            <CommandInput placeholder="Search..." />
            <CommandList>
              <CommandEmpty>No entities found.</CommandEmpty>
              {parentAccounts.length > 0 && (
                <CommandGroup heading="Accounts">
                  {parentAccounts.map((a) => {
                    const children = childAccounts.filter(
                      (c) => c.parentName === a.name
                    );
                    return (
                      <div key={a.id}>
                        <CommandItem
                          value={a.name}
                          onSelect={() => {
                            onChange(a.id, a.name);
                            setSelectedName(a.name);
                            setOpen(false);
                          }}
                        >
                          <Building2 className="mr-2 size-3.5 text-muted-foreground" />
                          {a.name}
                        </CommandItem>
                        {children.map((c) => (
                          <CommandItem
                            key={c.id}
                            value={`${a.name} ${c.name}`}
                            onSelect={() => {
                              onChange(c.id, c.name);
                              setSelectedName(c.name);
                              setOpen(false);
                            }}
                          >
                            <Building2 className="ml-4 mr-2 size-3.5 text-muted-foreground/60" />
                            <span className="text-muted-foreground">{c.name}</span>
                          </CommandItem>
                        ))}
                      </div>
                    );
                  })}
                </CommandGroup>
              )}
              {projects.length > 0 && (
                <CommandGroup heading="Projects">
                  {projects.map((p) => (
                    <CommandItem
                      key={p.id}
                      value={p.name}
                      onSelect={() => {
                        onChange(p.id, p.name);
                        setSelectedName(p.name);
                        setOpen(false);
                      }}
                    >
                      <FolderKanban className="mr-2 size-3.5 text-muted-foreground" />
                      {p.name}
                    </CommandItem>
                  ))}
                </CommandGroup>
              )}
            </CommandList>
          </Command>
        </div>
      )}
    </div>
  );
}
