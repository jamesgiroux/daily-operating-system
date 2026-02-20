import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EditorialEmpty } from "@/components/editorial/EditorialEmpty";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import type { ProcessingLogEntry } from "@/types";

export default function HistoryPage() {
  const { personality } = usePersonality();
  const [entries, setEntries] = useState<ProcessingLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Processing History",
      atmosphereColor: "olive" as const,
      activePage: "dropbox" as const,
    }),
    [],
  );
  useRegisterMagazineShell(shellConfig);

  useEffect(() => {
    async function load() {
      try {
        const result = await invoke<ProcessingLogEntry[]>(
          "get_processing_history",
          { limit: 50 },
        );
        setEntries(result);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    }
    load();
  }, []);

  if (loading) {
    return <EditorialLoading count={5} />;
  }

  if (error) {
    return <EditorialError message={error} />;
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ HERO ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 24 }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <h1
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 36,
              fontWeight: 400,
              letterSpacing: "-0.02em",
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Processing History
          </h1>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
            }}
          >
            {entries.length} entr{entries.length !== 1 ? "ies" : "y"}
          </span>
        </div>
        <div style={{ height: 2, background: "var(--color-desk-charcoal)", marginTop: 16 }} />
      </section>

      {/* ═══ ENTRIES ═══ */}
      {entries.length === 0 ? (
        <EditorialEmpty
          {...getPersonalityCopy("history-empty", personality)}
        />
      ) : (
        <>
          {/* Column headers */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "2fr 1fr 1fr 2fr 1fr",
              gap: 12,
              padding: "0 0 8px",
              borderBottom: "1px solid var(--color-rule-light)",
            }}
          >
            {["FILE", "CLASS", "STATUS", "DESTINATION", "TIME"].map((h) => (
              <span
                key={h}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 10,
                  fontWeight: 600,
                  letterSpacing: "0.08em",
                  textTransform: "uppercase",
                  color: "var(--color-text-tertiary)",
                }}
              >
                {h}
              </span>
            ))}
          </div>

          {/* Rows */}
          {entries.map((entry, i) => (
            <HistoryRow key={entry.id} entry={entry} showBorder={i < entries.length - 1} />
          ))}

          <FinisMarker />
        </>
      )}
    </div>
  );
}

function HistoryRow({
  entry,
  showBorder,
}: {
  entry: ProcessingLogEntry;
  showBorder: boolean;
}) {
  const isError = entry.status === "error";

  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "2fr 1fr 1fr 2fr 1fr",
        gap: 12,
        padding: "10px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        alignItems: "center",
      }}
    >
      {/* Filename */}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 12,
          color: "var(--color-text-primary)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
        title={entry.filename}
      >
        {entry.filename}
      </span>

      {/* Classification */}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--color-text-tertiary)",
        }}
      >
        {entry.classification}
      </span>

      {/* Status — colored dot + mono label */}
      <span style={{ display: "flex", alignItems: "center", gap: 6 }}>
        <span
          style={{
            width: 6,
            height: 6,
            borderRadius: "50%",
            background: isError
              ? "var(--color-spice-terracotta)"
              : "var(--color-garden-sage)",
            flexShrink: 0,
          }}
        />
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: isError
              ? "var(--color-spice-terracotta)"
              : "var(--color-text-tertiary)",
          }}
        >
          {entry.status}
        </span>
      </span>

      {/* Destination */}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--color-text-tertiary)",
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
        title={entry.destinationPath || undefined}
      >
        {entry.destinationPath || "\u2014"}
      </span>

      {/* Time */}
      <span
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 11,
          color: "var(--color-text-tertiary)",
          whiteSpace: "nowrap",
        }}
      >
        {formatTimestamp(entry.createdAt)}
      </span>

      {/* Error row — spans full width */}
      {isError && entry.errorMessage && (
        <div
          style={{
            gridColumn: "1 / -1",
            fontFamily: "var(--font-sans)",
            fontSize: 12,
            color: "var(--color-spice-terracotta)",
            paddingTop: 2,
          }}
        >
          {entry.errorMessage}
        </div>
      )}
    </div>
  );
}

function formatTimestamp(ts: string): string {
  try {
    const d = new Date(ts);
    return d.toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  } catch {
    return ts;
  }
}
