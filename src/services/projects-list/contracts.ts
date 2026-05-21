// v1.4.4 W1 substrate extension — TypeScript mirror of the
// `list_projects` ability contract. See `accounts-list/contracts` for
// shape rationale.

import type {
  Cursor,
  CursorState,
  Paginated,
} from "../entity-intelligence/contracts";

export const LIST_PROJECTS_SCHEMA_VERSION = 1;

export type ProjectTrajectory =
  | "improving"
  | "steady"
  | "degrading"
  | "unknown";

export interface ProjectListFilter {
  status?: string;
  trajectory?: ProjectTrajectory;
  parentAccountId?: string;
  nameContains?: string;
}

export interface ProjectListInput {
  schemaVersion: number;
  filter?: ProjectListFilter;
  cursor?: Cursor;
  pageSize: number;
}

export interface ProjectSummary {
  projectId: string;
  name: string;
  parentAccountId?: string | null;
  status: string;
  trajectory: ProjectTrajectory;
  lastTouchpointAt?: string | null;
}

export type ProjectListPage = Paginated<ProjectSummary>;

export type { Cursor, CursorState, Paginated };
