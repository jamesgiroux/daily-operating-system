// v1.4.4 W1 substrate extension — TypeScript mirror of the
// `list_accounts` ability contract. Parity for the Rust producer at
// `src-tauri/abilities-runtime/src/abilities/list_accounts/`. Renames
// Rust snake_case fields to camelCase per the producer's serde
// annotations.

import type {
  Cursor,
  CursorState,
  Paginated,
  TrustBand,
} from "../entity-intelligence/contracts";

export const LIST_ACCOUNTS_SCHEMA_VERSION = 1;

export interface AccountListFilter {
  status?: string;
  healthBand?: TrustBand;
  nameContains?: string;
}

export interface AccountListInput {
  schemaVersion: number;
  filter?: AccountListFilter;
  cursor?: Cursor;
  pageSize: number;
}

export interface AccountSummary {
  accountId: string;
  name: string;
  status: string;
  healthBand: TrustBand;
  lastTouchpointAt?: string | null;
  openLoopsCount: number;
}

export type AccountListPage = Paginated<AccountSummary>;

// Re-export the shared cursor + state types so callers needn't reach
// across packages to handle pagination uniformly.
export type { Cursor, CursorState, Paginated };
