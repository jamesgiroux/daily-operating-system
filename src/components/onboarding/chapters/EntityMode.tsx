import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Building2, FolderKanban, Layers, Check } from "lucide-react";
import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { EntityMode as EntityModeType } from "@/types";

interface EntityModeProps {
  onNext: (mode: EntityModeType) => void;
}

interface EntityModeOption {
  id: EntityModeType;
  title: string;
  description: string;
  detail: string;
  icon: typeof Building2;
}

const options: EntityModeOption[] = [
  {
    id: "account",
    title: "Account-based",
    description: "I manage external relationships — customers, clients, partners",
    detail: "Meetings, prep, and actions organized around the companies and people you work with.",
    icon: Building2,
  },
  {
    id: "project",
    title: "Project-based",
    description: "I manage internal efforts — features, campaigns, initiatives",
    detail: "Meetings, prep, and actions organized around the initiatives you're driving.",
    icon: FolderKanban,
  },
  {
    id: "both",
    title: "Both",
    description: "I manage relationships and initiatives",
    detail: "Accounts and Projects side by side. For roles that manage relationships and run initiatives.",
    icon: Layers,
  },
];

export function EntityMode({ onNext }: EntityModeProps) {
  const [selected, setSelected] = useState<EntityModeType | null>(null);
  const [saving, setSaving] = useState(false);

  async function handleSelect(mode: EntityModeType) {
    setSelected(mode);
    setSaving(true);
    try {
      await invoke("set_entity_mode", { mode });
      onNext(mode);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set entity mode");
      setSelected(null);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-5">
      <div className="space-y-2">
        <h2 className="text-2xl font-semibold tracking-tight">
          How do you organize your work?
        </h2>
        <p className="text-sm text-muted-foreground">
          This shapes your workspace. You can switch anytime in Settings.
        </p>
      </div>

      <div className="grid gap-3">
        {options.map((option) => {
          const Icon = option.icon;
          const isSelected = selected === option.id;
          return (
            <Card
              key={option.id}
              className={cn(
                "cursor-pointer transition-all hover:-translate-y-0.5 hover:shadow-lg",
                isSelected && "border-primary ring-1 ring-primary",
                saving && !isSelected && "pointer-events-none opacity-50",
              )}
              onClick={() => !saving && handleSelect(option.id)}
            >
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <div className="flex size-10 items-center justify-center rounded-lg bg-muted">
                      <Icon className="size-5" />
                    </div>
                    <div>
                      <CardTitle className="text-base">{option.title}</CardTitle>
                      <CardDescription>{option.description}</CardDescription>
                      <p className="mt-1 text-xs text-muted-foreground/70">
                        {option.detail}
                      </p>
                    </div>
                  </div>
                  {isSelected && (
                    <div className="flex size-6 items-center justify-center rounded-full bg-primary text-primary-foreground">
                      <Check className="size-4" />
                    </div>
                  )}
                </div>
              </CardHeader>
            </Card>
          );
        })}
      </div>
    </div>
  );
}
