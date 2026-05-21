// v1.4.4 W1 substrate extension — TypeScript mirror of the
// `list_people` ability contract. See `accounts-list/contracts` for
// shape rationale.

import type {
  Cursor,
  CursorState,
  Paginated,
} from "../entity-intelligence/contracts";

export const LIST_PEOPLE_SCHEMA_VERSION = 1;

export interface PersonListFilter {
  role?: string;
  primaryAccountId?: string;
  nameContains?: string;
}

export interface PersonListInput {
  schemaVersion: number;
  filter?: PersonListFilter;
  cursor?: Cursor;
  pageSize: number;
}

export interface PersonSummary {
  personId: string;
  displayName: string;
  primaryAccountId?: string | null;
  role: string;
  lastTouchpointAt?: string | null;
}

export type PersonListPage = Paginated<PersonSummary>;

export type { Cursor, CursorState, Paginated };
