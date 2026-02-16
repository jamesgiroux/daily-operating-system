/**
 * MeetingEntityChips — Inline entity assignment for meetings.
 *
 * Shows linked entities as removable chips + an EntityPicker for adding more.
 * Supports multiple entities per meeting (M2M junction table).
 * Calls add_meeting_entity / remove_meeting_entity Tauri commands.
 *
 * Styled to match the editorial design system — mono labels, spice/garden
 * color coding, no shadcn card chrome.
 */

import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { X, Building2, FolderKanban } from "lucide-react";
import { EntityPicker } from "./entity-picker";
import type { LinkedEntity } from "@/types";

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

const entityColor: Record<string, string> = {
  account: "var(--color-spice-turmeric)",
  project: "var(--color-garden-olive)",
};

const entityBg: Record<string, string> = {
  account: "rgba(201, 162, 39, 0.08)",
  project: "rgba(107, 124, 82, 0.08)",
};

export function MeetingEntityChips({
  meetingId,
  meetingTitle,
  meetingStartTime,
  meetingType,
  linkedEntities,
  onEntitiesChanged,
  compact = false,
}: MeetingEntityChipsProps) {
  const handleAdd = useCallback(
    async (entityId: string | null, _entityName?: string) => {
      if (!entityId) return;
      try {
        await invoke("add_meeting_entity", {
          meetingId,
          entityId,
          entityType: "account",
          meetingTitle,
          startTime: meetingStartTime,
          meetingTypeStr: meetingType,
        });
        onEntitiesChanged?.();
      } catch (err) {
        console.error("Failed to add meeting entity:", err);
      }
    },
    [meetingId, meetingTitle, meetingStartTime, meetingType, onEntitiesChanged],
  );

  const handleRemove = useCallback(
    async (entityId: string) => {
      try {
        await invoke("remove_meeting_entity", {
          meetingId,
          entityId,
        });
        onEntitiesChanged?.();
      } catch (err) {
        console.error("Failed to remove meeting entity:", err);
      }
    },
    [meetingId, onEntitiesChanged],
  );

  const fontSize = compact ? 11 : 12;
  const chipPadding = compact ? "2px 8px" : "3px 10px";
  const iconSize = compact ? 10 : 12;

  return (
    <div
      style={{
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: compact ? 6 : 8,
      }}
      onClick={(e) => e.stopPropagation()}
    >
      {linkedEntities.map((entity) => {
        const color = entityColor[entity.entityType] ?? "var(--color-text-tertiary)";
        const bg = entityBg[entity.entityType] ?? "rgba(30, 37, 48, 0.04)";
        const Icon = entity.entityType === "project" ? FolderKanban : Building2;
        const linkTo = entity.entityType === "project"
          ? "/projects/$projectId"
          : "/accounts/$accountId";
        const linkParams = entity.entityType === "project"
          ? { projectId: entity.id }
          : { accountId: entity.id };

        return (
          <span
            key={entity.id}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              fontFamily: "var(--font-sans)",
              fontSize,
              fontWeight: 400,
              color,
              background: bg,
              padding: chipPadding,
              borderRadius: 4,
              lineHeight: 1.3,
              transition: "background 0.15s ease",
            }}
          >
            <Icon style={{ width: iconSize, height: iconSize, opacity: 0.7, flexShrink: 0 }} />
            <Link
              to={linkTo}
              params={linkParams as any}
              style={{
                color: "inherit",
                textDecoration: "none",
              }}
            >
              {entity.name}
            </Link>
            <button
              onClick={(e) => {
                e.stopPropagation();
                e.preventDefault();
                handleRemove(entity.id);
              }}
              style={{
                display: "inline-flex",
                alignItems: "center",
                justifyContent: "center",
                width: compact ? 14 : 16,
                height: compact ? 14 : 16,
                padding: 0,
                border: "none",
                background: "transparent",
                cursor: "pointer",
                color: "inherit",
                opacity: 0.4,
                transition: "opacity 0.15s ease",
                borderRadius: 2,
                marginLeft: 2,
              }}
              onMouseEnter={(e) => { e.currentTarget.style.opacity = "0.8"; }}
              onMouseLeave={(e) => { e.currentTarget.style.opacity = "0.4"; }}
            >
              <X style={{ width: compact ? 10 : 12, height: compact ? 10 : 12 }} />
            </button>
          </span>
        );
      })}

      {/* Always-available picker for adding more entities */}
      <EntityPicker
        value={null}
        onChange={handleAdd}
        placeholder={linkedEntities.length === 0 ? "Link entity..." : "+"}
        className={compact ? "h-6 text-[10px] px-1.5" : undefined}
      />
    </div>
  );
}
