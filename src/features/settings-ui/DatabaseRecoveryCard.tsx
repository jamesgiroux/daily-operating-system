import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { relaunch } from "@tauri-apps/plugin-process";
import { save } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { AlertTriangle, DatabaseBackup, Download, RefreshCw, ShieldCheck } from "lucide-react";
import type { BackupInfo, DatabaseInfo } from "@/types";
import { useDatabaseRecoveryStatus } from "@/hooks/useDatabaseRecoveryStatus";
import { styles } from "@/components/settings/styles";

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function DatabaseRecoveryCard() {
  const { status, refresh } = useDatabaseRecoveryStatus();
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [loadingBackups, setLoadingBackups] = useState(true);
  const [creatingBackup, setCreatingBackup] = useState(false);
  const [restoringPath, setRestoringPath] = useState<string | null>(null);
  const [dbInfo, setDbInfo] = useState<DatabaseInfo | null>(null);

  async function loadBackups() {
    setLoadingBackups(true);
    try {
      const files = await invoke<BackupInfo[]>("list_database_backups");
      setBackups(files);
    } catch (e) {
      toast.error(typeof e === "string" ? e : "Failed to load backups");
    } finally {
      setLoadingBackups(false);
    }
  }

  async function loadDbInfo() {
    try {
      const info = await invoke<DatabaseInfo>("get_database_info");
      setDbInfo(info);
    } catch {
      // Non-critical — info section just won't show
    }
  }

  useEffect(() => {
    void loadBackups();
    void loadDbInfo();
  }, []);

  async function handleCreateBackup() {
    if (creatingBackup) return;
    setCreatingBackup(true);
    try {
      const path = await invoke<string>("backup_database");
      toast.success("Backup created");
      console.info("Backup created at", path);
      await loadBackups();
      await loadDbInfo();
    } catch (e) {
      toast.error(typeof e === "string" ? e : "Backup failed");
    } finally {
      setCreatingBackup(false);
    }
  }

  async function handleRestore(backupPath: string) {
    if (restoringPath) return;
    const confirmed = window.confirm(
      "Restore this backup? Current database content will be replaced.",
    );
    if (!confirmed) return;

    setRestoringPath(backupPath);
    try {
      await invoke("restore_database_from_backup", { backupPath });
      await refresh();
      toast.success("Backup restored. Relaunching...");
      setTimeout(() => void relaunch(), 300);
    } catch (e) {
      toast.error(typeof e === "string" ? e : "Restore failed");
    } finally {
      setRestoringPath(null);
    }
  }

  async function handleExport() {
    try {
      const destination = await save({
        defaultPath: "dailyos-backup.db",
        filters: [{ name: "SQLite Database", extensions: ["db"] }],
      });
      if (!destination) return;
      await invoke("export_database_copy", { destination });
      toast.success("Database exported");
    } catch (e) {
      toast.error(typeof e === "string" ? e : "Export failed");
    }
  }

  return (
    <div style={{ marginBottom: 24 }}>
      <p style={styles.subsectionLabel}>Database Recovery</p>

      {dbInfo && (
        <div
          style={{
            border: "1px solid var(--color-rule-light)",
            borderRadius: 6,
            padding: 14,
            marginBottom: 12,
            background: "var(--color-surface-secondary)",
          }}
        >
          <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <div style={{ ...styles.description, fontSize: 12 }}>
              <strong>Path:</strong>{" "}
              <span style={{ fontFamily: "var(--font-mono)", fontSize: 11 }}>{dbInfo.path}</span>
            </div>
            <div style={{ ...styles.description, fontSize: 12 }}>
              <strong>Size:</strong> {formatBytes(dbInfo.sizeBytes)} &bull;{" "}
              <strong>Schema:</strong> v{dbInfo.schemaVersion}
              {dbInfo.lastBackup && (
                <>
                  {" "}&bull; <strong>Last backup:</strong>{" "}
                  {new Date(dbInfo.lastBackup).toLocaleString()}
                </>
              )}
            </div>
          </div>
        </div>
      )}

      <div
        style={{
          border: "1px solid var(--color-rule-light)",
          borderRadius: 6,
          padding: 14,
          marginBottom: 12,
          background: "var(--color-surface-secondary)",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
          {status.required ? (
            <AlertTriangle size={14} style={{ color: "var(--color-spice-terracotta)" }} />
          ) : (
            <ShieldCheck size={14} style={{ color: "var(--color-garden-sage)" }} />
          )}
          <span style={styles.monoLabel}>{status.required ? "Recovery required" : "Database healthy"}</span>
        </div>
        {status.required && (
          <p style={{ ...styles.description, marginBottom: 0 }}>
            {status.reason}
            {status.detail ? `: ${status.detail}` : ""}
          </p>
        )}
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
        <button
          style={{ ...styles.btn, ...styles.btnPrimary }}
          onClick={handleCreateBackup}
          disabled={creatingBackup || status.required}
          title={status.required ? "Recovery mode active. Use restore instead." : "Create backup"}
        >
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <DatabaseBackup size={12} />
            {creatingBackup ? "Creating..." : "Create Backup"}
          </span>
        </button>

        <button
          style={{ ...styles.btn, ...styles.btnGhost }}
          onClick={handleExport}
          disabled={status.required}
          title="Export a copy of the database"
        >
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <Download size={12} />
            Export
          </span>
        </button>

        <button
          style={{ ...styles.btn, ...styles.btnGhost }}
          onClick={() => void loadBackups()}
          disabled={loadingBackups || Boolean(restoringPath)}
        >
          <span style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
            <RefreshCw size={12} className={loadingBackups ? "animate-spin" : ""} />
            Refresh List
          </span>
        </button>
      </div>

      <div style={{ border: "1px solid var(--color-rule-light)", borderRadius: 6, overflow: "hidden" }}>
        {loadingBackups && (
          <div style={{ padding: "10px 12px" }}>
            <span style={styles.description}>Loading backups...</span>
          </div>
        )}

        {!loadingBackups && backups.length === 0 && (
          <div style={{ padding: "10px 12px" }}>
            <span style={styles.description}>No backups found.</span>
          </div>
        )}

        {!loadingBackups && backups.map((backup, index) => {
          const restoring = restoringPath === backup.path;
          return (
            <div
              key={backup.path}
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                gap: 12,
                padding: "10px 12px",
                borderBottom: index === backups.length - 1 ? "none" : "1px solid var(--color-rule-light)",
              }}
            >
              <div style={{ minWidth: 0, flex: 1 }}>
                <div
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-primary)",
                    marginBottom: 4,
                    wordBreak: "break-all",
                  }}
                >
                  {backup.filename}
                </div>
                <div style={{ ...styles.description, fontSize: 12 }}>
                  {backup.kind} • {new Date(backup.createdAt).toLocaleString()} • {formatBytes(backup.sizeBytes)}
                  {backup.schemaVersion != null && ` • v${backup.schemaVersion}`}
                </div>
              </div>
              <button
                style={{ ...styles.btn, ...styles.btnGhost }}
                onClick={() => void handleRestore(backup.path)}
                disabled={Boolean(restoringPath) || creatingBackup}
              >
                {restoring ? "Restoring..." : "Restore"}
              </button>
            </div>
          );
        })}
      </div>
    </div>
  );
}
