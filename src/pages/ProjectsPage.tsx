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
} from "@/components/entity/EntityListShell";
import shellStyles from "@/components/entity/EntityListShell.module.css";
import { EditorialPageHeader } from "@/components/editorial/EditorialPageHeader";
import { EntityRow } from "@/components/entity/EntityRow";
import { EmptyState } from "@/components/editorial/EmptyState";
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
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedProjects, setArchivedProjects] = useState<ArchivedProject[]>([]);
  const [expandedParents, setExpandedParents] = useState<Set<string>>(new Set());
  const [childrenCache, setChildrenCache] = useState<Record<string, ProjectListItem[]>>({});
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkValue, setBulkValue] = useState("");

  const loadProjects = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ProjectListItem[]>("get_projects_list");
      setProjects(result);

      // Auto-expand all parents and pre-fetch children
      const expanded = new Set<string>();
      const cache: Record<string, ProjectListItem[]> = {};

      async function expandRecursive(items: ProjectListItem[]) {
        const parents = items.filter((p) => p.isParent);
        if (parents.length === 0) return;
        await Promise.all(
          parents.map(async (p) => {
            expanded.add(p.id);
            if (!cache[p.id]) {
              try {
                const children = await invoke<ProjectListItem[]>("get_child_projects_list", { parentId: p.id });
                cache[p.id] = children;
                await expandRecursive(children);
              } catch { /* ignore */ }
            }
          }),
        );
      }

      await expandRecursive(result);
      if (expanded.size > 0) {
        setExpandedParents(expanded);
        setChildrenCache((prev) => ({ ...prev, ...cache }));
      }
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

  async function toggleExpand(parentId: string) {
    const next = new Set(expandedParents);
    if (next.has(parentId)) {
      next.delete(parentId);
    } else {
      next.add(parentId);
      if (!childrenCache[parentId]) {
        try {
          const children = await invoke<ProjectListItem[]>(
            "get_child_projects_list",
            { parentId }
          );
          setChildrenCache((prev) => ({ ...prev, [parentId]: children }));
        } catch (e) {
          setError(String(e));
          return;
        }
      }
    }
    setExpandedParents(next);
  }

  // Filters — archived/active split handled by archiveTab; no status sub-filter
  const activeProjects = projects.filter((p) => !p.archived);

  const filtered = useMemo(() => {
    if (!searchQuery) return activeProjects;
    const q = searchQuery.toLowerCase();
    return activeProjects.filter((p) => {
      if (p.name.toLowerCase().includes(q) || (p.owner ?? "").toLowerCase().includes(q)) {
        return true;
      }
      const children = childrenCache[p.id];
      if (children && children.some((c) => c.name.toLowerCase().includes(q))) {
        return true;
      }
      return false;
    });
  }, [searchQuery, activeProjects, childrenCache]);

  const filteredArchived = searchQuery
    ? archivedProjects.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.owner ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedProjects;

  const isArchived = archiveTab === "archived";
  const displayList = isArchived ? filteredArchived : filtered;
  const activeCount = activeProjects.length;

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
      <div className={shellStyles.pageShell}>
        <EditorialPageHeader title="Projects" scale="standard" width="standard" />
        {(() => {
          const copy = getPersonalityCopy("projects-empty", personality);
          return (
            <EmptyState
              headline={copy.title}
              explanation={copy.explanation ?? copy.message ?? ""}
              benefit={copy.benefit}
              action={!creating ? { label: "Create your first project", onClick: () => setCreating(true) } : undefined}
            >
              {creating && (
                <div style={{ maxWidth: 400, margin: "0 auto", textAlign: "left" }}>
                  <InlineCreateForm
                    value={newName}
                    onChange={setNewName}
                    onCreate={handleCreate}
                    onCancel={() => setCreating(false)}
                    placeholder="Project name"
                  />
                </div>
              )}
            </EmptyState>
          );
        })()}
      </div>
    );
  }

  return (
    <div className={shellStyles.pageShell}>
      <EntityListHeader
        headline="Projects"
        count={isArchived ? filteredArchived.length : filtered.length}
        countLabel={isArchived ? "archived" : "active"}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder="⌘  Search projects..."
      >
        <ArchiveToggle archiveTab={archiveTab} onTabChange={setArchiveTab} />
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
                  <ProjectTreeNode
                    key={project.id}
                    project={project}
                    depth={0}
                    expandedParents={expandedParents}
                    childrenCache={childrenCache}
                    toggleExpand={toggleExpand}
                    isLastSibling={i === filtered.length - 1}
                  />
                ))}
          </div>
        )}
      </section>

      {displayList.length > 0 && <EntityListEndMark />}
    </div>
  );
}

// ─── Recursive Project Tree Node ──────────────────────────────────────────────

function ProjectTreeNode({
  project,
  depth,
  expandedParents,
  childrenCache,
  toggleExpand,
  isLastSibling,
}: {
  project: ProjectListItem;
  depth: number;
  expandedParents: Set<string>;
  childrenCache: Record<string, ProjectListItem[]>;
  toggleExpand: (id: string) => void;
  isLastSibling: boolean;
}) {
  const isExpanded = expandedParents.has(project.id);
  const children = childrenCache[project.id] ?? [];
  const hasExpandedChildren = project.isParent && isExpanded && children.length > 0;

  return (
    <div>
      <ProjectRow
        project={project}
        depth={depth}
        isExpanded={isExpanded}
        onToggleExpand={project.isParent ? () => toggleExpand(project.id) : undefined}
        showBorder={!isLastSibling || hasExpandedChildren}
      />
      {hasExpandedChildren &&
        children.map((child, ci) => (
          <ProjectTreeNode
            key={child.id}
            project={child}
            depth={depth + 1}
            expandedParents={expandedParents}
            childrenCache={childrenCache}
            toggleExpand={toggleExpand}
            isLastSibling={ci === children.length - 1 && isLastSibling}
          />
        ))}
    </div>
  );
}

// ─── Project Row ────────────────────────────────────────────────────────────

function ProjectRow({
  project,
  depth = 0,
  isExpanded,
  onToggleExpand,
  showBorder,
}: {
  project: ProjectListItem;
  depth?: number;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  showBorder: boolean;
}) {
  const subtitle = [
    project.owner,
    project.milestone,
  ].filter(Boolean).join(" \u00B7 ");

  const nameSuffix = (
    <>
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
      {onToggleExpand && (
        <button
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onToggleExpand();
          }}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: 0,
          }}
        >
          {isExpanded ? "\u25BE" : "\u25B8"} {project.childCount} sub{project.childCount !== 1 ? "s" : ""}
        </button>
      )}
    </>
  );

  return (
    <EntityRow
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      dotColor={statusDotColor[project.status] ?? "var(--color-paper-linen)"}
      name={project.name}
      showBorder={showBorder}
      paddingLeft={depth > 0 ? depth * 28 : 0}
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
    </EntityRow>
  );
}
