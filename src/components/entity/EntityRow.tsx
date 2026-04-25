import { type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import s from "./EntityRow.module.css";

interface EntityRowProps {
  to: string;
  params: Record<string, string>;
  dotColor: string;
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
  dotColor,
  name,
  showBorder,
  paddingLeft = 0,
  nameSuffix,
  subtitle,
  children,
  avatar,
}: EntityRowProps) {
  return (
    <Link
      to={to}
      params={params}
      className={`${s.row} ${showBorder ? s.rowBorder : ""}`}
      // Runtime hierarchy controls nested row indentation.
      style={{
        paddingLeft,
      }}
    >
      {/* Avatar or accent dot */}
      {avatar ?? (
        <div
          className={s.dot}
          // Runtime entity type controls accent color.
          style={{
            background: dotColor,
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
    </Link>
  );
}
