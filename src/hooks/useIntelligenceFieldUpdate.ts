import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

/**
 * Shared hook for updating intelligence fields via Tauri invoke (I352).
 *
 * Text edits show locally via EditableText. Successful saves trigger a parent
 * refresh callback so authoritative detail state catches up after async writes.
 *
 * Returns `saveStatus` for wiring into the folio bar status text.
 */
export function useIntelligenceFieldUpdate(
  entityType: string,
  entityId: string | undefined,
  onSaved?: () => void,
) {
  const [updatingField, setUpdatingField] = useState<string | null>(null);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved">("idle");
  const savedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const updateField = useCallback(
    async (fieldPath: string, value: string) => {
      if (!entityId) return;
      setUpdatingField(fieldPath);
      setSaveStatus("saving");

      // Clear any pending "saved" timeout
      if (savedTimerRef.current) {
        clearTimeout(savedTimerRef.current);
        savedTimerRef.current = null;
      }

      try {
        await invoke("update_intelligence_field", {
          entityId,
          entityType,
          fieldPath,
          value,
        });
        setSaveStatus("saved");
        savedTimerRef.current = setTimeout(() => {
          setSaveStatus("idle");
          savedTimerRef.current = null;
        }, 2000);
        if (onSaved) {
          void Promise.resolve(onSaved());
        }
      } catch (e) {
        console.error(`Failed to update ${fieldPath}:`, e);
        toast.error("Failed to save");
        setSaveStatus("idle");
      } finally {
        setUpdatingField(null);
      }
    },
    [entityType, entityId, onSaved],
  );

  return { updateField, updatingField, saveStatus, setSaveStatus };
}
