import { type CSSProperties, type ReactNode } from "react";
import { Link } from "@tanstack/react-router";
import s from "./EntityRow.module.css";

interface EntityRowSelection {
  selected: boolean;
  label?: string;
  onChange: (options: { shiftKey: boolean }) => void;
}

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
  /** Optional row-level selection control */
  selection?: EntityRowSelection;
  /** Interactive row controls rendered outside the entity link */
  controls?: ReactNode;
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
  selection,
  controls,
}: EntityRowProps) {
  const className = [
    s.row,
    showBorder ? s.rowBorder : "",
    selection?.selected ? s.rowSelected : "",
  ].filter(Boolean).join(" ");
  const rowStyle: CSSProperties = { paddingLeft };
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

  const bodyClassName = to || href ? s.rowLink : s.rowBody;

  let body: ReactNode;
  if (href) {
    body = (
      <a
        href={href}
        target="_blank"
        rel="noreferrer"
        className={bodyClassName}
      >
        {content}
      </a>
    );
  } else if (to) {
    body = (
      <Link
        to={to}
        params={params ?? {}}
        className={bodyClassName}
      >
        {content}
      </Link>
    );
  } else {
    body = <div className={bodyClassName}>{content}</div>;
  }

  return (
    <div className={className} style={rowStyle}>
      {selection && (
        <input
          type="checkbox"
          className={s.checkbox}
          checked={selection.selected}
          aria-label={selection.label ?? `Select ${name}`}
          onChange={(event) => {
            const nativeEvent = event.nativeEvent as Event & { shiftKey?: boolean };
            selection.onChange({ shiftKey: Boolean(nativeEvent.shiftKey) });
          }}
        />
      )}
      {body}
      {controls && (
        <div className={s.controls}>
          {controls}
        </div>
      )}
    </div>
  );
}
