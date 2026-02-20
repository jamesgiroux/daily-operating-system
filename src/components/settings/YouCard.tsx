import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { usePersonality, type Personality } from "@/hooks/usePersonality";
import { toast } from "sonner";
import { Check, X, Loader2 } from "lucide-react";
import { styles } from "./styles";

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

function AboutYouSection({
  config,
}: {
  config: {
    userName?: string;
    userCompany?: string;
    userTitle?: string;
    userFocus?: string;
  } | null;
}) {
  const [name, setName] = useState("");
  const [company, setCompany] = useState("");
  const [title, setTitle] = useState("");
  const [focus, setFocus] = useState("");

  const initial = useRef({ name: "", company: "", title: "", focus: "" });

  useEffect(() => {
    if (!config) return;
    const n = config.userName ?? "";
    const c = config.userCompany ?? "";
    const t = config.userTitle ?? "";
    const f = config.userFocus ?? "";
    setName(n);
    setCompany(c);
    setTitle(t);
    setFocus(f);
    initial.current = { name: n, company: c, title: t, focus: f };
  }, [config]);

  async function saveIfChanged() {
    const current = {
      name: name.trim(),
      company: company.trim(),
      title: title.trim(),
      focus: focus.trim(),
    };
    if (
      current.name === initial.current.name &&
      current.company === initial.current.company &&
      current.title === initial.current.title &&
      current.focus === initial.current.focus
    )
      return;
    try {
      await invoke("set_user_profile", {
        name: current.name || null,
        company: current.company || null,
        title: current.title || null,
        focus: current.focus || null,
        domain: null,
      });
      initial.current = current;
      toast.success("Profile updated");
    } catch (err) {
      toast.error(typeof err === "string" ? err : "Failed to update profile");
    }
  }

  return (
    <div>
      <p style={styles.subsectionLabel}>About You</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Helps DailyOS personalize your briefings and meeting prep
      </p>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "1fr 1fr",
          gap: "20px 32px",
        }}
      >
        <div>
          <label htmlFor="profile-name" style={styles.fieldLabel}>
            Name
          </label>
          <input
            id="profile-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Jamie"
            style={styles.input}
          />
        </div>
        <div>
          <label htmlFor="profile-company" style={styles.fieldLabel}>
            Company
          </label>
          <input
            id="profile-company"
            value={company}
            onChange={(e) => setCompany(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Acme Inc."
            style={styles.input}
          />
          <p
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              color: "var(--color-text-tertiary)",
              marginTop: 4,
              letterSpacing: "0.04em",
            }}
          >
            Updates your internal organization entity
          </p>
        </div>
        <div>
          <label htmlFor="profile-title" style={styles.fieldLabel}>
            Title
          </label>
          <input
            id="profile-title"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Customer Success Manager"
            style={styles.input}
          />
        </div>
        <div>
          <label htmlFor="profile-focus" style={styles.fieldLabel}>
            Current focus
          </label>
          <input
            id="profile-focus"
            value={focus}
            onChange={(e) => setFocus(e.target.value)}
            onBlur={saveIfChanged}
            placeholder="e.g. Driving Q2 renewals"
            style={styles.input}
          />
        </div>
      </div>
    </div>
  );
}

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
      <p style={styles.subsectionLabel}>Your Domains</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Your organization's email domains -- used to distinguish internal vs
        external meetings
      </p>
      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          alignItems: "center",
          gap: 6,
          borderBottom: "1px solid var(--color-rule-light)",
          padding: "8px 0",
          minHeight: 36,
        }}
      >
        {domains.map((d) => (
          <span
            key={d}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              fontFamily: "var(--font-mono)",
              fontSize: 12,
              color: "var(--color-text-primary)",
              background: "var(--color-rule-light)",
              padding: "2px 8px",
              borderRadius: 3,
            }}
          >
            {d}
            <button
              type="button"
              onClick={() => removeDomain(d)}
              disabled={saving}
              style={{
                background: "none",
                border: "none",
                padding: 0,
                cursor: "pointer",
                color: "var(--color-text-tertiary)",
                display: "flex",
                alignItems: "center",
              }}
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
          style={{
            minWidth: 120,
            flex: 1,
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            color: "var(--color-text-primary)",
            background: "transparent",
            border: "none",
            outline: "none",
          }}
          disabled={!config}
        />
        {saving && (
          <Loader2
            size={14}
            className="animate-spin"
            style={{ color: "var(--color-text-tertiary)" }}
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
      .catch(() => setPresets([]));
    invoke<{ id: string } | null>("get_active_preset")
      .then((p) => setActiveId(p?.id ?? null))
      .catch(() => setActiveId(null));
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
      <p style={styles.subsectionLabel}>Role Presets</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Select your role to tailor vitals, vocabulary, and AI emphasis across
        DailyOS.
      </p>
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(3, 1fr)",
          gap: 12,
        }}
      >
        {presets.map(([id, name, description]) => {
          const isActive = id === activeId;
          return (
            <button
              key={id}
              type="button"
              onClick={() => handleSelect(id)}
              disabled={saving}
              style={{
                display: "flex",
                flexDirection: "column",
                gap: 6,
                padding: 16,
                textAlign: "left" as const,
                background: "none",
                border: isActive
                  ? "2px solid var(--color-spice-turmeric)"
                  : "1px solid var(--color-rule-light)",
                borderRadius: 6,
                cursor: saving && !isActive ? "default" : "pointer",
                opacity: saving && !isActive ? 0.5 : 1,
                transition: "all 0.15s ease",
                position: "relative" as const,
              }}
            >
              <div
                style={{ display: "flex", alignItems: "center", gap: 8 }}
              >
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
                {isActive && (
                  <Check
                    size={14}
                    style={{
                      color: "var(--color-spice-turmeric)",
                      flexShrink: 0,
                    }}
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
      {activeId && (
        <p
          style={{
            ...styles.monoLabel,
            marginTop: 12,
            color: "var(--color-spice-turmeric)",
          }}
        >
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
      <p style={styles.subsectionLabel}>Workspace</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        The directory where DailyOS stores briefings, actions, and files
      </p>
      <div style={styles.settingRow}>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            color: "var(--color-text-primary)",
          }}
        >
          {path || "Not configured"}
        </span>
        <button
          style={{
            ...styles.btn,
            ...styles.btnGhost,
            opacity: saving ? 0.5 : 1,
          }}
          onClick={handleChooseWorkspace}
          disabled={saving}
        >
          {saving ? (
            <span
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 6,
              }}
            >
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
      <p style={styles.subsectionLabel}>Personality</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Sets the tone for empty states, loading messages, and notifications
      </p>
      <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
        {PERSONALITY_OPTIONS.map((option) => {
          const isSelected = personality === option.value;
          return (
            <button
              key={option.value}
              onClick={() => handleChange(option.value)}
              style={{
                display: "flex",
                flexDirection: "column",
                alignItems: "flex-start",
                gap: 4,
                padding: "12px 16px",
                textAlign: "left" as const,
                background: "none",
                border: isSelected
                  ? "1px solid var(--color-desk-charcoal)"
                  : "1px solid var(--color-rule-light)",
                borderRadius: 4,
                cursor: "pointer",
                transition: "border-color 0.15s ease",
              }}
            >
              <div
                style={{ display: "flex", alignItems: "center", gap: 8 }}
              >
                <span
                  style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 14,
                    fontWeight: 500,
                    color: "var(--color-text-primary)",
                  }}
                >
                  {option.label}
                </span>
                {isSelected && (
                  <Check
                    size={14}
                    style={{ color: "var(--color-garden-sage)" }}
                  />
                )}
              </div>
              <span style={{ ...styles.description, fontSize: 12 }}>
                {option.description}
              </span>
              <span
                style={{
                  fontFamily: "var(--font-serif)",
                  fontSize: 12,
                  fontStyle: "italic",
                  color: "var(--color-text-tertiary)",
                  marginTop: 2,
                }}
              >
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
      <p style={styles.subsectionLabel}>Your Day</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        When does your workday start? DailyOS prepares your briefing before this time.
      </p>
      <div style={styles.settingRow}>
        <div>
          <span
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 14,
              color: "var(--color-text-primary)",
            }}
          >
            Morning briefing at{" "}
            <span style={{ fontWeight: 500 }}>{displayTime}</span>
          </span>
          {schedule?.timezone && (
            <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
              {schedule.timezone}
            </p>
          )}
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
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
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--color-text-primary)",
              background: "none",
              border: "1px solid var(--color-rule-heavy)",
              borderRadius: 4,
              padding: "4px 8px",
              opacity: saving ? 0.5 : 1,
            }}
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
// YouCard — consolidated identity settings
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
      .catch(() => {})
      .finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div>
        <p style={styles.subsectionLabel}>About You</p>
        <div
          style={{
            height: 40,
            background: "var(--color-rule-light)",
            borderRadius: 4,
            animation: "pulse 1.5s ease-in-out infinite",
          }}
        />
      </div>
    );
  }

  return (
    <div>
      <AboutYouSection config={config} />
      <hr style={styles.thinRule} />
      <DomainsSection config={config} />
      <hr style={styles.thinRule} />
      <RoleSection />
      <hr style={styles.thinRule} />
      <WorkspaceSection workspacePath={config?.workspacePath ?? ""} />
      <hr style={styles.thinRule} />
      <DayStartSection schedule={config?.schedules?.today ?? null} />
      <hr style={styles.thinRule} />
      <PersonalitySection />
    </div>
  );
}
