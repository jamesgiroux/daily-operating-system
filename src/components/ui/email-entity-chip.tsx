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
import { Building2, FolderKanban, User } from "lucide-react";
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

const entityColor: Record<string, string> = {
  account: "var(--color-spice-turmeric)",
  project: "var(--color-garden-olive)",
  person: "var(--color-sky-larkspur)",
};

const entityBg: Record<string, string> = {
  account: "rgba(201, 162, 39, 0.08)",
  project: "rgba(107, 124, 82, 0.08)",
  person: "rgba(95, 130, 173, 0.08)",
};

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

  const color = entityColor[entityType ?? "account"] ?? "var(--color-text-tertiary)";
  const bg = entityBg[entityType ?? "account"] ?? "rgba(30, 37, 48, 0.04)";
  const Icon = entityType === "project"
    ? FolderKanban
    : entityType === "person"
      ? User
      : Building2;

  return (
    <span
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: 3,
        fontFamily: "var(--font-sans)",
        fontSize: 11,
        fontWeight: 400,
        color,
        background: bg,
        padding: "1px 7px",
        borderRadius: 3,
        lineHeight: 1.3,
        cursor: editable ? "pointer" : "default",
        transition: "background 0.15s ease",
      }}
      onClick={editable ? (e) => {
        e.stopPropagation();
        e.preventDefault();
        setEditing(true);
      } : undefined}
      title={editable ? "Click to change" : undefined}
    >
      <Icon style={{ width: 10, height: 10, opacity: 0.7, flexShrink: 0 }} />
      {entityName}
    </span>
  );
}
