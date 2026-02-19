import { useState, useMemo, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Shared keyword display + removal component for entity detail pages (I352).
 *
 * Extracted from duplicate patterns in AccountDetailEditorial and
 * ProjectDetailEditorial. Supports optimistic removal with rollback on error.
 */

interface EntityKeywordsProps {
  entityId: string | undefined;
  entityType: "account" | "project";
  keywordsJson: string | undefined | null;
}

export function EntityKeywords({
  entityId,
  entityType,
  keywordsJson,
}: EntityKeywordsProps) {
  const [removedKeywords, setRemovedKeywords] = useState<Set<string>>(
    new Set(),
  );

  const parsedKeywords = useMemo(() => {
    if (!keywordsJson) return [];
    try {
      const arr = JSON.parse(keywordsJson);
      return Array.isArray(arr)
        ? (arr as string[]).filter((k) => !removedKeywords.has(k))
        : [];
    } catch {
      return [];
    }
  }, [keywordsJson, removedKeywords]);

  const handleRemoveKeyword = useCallback(
    async (keyword: string) => {
      if (!entityId) return;
      setRemovedKeywords((prev) => new Set(prev).add(keyword));
      try {
        const command =
          entityType === "account"
            ? "remove_account_keyword"
            : "remove_project_keyword";
        await invoke(command, {
          [entityType === "account" ? "accountId" : "projectId"]: entityId,
          keyword,
        });
      } catch (e) {
        console.error("Failed to remove keyword:", e);
        setRemovedKeywords((prev) => {
          const next = new Set(prev);
          next.delete(keyword);
          return next;
        });
      }
    },
    [entityId, entityType],
  );

  if (parsedKeywords.length === 0) return null;

  return (
    <div className="editorial-reveal" style={{ padding: "12px 0 0" }}>
      <div
        style={{
          display: "flex",
          alignItems: "baseline",
          gap: 8,
          marginBottom: 8,
        }}
      >
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
            color: "var(--color-text-tertiary)",
          }}
        >
          Resolution Keywords
        </span>
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            fontStyle: "italic",
          }}
        >
          (auto-extracted)
        </span>
      </div>
      <div style={{ display: "flex", flexWrap: "wrap", gap: 6 }}>
        {parsedKeywords.map((kw) => (
          <span
            key={kw}
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: 4,
              padding: "2px 10px",
              borderRadius: 12,
              background: "var(--color-paper-linen)",
              fontFamily: "var(--font-sans)",
              fontSize: 12,
              color: "var(--color-text-secondary)",
              lineHeight: "20px",
            }}
          >
            {kw}
            <button
              onClick={() => handleRemoveKeyword(kw)}
              aria-label={`Remove keyword ${kw}`}
              style={{
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
                lineHeight: 1,
                fontSize: 14,
                color: "var(--color-text-tertiary)",
                marginLeft: 2,
              }}
            >
              &times;
            </button>
          </span>
        ))}
      </div>
    </div>
  );
}
