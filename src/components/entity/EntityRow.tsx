import { type ReactNode } from "react";
import { Link } from "@tanstack/react-router";

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
}: EntityRowProps) {
  return (
    <Link
      to={to}
      params={params}
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "14px 0",
        paddingLeft,
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        textDecoration: "none",
      }}
    >
      {/* Accent dot */}
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: dotColor,
          flexShrink: 0,
          marginTop: 8,
        }}
      />

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 17,
              fontWeight: 400,
              color: "var(--color-text-primary)",
            }}
          >
            {name}
          </span>
          {nameSuffix}
        </div>
        {subtitle && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              marginTop: 2,
            }}
          >
            {subtitle}
          </div>
        )}
      </div>

      {/* Right-aligned metadata */}
      {children && (
        <div style={{ display: "flex", alignItems: "baseline", gap: 16, flexShrink: 0 }}>
          {children}
        </div>
      )}
    </Link>
  );
}
