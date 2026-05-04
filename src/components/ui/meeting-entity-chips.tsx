/**
 * MeetingEntityChips — Inline entity assignment for meetings.
 *
 * Shows linked entities as removable chips + an EntityPicker for adding more.
 * Supports multiple entities per meeting (M2M junction table).
 * Calls add_meeting_entity / dismiss_meeting_entity Tauri commands.
 *
 * chip X invokes `dismiss_meeting_entity` (not the legacy
 * `remove_meeting_entity`) so that the dismissal is persisted into
 * `meeting_entity_dismissals` and the entity cannot silently re-link on
 * the next calendar-sync or resolver sweep.
 *
 * Updated to use the new deterministic link model:
 *   - role === 'auto_suggested' renders as a muted dashed chip
 *   - role === 'primary' with appliedRule === 'P5' shows the title-only banner
 *   - primary === null + related.length > 0 shows the "Which account?" picker
 *
 * Uses optimistic local state so chips appear/disappear instantly without
 * triggering a full dashboard reload.
 *
 * Styled to match the editorial design system — mono labels, spice/garden
 * color coding, no shadcn card chrome.
 */

import { useState, useCallback, useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { toast } from "sonner";
import { EntityChip } from "./EntityChip";
import { EntityPicker } from "./entity-picker";
import { EntityLinkPicker, TitleOnlyBanner } from "@/components/entity/EntityLinkPicker";
import { getPrimaryEntity, isTitleOnlyPrimary } from "@/lib/entity-helpers";
import type { LinkedEntity, LinkOutcome } from "@/types";

interface MeetingEntityChipsProps {
  meetingId: string;
  meetingTitle: string;
  meetingStartTime: string;
  meetingType: string;
  linkedEntities: LinkedEntity[];
  /** Callback after entity list changes (parent should refetch) */
  onEntitiesChanged?: () => void;
  /** Compact mode for briefing expansion panels (smaller chips) */
  compact?: boolean;
}

export function MeetingEntityChips({
  meetingId,
  meetingTitle,
  meetingStartTime,
  meetingType,
  linkedEntities,
  onEntitiesChanged,
  compact = false,
}: MeetingEntityChipsProps) {
  // Optimistic local state — syncs from props but updates instantly on add/remove
  const [localEntities, setLocalEntities] = useState(linkedEntities);

  // Keep local state in sync when parent props change (e.g. after a full refresh)
  useEffect(() => {
    setLocalEntities(linkedEntities);
  }, [linkedEntities]);

  const handleAdd = useCallback(
    async (entityId: string | null, entityName?: string, pickerEntityType?: "account" | "project" | "person") => {
      if (!entityId) return;
      try {
        // Optimistic: add chip immediately
        const entityType = pickerEntityType ?? "account";
        setLocalEntities((prev) => {
          if (prev.some((e) => e.id === entityId)) return prev;
          return [...prev, { id: entityId, name: entityName ?? entityId, entityType }];
        });

        await invoke("add_meeting_entity", {
          meetingId,
          entityId,
          entityType,
          meetingTitle,
          startTime: meetingStartTime,
          meetingTypeStr: meetingType,
        });
        // Background sync — don't await, no visible reload
        onEntitiesChanged?.();
      } catch (err) {
        console.error("Failed to add meeting entity:", err);
        toast.error("Failed to link account");
        // Rollback on failure
        setLocalEntities((prev) => prev.filter((e) => e.id !== entityId));
      }
    },
    [meetingId, meetingTitle, meetingStartTime, meetingType, onEntitiesChanged],
  );

  const handleRemove = useCallback(
    async (entityId: string, entityName: string, entityType: "account" | "project" | "person") => {
      // Optimistic: remove chip immediately
      setLocalEntities((prev) => prev.filter((e) => e.id !== entityId));

      try {
        // use `dismiss_meeting_entity` so a dismissal row is
        // recorded in `meeting_entity_dismissals`. The legacy
        // `remove_meeting_entity` only unlinked + recorded feedback, which
        // let the background resolver silently re-link the same entity on
        // its next pass.
        await invoke("dismiss_meeting_entity", {
          meetingId,
          entityId,
          entityType,
        });
        // Background sync — don't await, no visible reload
        onEntitiesChanged?.();
      } catch (err) {
        console.error("Failed to remove meeting entity:", err);
        toast.error("Failed to unlink account");
        // Rollback: re-add the entity on failure with original name
        setLocalEntities((prev) => {
          if (prev.some((e) => e.id === entityId)) return prev;
          return [...prev, { id: entityId, name: entityName, entityType }];
        });
      }
    },
    [meetingId, onEntitiesChanged],
  );

  // Derive the P9 LinkOutcome so EntityLinkPicker can decide whether
  // to show. We use the new `role` field when present; otherwise fall back to
  // the legacy `isPrimary`/`suggested` flags so old data still renders.
  const primaryEntity = useMemo(() => getPrimaryEntity(localEntities), [localEntities]);
  const showTitleOnlyBanner = isTitleOnlyPrimary(primaryEntity);

  const linkOutcome = useMemo<LinkOutcome>(() => {
    const hasDos258Roles = localEntities.some((e) => e.role !== undefined);

    if (hasDos258Roles) {
      // New model: primary and related come directly from role field.
      const primary = localEntities.find((e) => e.role === "primary") ?? null;
      const related = localEntities
        .filter((e) => e.role === "related" || e.role === "auto_suggested")
        .map((e) => ({ entityId: e.id, entityType: e.entityType }));

      return {
        ownerType: "meeting",
        ownerId: meetingId,
        primary: primary ? { entityId: primary.id, entityType: primary.entityType } : null,
        related,
        tier: "entity",
        appliedRule: primary?.appliedRule ?? null,
      };
    }

    // Legacy model: construct outcome from isPrimary/suggested flags.
    // primary === null when all entities are suggestions (P9 equivalent).
    const legacyPrimary = primaryEntity;
    const related = localEntities
      .filter((e) => e.id !== legacyPrimary?.id && (e.suggested === true || e.isPrimary === false))
      .map((e) => ({ entityId: e.id, entityType: e.entityType }));

    return {
      ownerType: "meeting",
      ownerId: meetingId,
      primary: legacyPrimary ? { entityId: legacyPrimary.id, entityType: legacyPrimary.entityType } : null,
      related,
      // Only show the picker when we have multiple candidates and no primary.
      tier: legacyPrimary === null && related.length > 0 ? "entity" : "minimal",
      appliedRule: null,
    };
  }, [localEntities, meetingId, primaryEntity]);

  // Build entityId -> name lookup for the picker chips.
  const entityNames = useMemo(
    () => Object.fromEntries(localEntities.map((e) => [e.id, e.name])),
    [localEntities],
  );

  return (
    <div
      className={compact ? "flex flex-col gap-1.5" : "flex flex-col gap-2"}
      onClick={(e) => e.stopPropagation()}
    >
      {/* Chips row */}
      <div
        className={compact ? "flex flex-wrap items-center gap-1.5" : "flex flex-wrap items-center gap-2"}
      >
        {localEntities.map((entity) => {
          const linkTo = entity.entityType === "project"
            ? "/projects/$projectId"
            : entity.entityType === "person"
              ? "/people/$personId"
              : "/accounts/$accountId";
          const linkParams = entity.entityType === "project"
            ? { projectId: entity.id }
            : entity.entityType === "person"
              ? { personId: entity.id }
              : { accountId: entity.id };

          // Render auto_suggested chips muted with a dashed border.
          // Falls back to the legacy `suggested` flag for backwards compatibility
          // with data that hasn't been migrated through the new link engine yet.
          const isAutoSuggested =
            entity.role === "auto_suggested" || (entity.role === undefined && entity.suggested === true);
          return (
            <EntityChip
              key={entity.id}
              title={isAutoSuggested ? "Auto-suggested — not confirmed" : undefined}
              entityType={entity.entityType}
              entityName={(
                <Link
                  to={linkTo}
                  params={linkParams as any}
                >
                  {entity.name}
                </Link>
              )}
              removable
              editable
              compact={compact}
              data-suggested={isAutoSuggested ? "true" : undefined}
              aria-label={`Remove ${entity.name}`}
              onRemove={() => handleRemove(entity.id, entity.name, entity.entityType)}
            />
          );
        })}

        {/* Always-available picker for adding more entities */}
        <EntityPicker
          value={null}
          onChange={handleAdd}
          placeholder={localEntities.length === 0 ? "Link to account or project..." : "+"}
          className={compact ? "h-6 text-[10px] px-1.5" : undefined}
        />
      </div>

      {/*  P5: "from title · undo" banner when the primary was matched
          by title keyword only (no attendee-domain or calendar-identity). */}
      {showTitleOnlyBanner && primaryEntity && (
        <TitleOnlyBanner
          entity={primaryEntity}
          ownerType="meeting"
          ownerId={meetingId}
          onUndo={onEntitiesChanged}
        />
      )}

      {/*  P9: "Which account is this about?" picker when the link
          engine found multiple candidates but could not elect a primary. */}
      <EntityLinkPicker
        outcome={linkOutcome}
        entityNames={entityNames}
        onPrimarySet={onEntitiesChanged}
      />
    </div>
  );
}
