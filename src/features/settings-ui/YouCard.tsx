import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { usePersonality, type Personality } from "@/hooks/usePersonality";
import { toast } from "sonner";
import { Check, X, Loader2 } from "lucide-react";
import s from "./YouCard.module.css";
import type { FeatureFlags } from "@/types";

// ═══════════════════════════════════════════════════════════════════════════
// Constants
// ═══════════════════════════════════════════════════════════════════════════

const PERSONALITY_OPTIONS = [
  {
    value: "professional",
    label: "Professional",
    description: "Straightforward, clean copy",
    example: "No data yet.",
  },
  {
    value: "friendly",
    label: "Friendly",
    description: "Warm, encouraging tone",
    example: "Nothing here yet — we'll have this ready for you soon.",
  },
  {
    value: "playful",
    label: "Playful",
    description: "Personality-rich, fun",
    example: "The hamsters are still running. Data incoming.",
  },
] as const;

// ═══════════════════════════════════════════════════════════════════════════
// Sub-sections
// ═══════════════════════════════════════════════════════════════════════════

function DomainsSection({
  config,
}: {
  config: { userDomains?: string[]; userDomain?: string } | null;
}) {
  const [domains, setDomains] = useState<string[]>([]);
  const [inputValue, setInputValue] = useState("");
  const [saving, setSaving] = useState(false);
  const initialized = useRef(false);

  useEffect(() => {
    if (!config || initialized.current) return;
    initialized.current = true;
    const loaded =
      config.userDomains ?? (config.userDomain ? [config.userDomain] : []);
    setDomains(loaded.filter(Boolean));
  }, [config]);

  async function saveDomains(next: string[]) {
    setSaving(true);
    try {
      const updated = await invoke<{
        userDomains?: string[];
        userDomain?: string;
      }>("set_user_domains", { domains: next.join(", ") });
      const saved =
        updated.userDomains ??
        (updated.userDomain ? [updated.userDomain] : []);
      setDomains(saved.filter(Boolean));
      toast.success("Domains updated");
    } catch (err) {
      toast.error(
        typeof err === "string" ? err : "Failed to update domains",
      );
    } finally {
      setSaving(false);
    }
  }

  function addDomain(raw: string) {
    const d = raw.trim().toLowerCase().replace(/^@/, "");
    if (!d || domains.includes(d)) return;
    const next = [...domains, d];
    setDomains(next);
    setInputValue("");
    saveDomains(next);
  }

  function removeDomain(d: string) {
    const next = domains.filter((x) => x !== d);
    setDomains(next);
    saveDomains(next);
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (
      (e.key === "," || e.key === "Enter" || e.key === "Tab") &&
      inputValue.trim()
    ) {
      e.preventDefault();
      addDomain(inputValue);
    }
    if (e.key === "Backspace" && !inputValue && domains.length > 0) {
      removeDomain(domains[domains.length - 1]);
    }
  }

  return (
    <div>
      <p className={s.subsectionLabel}>Your Domains</p>
      <p className={s.description}>
        Your organization's email domains -- used to distinguish internal vs
        external meetings
      </p>
      <div className={s.domainsInput}>
        {domains.map((d) => (
          <span key={d} className={s.domainChip}>
            {d}
            <button
              type="button"
              onClick={() => removeDomain(d)}
              disabled={saving}
              className={s.domainChipRemove}
            >
              <X size={12} />
            </button>
          </span>
        ))}
        <input
          type="text"
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value.replace(",", ""))}
          onKeyDown={handleKeyDown}
          onBlur={() => {
            if (inputValue.trim()) addDomain(inputValue);
          }}
          placeholder={domains.length === 0 ? "example.com" : ""}
          className={s.domainTextInput}
          disabled={!config}
        />
        {saving && (
          <Loader2
            size={14}
            className={`animate-spin ${s.iconSpinner}`}
          />
        )}
      </div>
    </div>
  );
}

function RoleSection() {
  const [presets, setPresets] = useState<[string, string, string][]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<[string, string, string][]>("get_available_presets")
      .then(setPresets)
      .catch((err) => {
        console.error("get_available_presets failed:", err);
        setPresets([]);
      });
    invoke<{ id: string } | null>("get_active_preset")
      .then((p) => setActiveId(p?.id ?? null))
      .catch((err) => {
        console.error("get_active_preset failed:", err);
        setActiveId(null);
      });
  }, []);

  async function handleSelect(presetId: string) {
    if (presetId === activeId || saving) return;
    setSaving(true);
    try {
      await invoke("set_role", { role: presetId });
      setActiveId(presetId);
      toast.success("Role updated -- reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to set role");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <p className={s.subsectionLabel}>Role Presets</p>
      <p className={s.description}>
        Select your role to tailor vitals, vocabulary, and AI emphasis across
        DailyOS.
      </p>
      <div className={s.roleGrid}>
        {presets.map(([id, name, description]) => {
          const isActive = id === activeId;
          return (
            <button
              key={id}
              type="button"
              onClick={() => handleSelect(id)}
              disabled={saving && !isActive}
              className={`${s.roleCard} ${isActive ? s.roleCardActive : ""}`}
            >
              <div className={s.roleCardHeader}>
                <span className={s.roleCardName}>{name}</span>
                {isActive && (
                  <Check
                    size={14}
                    className={s.iconCheck}
                  />
                )}
              </div>
              <span className={s.roleCardDescription}>{description}</span>
            </button>
          );
        })}
      </div>
      {activeId && (
        <p className={s.activePresetLabel}>
          Active: {presets.find(([id]) => id === activeId)?.[1] ?? activeId}
        </p>
      )}
    </div>
  );
}

function WorkspaceSection({
  workspacePath,
}: {
  workspacePath: string;
}) {
  const [path, setPath] = useState(workspacePath);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setPath(workspacePath);
  }, [workspacePath]);

  async function handleChooseWorkspace() {
    const selected = await open({
      directory: true,
      title: "Choose workspace directory",
    });
    if (!selected) return;

    setSaving(true);
    try {
      await invoke("set_workspace_path", { path: selected });
      setPath(selected);
      toast.success("Workspace updated -- reloading...");
      setTimeout(() => window.location.reload(), 800);
    } catch (err) {
      toast.error(
        typeof err === "string" ? err : "Failed to set workspace",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <p className={s.subsectionLabel}>Workspace</p>
      <p className={s.description}>
        The directory where DailyOS stores briefings, actions, and files
      </p>
      <div className={s.settingRow}>
        <span className={s.workspacePath}>
          {path || "Not configured"}
        </span>
        <button
          className={saving ? s.btnGhostDisabled : s.btnGhost}
          onClick={handleChooseWorkspace}
          disabled={saving}
        >
          {saving ? (
            <span className={s.savingInline}>
              <Loader2 size={12} className="animate-spin" /> ...
            </span>
          ) : (
            "Change"
          )}
        </button>
      </div>
    </div>
  );
}

function PersonalitySection() {
  const { personality, setPersonality: setCtxPersonality } = usePersonality();

  async function handleChange(value: string) {
    const previous = personality;
    setCtxPersonality(value as Personality);
    try {
      await invoke("set_personality", { personality: value });
      toast.success("Personality updated");
    } catch (err) {
      setCtxPersonality(previous);
      toast.error(
        typeof err === "string" ? err : "Failed to update personality",
      );
    }
  }

  return (
    <div>
      <p className={s.subsectionLabel}>Personality</p>
      <p className={s.description}>
        Sets the tone for empty states, loading messages, and notifications
      </p>
      <div className={s.personalityList}>
        {PERSONALITY_OPTIONS.map((option) => {
          const isSelected = personality === option.value;
          return (
            <button
              key={option.value}
              onClick={() => handleChange(option.value)}
              className={`${s.personalityCard} ${isSelected ? s.personalityCardActive : ""}`}
            >
              <div className={s.personalityCardHeader}>
                <span className={s.personalityCardLabel}>
                  {option.label}
                </span>
                {isSelected && (
                  <Check
                    size={14}
                    className={s.iconCheckSage}
                  />
                )}
              </div>
              <span className={s.descriptionSmall}>
                {option.description}
              </span>
              <span className={s.personalityExample}>
                "{option.example}"
              </span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ═══════════════════════════════════════════════════════════════════════════
// DayStartSection — user-friendly time picker for morning briefing schedule
// ═══════════════════════════════════════════════════════════════════════════

function DayStartSection({
  schedule,
}: {
  schedule: { cron: string; timezone: string } | null;
}) {
  const parsed = parseCronTime(schedule?.cron);
  const [hour, setHour] = useState(parsed.hour);
  const [minute, setMinute] = useState(parsed.minute);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const p = parseCronTime(schedule?.cron);
    setHour(p.hour);
    setMinute(p.minute);
  }, [schedule?.cron]);

  async function handleSave(h: number, m: number) {
    setSaving(true);
    try {
      await invoke("set_schedule", {
        workflow: "today",
        hour: h,
        minute: m,
        timezone: schedule?.timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone,
      });
      setHour(h);
      setMinute(m);
      toast.success("Briefing time updated");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update schedule");
    } finally {
      setSaving(false);
    }
  }

  const displayTime = formatHumanTime(hour, minute);

  return (
    <div>
      <p className={s.subsectionLabel}>Your Day</p>
      <p className={s.description}>
        When does your workday start? DailyOS prepares your briefing before this time.
      </p>
      <div className={s.settingRow}>
        <div>
          <span className={s.dayStartLabel}>
            Morning briefing at{" "}
            <span className={s.dayStartBold}>{displayTime}</span>
          </span>
          {schedule?.timezone && (
            <p className={s.descriptionSmallSpaced}>
              {schedule.timezone}
            </p>
          )}
        </div>
        <div className={s.timeInputRow}>
          <input
            type="time"
            value={`${hour.toString().padStart(2, "0")}:${minute.toString().padStart(2, "0")}`}
            onChange={(e) => {
              const [h, m] = e.target.value.split(":").map(Number);
              if (!isNaN(h) && !isNaN(m)) {
                handleSave(h, m);
              }
            }}
            disabled={saving}
            className={s.timeInput}
          />
        </div>
      </div>
    </div>
  );
}

function parseCronTime(cron?: string): { hour: number; minute: number } {
  if (!cron) return { hour: 8, minute: 0 };
  const parts = cron.split(" ");
  if (parts.length < 2) return { hour: 8, minute: 0 };
  const m = parseInt(parts[0], 10);
  const h = parseInt(parts[1], 10);
  if (isNaN(m) || isNaN(h)) return { hour: 8, minute: 0 };
  return { hour: h, minute: m };
}

function formatHumanTime(hour: number, minute: number): string {
  const h = hour % 12 || 12;
  const ampm = hour < 12 ? "AM" : "PM";
  const m = minute.toString().padStart(2, "0");
  return `${h}:${m} ${ampm}`;
}

// ═══════════════════════════════════════════════════════════════════════════
// YouCard — consolidated identity settings with 3 subsection groups
// ═══════════════════════════════════════════════════════════════════════════

export default function YouCard() {
  const [config, setConfig] = useState<{
    userName?: string;
    userCompany?: string;
    userTitle?: string;
    userFocus?: string;
    userDomains?: string[];
    userDomain?: string;
    workspacePath?: string;
    schedules?: {
      today?: { cron: string; timezone: string };
    };
  } | null>(null);
  const [loading, setLoading] = useState(true);
  const [rolePresetsEnabled, setRolePresetsEnabled] = useState(false);

  useEffect(() => {
    invoke<{
      userName?: string;
      userCompany?: string;
      userTitle?: string;
      userFocus?: string;
      userDomains?: string[];
      userDomain?: string;
      workspacePath?: string;
      schedules?: {
        today?: { cron: string; timezone: string };
      };
    }>("get_config")
      .then(setConfig)
      .catch((err) => console.error("get_config (you) failed:", err))
      .finally(() => setLoading(false));
    invoke<FeatureFlags>("get_feature_flags")
      .then((flags) => setRolePresetsEnabled(flags.role_presets_enabled))
      .catch(() => setRolePresetsEnabled(false));
  }, []);

  if (loading) {
    return (
      <div>
        <p className={s.subsectionLabel}>Workspace</p>
        <div className={s.skeleton} />
      </div>
    );
  }

  return (
    <div>
      {/* ── Identity ── */}
      <div className={s.subsectionGroup}>
        <h3 className={s.subsectionTitle}>Identity</h3>
        <DomainsSection config={config} />
      </div>

      {/* ── Workspace ── */}
      <div className={s.subsectionGroup}>
        <h3 className={s.subsectionTitle}>Workspace</h3>
        <WorkspaceSection workspacePath={config?.workspacePath ?? ""} />
        <hr className={s.thinRule} />
        <DayStartSection schedule={config?.schedules?.today ?? null} />
      </div>

      {/* ── Preferences ── */}
      <div className={s.subsectionGroup}>
        <h3 className={s.subsectionTitle}>Preferences</h3>
        {rolePresetsEnabled && (
          <>
            <RoleSection />
            <hr className={s.thinRule} />
          </>
        )}
        <PersonalitySection />
      </div>
    </div>
  );
}
