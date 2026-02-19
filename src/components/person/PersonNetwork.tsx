/**
 * PersonNetwork — "The Network" chapter showing linked accounts + projects.
 * Inverted from StakeholderGallery: that shows people for an entity,
 * this shows entities for a person.
 */
import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { EntityPicker } from "@/components/ui/entity-picker";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

interface LinkedEntity {
  id: string;
  name: string;
  entityType: string;
}

interface PersonNetworkProps {
  entities?: LinkedEntity[];
  onLink?: (entityId: string) => Promise<void> | void;
  onUnlink?: (entityId: string) => Promise<void> | void;
  sectionId?: string;
  chapterTitle?: string;
}

const entityColor: Record<string, string> = {
  account: "var(--color-spice-turmeric)",
  project: "var(--color-garden-olive)",
};

export function PersonNetwork({
  entities,
  onLink,
  onUnlink,
  sectionId = "the-network",
  chapterTitle = "The Network",
}: PersonNetworkProps) {
  // Optimistic local state — updates immediately, syncs in background
  const [localEntities, setLocalEntities] = useState<LinkedEntity[]>(entities ?? []);
  useEffect(() => { setLocalEntities(entities ?? []); }, [entities]);

  const handleLink = useCallback(async (entityId: string, entityName?: string, entityType?: string) => {
    if (!entityId) return;
    // Optimistic add
    const newEntity: LinkedEntity = {
      id: entityId,
      name: entityName ?? entityId,
      entityType: entityType ?? "account",
    };
    setLocalEntities((prev) => {
      if (prev.some((e) => e.id === entityId)) return prev;
      return [...prev, newEntity];
    });
    // Background persist — no await, no reload
    onLink?.(entityId);
  }, [onLink]);

  const handleUnlink = useCallback(async (entityId: string) => {
    // Optimistic remove
    setLocalEntities((prev) => prev.filter((e) => e.id !== entityId));
    // Background persist
    onUnlink?.(entityId);
  }, [onUnlink]);

  const accounts = localEntities.filter((e) => e.entityType === "account");
  const projects = localEntities.filter((e) => e.entityType === "project");

  return (
    <section id={sectionId} style={{ scrollMarginTop: 60 }}>
      <ChapterHeading title={chapterTitle} />

      {accounts.length === 0 && projects.length === 0 && !onLink && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            color: "var(--color-text-tertiary)",
            fontStyle: "italic",
            marginTop: 16,
          }}
        >
          No linked accounts or projects yet.
        </p>
      )}

      {/* Two-column grid for accounts and projects */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: accounts.length > 0 && projects.length > 0 ? "1fr 1fr" : "1fr",
          gap: 40,
          marginTop: 24,
        }}
      >
        {/* Accounts column */}
        {accounts.length > 0 && (
          <div>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--color-text-tertiary)",
                marginBottom: 16,
              }}
            >
              Accounts
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {accounts.map((e) => (
                <EntityRow key={e.id} entity={e} onUnlink={onUnlink ? handleUnlink : undefined} />
              ))}
            </div>
          </div>
        )}

        {/* Projects column */}
        {projects.length > 0 && (
          <div>
            <div
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 10,
                fontWeight: 500,
                textTransform: "uppercase",
                letterSpacing: "0.1em",
                color: "var(--color-text-tertiary)",
                marginBottom: 16,
              }}
            >
              Projects
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {projects.map((e) => (
                <EntityRow key={e.id} entity={e} onUnlink={onUnlink ? handleUnlink : undefined} />
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Entity picker for linking — add multiple without page reload */}
      {onLink && (
        <div style={{ marginTop: 24, maxWidth: 320 }}>
          <EntityPicker
            value={null}
            onChange={(entityId, entityName, entityType) => {
              if (entityId) handleLink(entityId, entityName, entityType);
            }}
            excludeIds={localEntities.map((e) => e.id)}
            placeholder="Link account or project…"
          />
        </div>
      )}
    </section>
  );
}

function EntityRow({
  entity,
  onUnlink,
}: {
  entity: LinkedEntity;
  onUnlink?: (entityId: string) => void;
}) {
  const route = entity.entityType === "project"
    ? "/projects/$projectId"
    : "/accounts/$accountId";
  const params = entity.entityType === "project"
    ? { projectId: entity.id }
    : { accountId: entity.id };
  const color = entityColor[entity.entityType] ?? "var(--color-text-tertiary)";

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "10px 0",
        borderBottom: "1px solid var(--color-rule-light)",
      }}
    >
      <Link
        to={route}
        params={params}
        style={{
          display: "flex",
          alignItems: "center",
          gap: 10,
          textDecoration: "none",
          color: "var(--color-text-primary)",
        }}
      >
        {/* Entity type dot */}
        <span
          style={{
            width: 8,
            height: 8,
            borderRadius: "50%",
            background: color,
            flexShrink: 0,
          }}
        />
        <span
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 15,
            fontWeight: 500,
          }}
        >
          {entity.name}
        </span>
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 9,
            fontWeight: 500,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: "var(--color-text-tertiary)",
          }}
        >
          {entity.entityType}
        </span>
      </Link>

      {onUnlink && (
        <button
          onClick={() => onUnlink(entity.id)}
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            fontFamily: "var(--font-mono)",
            fontSize: 9,
            color: "var(--color-text-tertiary)",
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            padding: "2px 6px",
          }}
        >
          Unlink
        </button>
      )}
    </div>
  );
}
