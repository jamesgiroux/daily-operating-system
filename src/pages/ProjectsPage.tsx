import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PageError } from "@/components/PageState";
import { cn } from "@/lib/utils";
import { FolderKanban, Plus, RefreshCw, Search } from "lucide-react";
import type { ProjectListItem } from "@/types";

type StatusTab = "all" | "active" | "on_hold" | "completed";

const tabs: { key: StatusTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "active", label: "Active" },
  { key: "on_hold", label: "On Hold" },
  { key: "completed", label: "Completed" },
];

const statusStyles: Record<string, string> = {
  active:
    "bg-green-100 text-green-800 border-green-300 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700",
  on_hold:
    "bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/30 dark:text-yellow-400 dark:border-yellow-700",
  completed:
    "bg-blue-100 text-blue-800 border-blue-300 dark:bg-blue-900/30 dark:text-blue-400 dark:border-blue-700",
  archived:
    "bg-muted text-muted-foreground border-muted",
};

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
            <div className="flex items-center gap-2">
              <input
                type="text"
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleCreate();
                  if (e.key === "Escape") setCreating(false);
                }}
                placeholder="Project name"
                className="rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
              />
              <Button size="sm" onClick={handleCreate}>
                Create
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setCreating(false)}
              >
                Cancel
              </Button>
            </div>
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
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    autoFocus
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleCreate();
                      if (e.key === "Escape") {
                        setCreating(false);
                        setNewName("");
                      }
                    }}
                    placeholder="Project name"
                    className="rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  />
                  <Button size="sm" onClick={handleCreate}>
                    Create
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setCreating(false);
                      setNewName("");
                    }}
                  >
                    Cancel
                  </Button>
                </div>
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

          {/* Search */}
          <div className="relative mb-4">
            <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search projects..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full rounded-md border bg-background py-2 pl-10 pr-4 text-sm outline-none focus:ring-1 focus:ring-ring"
            />
          </div>

          {/* Status tabs */}
          <div className="mb-6 flex gap-2">
            {tabs.map((t) => (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={cn(
                  "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                  tab === t.key
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted hover:bg-muted/80"
                )}
              >
                {t.label}
                {tabCounts[t.key] > 0 && (
                  <span
                    className={cn(
                      "ml-1.5 inline-flex size-5 items-center justify-center rounded-full text-xs",
                      tab === t.key
                        ? "bg-primary-foreground/20 text-primary-foreground"
                        : "bg-muted-foreground/15 text-muted-foreground"
                    )}
                  >
                    {tabCounts[t.key]}
                  </span>
                )}
              </button>
            ))}
          </div>

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
              <StatusBadge status={project.status} />
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

function StatusBadge({ status }: { status: string }) {
  return (
    <Badge
      variant="outline"
      className={cn("text-xs", statusStyles[status] ?? statusStyles.active)}
    >
      {status.replace("_", " ")}
    </Badge>
  );
}
