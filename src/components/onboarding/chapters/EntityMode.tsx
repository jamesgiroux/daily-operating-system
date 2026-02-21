import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Check } from "lucide-react";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { EntityMode as EntityModeType } from "@/types";

interface EntityModeProps {
  onNext: (mode: EntityModeType) => void;
}

export function EntityMode({ onNext }: EntityModeProps) {
  const [presets, setPresets] = useState<[string, string, string][]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<[string, string, string][]>("get_available_presets")
      .then(setPresets)
      .catch((err) => {
        console.error("get_available_presets failed:", err);
        setPresets([]);
      });
  }, []);

  async function handleSelect(presetId: string) {
    if (saving) return;
    setSelected(presetId);
    setSaving(true);
    try {
      await invoke("set_role", { role: presetId });
      // Fetch the resulting entity mode from the active preset
      const preset = await invoke<{ defaultEntityMode: string } | null>("get_active_preset");
      const mode = (preset?.defaultEntityMode ?? "account") as EntityModeType;
      onNext(mode);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set role");
      setSelected(null);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div style={{ display: "flex", flexDirection: "column", gap: 24 }}>
      <ChapterHeading
        title="What's your role?"
        epigraph="This shapes your vitals, vocabulary, and how AI prepares your briefings. You can change anytime in Settings."
      />

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: 12,
        }}
      >
        {presets.map(([id, name, description]) => {
          const isSelected = selected === id;
          return (
            <button
              key={id}
              onClick={() => handleSelect(id)}
              disabled={saving && !isSelected}
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 6,
                padding: 16,
                textAlign: "left" as const,
                background: isSelected ? "var(--color-paper-warm-white)" : "none",
                border: isSelected
                  ? "2px solid var(--color-spice-turmeric)"
                  : "1px solid var(--color-rule-light)",
                borderRadius: 6,
                cursor: saving && !isSelected ? "default" : "pointer",
                opacity: saving && !isSelected ? 0.5 : 1,
                transition: "all 0.15s ease",
              }}
            >
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 600,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {name}
                </span>
                {isSelected && (
                  <Check
                    size={14}
                    style={{ color: "var(--color-spice-turmeric)", flexShrink: 0 }}
                  />
                )}
              </div>
              <span
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 12,
                  color: "var(--color-text-tertiary)",
                  lineHeight: 1.4,
                }}
              >
                {description}
              </span>
            </button>
          );
        })}
      </div>

      {presets.length === 0 && !saving && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            color: "var(--color-text-tertiary)",
            textAlign: "center",
          }}
        >
          Loading roles...
        </p>
      )}
    </div>
  );
}
