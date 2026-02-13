import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import {
  BulkCreateForm,
  parseBulkCreateInput,
} from "@/components/ui/bulk-create-form";
import { ListRow, ListColumn } from "@/components/ui/list-row";
import {
  StatusBadge,
  projectStatusStyles,
} from "@/components/ui/status-badge";
import { PageError } from "@/components/PageState";
import { FolderKanban, Plus, RefreshCw } from "lucide-react";
import { cn } from "@/lib/utils";
import type { ProjectListItem } from "@/types";

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

const archiveTabs: { key: ArchiveTab; label: string }[] = [
  { key: "active", label: "Active" },
  { key: "archived", label: "Archived" },
];

const statusTabs: { key: StatusTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "active", label: "Active" },
  { key: "on_hold", label: "On Hold" },
  { key: "completed", label: "Completed" },
];


export default function ProjectsPage() {
  const [projects, setProjects] = useState<ProjectListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<StatusTab>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  // I176: archive tab
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedProjects, setArchivedProjects] = useState<ArchivedProject[]>([]);
  // I162: bulk create mode
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

  // I176: load archived projects
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

  // I162: bulk create
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

  const statusFiltered =
    tab === "all" ? projects : projects.filter((p) => p.status === tab);

  const filtered = searchQuery
    ? statusFiltered.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.owner ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : statusFiltered;

  const tabCounts: Record<StatusTab, number> = {
    all: projects.length,
    active: projects.filter((p) => p.status === "active").length,
    on_hold: projects.filter((p) => p.status === "on_hold").length,
    completed: projects.filter((p) => p.status === "completed").length,
  };

  // I176: filter archived projects by search query
  const filteredArchived = searchQuery
    ? archivedProjects.filter(
        (p) =>
          p.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (p.owner ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedProjects;

  const isArchived = archiveTab === "archived";

  if (loading && (isArchived ? archivedProjects.length === 0 : projects.length === 0)) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-20 w-full" />
          ))}
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} onRetry={loadProjects} />
      </main>
    );
  }

  if (!isArchived && projects.length === 0) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="flex h-full flex-col items-center justify-center gap-4 p-6">
          <FolderKanban className="size-16 text-muted-foreground/40" />
          <div className="text-center">
            <h2 className="text-lg font-semibold">No projects yet</h2>
            <p className="text-sm text-muted-foreground">
              Create your first project to get started.
            </p>
          </div>
          {creating ? (
            <InlineCreateForm
              value={newName}
              onChange={setNewName}
              onCreate={handleCreate}
              onCancel={() => setCreating(false)}
              placeholder="Project name"
            />
          ) : (
            <Button onClick={() => setCreating(true)}>
              <Plus className="mr-1 size-4" />
              New Project
            </Button>
          )}
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6 flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">
                Projects
                <span className="ml-2 text-base font-normal text-muted-foreground">
                  {isArchived ? filteredArchived.length : filtered.length}
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                {isArchived
                  ? "Previously tracked projects"
                  : "Project status, milestones, and deliverables"}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {!isArchived && (
                <>
                  {creating ? (
                    <>
                      {bulkMode ? (
                        <BulkCreateForm
                          value={bulkValue}
                          onChange={setBulkValue}
                          onCreate={handleBulkCreate}
                          onSingleMode={() => {
                            setBulkMode(false);
                            setBulkValue("");
                          }}
                          onCancel={() => {
                            setCreating(false);
                            setBulkMode(false);
                            setBulkValue("");
                            setNewName("");
                          }}
                          placeholder="One project name per line"
                        />
                      ) : (
                        <div className="flex items-center gap-2">
                          <InlineCreateForm
                            value={newName}
                            onChange={setNewName}
                            onCreate={handleCreate}
                            onCancel={() => {
                              setCreating(false);
                              setNewName("");
                            }}
                            placeholder="Project name"
                          />
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => setBulkMode(true)}
                          >
                            Bulk
                          </Button>
                        </div>
                      )}
                    </>
                  ) : (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setCreating(true)}
                    >
                      <Plus className="mr-1 size-4" />
                      New Project
                    </Button>
                  )}
                </>
              )}
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={isArchived ? loadArchivedProjects : loadProjects}
              >
                <RefreshCw className="size-4" />
              </Button>
            </div>
          </div>

          <TabFilter
            tabs={archiveTabs}
            active={archiveTab}
            onChange={setArchiveTab}
            className="mb-4"
          />

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search projects..."
            className="mb-4"
          />

          {!isArchived && (
            <TabFilter
              tabs={statusTabs}
              active={tab}
              onChange={setTab}
              counts={tabCounts}
              className="mb-6"
            />
          )}

          {/* Projects list */}
          <div>
            {isArchived ? (
              filteredArchived.length === 0 ? (
                <Card>
                  <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                    <FolderKanban className="mb-4 size-12 text-muted-foreground/40" />
                    <p className="text-lg font-medium">No archived projects</p>
                    <p className="text-sm text-muted-foreground">
                      Archived projects will appear here.
                    </p>
                  </CardContent>
                </Card>
              ) : (
                filteredArchived.map((project) => (
                  <ArchivedProjectRow key={project.id} project={project} />
                ))
              )
            ) : filtered.length === 0 ? (
              <Card>
                <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                  <FolderKanban className="mb-4 size-12 text-muted-foreground/40" />
                  <p className="text-lg font-medium">No matches</p>
                  <p className="text-sm text-muted-foreground">
                    Try a different search or filter.
                  </p>
                </CardContent>
              </Card>
            ) : (
              filtered.map((project) => (
                <ProjectRow key={project.id} project={project} />
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

const statusDot: Record<string, string> = {
  active: "bg-success",
  on_hold: "bg-primary",
  completed: "bg-blue-500",
};

function ProjectRow({ project }: { project: ProjectListItem }) {
  const daysSince = project.daysSinceLastMeeting;
  const isStale = daysSince != null && daysSince > 30;

  const subtitle = [
    project.owner ? `Owner: ${project.owner}` : null,
    project.milestone,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <ListRow
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      signalColor={statusDot[project.status] ?? "bg-muted-foreground/30"}
      name={project.name}
      badges={
        <StatusBadge
          value={project.status}
          styles={projectStatusStyles}
          fallback={projectStatusStyles.active}
        />
      }
      subtitle={subtitle || undefined}
      columns={
        <>
          {project.targetDate && (
            <ListColumn value={project.targetDate} label="target" className="w-20" />
          )}
          {project.openActionCount > 0 && (
            <ListColumn
              value={project.openActionCount}
              label="actions"
              className="w-14"
            />
          )}
          {daysSince != null && (
            <ListColumn
              value={
                <span className={cn(isStale && "text-destructive")}>
                  {daysSince === 0 ? "Today" : `${daysSince}d`}
                </span>
              }
              label="last mtg"
              className="w-14"
            />
          )}
        </>
      }
    />
  );
}

/** I176: Simplified row for archived projects (no active metrics). */
function ArchivedProjectRow({ project }: { project: ArchivedProject }) {
  return (
    <ListRow
      to="/projects/$projectId"
      params={{ projectId: project.id }}
      signalColor={statusDot[project.status] ?? "bg-muted-foreground/30"}
      name={project.name}
      subtitle={
        [project.owner ? `Owner: ${project.owner}` : "", project.status]
          .filter(Boolean)
          .join(" \u00B7 ") || undefined
      }
    />
  );
}
