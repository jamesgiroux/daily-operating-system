import { useState, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { EmptyState } from "@/components/editorial/EmptyState";
import { EditorialLoading } from "@/components/editorial/EditorialLoading";
import { EditorialError } from "@/components/editorial/EditorialError";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { getPersonalityCopy } from "@/lib/personality";
import { usePersonality } from "@/hooks/usePersonality";
import type { ProcessingLogEntry } from "@/types";
import styles from "./HistoryPage.module.css";

export default function HistoryPage() {
  const navigate = useNavigate();
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
    <div className={styles.container}>
      {/* ═══ HERO ═══ */}
      <section className={styles.hero}>
        <div className={styles.heroRow}>
          <h1 className={styles.title}>
            Processing History
          </h1>
          <span className={styles.entryCount}>
            {entries.length} entr{entries.length !== 1 ? "ies" : "y"}
          </span>
        </div>
        <div className={styles.heroRule} />
      </section>

      {/* ═══ ENTRIES ═══ */}
      {entries.length === 0 ? (
        (() => {
          const copy = getPersonalityCopy("history-empty", personality);
          return (
            <EmptyState
              headline={copy.title}
              explanation={copy.explanation ?? copy.message ?? ""}
              benefit={copy.benefit}
              action={{ label: "Go to inbox", onClick: () => navigate({ to: "/inbox", search: { entityId: undefined } }) }}
            />
          );
        })()
      ) : (
        <>
          {/* Column headers */}
          <div className={styles.headerRow}>
            {["FILE", "CLASS", "STATUS", "DESTINATION", "TIME"].map((h) => (
              <span
                key={h}
                className={styles.headerCell}
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
      className={`${styles.entryRow} ${showBorder ? styles.entryRowBorder : ''}`}
    >
      {/* Filename */}
      <span
        className={styles.cellFilename}
        title={entry.filename}
      >
        {entry.filename}
      </span>

      {/* Classification */}
      <span className={styles.cellMono}>
        {entry.classification}
      </span>

      {/* Status — colored dot + mono label */}
      <span className={styles.statusCell}>
        <span
          className={`${styles.statusDot} ${isError ? styles.statusDotError : styles.statusDotSuccess}`}
        />
        <span
          className={`${styles.cellMono} ${isError ? styles.statusTextError : ''}`}
        >
          {entry.status}
        </span>
      </span>

      {/* Destination */}
      <span
        className={`${styles.cellMono} ${styles.cellTruncate}`}
        title={entry.destinationPath || undefined}
      >
        {entry.destinationPath || "\u2014"}
      </span>

      {/* Time */}
      <span className={`${styles.cellMono} ${styles.cellNoWrap}`}>
        {formatTimestamp(entry.createdAt)}
      </span>

      {/* Error row — spans full width */}
      {isError && entry.errorMessage && (
        <div className={styles.errorRow}>
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
