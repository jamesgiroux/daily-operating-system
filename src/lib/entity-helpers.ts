import type { LinkedEntity } from "@/types";

/** Return the name of the first linked entity, or null if none. */
export function getPrimaryEntityName(entities?: LinkedEntity[]): string | null {
  if (!entities?.length) return null;
  return entities[0]?.name ?? null;
}

/** Format entity byline: "{Name} · {TypeLabel}" */
export function formatEntityByline(entities?: LinkedEntity[]): string | null {
  if (!entities?.length) return null;
  const entity = entities[0];
  if (!entity?.name) return null;
  const typeLabels: Record<string, string> = {
    account: "Customer",
    project: "Project",
    person: "1:1",
  };
  const label = typeLabels[entity.entityType] ?? entity.entityType;
  return `${entity.name} \u00B7 ${label}`;
}

/** Get the primary entity type, or null. */
export function getPrimaryEntityType(entities?: LinkedEntity[]): string | null {
  if (!entities?.length) return null;
  return entities[0]?.entityType ?? null;
}
