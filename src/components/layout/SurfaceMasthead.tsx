import clsx from "clsx";
import type { ComponentPropsWithoutRef, CSSProperties, ReactNode } from "react";
import styles from "./SurfaceMasthead.module.css";

export type SurfaceMastheadDensity = "compact" | "default" | "rich";
export type SurfaceMastheadWidth = "standard" | "reading";

export interface SurfaceMastheadProps
  extends Omit<ComponentPropsWithoutRef<"section">, "title"> {
  eyebrow?: ReactNode;
  title: ReactNode;
  lede?: ReactNode;
  accessory?: ReactNode;
  glance?: ReactNode;
  density?: SurfaceMastheadDensity;
  width?: SurfaceMastheadWidth;
  rule?: boolean;
  ruleColor?: string;
}

export function SurfaceMasthead({
  eyebrow,
  title,
  lede,
  accessory,
  glance,
  density = "default",
  width = "standard",
  rule = true,
  ruleColor,
  className,
  style,
  ...rest
}: SurfaceMastheadProps) {
  const cssVars = ruleColor
    ? ({ "--surface-masthead-rule-color": ruleColor, ...style } as CSSProperties)
    : style;

  return (
    <section
      className={clsx(
        styles.masthead,
        styles[density],
        styles[`width${capitalize(width)}`],
        className,
      )}
      style={cssVars}
      data-ds-name="SurfaceMasthead"
      data-ds-spec="patterns/SurfaceMasthead.md"
      {...rest}
    >
      <div className={styles.topRow}>
        <div className={styles.titleBlock}>
          {eyebrow ? <p className={styles.eyebrow}>{eyebrow}</p> : null}
          <h1 className={styles.title}>{title}</h1>
          {lede ? <p className={styles.lede}>{lede}</p> : null}
        </div>
        {accessory ? <div className={styles.accessory}>{accessory}</div> : null}
      </div>
      {rule ? <div className={styles.rule} /> : null}
      {glance ? <div className={styles.glance}>{glance}</div> : null}
    </section>
  );
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
