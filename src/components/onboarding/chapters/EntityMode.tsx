import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Building2, FolderKanban, Layers, Check } from "lucide-react";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
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
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="How do you organize your work?"
        epigraph="This shapes your workspace. You can switch anytime in Settings."
      />

      <div style={{ display: "flex", flexDirection: "column", gap: 0 }}>
        {options.map((option) => {
          const Icon = option.icon;
          const isSelected = selected === option.id;
          return (
            <button
              key={option.id}
              onClick={() => !saving && handleSelect(option.id)}
              style={{
                display: "flex",
                alignItems: "flex-start",
                justifyContent: "space-between",
                gap: 16,
                padding: "16px 0",
                borderTop: "1px solid var(--color-rule-light)",
                borderLeft: isSelected ? "3px solid var(--color-spice-turmeric)" : "3px solid transparent",
                paddingLeft: isSelected ? 16 : 16,
                background: isSelected ? "var(--color-paper-warm-white)" : "transparent",
                cursor: saving && !isSelected ? "default" : "pointer",
                opacity: saving && !isSelected ? 0.5 : 1,
                textAlign: "left",
                border: "none",
                borderTopStyle: "solid",
                borderTopWidth: 1,
                borderTopColor: "var(--color-rule-light)",
                borderLeftStyle: "solid",
                borderLeftWidth: 3,
                borderLeftColor: isSelected ? "var(--color-spice-turmeric)" : "transparent",
                transition: "all 0.15s ease",
              }}
            >
              <div style={{ display: "flex", alignItems: "flex-start", gap: 12 }}>
                <Icon
                  size={20}
                  style={{
                    marginTop: 2,
                    flexShrink: 0,
                    color: "var(--color-text-tertiary)",
                  }}
                />
                <div>
                  <div
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 15,
                      fontWeight: 500,
                      color: "var(--color-text-primary)",
                    }}
                  >
                    {option.title}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 13,
                      color: "var(--color-text-secondary)",
                      marginTop: 2,
                    }}
                  >
                    {option.description}
                  </div>
                  <div
                    style={{
                      fontFamily: "var(--font-sans)",
                      fontSize: 12,
                      color: "var(--color-text-tertiary)",
                      marginTop: 4,
                    }}
                  >
                    {option.detail}
                  </div>
                </div>
              </div>
              {isSelected && (
                <Check
                  size={18}
                  style={{
                    flexShrink: 0,
                    color: "var(--color-spice-turmeric)",
                    marginTop: 2,
                  }}
                />
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}
