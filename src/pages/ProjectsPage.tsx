import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import {
  BulkCreateForm,
  parseBulkCreateInput,
} from "@/components/ui/bulk-create-form";
import {
  EntityListSkeleton,
  EntityListError,
  EntityListEmpty,
  EntityListHeader,
  EntityListEndMark,
  ArchiveToggle,
  FilterTabs,
} from "@/components/entity/EntityListShell";
import { EntityRow } from "@/components/entity/EntityRow";
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

const statusTabs: readonly StatusTab[] = ["all", "active", "on_hold", "completed"];

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

const statusTabLabels: Partial<Record<StatusTab, string>> = {
  on_hold: "on hold",
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
    return <EntityListSkeleton />;
  }

  // Error
  if (error) {
    return <EntityListError error={error} onRetry={loadProjects} />;
  }

  // Empty
  if (!isArchived && projects.length === 0) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        <h1 style={{ fontFamily: "var(--font-serif)", fontSize: 36, fontWeight: 400, letterSpacing: "-0.02em", color: "var(--color-text-primary)", margin: "0 0 24px 0" }}>
          Projects
        </h1>
        <EntityListEmpty
          title={getPersonalityCopy("projects-no-matches", personality).title}
          message="Create your first project to get started."
        >
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
        </EntityListEmpty>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      <EntityListHeader
        headline="Projects"
        count={isArchived ? filteredArchived.length : filtered.length}
        countLabel={isArchived ? "archived" : "active"}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder="⌘  Search projects..."
      >
        <ArchiveToggle archiveTab={archiveTab} onTabChange={setArchiveTab} />
        {!isArchived && (
          <FilterTabs
            tabs={statusTabs}
            active={statusTab}
            onChange={setStatusTab}
            labelMap={statusTabLabels}
          />
        )}
      </EntityListHeader>

      {/* Create form */}
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

      {/* Project rows */}
      <section>
        {displayList.length === 0 ? (
          <EntityListEmpty
            title={isArchived
              ? getPersonalityCopy("projects-archived-empty", personality).title
              : getPersonalityCopy("projects-no-matches", personality).title}
            message={isArchived
              ? getPersonalityCopy("projects-archived-empty", personality).message ?? ""
              : getPersonalityCopy("projects-no-matches", personality).message ?? ""}
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {isArchived
              ? filteredArchived.map((project, i) => {
                  const subtitle = [
                    project.owner ? `Owner: ${project.owner}` : "",
                    project.status,
                  ].filter(Boolean).join(" \u00B7 ");
                  return (
                    <EntityRow
                      key={project.id}
                      to="/projects/$projectId"
                      params={{ projectId: project.id }}
                      dotColor={statusDotColor[project.status] ?? "var(--color-paper-linen)"}
                      name={project.name}
                      showBorder={i < filteredArchived.length - 1}
                      subtitle={subtitle || undefined}
                    />
                  );
                })
              : filtered.map((project, i) => (
                  <ProjectRow key={project.id} project={project} showBorder={i < filtered.length - 1} />
                ))}
          </div>
        )}
      </section>

      {displayList.length > 0 && <EntityListEndMark />}
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

  const nameSuffix = (
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
  );

  return (
    <EntityRow
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      dotColor={statusDotColor[project.status] ?? "var(--color-paper-linen)"}
      name={project.name}
      showBorder={showBorder}
      nameSuffix={nameSuffix}
      subtitle={subtitle || undefined}
    >
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
    </EntityRow>
  );
}
