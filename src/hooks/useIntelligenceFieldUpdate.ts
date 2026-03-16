import { useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

/**
 * Shared hook for updating intelligence fields via Tauri invoke (I352).
 *
 * Text edits show locally via EditableText. Dismissals (empty string) trigger
 * `onDismiss` callback to refresh the parent data so removed items disappear.
 *
 * Returns `saveStatus` for wiring into the folio bar status text.
 */
export function useIntelligenceFieldUpdate(
  entityType: string,
  entityId: string | undefined,
  onDismiss?: () => void,
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
        // Dismiss (empty value) = item removed from intelligence JSON.
        // Trigger refresh so it disappears from the UI.
        if (!value.trim() && onDismiss) {
          onDismiss();
        }
      } catch (e) {
        console.error(`Failed to update ${fieldPath}:`, e);
        toast.error("Failed to save");
        setSaveStatus("idle");
      } finally {
        setUpdatingField(null);
      }
    },
    [entityType, entityId, onDismiss],
  );

  return { updateField, updatingField, saveStatus, setSaveStatus };
}
