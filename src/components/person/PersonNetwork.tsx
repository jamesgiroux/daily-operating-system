/**
 * PersonNetwork — "Their Orbit" chapter showing linked accounts + projects.
 * Inverted from StakeholderGallery: that shows people for an entity,
 * this shows entities for a person.
 *
 * Also shows stakeholder role badges per account, with inline add/remove.
 */
import { useState, useEffect, useCallback, useRef } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { EntityPicker } from "@/components/ui/entity-picker";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { PersonAccountRole } from "@/types";
import s from "./PersonNetwork.module.css";

interface LinkedEntity {
  id: string;
  name: string;
  entityType: string;
}

interface PersonNetworkProps {
  entities?: LinkedEntity[];
  personId?: string;
  onLink?: (entityId: string) => Promise<void> | void;
  onUnlink?: (entityId: string) => Promise<void> | void;
  chapterTitle?: string;
}

const entityColor: Record<string, string> = {
  account: "var(--color-spice-turmeric)",
  project: "var(--color-garden-olive)",
};

/** Stakeholder role definitions — same as StakeholderGallery. */
const STAKEHOLDER_ROLES = [
  { stored: "champion", label: "Champion", bg: "var(--color-spice-turmeric-12)", fg: "var(--color-spice-turmeric)" },
  { stored: "executive_sponsor", label: "Exec Sponsor", bg: "var(--color-garden-rosemary-14)", fg: "var(--color-garden-rosemary)" },
  { stored: "decision_maker", label: "Decision Maker", bg: "var(--color-garden-rosemary-14)", fg: "var(--color-garden-rosemary)" },
  { stored: "economic_buyer", label: "Economic Buyer", bg: "var(--color-garden-sage-14)", fg: "var(--color-garden-sage)" },
  { stored: "technical_buyer", label: "Technical Buyer", bg: "var(--color-garden-larkspur-14)", fg: "var(--color-garden-larkspur)" },
  { stored: "primary_user", label: "Primary User", bg: "var(--color-garden-larkspur-14)", fg: "var(--color-garden-larkspur)" },
  { stored: "technical_user", label: "Technical User", bg: "var(--color-garden-larkspur-14)", fg: "var(--color-garden-larkspur)" },
  { stored: "csm", label: "CSM", bg: "var(--color-text-tertiary-8)", fg: "var(--color-text-secondary)" },
  { stored: "implementation", label: "Implementation", bg: "var(--color-text-tertiary-8)", fg: "var(--color-text-secondary)" },
  { stored: "associated", label: "Associated", bg: "var(--color-text-tertiary-8)", fg: "var(--color-text-tertiary)" },
];

function getRoleConfig(stored: string) {
  return (
    STAKEHOLDER_ROLES.find((r) => r.stored === stored.toLowerCase()) ??
    STAKEHOLDER_ROLES[STAKEHOLDER_ROLES.length - 1]
  );
}

export function PersonNetwork({
  entities,
  personId,
  onLink,
  onUnlink,
  chapterTitle = "Their Orbit",
}: PersonNetworkProps) {
  const [localEntities, setLocalEntities] = useState<LinkedEntity[]>(entities ?? []);
  useEffect(() => { setLocalEntities(entities ?? []); }, [entities]);

  // Fetch stakeholder roles for this person across all accounts
  const [accountRoles, setAccountRoles] = useState<PersonAccountRole[]>([]);
  const loadRoles = useCallback(() => {
    if (!personId) return;
    invoke<PersonAccountRole[]>("get_person_stakeholder_roles", { personId })
      .then(setAccountRoles)
      .catch(() => setAccountRoles([]));
  }, [personId]);
  useEffect(() => { loadRoles(); }, [loadRoles, localEntities]);

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

  // Group roles by accountId
  const rolesByAccount = accountRoles.reduce<Record<string, PersonAccountRole[]>>((acc, r) => {
    (acc[r.accountId] ??= []).push(r);
    return acc;
  }, {});

  const handleAddRole = useCallback(async (accountId: string, role: string) => {
    if (!personId) return;
    try {
      await invoke("add_stakeholder_role", { accountId, personId, role });
      loadRoles();
    } catch (err) {
      console.error("add_stakeholder_role failed:", err);
    }
  }, [personId, loadRoles]);

  const handleRemoveRole = useCallback(async (accountId: string, role: string) => {
    if (!personId) return;
    try {
      await invoke("remove_stakeholder_role", { accountId, personId, role });
      loadRoles();
    } catch (err) {
      console.error("remove_stakeholder_role failed:", err);
    }
  }, [personId, loadRoles]);

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
                <AccountEntityRow
                  key={e.id}
                  entity={e}
                  roles={rolesByAccount[e.id] ?? []}
                  onUnlink={onUnlink ? handleUnlink : undefined}
                  onAddRole={personId ? handleAddRole : undefined}
                  onRemoveRole={personId ? handleRemoveRole : undefined}
                />
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

/** Account row with role badges and role picker. */
function AccountEntityRow({
  entity,
  roles,
  onUnlink,
  onAddRole,
  onRemoveRole,
}: {
  entity: LinkedEntity;
  roles: PersonAccountRole[];
  onUnlink?: (entityId: string) => void;
  onAddRole?: (accountId: string, role: string) => void;
  onRemoveRole?: (accountId: string, role: string) => void;
}) {
  const [pickerOpen, setPickerOpen] = useState(false);
  const color = entityColor[entity.entityType] ?? "var(--color-text-tertiary)";

  return (
    <div className={s.entityRow}>
      <div className={s.entityContent}>
        <Link to="/accounts/$accountId" params={{ accountId: entity.id }} className={s.entityLink}>
          <span className={s.entityDot} style={{ background: color }} />
          <span className={s.entityName}>{entity.name}</span>
          <span className={s.entityType}>{entity.entityType}</span>
        </Link>

        {/* Role badges */}
        {(roles.length > 0 || onAddRole) && (
          <div className={s.roleBadges}>
            {roles.map((r) => {
              const cfg = getRoleConfig(r.role);
              return (
                <span
                  key={r.role}
                  className={s.roleBadge}
                  style={{ background: cfg.bg, color: cfg.fg }}
                >
                  {cfg.label}
                  {r.dataSource === "user" && onRemoveRole && (
                    <button
                      className={s.roleRemove}
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        onRemoveRole(entity.id, r.role);
                      }}
                    >
                      &times;
                    </button>
                  )}
                </span>
              );
            })}
            {onAddRole && (
              <div style={{ position: "relative", display: "inline-block" }}>
                <button
                  className={s.addRoleBtn}
                  onClick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    setPickerOpen(!pickerOpen);
                  }}
                >
                  +
                </button>
                {pickerOpen && (
                  <RolePicker
                    existingRoles={roles.map((r) => r.role)}
                    onSelect={(role) => {
                      onAddRole(entity.id, role);
                      setPickerOpen(false);
                    }}
                    onClose={() => setPickerOpen(false)}
                  />
                )}
              </div>
            )}
          </div>
        )}
      </div>

      {onUnlink && (
        <button onClick={() => onUnlink(entity.id)} className={s.unlinkBtn}>
          Unlink
        </button>
      )}
    </div>
  );
}

/** Simple entity row for projects (no role badges). */
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

/** Role picker dropdown — same pattern as StakeholderGallery. */
function RolePicker({
  existingRoles,
  onSelect,
  onClose,
}: {
  existingRoles: string[];
  onSelect: (role: string) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const existing = new Set(existingRoles.map((r) => r.toLowerCase()));
  const available = STAKEHOLDER_ROLES.filter((r) => !existing.has(r.stored));

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [onClose]);

  if (available.length === 0) return null;

  return (
    <div ref={ref} className={s.rolePickerDropdown}>
      {available.map((r) => (
        <button
          key={r.stored}
          className={s.rolePickerItem}
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onSelect(r.stored);
          }}
        >
          {r.label}
        </button>
      ))}
    </div>
  );
}
