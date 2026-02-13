import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import { cn } from "@/lib/utils";
import { Building2, FolderKanban, X, ChevronsUpDown } from "lucide-react";

interface EntityOption {
  id: string;
  name: string;
  type: "account" | "project";
  parentName?: string;
  isInternal?: boolean;
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

  useEffect(() => {
    async function load() {
      const items: EntityOption[] = [];
      if (entityType !== "project") {
        try {
          const accounts = await invoke<
            { id: string; name: string; parentName?: string; isInternal: boolean }[]
          >("get_accounts_for_picker");
          items.push(
            ...accounts.map((a) => ({
              id: a.id,
              name: a.name,
              type: "account" as const,
              parentName: a.parentName ?? undefined,
              isInternal: a.isInternal,
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

  const internalAccounts = entities.filter(
    (e) => e.type === "account" && e.isInternal
  );
  const externalParentAccounts = entities.filter(
    (e) => e.type === "account" && !e.isInternal && !e.parentName
  );
  const externalChildAccounts = entities.filter(
    (e) => e.type === "account" && !e.isInternal && e.parentName
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
              onClick={(e) => {
                e.stopPropagation();
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
    <Popover open={open} onOpenChange={setOpen}>
      {/* No asChild — PopoverTrigger renders its own <button> so Radix can
          attach the ref it needs (React 18 Button lacks forwardRef). */}
      <PopoverTrigger
        className={cn(
          "inline-flex items-center justify-center gap-1 whitespace-nowrap rounded-md border bg-background px-2.5 h-7 text-xs text-muted-foreground shadow-xs hover:bg-accent hover:text-accent-foreground transition-all",
          className
        )}
        onClick={(e) => e.stopPropagation()}
      >
        <ChevronsUpDown className="size-3" />
        {placeholder}
      </PopoverTrigger>
      <PopoverContent
        className="w-64 p-0"
        align="start"
        onOpenAutoFocus={(e) => e.preventDefault()}
      >
        <Command>
          <CommandInput placeholder="Search..." />
          <CommandList>
            <CommandEmpty>No entities found.</CommandEmpty>
            {internalAccounts.length > 0 && (
              <CommandGroup heading="Internal Teams">
                {internalAccounts.map((a) => (
                  <CommandItem
                    key={a.id}
                    value={`internal ${a.name}`}
                    onSelect={() => {
                      onChange(a.id, a.name);
                      setSelectedName(a.name);
                      setOpen(false);
                    }}
                  >
                    <Building2 className="mr-2 size-3.5 text-primary" />
                    {a.name}
                  </CommandItem>
                ))}
              </CommandGroup>
            )}
            {externalParentAccounts.length > 0 && (
              <CommandGroup heading="External Accounts">
                {externalParentAccounts.map((a) => {
                  const children = externalChildAccounts.filter(
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
      </PopoverContent>
    </Popover>
  );
}
