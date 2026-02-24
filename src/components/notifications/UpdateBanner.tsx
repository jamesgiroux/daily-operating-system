import { useState, useCallback } from "react";
import { useUpdate } from "@/contexts/UpdateContext";
import { Loader2, X } from "lucide-react";

interface UpdateBannerProps {
  onWhatsNew: () => void;
}

export function UpdateBanner({ onWhatsNew }: UpdateBannerProps) {
  const { available, version, installing, installAndRestart } = useUpdate();
  const [dismissed, setDismissed] = useState<string | null>(
    () => localStorage.getItem("dailyos_dismissed_update"),
  );

  const handleDismiss = useCallback(() => {
    if (version) {
      setDismissed(version);
      localStorage.setItem("dailyos_dismissed_update", version);
    }
  }, [version]);

  if (!available || !version || dismissed === version) return null;

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        gap: 24,
        padding: "16px 32px",
        borderBottom: "2px solid var(--color-rule-light)",
        backgroundColor: "var(--color-paper-cream)",
        flexShrink: 0,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 16, flex: 1 }}>
        <div>
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 18,
              fontWeight: 400,
              color: "var(--color-text-primary)",
              marginBottom: 4,
              lineHeight: 1.2,
            }}
          >
            DailyOS v{version} available
          </div>
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
              lineHeight: 1.4,
            }}
          >
            A new version is ready to install. See what's changed.
          </div>
        </div>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 16, flexShrink: 0 }}>
        <button
          onClick={onWhatsNew}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-garden-eucalyptus)",
            textDecoration: "underline",
            textDecorationColor: "var(--color-garden-eucalyptus)",
            textUnderlineOffset: 3,
            padding: "6px 0",
            fontWeight: 500,
            transition: "color 0.2s ease",
          }}
          onMouseEnter={(e) => (e.currentTarget.style.color = "var(--color-garden-rosemary)")}
          onMouseLeave={(e) => (e.currentTarget.style.color = "var(--color-garden-eucalyptus)")}
        >
          What's New
        </button>

        <button
          onClick={installAndRestart}
          disabled={installing}
          style={{
            background: "none",
            border: "none",
            cursor: installing ? "default" : "pointer",
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-garden-eucalyptus)",
            textDecoration: "underline",
            textDecorationColor: "var(--color-garden-eucalyptus)",
            textUnderlineOffset: 3,
            padding: "6px 0",
            fontWeight: 500,
            display: "inline-flex",
            alignItems: "center",
            gap: 6,
            opacity: installing ? 0.6 : 1,
            transition: "color 0.2s ease, opacity 0.2s ease",
          }}
          onMouseEnter={(e) => !installing && (e.currentTarget.style.color = "var(--color-garden-rosemary)")}
          onMouseLeave={(e) => !installing && (e.currentTarget.style.color = "var(--color-garden-eucalyptus)")}
        >
          {installing ? (
            <>
              <Loader2 size={13} className="animate-spin" />
              Installing...
            </>
          ) : (
            "Install & Restart"
          )}
        </button>

        <button
          onClick={handleDismiss}
          aria-label="Dismiss update banner"
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            color: "var(--color-text-tertiary)",
            padding: 4,
            marginLeft: 4,
            display: "flex",
            alignItems: "center",
            transition: "color 0.2s ease",
          }}
          onMouseEnter={(e) => (e.currentTarget.style.color = "var(--color-text-secondary)")}
          onMouseLeave={(e) => (e.currentTarget.style.color = "var(--color-text-tertiary)")}
        >
          <X size={16} />
        </button>
      </div>
    </div>
  );
}
