import type { LinkedEntity } from "@/types";

/**
 * DOS-74: Select the primary entity from a meeting's linked entities.
 *
 * Preference order:
 *   1. An entity explicitly flagged `isPrimary`
 *   2. The highest-confidence non-suggested entity
 *   3. The first entity in the list (legacy fallback — backend pre-sorts
 *      by `is_primary DESC, confidence DESC`, so `[0]` is usually correct)
 *
 * Returns `null` if the list is empty or contains only suggestions.
 */
export function getPrimaryEntity(
  entities?: LinkedEntity[],
): LinkedEntity | null {
  if (!entities?.length) return null;
  const explicitPrimary = entities.find((e) => e.isPrimary === true);
  if (explicitPrimary) return explicitPrimary;
  const nonSuggested = entities.filter((e) => !e.suggested);
  if (nonSuggested.length === 0) return null;
  // Sort by confidence descending; undefined confidence sorts last.
  const sorted = [...nonSuggested].sort((a, b) => {
    const ca = a.confidence ?? 0;
    const cb = b.confidence ?? 0;
    return cb - ca;
  });
  return sorted[0] ?? null;
}

/** Return the name of the primary linked entity, or null. */
export function getPrimaryEntityName(entities?: LinkedEntity[]): string | null {
  return getPrimaryEntity(entities)?.name ?? null;
}

/** Format entity byline: "{Name} · {TypeLabel}" using the primary entity. */
export function formatEntityByline(entities?: LinkedEntity[]): string | null {
  const entity = getPrimaryEntity(entities);
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
  return getPrimaryEntity(entities)?.entityType ?? null;
}

/** DOS-74: Split entities into (primary, suggestions) for dual-render UIs. */
export function splitPrimaryAndSuggestions(
  entities?: LinkedEntity[],
): { primary: LinkedEntity | null; suggestions: LinkedEntity[] } {
  const primary = getPrimaryEntity(entities);
  const suggestions = (entities ?? []).filter(
    (e) => e.id !== primary?.id && (e.suggested === true || e.isPrimary === false),
  );
  return { primary, suggestions };
}
