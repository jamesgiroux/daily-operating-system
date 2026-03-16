import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { save } from "@tauri-apps/plugin-dialog";
import type { BackupInfo, DatabaseRecoveryStatus } from "@/types";

interface DatabaseRecoveryProps {
  status: DatabaseRecoveryStatus;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export function DatabaseRecovery({ status }: DatabaseRecoveryProps) {
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoringPath, setRestoringPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const isForwardCompat = useMemo(
    () => status.reason.includes("forward") || status.detail.includes("forward"),
    [status.reason, status.detail],
  );

  async function loadBackups() {
    setLoading(true);
    setError(null);
    try {
      const files = await invoke<BackupInfo[]>("list_database_backups");
      setBackups(files);
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to load backups");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadBackups();
  }, []);

  const summary = useMemo(() => {
    if (!status.reason && !status.detail) return "Database startup checks failed.";
    return `${status.reason}${status.detail ? `: ${status.detail}` : ""}`;
  }, [status.reason, status.detail]);

  async function handleRestore(path: string) {
    if (restoringPath || busy) return;
    setError(null);
    setRestoringPath(path);
    try {
      await invoke("restore_database_from_backup", { backupPath: path });
      await relaunch();
    } catch (e) {
      setError(typeof e === "string" ? e : "Restore failed");
    } finally {
      setRestoringPath(null);
    }
  }

  async function handleStartFresh() {
    if (busy) return;
    const confirmed = window.confirm(
      "This will delete all your data and start with a clean database. This cannot be undone. Continue?",
    );
    if (!confirmed) return;
    setBusy(true);
    setError(null);
    try {
      await invoke("start_fresh_database");
      await relaunch();
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start fresh");
    } finally {
      setBusy(false);
    }
  }

  async function handleExport() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      const destination = await save({
        defaultPath: "dailyos-backup.db",
        filters: [{ name: "SQLite Database", extensions: ["db"] }],
      });
      if (!destination) {
        setBusy(false);
        return;
      }
      await invoke("export_database_copy", { destination });
    } catch (e) {
      setError(typeof e === "string" ? e : "Export failed");
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="db-recovery-screen">
      <div className="db-recovery-content">
        <h1>Database Recovery Required</h1>

        {isForwardCompat ? (
          <p>
            This database was created by a newer version of DailyOS. Please update the app.
          </p>
        ) : (
          <p>
            DailyOS could not start safely because database migration or integrity checks failed.
          </p>
        )}

        <p className="db-recovery-detail">{summary}</p>

        {!isForwardCompat && (
          <div className="db-recovery-section">
            <h3>Available Backups</h3>
            {loading && <p>Loading backups...</p>}
            {!loading && backups.length === 0 && (
              <p>No backup files were found. You can start fresh with a clean database.</p>
            )}
            {!loading && backups.length > 0 && (
              <ul>
                {backups.map((backup) => {
                  const restoring = restoringPath === backup.path;
                  return (
                    <li key={backup.path}>
                      <div>
                        <div className="db-recovery-path">{backup.filename}</div>
                        <div className="db-recovery-meta">
                          {backup.kind} • {new Date(backup.createdAt).toLocaleString()} • {formatBytes(backup.sizeBytes)}
                          {backup.schemaVersion != null && ` • v${backup.schemaVersion}`}
                        </div>
                      </div>
                      <button onClick={() => handleRestore(backup.path)} disabled={Boolean(restoringPath) || busy}>
                        {restoring ? "Restoring..." : "Restore"}
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}

            <div className="db-recovery-actions">
              <button className="db-recovery-refresh" onClick={() => void loadBackups()} disabled={loading || Boolean(restoringPath) || busy}>
                Refresh backup list
              </button>
              {backups.length === 0 ? (
                <button className="db-recovery-primary" onClick={handleStartFresh} disabled={busy || Boolean(restoringPath)}>
                  {busy ? "Starting fresh..." : "Start Fresh"}
                </button>
              ) : (
                <button className="db-recovery-refresh" onClick={handleStartFresh} disabled={busy || Boolean(restoringPath)}>
                  {busy ? "Starting fresh..." : "Start Fresh"}
                </button>
              )}
              <button className="db-recovery-refresh" onClick={handleExport} disabled={busy || Boolean(restoringPath)}>
                Export Database
              </button>
            </div>
            {error && <p className="db-recovery-error">{error}</p>}
          </div>
        )}

        {status.dbPath && (
          <details className="db-recovery-technical">
            <summary>Technical Details</summary>
            <div className="db-recovery-detail">
              <strong>Database path:</strong> {status.dbPath}
            </div>
          </details>
        )}
      </div>

      <style>{`
        .db-recovery-screen {
          position: fixed;
          inset: 0;
          background: var(--color-paper-cream, #f5f0e8);
          display: flex;
          align-items: center;
          justify-content: center;
          z-index: 1100;
          padding: 32px;
        }
        .db-recovery-content {
          width: min(860px, 100%);
        }
        .db-recovery-content h1 {
          font-family: var(--font-serif, 'Newsreader', serif);
          font-size: 30px;
          font-weight: 500;
          margin: 0 0 16px;
          color: var(--color-text-primary);
        }
        .db-recovery-content p {
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 14px;
          line-height: 1.6;
          color: var(--color-text-secondary);
          margin: 0 0 12px;
        }
        .db-recovery-detail {
          padding: 10px 12px;
          border: 1px solid var(--color-rule-light);
          border-radius: 4px;
          background: var(--color-surface-secondary, #ede8df);
          font-family: var(--font-mono, 'JetBrains Mono', monospace);
          font-size: 12px !important;
        }
        .db-recovery-section {
          margin-top: 24px;
        }
        .db-recovery-section h3 {
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 14px;
          font-weight: 600;
          margin: 0 0 10px;
        }
        .db-recovery-section ul {
          list-style: none;
          padding: 0;
          margin: 0;
          border: 1px solid var(--color-rule-light);
          border-radius: 6px;
          overflow: hidden;
        }
        .db-recovery-section li {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 16px;
          padding: 10px 12px;
          border-bottom: 1px solid var(--color-rule-light);
          background: var(--color-surface-primary, #faf7f2);
        }
        .db-recovery-section li:last-child {
          border-bottom: none;
        }
        .db-recovery-path {
          font-family: var(--font-mono, 'JetBrains Mono', monospace);
          font-size: 11px;
          color: var(--color-text-primary);
          word-break: break-all;
          margin-bottom: 4px;
        }
        .db-recovery-meta {
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 12px;
          color: var(--color-text-tertiary);
        }
        .db-recovery-section button {
          font-family: var(--font-mono, 'JetBrains Mono', monospace);
          font-size: 11px;
          text-transform: uppercase;
          letter-spacing: 0.06em;
          border: 1px solid var(--color-rule-heavy);
          background: transparent;
          color: var(--color-text-primary);
          border-radius: 4px;
          padding: 6px 10px;
          cursor: pointer;
          white-space: nowrap;
        }
        .db-recovery-section button:disabled {
          opacity: 0.55;
          cursor: default;
        }
        .db-recovery-actions {
          display: flex;
          gap: 8px;
          margin-top: 12px;
          flex-wrap: wrap;
        }
        .db-recovery-primary {
          background: var(--color-text-primary) !important;
          color: var(--color-paper-cream, #f5f0e8) !important;
          border-color: var(--color-text-primary) !important;
        }
        .db-recovery-refresh {
          margin-top: 0;
        }
        .db-recovery-error {
          margin-top: 10px !important;
          color: var(--color-spice-terracotta, #c97d60) !important;
          font-size: 12px !important;
        }
        .db-recovery-technical {
          margin-top: 20px;
          font-family: var(--font-sans, 'DM Sans', sans-serif);
          font-size: 13px;
          color: var(--color-text-tertiary);
        }
        .db-recovery-technical summary {
          cursor: pointer;
          margin-bottom: 8px;
        }
      `}</style>
    </div>
  );
}
