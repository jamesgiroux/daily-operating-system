import type { LinkedEntity } from "@/types";

/** Return the name of the first linked entity, or null if none. */
export function getPrimaryEntityName(entities?: LinkedEntity[]): string | null {
  if (!entities?.length) return null;
  return entities[0]?.name ?? null;
}
