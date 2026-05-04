/**
 * EmailEntityChip — Entity badge for emails.
 *
 * Compact chip showing the linked entity with editorial color coding:
 * turmeric (account), olive (project), larkspur (person).
 *
 * Two modes:
 * - Read-only (default): static label, safe inside parent <Link> elements
 * - Editable: click to reassign via EntityPicker, for use on the emails page
 */

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { EntityChip } from "./EntityChip";
import { EntityPicker } from "./entity-picker";

interface EmailEntityChipProps {
  entityType?: string;
  entityName?: string;
  /** Enable click-to-edit via EntityPicker. Only use where there's no parent <Link>. */
  editable?: boolean;
  /** Required when editable=true */
  emailId?: string;
  /** Called after entity is changed so parent can refetch */
  onEntityChanged?: () => void;
}

export function EmailEntityChip({
  entityType,
  entityName,
  editable = false,
  emailId,
  onEntityChanged,
}: EmailEntityChipProps) {
  const [editing, setEditing] = useState(false);

  const handleChange = useCallback(
    async (newEntityId: string | null, _name?: string, newEntityType?: "account" | "project") => {
      if (!newEntityId || !emailId) return;
      try {
        await invoke("update_email_entity", {
          emailId,
          entityId: newEntityId,
          entityType: newEntityType ?? "account",
        });
        onEntityChanged?.();
      } catch (err) {
        console.error("Failed to update email entity:", err);
        toast.error("Failed to update assignment");
      }
      setEditing(false);
    },
    [emailId, onEntityChanged],
  );

  // Editable mode: show picker inline
  if (editable && editing) {
    return (
      <span
        className="inline-flex items-center"
        onClick={(e) => e.stopPropagation()}
      >
        <EntityPicker
          value={null}
          onChange={handleChange}
          placeholder="Link to account…"
          className="h-5 text-[10px] px-1"
        />
      </span>
    );
  }

  if (!entityName) {
    // No entity — show a subtle picker trigger if editable
    if (editable && emailId) {
      return (
        <span
          className="inline-flex items-center"
          onClick={(e) => e.stopPropagation()}
        >
          <EntityPicker
            value={null}
            onChange={handleChange}
            placeholder="Link…"
            className="h-5 text-[10px] px-1"
          />
        </span>
      );
    }
    return null;
  }

  return (
    <EntityChip
      entityType={entityType}
      entityName={entityName}
      compact
      editable={editable}
      onEdit={editable ? (e) => {
        e.stopPropagation();
        e.preventDefault();
        setEditing(true);
      } : undefined}
      title={editable ? "Click to change" : undefined}
    />
  );
}
