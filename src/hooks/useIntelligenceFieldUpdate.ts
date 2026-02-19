import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Shared hook for updating intelligence fields via Tauri invoke (I352).
 *
 * Extracted from duplicate patterns in AccountDetailEditorial, ProjectDetailEditorial,
 * and PersonDetailEditorial. No reload needed — EditableText already shows the
 * edited value locally.
 */
export function useIntelligenceFieldUpdate(
  entityType: string,
  entityId: string | undefined,
) {
  const [updatingField, setUpdatingField] = useState<string | null>(null);

  const updateField = useCallback(
    async (fieldPath: string, value: string) => {
      if (!entityId) return;
      setUpdatingField(fieldPath);
      try {
        await invoke("update_intelligence_field", {
          entityId,
          entityType,
          fieldPath,
          value,
        });
      } catch (e) {
        console.error(`Failed to update ${fieldPath}:`, e);
      } finally {
        setUpdatingField(null);
      }
    },
    [entityType, entityId],
  );

  return { updateField, updatingField };
}
