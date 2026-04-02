import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { getVersion } from "@tauri-apps/api/app";
import DOMPurify from "dompurify";
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
    () => (displayNotes ? DOMPurify.sanitize(renderMarkdownToHtml(displayNotes)) : null),
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
        backgroundColor: "var(--color-desk-charcoal-40)",
        backdropFilter: "blur(4px)",
      }}
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          backgroundColor: "var(--color-paper-cream)",
          borderRadius: 0,
          boxShadow: "var(--shadow-modal)",
          width: "100%",
          maxWidth: 640,
          maxHeight: "80vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Header */}
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            justifyContent: "space-between",
            padding: "32px 40px 24px",
            borderBottom: "2px solid var(--color-rule-light)",
          }}
        >
          <div style={{ flex: 1 }}>
            <h2
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 32,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                margin: "0 0 8px 0",
                lineHeight: 1.1,
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
                  letterSpacing: "0.1em",
                }}
              >
                Version {displayVersion}
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
              padding: "4px 8px",
              display: "flex",
              alignItems: "center",
              transition: "color 0.2s ease",
              marginTop: 4,
            }}
            onMouseEnter={(e) => (e.currentTarget.style.color = "var(--color-text-secondary)")}
            onMouseLeave={(e) => (e.currentTarget.style.color = "var(--color-text-tertiary)")}
          >
            <X size={20} />
          </button>
        </div>

        {/* Body */}
        <div
          style={{
            padding: "32px 40px",
            overflow: "auto",
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            lineHeight: 1.65,
            color: "var(--color-text-secondary)",
          }}
        >
          {notesHtml ? (
            <div
              className="whats-new-notes"
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 15,
                lineHeight: 1.65,
                color: "var(--color-text-secondary)",
              }}
              dangerouslySetInnerHTML={{
                __html: notesHtml
                  .replace(/<h2>/g, '<h2 style="font-family: var(--font-serif); font-size: 22px; font-weight: 400; color: var(--color-text-primary); margin: 24px 0 12px 0; line-height: 1.2;">')
                  .replace(/<h3>/g, '<h3 style="font-family: var(--font-serif); font-size: 18px; font-weight: 400; color: var(--color-text-primary); margin: 18px 0 10px 0; line-height: 1.3;">')
                  .replace(/<h4>/g, '<h4 style="font-family: var(--font-sans); font-size: 14px; font-weight: 600; color: var(--color-text-primary); margin: 12px 0 6px 0; text-transform: uppercase; letter-spacing: 0.05em;">')
                  .replace(/<p>/g, '<p style="margin: 0 0 12px 0;">')
                  .replace(/<li>/g, '<li style="margin-bottom: 8px;">')
                  .replace(/<ul>/g, '<ul style="margin: 12px 0; padding-left: 24px;">')
                  .replace(/<strong>/g, '<strong style="font-weight: 600; color: var(--color-text-primary);">')
                  .replace(/<em>/g, '<em style="color: var(--color-text-secondary); font-style: italic;">')
              }}
            />
          ) : (
            <div>
              <p style={{ color: "var(--color-text-tertiary)", margin: "0 0 12px 0", fontStyle: "italic" }}>
                You're running the latest version of DailyOS.
              </p>
              <p style={{ color: "var(--color-text-tertiary)", margin: 0 }}>
                Check the changelog for release details.
              </p>
            </div>
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
