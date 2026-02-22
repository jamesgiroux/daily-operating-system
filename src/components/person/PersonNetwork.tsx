/**
 * PersonNetwork — "Their Orbit" chapter showing linked accounts + projects.
 * Inverted from StakeholderGallery: that shows people for an entity,
 * this shows entities for a person.
 */
import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { EntityPicker } from "@/components/ui/entity-picker";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import s from "./PersonNetwork.module.css";

interface LinkedEntity {
  id: string;
  name: string;
  entityType: string;
}

interface PersonNetworkProps {
  entities?: LinkedEntity[];
  onLink?: (entityId: string) => Promise<void> | void;
  onUnlink?: (entityId: string) => Promise<void> | void;
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
  chapterTitle = "Their Orbit",
}: PersonNetworkProps) {
  const [localEntities, setLocalEntities] = useState<LinkedEntity[]>(entities ?? []);
  useEffect(() => { setLocalEntities(entities ?? []); }, [entities]);

  const handleLink = useCallback(async (entityId: string, entityName?: string, entityType?: string) => {
    if (!entityId) return;
    const newEntity: LinkedEntity = {
      id: entityId,
      name: entityName ?? entityId,
      entityType: entityType ?? "account",
    };
    setLocalEntities((prev) => {
      if (prev.some((e) => e.id === entityId)) return prev;
      return [...prev, newEntity];
    });
    onLink?.(entityId);
  }, [onLink]);

  const handleUnlink = useCallback(async (entityId: string) => {
    setLocalEntities((prev) => prev.filter((e) => e.id !== entityId));
    onUnlink?.(entityId);
  }, [onUnlink]);

  const accounts = localEntities.filter((e) => e.entityType === "account");
  const projects = localEntities.filter((e) => e.entityType === "project");
  const hasBoth = accounts.length > 0 && projects.length > 0;

  return (
    <section className={s.section}>
      <ChapterHeading title={chapterTitle} />

      {accounts.length === 0 && projects.length === 0 && !onLink && (
        <p className={s.emptyState}>No linked accounts or projects yet.</p>
      )}

      <div className={hasBoth ? s.gridTwoCol : s.gridOneCol}>
        {accounts.length > 0 && (
          <div>
            <div className={s.columnLabel}>Accounts</div>
            <div className={s.columnList}>
              {accounts.map((e) => (
                <EntityRow key={e.id} entity={e} onUnlink={onUnlink ? handleUnlink : undefined} />
              ))}
            </div>
          </div>
        )}

        {projects.length > 0 && (
          <div>
            <div className={s.columnLabel}>Projects</div>
            <div className={s.columnList}>
              {projects.map((e) => (
                <EntityRow key={e.id} entity={e} onUnlink={onUnlink ? handleUnlink : undefined} />
              ))}
            </div>
          </div>
        )}
      </div>

      {onLink && (
        <div className={s.pickerWrap}>
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
    <div className={s.entityRow}>
      <Link to={route} params={params} className={s.entityLink}>
        <span className={s.entityDot} style={{ background: color }} />
        <span className={s.entityName}>{entity.name}</span>
        <span className={s.entityType}>{entity.entityType}</span>
      </Link>

      {onUnlink && (
        <button onClick={() => onUnlink(entity.id)} className={s.unlinkBtn}>
          Unlink
        </button>
      )}
    </div>
  );
}
