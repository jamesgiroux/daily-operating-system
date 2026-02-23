import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import type { LinearStatusData } from "@/types";
import { styles } from "../styles";

interface LinearIssue {
  id: string;
  identifier: string;
  title: string;
  stateName: string | null;
  stateType: string | null;
  priorityLabel: string | null;
  dueDate: string | null;
  syncedAt: string | null;
}

interface LinearEntityLink {
  id: string;
  linearProjectId: string;
  projectName: string | null;
  entityId: string;
  entityType: string;
  confirmed: boolean;
  entityName: string | null;
}

interface LinearProject {
  id: string;
  name: string;
}

interface PickerEntity {
  id: string;
  name: string;
  type: "account" | "project";
}

export default function LinearConnection() {
  const [status, setStatus] = useState<LinearStatusData | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [apiKeyDirty, setApiKeyDirty] = useState(false);
  const [testing, setTesting] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [viewerName, setViewerName] = useState<string | null>(null);
  const [recentIssues, setRecentIssues] = useState<LinearIssue[]>([]);
  const [entityLinks, setEntityLinks] = useState<LinearEntityLink[]>([]);
  const [autoLinking, setAutoLinking] = useState(false);

  // Manual link picker state
  const [linearProjects, setLinearProjects] = useState<LinearProject[]>([]);
  const [entities, setEntities] = useState<PickerEntity[]>([]);
  const [selectedProject, setSelectedProject] = useState("");
  const [selectedEntity, setSelectedEntity] = useState("");
  const [showLinkForm, setShowLinkForm] = useState(false);

  const loadRecentIssues = useCallback(async () => {
    try {
      const issues = await invoke<LinearIssue[]>("get_linear_recent_issues");
      setRecentIssues(issues);
    } catch {
      // Silently fail - issues section just won't show
    }
  }, []);

  const loadEntityLinks = useCallback(async () => {
    try {
      const links = await invoke<LinearEntityLink[]>("get_linear_entity_links");
      setEntityLinks(links);
    } catch {
      // Silently fail
    }
  }, []);

  useEffect(() => {
    invoke<LinearStatusData>("get_linear_status")
      .then((s) => {
        setStatus(s);
        if (s.apiKeySet) setApiKey("\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022");
        if (s.enabled && s.issueCount > 0) {
          loadRecentIssues();
          loadEntityLinks();
        }
      })
      .catch((err) => console.error("get_linear_status failed:", err));
  }, [loadRecentIssues, loadEntityLinks]);

  async function toggleEnabled() {
    if (!status) return;
    const newEnabled = !status.enabled;
    try {
      await invoke("set_linear_enabled", { enabled: newEnabled });
      setStatus({ ...status, enabled: newEnabled });
    } catch (err) {
      console.error("Failed to toggle Linear:", err);
    }
  }

  async function saveApiKey() {
    const trimmed = apiKey.trim();
    if (!trimmed || trimmed === "\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022") return;
    try {
      await invoke("set_linear_api_key", { key: trimmed });
      setApiKeyDirty(false);
      setStatus((prev) => prev ? { ...prev, apiKeySet: true } : prev);
      toast("Linear API key saved");
    } catch (err) {
      toast.error("Failed to save API key");
    }
  }

  async function testConnection() {
    setTesting(true);
    try {
      const name = await invoke<string>("test_linear_connection");
      setViewerName(name);
      toast(`Connected as ${name}`);
    } catch (err) {
      toast.error("Linear connection failed");
      setViewerName(null);
    } finally {
      setTesting(false);
    }
  }

  async function handleSync() {
    setSyncing(true);
    try {
      await invoke("start_linear_sync");
      toast("Linear sync started");
      setTimeout(async () => {
        try {
          const refreshed = await invoke<LinearStatusData>("get_linear_status");
          setStatus(refreshed);
          loadRecentIssues();
          loadEntityLinks();
        } catch {}
        setSyncing(false);
      }, 3000);
    } catch (err) {
      toast.error("Sync failed");
      setSyncing(false);
    }
  }

  async function handleAutoLink() {
    setAutoLinking(true);
    try {
      const count = await invoke<number>("run_linear_auto_link");
      if (count > 0) {
        toast(`Linked ${count} project${count > 1 ? "s" : ""}`);
        loadEntityLinks();
      } else {
        toast("No new matches found");
      }
    } catch (err) {
      toast.error("Auto-link failed");
    } finally {
      setAutoLinking(false);
    }
  }

  async function handleDeleteLink(linkId: string) {
    try {
      await invoke("delete_linear_entity_link", { linkId });
      setEntityLinks((prev) => prev.filter((l) => l.id !== linkId));
      toast("Link removed");
    } catch (err) {
      toast.error("Failed to remove link");
    }
  }

  async function openLinkForm() {
    setShowLinkForm(true);
    setSelectedProject("");
    setSelectedEntity("");
    try {
      const [projects, accounts, projectsList] = await Promise.all([
        invoke<LinearProject[]>("get_linear_projects"),
        invoke<{ id: string; name: string }[]>("get_accounts_for_picker"),
        invoke<{ id: string; name: string }[]>("get_projects_list"),
      ]);
      setLinearProjects(projects);
      const merged: PickerEntity[] = [
        ...accounts.map((a) => ({ id: a.id, name: a.name, type: "account" as const })),
        ...projectsList.map((p) => ({ id: p.id, name: p.name, type: "project" as const })),
      ];
      merged.sort((a, b) => a.name.localeCompare(b.name));
      setEntities(merged);
    } catch {
      toast.error("Failed to load picker data");
      setShowLinkForm(false);
    }
  }

  async function handleCreateLink() {
    if (!selectedProject || !selectedEntity) return;
    const entity = entities.find((e) => e.id === selectedEntity);
    if (!entity) return;
    try {
      await invoke("create_linear_entity_link", {
        linearProjectId: selectedProject,
        entityId: entity.id,
        entityType: entity.type,
      });
      toast("Link created");
      setShowLinkForm(false);
      loadEntityLinks();
    } catch (err) {
      toast.error("Failed to create link");
    }
  }

  const statusColor = !status
    ? "var(--color-text-tertiary)"
    : status.enabled && status.issueCount > 0
      ? "var(--color-garden-olive)"
      : "var(--color-text-tertiary)";

  const statusLabel = !status
    ? "Loading..."
    : `${status.issueCount} issues \u00b7 ${status.projectCount} projects`;

  const priorityColor = (label: string | null) => {
    switch (label) {
      case "Urgent": return "var(--color-spice-terracotta)";
      case "High": return "var(--color-warm-turmeric)";
      default: return "var(--color-text-tertiary)";
    }
  };

  const selectStyle: React.CSSProperties = {
    fontFamily: "var(--font-sans)",
    fontSize: 13,
    color: "var(--color-text-primary)",
    backgroundColor: "var(--color-bg-secondary, #f5f5f0)",
    border: "1px solid var(--color-rule-light)",
    borderRadius: 4,
    padding: "6px 8px",
    flex: 1,
    minWidth: 0,
  };

  return (
    <div>
      <p style={styles.subsectionLabel}>Linear Issue Tracking</p>
      <p style={{ ...styles.description, marginBottom: 16 }}>
        Sync your assigned issues and projects from Linear
      </p>

      <div style={styles.settingRow}>
        <div>
          <span style={{ fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)" }}>
            {status?.enabled ? "Enabled" : "Disabled"}
          </span>
          <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
            {status?.enabled
              ? "Issues and projects will sync from Linear"
              : "Linear sync is turned off"}
          </p>
        </div>
        <button
          style={{ ...styles.btn, ...styles.btnGhost, opacity: !status ? 0.5 : 1 }}
          onClick={toggleEnabled}
          disabled={!status}
        >
          {status?.enabled ? "Disable" : "Enable"}
        </button>
      </div>

      {status?.enabled && (
        <>
          <div style={styles.settingRow}>
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <div style={styles.statusDot(statusColor)} />
              <span style={{ fontFamily: "var(--font-sans)", fontSize: 13, color: "var(--color-text-secondary)" }}>
                {statusLabel}
              </span>
              {viewerName && (
                <span style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
                  ({viewerName})
                </span>
              )}
            </div>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                style={{ ...styles.btn, ...styles.btnGhost, opacity: testing ? 0.5 : 1 }}
                onClick={testConnection}
                disabled={testing || !status.apiKeySet}
              >
                {testing ? "Testing..." : "Test Connection"}
              </button>
              <button
                style={{ ...styles.btn, ...styles.btnGhost, opacity: syncing ? 0.5 : 1 }}
                onClick={handleSync}
                disabled={syncing}
              >
                {syncing ? "Syncing..." : "Sync Now"}
              </button>
            </div>
          </div>

          <div style={{ ...styles.settingRow, borderBottom: "none" }}>
            <div style={{ flex: 1 }}>
              <span style={styles.monoLabel}>API Key</span>
              <p style={{ ...styles.description, fontSize: 12, marginTop: 2 }}>
                Personal API key from Linear settings
              </p>
              <div style={{ display: "flex", alignItems: "center", gap: 8, marginTop: 8 }}>
                <input
                  type="password"
                  value={apiKey}
                  onChange={(e) => {
                    setApiKey(e.target.value);
                    setApiKeyDirty(true);
                  }}
                  placeholder="Enter Linear API key"
                  style={{
                    ...styles.input,
                    width: 260,
                  }}
                />
                {apiKeyDirty && apiKey.trim() && (
                  <button
                    style={{ ...styles.btn, ...styles.btnPrimary }}
                    onClick={saveApiKey}
                  >
                    Save
                  </button>
                )}
              </div>
            </div>
          </div>

          {status.lastSyncAt && (
            <div style={{ padding: "8px 0", fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)" }}>
              Last sync: {new Date(status.lastSyncAt).toLocaleString()}
            </div>
          )}

          {/* Recent Issues */}
          {recentIssues.length > 0 && (
            <div style={{ marginTop: 16 }}>
              <hr style={styles.thinRule} />
              <p style={{ ...styles.monoLabel, marginBottom: 8 }}>Active Issues</p>
              {recentIssues.map((issue) => (
                <div
                  key={issue.id}
                  style={{
                    display: "flex",
                    alignItems: "baseline",
                    gap: 8,
                    padding: "4px 0",
                    borderBottom: "1px solid var(--color-rule-light)",
                  }}
                >
                  <span style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                    flexShrink: 0,
                    width: 72,
                  }}>
                    {issue.identifier}
                  </span>
                  <span style={{
                    fontFamily: "var(--font-sans)",
                    fontSize: 13,
                    color: "var(--color-text-primary)",
                    flex: 1,
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}>
                    {issue.title}
                  </span>
                  {issue.priorityLabel && (
                    <span style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 10,
                      color: priorityColor(issue.priorityLabel),
                      flexShrink: 0,
                    }}>
                      {issue.priorityLabel}
                    </span>
                  )}
                  <span style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 10,
                    color: "var(--color-text-tertiary)",
                    flexShrink: 0,
                  }}>
                    {issue.stateName ?? ""}
                  </span>
                </div>
              ))}
            </div>
          )}

          {/* Entity Links */}
          {status.issueCount > 0 && (
            <div style={{ marginTop: 16 }}>
              <hr style={styles.thinRule} />
              <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 8 }}>
                <p style={{ ...styles.monoLabel, margin: 0 }}>Entity Links</p>
                <div style={{ display: "flex", gap: 8 }}>
                  <button
                    style={{ ...styles.btn, ...styles.btnGhost, opacity: autoLinking ? 0.5 : 1 }}
                    onClick={handleAutoLink}
                    disabled={autoLinking}
                  >
                    {autoLinking ? "Detecting..." : "Auto-detect"}
                  </button>
                  <button
                    style={{ ...styles.btn, ...styles.btnPrimary }}
                    onClick={openLinkForm}
                  >
                    Link Project
                  </button>
                </div>
              </div>
              <p style={{ ...styles.description, fontSize: 12, marginBottom: 8 }}>
                Link Linear projects to DailyOS entities for signal routing and meeting context
              </p>

              {/* Manual link form */}
              {showLinkForm && (
                <div style={{
                  padding: 12,
                  marginBottom: 12,
                  border: "1px solid var(--color-rule-light)",
                  borderRadius: 4,
                  backgroundColor: "var(--color-bg-secondary, #f5f5f0)",
                }}>
                  <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 8 }}>
                    <select
                      value={selectedProject}
                      onChange={(e) => setSelectedProject(e.target.value)}
                      style={selectStyle}
                    >
                      <option value="">Select Linear project...</option>
                      {linearProjects.map((p) => (
                        <option key={p.id} value={p.id}>{p.name}</option>
                      ))}
                    </select>
                    <span style={{
                      fontFamily: "var(--font-mono)",
                      fontSize: 11,
                      color: "var(--color-text-tertiary)",
                      flexShrink: 0,
                    }}>
                      &rarr;
                    </span>
                    <select
                      value={selectedEntity}
                      onChange={(e) => setSelectedEntity(e.target.value)}
                      style={selectStyle}
                    >
                      <option value="">Select entity...</option>
                      {entities.map((e) => (
                        <option key={e.id} value={e.id}>
                          {e.name} ({e.type})
                        </option>
                      ))}
                    </select>
                  </div>
                  <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
                    <button
                      style={{ ...styles.btn, ...styles.btnGhost }}
                      onClick={() => setShowLinkForm(false)}
                    >
                      Cancel
                    </button>
                    <button
                      style={{
                        ...styles.btn,
                        ...styles.btnPrimary,
                        opacity: !selectedProject || !selectedEntity ? 0.5 : 1,
                      }}
                      onClick={handleCreateLink}
                      disabled={!selectedProject || !selectedEntity}
                    >
                      Link
                    </button>
                  </div>
                </div>
              )}

              {entityLinks.length === 0 && !showLinkForm ? (
                <p style={{ ...styles.description, fontSize: 12, fontStyle: "italic" }}>
                  No entity links configured. Use auto-detect or link manually.
                </p>
              ) : (
                entityLinks.map((link) => (
                  <div
                    key={link.id}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      padding: "6px 0",
                      borderBottom: "1px solid var(--color-rule-light)",
                    }}
                  >
                    <div style={{ display: "flex", alignItems: "center", gap: 8, flex: 1, minWidth: 0 }}>
                      <span style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-primary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}>
                        {link.projectName ?? link.linearProjectId}
                      </span>
                      <span style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 11,
                        color: "var(--color-text-tertiary)",
                        flexShrink: 0,
                      }}>
                        &rarr;
                      </span>
                      <span style={{
                        fontFamily: "var(--font-sans)",
                        fontSize: 13,
                        color: "var(--color-text-secondary)",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        whiteSpace: "nowrap",
                      }}>
                        {link.entityName ?? link.entityId}
                      </span>
                      <span style={{
                        fontFamily: "var(--font-mono)",
                        fontSize: 10,
                        color: "var(--color-text-tertiary)",
                        flexShrink: 0,
                        textTransform: "uppercase",
                        letterSpacing: "0.04em",
                      }}>
                        {link.entityType}
                      </span>
                      {!link.confirmed && (
                        <span style={{
                          fontFamily: "var(--font-mono)",
                          fontSize: 9,
                          color: "var(--color-warm-turmeric)",
                          flexShrink: 0,
                        }}>
                          auto
                        </span>
                      )}
                    </div>
                    <button
                      style={{
                        ...styles.btn,
                        ...styles.btnGhost,
                        fontSize: 10,
                        padding: "2px 8px",
                        flexShrink: 0,
                      }}
                      onClick={() => handleDeleteLink(link.id)}
                    >
                      Remove
                    </button>
                  </div>
                ))
              )}
            </div>
          )}
        </>
      )}
    </div>
  );
}
