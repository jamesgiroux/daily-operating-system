import type { LinkedEntity } from "@/types";

/**
 * DOS-258: Returns true when a LinkedEntity carries the new `role` field from
 * the deterministic link engine and that role is 'primary'.
 */
function isRolePrimary(e: LinkedEntity): boolean {
  return e.role === "primary";
}

/**
 * DOS-258: Returns true when a LinkedEntity is an auto-suggested (muted) chip.
 * Handles both the new `role` field (preferred) and the legacy `suggested` flag.
 */
export function isAutoSuggested(e: LinkedEntity): boolean {
  if (e.role !== undefined) return e.role === "auto_suggested";
  // Legacy fallback: suggested flag from DOS-74 schema.
  return e.suggested === true;
}

/**
 * DOS-258 / DOS-74: Select the primary entity from a meeting's linked entities.
 *
 * Preference order:
 *   1. DOS-258: entity with role === 'primary'
 *   2. DOS-74: entity with isPrimary === true
 *   3. Highest-confidence non-suggested entity
 *   4. First entity in the list (legacy fallback — backend pre-sorts
 *      by `is_primary DESC, confidence DESC`, so `[0]` is usually correct)
 *
 * Returns `null` if the list is empty or contains only suggestions.
 */
export function getPrimaryEntity(
  entities?: LinkedEntity[],
): LinkedEntity | null {
  if (!entities?.length) return null;

  // DOS-258: new role-based primary wins first.
  const rolePrimary = entities.find(isRolePrimary);
  if (rolePrimary) return rolePrimary;

  // DOS-74 legacy: explicit isPrimary flag.
  const explicitPrimary = entities.find((e) => e.isPrimary === true);
  if (explicitPrimary) return explicitPrimary;

  // Filter out auto-suggested entries before fallback selection.
  const nonSuggested = entities.filter((e) => !isAutoSuggested(e));
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

/**
 * DOS-258 / DOS-74: Split entities into (primary, suggestions).
 *
 * "suggestions" are entities that are NOT the primary. Under the new model
 * this is role === 'auto_suggested' or 'related'; under the legacy model it
 * is suggested === true or isPrimary === false.
 */
export function splitPrimaryAndSuggestions(
  entities?: LinkedEntity[],
): { primary: LinkedEntity | null; suggestions: LinkedEntity[] } {
  const primary = getPrimaryEntity(entities);
  const suggestions = (entities ?? []).filter((e) => {
    if (e.id === primary?.id) return false;
    // DOS-258: only surface auto_suggested as the muted/dim chip set.
    if (e.role !== undefined) return e.role === "auto_suggested" || e.role === "related";
    // Legacy fallback.
    return e.suggested === true || e.isPrimary === false;
  });
  return { primary, suggestions };
}

/**
 * DOS-258: Returns true when the P5 title-only banner should be shown for an
 * entity. P5 is the rule that matches an entity by meeting title keyword when
 * no attendee-domain or calendar-identity signal is present.
 */
export function isTitleOnlyPrimary(entity: LinkedEntity | null | undefined): boolean {
  return entity?.appliedRule === "P5";
}
