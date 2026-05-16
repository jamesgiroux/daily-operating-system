import { type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import s from "./EntityRow.module.css";

interface EntityRowProps {
  to?: string;
  params?: Record<string, string>;
  href?: string | null;
  dotColor?: string;
  name: string;
  showBorder: boolean;
  paddingLeft?: number;
  /** Inline badges/tags rendered beside the name */
  nameSuffix?: ReactNode;
  /** Subtitle line beneath the name */
  subtitle?: ReactNode;
  /** Right-aligned metadata slot */
  children?: ReactNode;
  /** Optional avatar element to replace the accent dot */
  avatar?: ReactNode;
}

export function EntityRow({
  to,
  params,
  href,
  dotColor,
  name,
  showBorder,
  paddingLeft = 0,
  nameSuffix,
  subtitle,
  children,
  avatar,
}: EntityRowProps) {
  const className = `${s.row} ${showBorder ? s.rowBorder : ""}`;
  const rowStyle = { paddingLeft };
  const content = (
    <>
      {/* Avatar or accent dot */}
      {avatar ?? (
        <div
          className={s.dot}
          // Runtime entity type controls accent color.
          style={{
            background: dotColor ?? "var(--color-text-tertiary)",
          }}
        />
      )}

      {/* Content */}
      <div className={s.content}>
        <div className={s.nameRow}>
          <span className={s.name}>
            {name}
          </span>
          {nameSuffix}
        </div>
        {subtitle && (
          <div className={s.subtitle}>
            {subtitle}
          </div>
        )}
      </div>

      {/* Right-aligned metadata */}
      {children && (
        <div className={s.meta}>
          {children}
        </div>
      )}
    </>
  );

  if (href) {
    return (
      <a
        href={href}
        target="_blank"
        rel="noreferrer"
        className={className}
        style={rowStyle}
      >
        {content}
      </a>
    );
  }

  if (to) {
    return (
      <Link
        to={to}
        params={params ?? {}}
        className={className}
        // Runtime hierarchy controls nested row indentation.
        style={rowStyle}
      >
        {content}
      </Link>
    );
  }

  return (
    <div className={className} style={rowStyle}>
      {content}
    </div>
  );
}
