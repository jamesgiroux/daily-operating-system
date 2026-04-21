/**
 * EntityLinkPicker — DOS-258 Lane G
 *
 * "Which account is this about?" picker for the P9 ambiguous case.
 *
 * Shows when:
 *   outcome.primary === null &&
 *   outcome.related.length > 0 &&
 *   outcome.tier === 'entity'
 *
 * Renders a list of related entity chips. Clicking one calls
 * `invoke('manual_set_primary', ...)` and optimistically updates local state.
 *
 * Also renders the P5 title-only banner:
 *   Shows "from title · undo" when a primary entity's appliedRule === 'P5'.
 *
 * NOTE: `manual_set_primary` is a DOS-258 backend command that will be wired
 * in a later Lane. Until it lands, the button is present and fires the invoke
 * but the backend will return an error (gracefully caught and toasted).
 * TODO(Lane-G): remove the try/catch fallback once Lane H/I ships the command.
 */

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Building2 } from "lucide-react";
import type { LinkOutcome, OwnerType, LinkedEntity } from "@/types";
import css from "./EntityLinkPicker.module.css";

// ─── P9 Picker ────────────────────────────────────────────────────────────────

interface EntityLinkPickerProps {
  outcome: LinkOutcome;
  /** Account names keyed by entityId — used to label chips. */
  entityNames?: Record<string, string>;
  /** Called after the user picks a primary so the parent can refetch. */
  onPrimarySet?: () => void;
}

/**
 * Renders the "Which account is this about?" picker when the link engine
 * found multiple candidates but could not elect a primary.
 *
 * Hidden entirely when the show condition is not met.
 */
export function EntityLinkPicker({
  outcome,
  entityNames = {},
  onPrimarySet,
}: EntityLinkPickerProps) {
  const { primary, related, tier, ownerType, ownerId } = outcome;

  const shouldShow =
    primary === null && related.length > 0 && tier === "entity";

  const [inFlight, setInFlight] = useState<string | null>(null);

  const handleSetPrimary = useCallback(
    async (entityId: string, entityType: string) => {
      setInFlight(entityId);
      try {
        // TODO(Lane-G): `manual_set_primary` ships in a later Lane.
        // Until then this invoke will fail; the error is caught and toasted.
        await invoke("set_entity_link_primary", {
          ownerType,
          ownerId,
          entityId,
          entityType,
        });
        onPrimarySet?.();
      } catch (err) {
        console.error("manual_set_primary failed:", err);
        toast.error("Could not set primary — backend not yet available");
      } finally {
        setInFlight(null);
      }
    },
    [ownerType, ownerId, onPrimarySet],
  );

  if (!shouldShow) return null;

  return (
    <div className={css.picker}>
      <p className={css.heading}>Which account is this about?</p>
      <div className={css.chipList}>
        {related.filter(({ entityType }) => entityType === "account").map(({ entityId, entityType }) => {
          const name = entityNames[entityId] ?? entityId;
          const busy = inFlight === entityId;
          return (
            <button
              key={entityId}
              type="button"
              className={css.chipBtn}
              disabled={busy}
              onClick={() => void handleSetPrimary(entityId, entityType)}
              title={`Set ${name} as the primary account for this ${ownerType}`}
            >
              <Building2 style={{ width: 11, height: 11, opacity: 0.7, flexShrink: 0 }} />
              {name}
              <span className={css.chipLabel}>Set as primary</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}

// ─── P5 Title-Only Banner ─────────────────────────────────────────────────────

interface TitleOnlyBannerProps {
  /** The primary entity that was matched by title keyword (P5 rule). */
  entity: LinkedEntity;
  ownerType: OwnerType;
  ownerId: string;
  /** Called after undo so the parent can refetch. */
  onUndo?: () => void;
}

/**
 * Renders a "from title · undo" banner below the primary chip when the
 * P5 rule fired (title-only match with no attendee-domain confirmation).
 *
 * The undo action dismisses the current primary link so the user can pick
 * the correct account via EntityLinkPicker or the EntityPicker.
 */
export function TitleOnlyBanner({
  entity,
  ownerType,
  ownerId,
  onUndo,
}: TitleOnlyBannerProps) {
  const [busy, setBusy] = useState(false);

  const handleUndo = useCallback(async () => {
    setBusy(true);
    try {
      // TODO(Lane-G): `dismiss_meeting_entity` already exists (DOS-240).
      // For email/thread, a parallel command will be needed. For now wire
      // meeting surface only; other surfaces fall through gracefully.
      if (ownerType === "meeting") {
        await invoke("dismiss_meeting_entity", {
          meetingId: ownerId,
          entityId: entity.id,
          entityType: entity.entityType,
        });
      } else {
        toast.info("Undo not yet available for email links");
      }
      onUndo?.();
    } catch (err) {
      console.error("TitleOnlyBanner undo failed:", err);
      toast.error("Could not undo link");
    } finally {
      setBusy(false);
    }
  }, [ownerType, ownerId, entity, onUndo]);

  return (
    <div className={css.titleOnlyBanner}>
      <span>from title</span>
      <button
        type="button"
        className={css.undoBtn}
        disabled={busy}
        onClick={() => void handleUndo()}
      >
        undo
      </button>
    </div>
  );
}
