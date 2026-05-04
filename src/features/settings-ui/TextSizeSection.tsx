import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SettingsSectionLabel } from "@/components/settings/FormRow";
import s from "./TextSizeSection.module.css";

const SCALE_PRESETS = [90, 100, 110, 120] as const;

function applyTextScale(percent: number) {
  document.documentElement.style.zoom = `${percent / 100}`;
}

export default function TextSizeSection() {
  const [scale, setScale] = useState<number | null>(null);
  const [saveMessage, setSaveMessage] = useState("");

  useEffect(() => {
    invoke<{ textScalePercent?: number }>("get_config")
      .then((cfg) => {
        const value = cfg.textScalePercent ?? 100;
        setScale(value);
        applyTextScale(value);
      })
      .catch((err) => console.warn("Failed to load text scale config:", err));
  }, []);

  const save = useCallback(async (percent: number) => {
    setScale(percent);
    applyTextScale(percent);
    setSaveMessage("");
    try {
      await invoke("set_text_scale", { percent });
      setSaveMessage("Saved");
      setTimeout(() => setSaveMessage(""), 2000);
    } catch (err) {
      console.error("Failed to save text scale:", err);
      setSaveMessage("Failed to save");
    }
  }, []);

  if (scale === null) return null;

  return (
    <div className={s.container}>
      <SettingsSectionLabel>Text Size</SettingsSectionLabel>
      <p className={s.description}>
        Adjust the interface text size for readability.
      </p>

      <div className={s.presetRow}>
        {SCALE_PRESETS.map((preset) => (
          <button
            key={preset}
            type="button"
            className={s.presetButton}
            data-active={scale === preset}
            onClick={() => save(preset)}
          >
            {preset}%
          </button>
        ))}
      </div>

      <p className={s.preview} style={{ fontSize: `${14 * (scale / 100)}px` }}>
        The quick brown fox jumps over the lazy dog.
      </p>

      {saveMessage && <div className={s.saveStatus}>{saveMessage}</div>}
    </div>
  );
}
