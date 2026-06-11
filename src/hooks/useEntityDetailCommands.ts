import { useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { PersonRelationshipEdge } from "@/types";

interface EntityAncestor {
  id: string;
  name: string;
}

/**
 * Commands shared by the entity detail editorial surfaces
 * (account / project / person): ancestor breadcrumbs, relationship
 * edges, and entity metadata reads.
 */
export function useEntityDetailCommands() {
  const getAccountAncestors = useCallback((accountId: string) => {
    return invoke<EntityAncestor[]>("get_account_ancestors", { accountId });
  }, []);

  const getProjectAncestors = useCallback((projectId: string) => {
    return invoke<EntityAncestor[]>("get_project_ancestors", { projectId });
  }, []);

  const getPersonRelationships = useCallback((personId: string) => {
    return invoke<PersonRelationshipEdge[]>("get_person_relationships", { personId });
  }, []);

  const getEntityMetadata = useCallback((entityType: string, entityId: string) => {
    return invoke<string>("get_entity_metadata", { entityType, entityId });
  }, []);

  return useMemo(() => ({
    getAccountAncestors,
    getEntityMetadata,
    getPersonRelationships,
    getProjectAncestors,
  }), [
    getAccountAncestors,
    getEntityMetadata,
    getPersonRelationships,
    getProjectAncestors,
  ]);
}
