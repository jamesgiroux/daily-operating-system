import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { useUpdate } from "@/contexts/UpdateContext";
import { X } from "lucide-react";

const LAST_SEEN_VERSION_KEY = "dailyos_last_seen_version";
const STORED_RELEASE_NOTES_KEY = "dailyos_release_notes";

/**
 * Minimal markdown-to-HTML converter.
 * Handles: # headings, - bullets, **bold**, *italic*, blank lines as <br/>.
 */
function renderMarkdownToHtml(md: string): string {
  return md
    .split("\n")
    .map((line) => {
      // Headings
      if (line.startsWith("### ")) return `<h4>${line.slice(4)}</h4>`;
      if (line.startsWith("## ")) return `<h3>${line.slice(3)}</h3>`;
      if (line.startsWith("# ")) return `<h2>${line.slice(2)}</h2>`;
      // Bullets
      if (/^\s*[-*] /.test(line)) {
        const content = line.replace(/^\s*[-*] /, "");
        return `<li>${content}</li>`;
      }
      // Blank line
      if (line.trim() === "") return "<br/>";
      // Regular paragraph
      return `<p>${line}</p>`;
    })
    .join("")
    // Wrap consecutive <li> in <ul>
    .replace(/(<li>.*?<\/li>)+/g, (m) => `<ul>${m}</ul>`)
    // Bold
    .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
    // Italic
    .replace(/\*(.+?)\*/g, "<em>$1</em>");
}

interface WhatsNewModalProps {
  open: boolean;
  onClose: () => void;
}

export function WhatsNewModal({ open, onClose }: WhatsNewModalProps) {
  const { notes, version } = useUpdate();
  const displayVersion = version;
  // Try update context notes first, then stored notes from pre-install, then fallback
  const storedNotes = useMemo(() => localStorage.getItem(STORED_RELEASE_NOTES_KEY), []);
  const displayNotes = notes || storedNotes;
  const notesHtml = useMemo(
    () => (displayNotes ? renderMarkdownToHtml(displayNotes) : null),
    [displayNotes],
  );

  if (!open) return null;

  return (
    <div
      style={{
        position: "fixed",
        inset: 0,
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "rgba(30, 37, 48, 0.4)",
        backdropFilter: "blur(4px)",
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          backgroundColor: "var(--color-paper-warm-white)",
          borderRadius: "var(--radius-editorial-lg)",
          boxShadow: "var(--shadow-md)",
          width: "100%",
          maxWidth: 520,
          maxHeight: "70vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "20px 24px 16px",
            borderBottom: "1px solid var(--color-rule-light)",
          }}
        >
          <div>
            <h2
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 22,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                margin: 0,
                lineHeight: 1.2,
              }}
            >
              What's New
            </h2>
            {displayVersion && (
              <span
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 11,
                  color: "var(--color-text-tertiary)",
                  textTransform: "uppercase",
                  letterSpacing: "0.06em",
                }}
              >
                v{displayVersion}
              </span>
            )}
          </div>
          <button
            onClick={onClose}
            aria-label="Close"
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              color: "var(--color-text-tertiary)",
              padding: 4,
              display: "flex",
              alignItems: "center",
            }}
          >
            <X size={18} />
          </button>
        </div>

        {/* Body */}
        <div
          style={{
            padding: "20px 24px 24px",
            overflow: "auto",
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            lineHeight: 1.6,
            color: "var(--color-text-secondary)",
          }}
        >
          {notesHtml ? (
            <div
              className="whats-new-notes"
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                lineHeight: 1.6,
                color: "var(--color-text-secondary)",
              }}
              dangerouslySetInnerHTML={{ __html: notesHtml }}
            />
          ) : (
            <p style={{ color: "var(--color-text-tertiary)", margin: 0 }}>
              You're running the latest version of DailyOS. Check the changelog for details.
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Hook that auto-shows the WhatsNewModal on first launch after updating.
 * Compares current app version against localStorage's `last_seen_version`.
 * Returns `{ autoShowOpen, dismissAutoShow }` for the root layout to use.
 */
export function useWhatsNewAutoShow() {
  const [autoShowOpen, setAutoShowOpen] = useState(false);
  const checkedRef = useRef(false);

  useEffect(() => {
    if (checkedRef.current) return;
    checkedRef.current = true;

    getVersion()
      .then((currentVersion) => {
        const lastSeen = localStorage.getItem(LAST_SEEN_VERSION_KEY);
        if (lastSeen !== currentVersion) {
          // First launch on this version — auto-show is handled by the parent
          // but we only set the flag; the parent decides whether to show
          // (e.g., only if there are notes to display).
          setAutoShowOpen(true);
        }
      })
      .catch(() => {
        // Can't determine version — skip auto-show
      });
  }, []);

  const dismissAutoShow = useCallback(() => {
    setAutoShowOpen(false);
    getVersion()
      .then((v) => localStorage.setItem(LAST_SEEN_VERSION_KEY, v))
      .catch(() => {});
  }, []);

  return { autoShowOpen, dismissAutoShow };
}
