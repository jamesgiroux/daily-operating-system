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
import { Building2, FolderKanban, User } from "lucide-react";
import { EntityPicker } from "./entity-picker";
import { Pill, type PillTone } from "./Pill";

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

function toneForEntityType(entityType?: string): PillTone {
  if (entityType === "project") return "olive";
  if (entityType === "person") return "larkspur";
  return "turmeric";
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
        style={{ display: "inline-flex", alignItems: "center" }}
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
          style={{ display: "inline-flex", alignItems: "center" }}
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

  const Icon = entityType === "project"
    ? FolderKanban
    : entityType === "person"
      ? User
      : Building2;

  return (
    <Pill
      tone={toneForEntityType(entityType)}
      size="compact"
      interactive={editable}
      onClick={editable ? (e) => {
        e.stopPropagation();
        e.preventDefault();
        setEditing(true);
      } : undefined}
      title={editable ? "Click to change" : undefined}
    >
      <Icon size={10} strokeWidth={2} aria-hidden="true" />
      {entityName}
    </Pill>
  );
}
