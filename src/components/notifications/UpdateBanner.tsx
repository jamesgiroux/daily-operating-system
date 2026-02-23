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
        justifyContent: "center",
        gap: 16,
        padding: "8px 24px",
        borderBottom: "1px solid var(--color-rule-light)",
        backgroundColor: "var(--color-paper-warm-white)",
        fontFamily: "var(--font-sans)",
        fontSize: 13,
        color: "var(--color-text-secondary)",
        position: "relative",
        zIndex: "var(--z-app-shell)",
        flexShrink: 0,
      }}
    >
      <span>
        <span style={{ fontWeight: 500, color: "var(--color-text-primary)" }}>
          DailyOS v{version}
        </span>
        {" available"}
      </span>

      <button
        onClick={onWhatsNew}
        style={{
          background: "none",
          border: "none",
          cursor: "pointer",
          fontFamily: "var(--font-sans)",
          fontSize: 13,
          color: "var(--color-garden-sage)",
          textDecoration: "underline",
          textDecorationColor: "var(--color-garden-sage-12)",
          textUnderlineOffset: 2,
          padding: 0,
        }}
      >
        What's New
      </button>

      <button
        onClick={installAndRestart}
        disabled={installing}
        style={{
          background: installing ? "var(--color-rule-light)" : "var(--color-desk-charcoal)",
          color: installing ? "var(--color-text-tertiary)" : "var(--color-paper-warm-white)",
          border: "none",
          borderRadius: "var(--radius-editorial-sm)",
          padding: "4px 12px",
          fontFamily: "var(--font-sans)",
          fontSize: 12,
          fontWeight: 500,
          cursor: installing ? "default" : "pointer",
          display: "inline-flex",
          alignItems: "center",
          gap: 6,
        }}
      >
        {installing ? (
          <>
            <Loader2 size={12} className="animate-spin" />
            Installing
          </>
        ) : (
          "Install & Restart"
        )}
      </button>

      <button
        onClick={handleDismiss}
        aria-label="Dismiss update banner"
        style={{
          position: "absolute",
          right: 12,
          top: "50%",
          transform: "translateY(-50%)",
          background: "none",
          border: "none",
          cursor: "pointer",
          color: "var(--color-text-tertiary)",
          padding: 4,
          display: "flex",
          alignItems: "center",
        }}
      >
        <X size={14} />
      </button>
    </div>
  );
}
