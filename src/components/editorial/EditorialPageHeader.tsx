import type { CSSProperties, ReactNode } from "react";
import styles from "./EditorialPageHeader.module.css";

type HeaderScale = "standard" | "page" | "profile";
type HeaderWidth = "standard" | "reading";
type HeaderRule = "standard" | "subtle";

interface EditorialPageHeaderProps {
  title: ReactNode;
  subtitle?: ReactNode;
  meta?: ReactNode;
  children?: ReactNode;
  scale?: HeaderScale;
  width?: HeaderWidth;
  rule?: HeaderRule;
  ruleColor?: string;
  className?: string;
}

export function EditorialPageHeader({
  title,
  subtitle,
  meta,
  children,
  scale = "standard",
  width = "standard",
  rule = "standard",
  ruleColor,
  className,
}: EditorialPageHeaderProps) {
  const cssVars = ruleColor
    ? ({ "--editorial-header-rule-color": ruleColor } as CSSProperties)
    : undefined;

  return (
    <section
      className={[
        styles.header,
        styles[`scale${capitalize(scale)}`],
        styles[`width${capitalize(width)}`],
        styles[`rule${capitalize(rule)}`],
        className,
      ]
        .filter(Boolean)
        .join(" ")}
      style={cssVars}
    >
      <div className={styles.topRow}>
        <div className={styles.titleBlock}>
          <h1 className={styles.title}>{title}</h1>
          {subtitle ? <p className={styles.subtitle}>{subtitle}</p> : null}
        </div>
        {meta ? <div className={styles.meta}>{meta}</div> : null}
      </div>
      <div className={styles.rule} />
      {children ? <div className={styles.after}>{children}</div> : null}
    </section>
  );
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
