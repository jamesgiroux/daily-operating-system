import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactElement } from "react";
import { GlanceCell, type GlanceCellProps, type GlanceCellStatus } from "@/components/ui/GlanceCell";
import styles from "./GlanceRow.module.css";

export type GlanceRowVariant = "default" | "compact" | "wrap";

export interface GlanceRowCell {
  key: GlanceCellProps["label"];
  value: GlanceCellProps["value"];
  status?: GlanceCellStatus;
}

export interface GlanceRowProps extends ComponentPropsWithoutRef<"div"> {
  cells?: readonly GlanceRowCell[];
  children?: ReactElement<GlanceCellProps> | ReactElement<GlanceCellProps>[];
  variant?: GlanceRowVariant;
}

export function GlanceRow({
  cells,
  children,
  variant = "default",
  className,
  ...rest
}: GlanceRowProps) {
  return (
    <div
      className={clsx(styles.row, styles[variant], className)}
      data-ds-name="GlanceRow"
      data-ds-spec="patterns/GlanceRow.md"
      {...rest}
    >
      {cells
        ? cells.map((cell, index) => (
          <div className={styles.cellSlot} key={cellKey(cell, index)}>
            <GlanceCell label={cell.key} value={cell.value} status={cell.status} />
          </div>
        ))
        : null}
      {children
        ? Array.isArray(children)
          ? children.map((child, index) => (
            <div className={styles.cellSlot} key={child.key ?? index}>
              {child}
            </div>
          ))
          : <div className={styles.cellSlot}>{children}</div>
        : null}
    </div>
  );
}

function cellKey(cell: GlanceRowCell, index: number): string | number {
  if (typeof cell.key === "string" || typeof cell.key === "number") {
    return cell.key;
  }
  return index;
}
