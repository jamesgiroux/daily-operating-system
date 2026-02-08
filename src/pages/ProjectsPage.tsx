import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import {
  StatusBadge,
  projectStatusStyles,
} from "@/components/ui/status-badge";
import { PageError } from "@/components/PageState";
import { FolderKanban, Plus, RefreshCw } from "lucide-react";
import type { ProjectListItem } from "@/types";

type StatusTab = "all" | "active" | "on_hold" | "completed";

const tabs: { key: StatusTab; label: string }[] = [
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

  useEffect(() => {
    loadProjects();
  }, [loadProjects]);

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

  if (loading && projects.length === 0) {
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

  if (projects.length === 0) {
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
                  {filtered.length}
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                Project status, milestones, and deliverables
              </p>
            </div>
            <div className="flex items-center gap-2">
              {creating ? (
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
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={loadProjects}
              >
                <RefreshCw className="size-4" />
              </Button>
            </div>
          </div>

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search projects..."
            className="mb-4"
          />

          <TabFilter
            tabs={tabs}
            active={tab}
            onChange={setTab}
            counts={tabCounts}
            className="mb-6"
          />

          {/* Projects list */}
          <div className="space-y-2">
            {filtered.length === 0 ? (
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

function ProjectRow({ project }: { project: ProjectListItem }) {
  return (
    <Link to="/projects/$projectId" params={{ projectId: project.id }}>
      <Card className="cursor-pointer transition-all hover:-translate-y-0.5 hover:shadow-md">
        <CardContent className="flex items-center gap-4 p-4">
          {/* Avatar initial */}
          <div className="flex size-10 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">
            {project.name.charAt(0).toUpperCase()}
          </div>

          {/* Name + badges */}
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="truncate font-medium">{project.name}</span>
              <StatusBadge
                value={project.status}
                styles={projectStatusStyles}
                fallback={projectStatusStyles.active}
              />
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              {project.owner && <span>Owner: {project.owner}</span>}
              {project.owner && project.milestone && (
                <span className="text-muted-foreground/40">&middot;</span>
              )}
              {project.milestone && <span>{project.milestone}</span>}
            </div>
          </div>

          {/* Target date */}
          {project.targetDate && (
            <div className="shrink-0 text-right">
              <div className="text-xs text-muted-foreground">
                Target: {project.targetDate}
              </div>
            </div>
          )}

          {/* Open actions count */}
          {project.openActionCount > 0 && (
            <div className="shrink-0 text-right">
              <div className="text-sm font-medium">
                {project.openActionCount}
              </div>
              <div className="text-xs text-muted-foreground">actions</div>
            </div>
          )}

          {/* Days since last meeting */}
          {project.daysSinceLastMeeting != null && (
            <div className="w-16 shrink-0 text-right">
              <div className="text-xs text-muted-foreground">
                {project.daysSinceLastMeeting === 0
                  ? "Today"
                  : `${project.daysSinceLastMeeting}d ago`}
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </Link>
  );
}

