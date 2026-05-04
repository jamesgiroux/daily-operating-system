import clsx from "clsx";
import { Building2, FolderKanban, User } from "lucide-react";
import type {
  ComponentPropsWithoutRef,
  MouseEventHandler,
  ReactNode,
} from "react";
import { Pill, type PillSize } from "./Pill";
import { RemovableChip } from "./RemovableChip";
import styles from "./EntityChip.module.css";

export type EntityChipType = "account" | "project" | "person";

export interface EntityChipProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children" | "onClick"> {
  entityType?: EntityChipType | string;
  entityName: ReactNode;
  removable?: boolean;
  editable?: boolean;
  compact?: boolean;
  onRemove?: () => void;
  onEdit?: MouseEventHandler<HTMLSpanElement>;
  removeLabel?: string;
}

function normalizeEntityType(entityType?: EntityChipType | string): EntityChipType {
  if (entityType === "project" || entityType === "person") return entityType;
  return "account";
}

function iconForEntityType(entityType: EntityChipType) {
  if (entityType === "project") return FolderKanban;
  if (entityType === "person") return User;
  return Building2;
}

function defaultRemoveLabel(entityName: ReactNode) {
  return typeof entityName === "string" ? `Remove ${entityName}` : "Remove entity";
}

const noop = () => undefined;

export function EntityChip({
  entityType,
  entityName,
  removable = false,
  editable = false,
  compact = false,
  onRemove,
  onEdit,
  removeLabel,
  className,
  title,
  "aria-label": ariaLabel,
  ...rest
}: EntityChipProps) {
  const normalizedEntityType = normalizeEntityType(entityType);
  const Icon = iconForEntityType(normalizedEntityType);
  const size: PillSize = compact ? "compact" : "standard";
  const iconSize = compact ? 10 : 12;
  const chipClassName = clsx(
    styles.chip,
    styles[`chip-${normalizedEntityType}`],
    className,
  );
  const label = (
    <>
      <Icon
        className={styles.icon}
        size={iconSize}
        strokeWidth={2}
        aria-hidden="true"
      />
      <span className={styles.name}>{entityName}</span>
    </>
  );

  if (removable) {
    return (
      <RemovableChip
        label={label}
        tone="neutral"
        size={size}
        className={chipClassName}
        onRemove={onRemove ?? noop}
        removeLabel={removeLabel}
        aria-label={ariaLabel ?? defaultRemoveLabel(entityName)}
        data-entity-type={normalizedEntityType}
        data-ds-name="EntityChip"
        data-ds-spec="primitives/EntityChip.md"
        {...rest}
      />
    );
  }

  return (
    <Pill
      tone="neutral"
      size={size}
      interactive={editable && Boolean(onEdit)}
      className={chipClassName}
      onClick={editable ? onEdit : undefined}
      title={title ?? (editable ? "Click to change" : undefined)}
      aria-label={ariaLabel}
      data-entity-type={normalizedEntityType}
      data-ds-name="EntityChip"
      data-ds-spec="primitives/EntityChip.md"
      {...rest}
    >
      {label}
    </Pill>
  );
}
