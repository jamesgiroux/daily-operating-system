import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import {
  BulkCreateForm,
  parseBulkCreateInput,
} from "@/components/ui/bulk-create-form";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import type { ProjectListItem } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";

/** Lightweight shape returned by get_archived_projects (DbProject from Rust). */
interface ArchivedProject {
  id: string;
  name: string;
  status: string;
  milestone?: string;
  owner?: string;
  targetDate?: string;
  archived: boolean;
}

type ArchiveTab = "active" | "archived";
type StatusTab = "all" | "active" | "on_hold" | "completed";

const archiveTabs: ArchiveTab[] = ["active", "archived"];
const statusTabs: StatusTab[] = ["all", "active", "on_hold", "completed"];

const statusDotColor: Record<string, string> = {
  active: "var(--color-garden-sage)",
  on_hold: "var(--color-spice-turmeric)",
  completed: "var(--color-garden-larkspur)",
};

const statusLabel: Record<string, string> = {
  active: "Active",
  on_hold: "On Hold",
  completed: "Completed",
};

export default function ProjectsPage() {
  const { personality } = usePersonality();
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [statusTab, setStatusTab] = useState<StatusTab>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedProjects, setArchivedProjects] = useState<ArchivedProject[]>([]);
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkValue, setBulkValue] = useState("");

  const loadProjects = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ProjectListItem[]>("get_projects_list");
      setProjects(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadArchivedProjects = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ArchivedProject[]>("get_archived_projects");
      setArchivedProjects(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (archiveTab === "active") {
      loadProjects();
    } else {
      loadArchivedProjects();
    }
  }, [archiveTab, loadProjects, loadArchivedProjects]);

  async function handleCreate() {
    if (!newName.trim()) return;
    try {
      await invoke<string>("create_project", { name: newName.trim() });
      setNewName("");
      setCreating(false);
      await loadProjects();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleBulkCreate() {
    const names = parseBulkCreateInput(bulkValue);
    if (names.length === 0) return;
    try {
      await invoke<string[]>("bulk_create_projects", { names });
      setBulkValue("");
      setBulkMode(false);
      await loadProjects();
    } catch (e) {
      setError(String(e));
    }
  }

  // Filters
  const statusFiltered =
    statusTab === "all" ? projects : projects.filter((p) => p.status === statusTab);

  const filtered = searchQuery
    ? statusFiltered.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.owner ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : statusFiltered;

  const filteredArchived = searchQuery
    ? archivedProjects.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.owner ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedProjects;

  const isArchived = archiveTab === "archived";
  const displayList = isArchived ? filteredArchived : filtered;
  const activeCount = projects.filter((p) => p.status === "active").length;

  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // FolioBar stats
  const folioStats = useMemo((): ReadinessStat[] => {
    const stats: ReadinessStat[] = [];
    if (activeCount > 0) stats.push({ label: `${activeCount} active`, color: "sage" });
    return stats;
  }, [activeCount]);

  const folioNewButton = useMemo(() => (
    <button
      onClick={() => setCreating(true)}
      style={{
        fontFamily: "var(--font-mono)",
        fontSize: 11,
        fontWeight: 600,
        letterSpacing: "0.06em",
        textTransform: "uppercase" as const,
        color: "var(--color-garden-olive)",
        background: "none",
        border: "1px solid var(--color-garden-olive)",
        borderRadius: 4,
        padding: "2px 10px",
        cursor: "pointer",
      }}
    >
      + New
    </button>
  ), []);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Projects",
      atmosphereColor: "olive" as const,
      activePage: "projects" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioStats,
      folioActions: isArchived ? undefined : folioNewButton,
    }),
    [formattedDate, folioStats, isArchived, folioNewButton],
  );
  useRegisterMagazineShell(shellConfig);

  // Loading
  if (loading && (isArchived ? archivedProjects.length === 0 : projects.length === 0)) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        {[1, 2, 3, 4].map((i) => (
          <div key={i} style={{ height: 52, background: "var(--color-rule-light)", borderRadius: 8, marginBottom: 12 }} />
        ))}
      </div>
    );
  }

  // Error
  if (error) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80, textAlign: "center" }}>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-spice-terracotta)" }}>{error}</p>
        <button
          onClick={loadProjects}
          style={{ fontFamily: "var(--font-mono)", fontSize: 12, color: "var(--color-text-tertiary)", background: "none", border: "1px solid var(--color-rule-heavy)", borderRadius: 4, padding: "4px 12px", cursor: "pointer", marginTop: 12 }}
        >
          Retry
        </button>
      </div>
    );
  }

  // Empty
  if (!isArchived && projects.length === 0) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        <h1 style={{ fontFamily: "var(--font-serif)", fontSize: 36, fontWeight: 400, letterSpacing: "-0.02em", color: "var(--color-text-primary)", margin: "0 0 24px 0" }}>
          Projects
        </h1>
        <div style={{ textAlign: "center", padding: "64px 0" }}>
          <p style={{ fontFamily: "var(--font-serif)", fontSize: 18, fontStyle: "italic", color: "var(--color-text-tertiary)" }}>
            {getPersonalityCopy("projects-no-matches", personality).title}
          </p>
          <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 300, color: "var(--color-text-tertiary)", marginTop: 8 }}>
            Create your first project to get started.
          </p>
          {creating ? (
            <div style={{ maxWidth: 400, margin: "24px auto 0" }}>
              <InlineCreateForm
                value={newName}
                onChange={setNewName}
                onCreate={handleCreate}
                onCancel={() => setCreating(false)}
                placeholder="Project name"
              />
            </div>
          ) : (
            <button
              onClick={() => setCreating(true)}
              style={{ fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 600, color: "var(--color-garden-olive)", background: "none", border: "1px solid var(--color-garden-olive)", borderRadius: 4, padding: "6px 16px", cursor: "pointer", marginTop: 24 }}
            >
              + New Project
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ PAGE HEADER ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 24 }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <h1 style={{ fontFamily: "var(--font-serif)", fontSize: 36, fontWeight: 400, letterSpacing: "-0.02em", color: "var(--color-text-primary)", margin: 0 }}>
            Projects
          </h1>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
            {isArchived ? filteredArchived.length : filtered.length} {isArchived ? "archived" : "active"}
          </span>
        </div>

        <div style={{ height: 1, background: "var(--color-rule-heavy)", marginTop: 16, marginBottom: 16 }} />

        {/* Archive toggle */}
        <div style={{ display: "flex", gap: 20, marginBottom: 12 }}>
          {archiveTabs.map((tab) => (
            <button
              key={tab}
              onClick={() => setArchiveTab(tab)}
              style={{
                fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 500, letterSpacing: "0.06em", textTransform: "uppercase",
                color: archiveTab === tab ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
                textDecoration: archiveTab === tab ? "underline" : "none", textUnderlineOffset: "4px",
                background: "none", border: "none", padding: 0, cursor: "pointer",
              }}
            >
              {tab}
            </button>
          ))}
        </div>

        {/* Status filter (active only) */}
        {!isArchived && (
          <div style={{ display: "flex", gap: 20, marginBottom: 16 }}>
            {statusTabs.map((tab) => (
              <button
                key={tab}
                onClick={() => setStatusTab(tab)}
                style={{
                  fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 500, letterSpacing: "0.06em", textTransform: "uppercase",
                  color: statusTab === tab ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
                  textDecoration: statusTab === tab ? "underline" : "none", textUnderlineOffset: "4px",
                  background: "none", border: "none", padding: 0, cursor: "pointer",
                }}
              >
                {tab === "on_hold" ? "on hold" : tab}
              </button>
            ))}
          </div>
        )}

        {/* Search */}
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="⌘  Search projects..."
          style={{ width: "100%", fontFamily: "var(--font-sans)", fontSize: 14, color: "var(--color-text-primary)", background: "none", border: "none", borderBottom: "1px solid var(--color-rule-light)", padding: "8px 0", outline: "none" }}
        />
      </section>

      {/* ═══ CREATE FORM ═══ */}
      {creating && !isArchived && (
        <div style={{ marginBottom: 16 }}>
          {bulkMode ? (
            <BulkCreateForm
              value={bulkValue}
              onChange={setBulkValue}
              onCreate={handleBulkCreate}
              onSingleMode={() => { setBulkMode(false); setBulkValue(""); }}
              onCancel={() => { setCreating(false); setBulkMode(false); setBulkValue(""); setNewName(""); }}
              placeholder="One project name per line"
            />
          ) : (
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <InlineCreateForm
                value={newName}
                onChange={setNewName}
                onCreate={handleCreate}
                onCancel={() => { setCreating(false); setNewName(""); }}
                placeholder="Project name"
              />
              <button
                onClick={() => setBulkMode(true)}
                style={{ fontFamily: "var(--font-mono)", fontSize: 11, color: "var(--color-text-tertiary)", background: "none", border: "none", cursor: "pointer" }}
              >
                Bulk
              </button>
            </div>
          )}
        </div>
      )}

      {/* ═══ PROJECT ROWS ═══ */}
      <section>
        {displayList.length === 0 ? (
          <div style={{ textAlign: "center", padding: "64px 0" }}>
            <p style={{ fontFamily: "var(--font-serif)", fontSize: 18, fontStyle: "italic", color: "var(--color-text-tertiary)" }}>
              {isArchived
                ? getPersonalityCopy("projects-archived-empty", personality).title
                : getPersonalityCopy("projects-no-matches", personality).title}
            </p>
            <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 300, color: "var(--color-text-tertiary)", marginTop: 8 }}>
              {isArchived
                ? getPersonalityCopy("projects-archived-empty", personality).message ?? ""
                : getPersonalityCopy("projects-no-matches", personality).message ?? ""}
            </p>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {isArchived
              ? filteredArchived.map((project, i) => (
                  <ArchivedProjectRow key={project.id} project={project} showBorder={i < filteredArchived.length - 1} />
                ))
              : filtered.map((project, i) => (
                  <ProjectRow key={project.id} project={project} showBorder={i < filtered.length - 1} />
                ))}
          </div>
        )}
      </section>

      {/* ═══ END MARK ═══ */}
      {displayList.length > 0 && (
        <div style={{ borderTop: "1px solid var(--color-rule-heavy)", marginTop: 48, paddingTop: 32, paddingBottom: 120, textAlign: "center" }}>
          <div style={{ fontFamily: "var(--font-serif)", fontSize: 14, fontStyle: "italic", color: "var(--color-text-tertiary)" }}>
            That's everything.
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Project Row ────────────────────────────────────────────────────────────

function ProjectRow({ project, showBorder }: { project: ProjectListItem; showBorder: boolean }) {
  const daysSince = project.daysSinceLastMeeting;
  const isStale = daysSince != null && daysSince > 30;

  const subtitle = [
    project.owner,
    project.milestone,
  ].filter(Boolean).join(" \u00B7 ");

  return (
    <Link
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "14px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        textDecoration: "none",
      }}
    >
      {/* Status dot */}
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: statusDotColor[project.status] ?? "var(--color-paper-linen)",
          flexShrink: 0,
          marginTop: 8,
        }}
      />

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span style={{ fontFamily: "var(--font-serif)", fontSize: 17, fontWeight: 400, color: "var(--color-text-primary)" }}>
            {project.name}
          </span>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase",
              color: project.status === "active"
                ? "var(--color-garden-sage)"
                : project.status === "on_hold"
                  ? "var(--color-spice-turmeric)"
                  : "var(--color-garden-larkspur)",
            }}
          >
            {statusLabel[project.status] ?? project.status}
          </span>
        </div>
        {subtitle && (
          <div style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 300, color: "var(--color-text-tertiary)", marginTop: 2 }}>
            {subtitle}
          </div>
        )}
      </div>

      {/* Right metrics */}
      <div style={{ display: "flex", alignItems: "baseline", gap: 16, flexShrink: 0 }}>
        {project.targetDate && (
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
            {project.targetDate}
          </span>
        )}
        {project.openActionCount > 0 && (
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-tertiary)" }}>
            {project.openActionCount} action{project.openActionCount !== 1 ? "s" : ""}
          </span>
        )}
        {daysSince != null && (
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: isStale ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
            }}
          >
            {daysSince === 0 ? "Today" : `${daysSince}d`}
          </span>
        )}
      </div>
    </Link>
  );
}

// ─── Archived Project Row ───────────────────────────────────────────────────

function ArchivedProjectRow({ project, showBorder }: { project: ArchivedProject; showBorder: boolean }) {
  const subtitle = [
    project.owner ? `Owner: ${project.owner}` : "",
    project.status,
  ].filter(Boolean).join(" \u00B7 ");

  return (
    <Link
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "14px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        textDecoration: "none",
      }}
    >
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: statusDotColor[project.status] ?? "var(--color-paper-linen)",
          flexShrink: 0,
          marginTop: 8,
        }}
      />
      <div style={{ flex: 1, minWidth: 0 }}>
        <span style={{ fontFamily: "var(--font-serif)", fontSize: 17, fontWeight: 400, color: "var(--color-text-primary)" }}>
          {project.name}
        </span>
        {subtitle && (
          <div style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 300, color: "var(--color-text-tertiary)", marginTop: 2 }}>
            {subtitle}
          </div>
        )}
      </div>
    </Link>
  );
}
