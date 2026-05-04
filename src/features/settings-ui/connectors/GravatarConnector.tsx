import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { GravatarStatus } from "@/types";
import {
  SettingsButton,
  SettingsInput,
  SettingsSectionLabel,
  SettingsStatusDot,
  formRowStyles,
} from "@/components/settings/FormRow";

export default function GravatarConnection() {
  const [status, setStatus] = useState<GravatarStatus | null>(null);
  const [fetching, setFetching] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [apiKeyDirty, setApiKeyDirty] = useState(false);

  useEffect(() => {
    invoke<GravatarStatus>("get_gravatar_status")
      .then((s) => {
        setStatus(s);
        if (s.apiKeySet) setApiKey("\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022");
      })
      .catch((err) => console.error("get_gravatar_status failed:", err)); // Expected: background init on mount
  }, []);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_gravatar_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Gravatar:", err);
      toast.error("Failed to toggle Gravatar");
    }
  }

  async function saveApiKey() {
    const trimmed = apiKey.trim();
    if (!trimmed || trimmed === "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022") return;
    try {
      await invoke("set_gravatar_api_key", { apiKey: trimmed });
      setApiKeyDirty(false);
      setStatus((prev) => prev ? { ...prev, apiKeySet: true } : prev);
      toast("Gravatar API key saved");
    } catch (err) {
      toast.error("Failed to save API key");
    }
  }

  async function handleFetchNow() {
    setFetching(true);
    try {
      const count = await invoke<number>("bulk_fetch_gravatars");
      toast(`Fetched ${count} Gravatar profile${count !== 1 ? "s" : ""}`);
      const refreshed = await invoke<GravatarStatus>("get_gravatar_status");
      setStatus(refreshed);
    } catch (err) {
      toast.error("Gravatar fetch failed");
    } finally {
      setFetching(false);
    }
  }

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : status.enabled && status.cachedCount > 0
      ? "var(--color-garden-olive)"
      : "var(--color-text-tertiary)";

  const statusLabel = !status
    ? "Loading..."
    : `${status.cachedCount} profile${status.cachedCount !== 1 ? "s" : ""} cached`;

  return (
    <div>
      <SettingsSectionLabel>Gravatar Avatars</SettingsSectionLabel>
      <p className={formRowStyles.descriptionLead}>
        Fetch profile photos for your contacts from Gravatar
      </p>

      <div className={formRowStyles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p className={formRowStyles.descriptionSmallTop2}>
            {status?.enabled
              ? "Avatars will be fetched for contacts with email addresses"
              : "Gravatar avatar fetching is turned off"}
          </p>
        </div>
        <SettingsButton
          tone="ghost"
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </SettingsButton>
      </div>

      {status?.enabled && (
        <>
          <div className={formRowStyles.settingRow}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <SettingsStatusDot color={statusColor} />
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
                {statusLabel}
              </span>
            </div>
            <SettingsButton
              tone="ghost"
              onClick={handleFetchNow}
              disabled={fetching}
            >
              {fetching ? "Fetching..." : "Fetch Now"}
            </SettingsButton>
          </div>

          <div className={`${formRowStyles.settingRow} ${formRowStyles.noBorder}`}>
            <div style={{ flex: 1 }}>
              <span className={formRowStyles.monoLabel}>API Key</span>
              <p className={formRowStyles.descriptionSmallTop2}>
                Optional — improves rate limits for large contact lists
              </p>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
                <SettingsInput
                  width={260}
                  type="password"
                  value={apiKey}
                  onChange={(e) => {
                    setApiKey(e.target.value);
                    setApiKeyDirty(true);
                  }}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") saveApiKey();
                  }}
                  onFocus={() => {
                    if (apiKey === "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022") { setApiKey(""); setApiKeyDirty(true); }
                  }}
                  placeholder="Gravatar API key"
                />
                {apiKeyDirty && apiKey.trim() && (
                  <SettingsButton tone="primary" onClick={saveApiKey}>
                    Save
                  </SettingsButton>
                )}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
