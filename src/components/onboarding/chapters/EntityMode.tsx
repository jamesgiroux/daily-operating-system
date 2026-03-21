import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Check } from "lucide-react";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { EntityMode as EntityModeType } from "@/types";
import styles from "../onboarding.module.css";

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
        console.error("get_available_presets failed:", err); // Expected: background init on mount
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
    <div className={`${styles.flexCol} ${styles.gap24}`}>
      <ChapterHeading
        title="What's your role?"
        epigraph="This shapes your vitals, vocabulary, and how AI prepares your briefings. You can change anytime in Settings."
      />

      <div className={styles.presetGrid}>
        {presets.map(([id, name, description]) => {
          const isSelected = selected === id;
          return (
            <button
              key={id}
              onClick={() => handleSelect(id)}
              disabled={saving && !isSelected}
              className={`${styles.presetCard} ${isSelected ? styles.presetCardSelected : ""} ${saving && !isSelected ? styles.presetCardDisabled : ""}`}
            >
              <div className={`${styles.flexRow} ${styles.gap8}`}>
                <span className={styles.presetName}>
                  {name}
                </span>
                {isSelected && (
                  <Check
                    size={14}
                    className={`${styles.accentColor} ${styles.flexShrink0}`}
                  />
                )}
              </div>
              <span className={styles.presetDesc}>
                {description}
              </span>
            </button>
          );
        })}
      </div>

      {presets.length === 0 && !saving && (
        <p className={styles.loadingText}>
          Loading roles...
        </p>
      )}
    </div>
  );
}
