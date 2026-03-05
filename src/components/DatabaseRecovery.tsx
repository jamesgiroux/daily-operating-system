import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
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
    if (restoringPath) return;
    setError(null);
    setRestoringPath(path);
    try {
      await invoke("restore_database_from_backup", { backupPath: path });
      window.location.reload();
    } catch (e) {
      setError(typeof e === "string" ? e : "Restore failed");
    } finally {
      setRestoringPath(null);
    }
  }

  return (
    <div className="db-recovery-screen">
      <div className="db-recovery-content">
        <h1>Database Recovery Required</h1>
        <p>
          DailyOS could not start safely because database migration or integrity checks failed.
        </p>
        <p className="db-recovery-detail">{summary}</p>

        <div className="db-recovery-section">
          <h3>Available Backups</h3>
          {loading && <p>Loading backups...</p>}
          {!loading && backups.length === 0 && (
            <p>No backup files were found. Use support/devtools to recover manually.</p>
          )}
          {!loading && backups.length > 0 && (
            <ul>
              {backups.map((backup) => {
                const restoring = restoringPath === backup.path;
                return (
                  <li key={backup.path}>
                    <div>
                      <div className="db-recovery-path">{backup.path}</div>
                      <div className="db-recovery-meta">
                        {backup.kind} • {new Date(backup.createdAt).toLocaleString()} • {formatBytes(backup.sizeBytes)}
                      </div>
                    </div>
                    <button onClick={() => handleRestore(backup.path)} disabled={Boolean(restoringPath)}>
                      {restoring ? "Restoring..." : "Restore"}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
          <button className="db-recovery-refresh" onClick={() => void loadBackups()} disabled={loading || Boolean(restoringPath)}>
            Refresh backup list
          </button>
          {error && <p className="db-recovery-error">{error}</p>}
        </div>
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
        .db-recovery-refresh {
          margin-top: 12px;
        }
        .db-recovery-error {
          margin-top: 10px !important;
          color: var(--color-spice-terracotta, #c97d60) !important;
          font-size: 12px !important;
        }
      `}</style>
    </div>
  );
}
