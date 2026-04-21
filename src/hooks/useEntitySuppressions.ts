import { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface SuppressionTombstone {
  fieldKey: string;
  itemKey: string | null;
}

function makeSuppressionKey(field: string, itemKey?: string | null): string {
  return `${field}\u0000${itemKey ?? ""}`;
}

export function useEntitySuppressions(entityId: string | null | undefined) {
  const [suppressed, setSuppressed] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    if (!entityId) {
      setSuppressed(new Set());
      return;
    }
    invoke<SuppressionTombstone[]>("get_entity_suppressions", { entityId })
      .then((rows) => {
        setSuppressed(
          new Set(
            rows.map((row) => makeSuppressionKey(row.fieldKey, row.itemKey)),
          ),
        );
      })
      .catch((e) => {
        console.error("Failed to load entity suppressions:", e);
        setSuppressed(new Set());
      });
  }, [entityId]);

  const isSuppressed = useCallback(
    (field: string, itemKey?: string | null) => {
      return suppressed.has(makeSuppressionKey(field, itemKey));
    },
    [suppressed],
  );

  const markSuppressed = useCallback((field: string, itemKey?: string | null) => {
    setSuppressed((prev) => {
      const next = new Set(prev);
      next.add(makeSuppressionKey(field, itemKey));
      return next;
    });
  }, []);

  return useMemo(
    () => ({
      isSuppressed,
      markSuppressed,
    }),
    [isSuppressed, markSuppressed],
  );
}
