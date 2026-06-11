import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectListItem } from "@/types";

/** Lightweight shape returned by get_archived_projects (DbProject from Rust). */
export interface ArchivedProject {
  id: string;
  name: string;
  status: string;
  milestone?: string;
  owner?: string;
  targetDate?: string;
  archived: boolean;
}

export function useProjectsCommands() {
  const getProjectsList = useCallback(() => {
    return invoke<ProjectListItem[]>("get_projects_list");
  }, []);

  const getChildProjectsList = useCallback((parentId: string) => {
    return invoke<ProjectListItem[]>("get_child_projects_list", { parentId });
  }, []);

  const getArchivedProjects = useCallback(() => {
    return invoke<ArchivedProject[]>("get_archived_projects");
  }, []);

  const createProject = useCallback((name: string) => {
    return invoke<string>("create_project", { name });
  }, []);

  const bulkCreateProjects = useCallback((names: string[]) => {
    return invoke<string[]>("bulk_create_projects", { names });
  }, []);

  return useMemo(() => ({
    bulkCreateProjects,
    createProject,
    getArchivedProjects,
    getChildProjectsList,
    getProjectsList,
  }), [
    bulkCreateProjects,
    createProject,
    getArchivedProjects,
    getChildProjectsList,
    getProjectsList,
  ]);
}
