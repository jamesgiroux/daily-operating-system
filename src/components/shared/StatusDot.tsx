import s from "./StatusDot.module.css";

export type StatusDotStatus = "connected" | "disconnected" | "loading" | "error";
export type StatusDotSize = "sm" | "md";

interface StatusDotProps {
  status: StatusDotStatus;
  size?: StatusDotSize;
  label?: string;
}

/**
 * Reusable status indicator dot with optional label.
 *
 * Colors: connected → sage, disconnected → terracotta,
 * loading → saffron (pulsing), error → chili.
 */
export default function StatusDot({ status, size = "md", label }: StatusDotProps) {
  const dotEl = (
    <span
      className={`${s.dot} ${s[size]} ${s[status]}`}
      role="img"
      aria-label={label ?? status}
    />
  );

  if (!label) return dotEl;

  return (
    <span className={s.wrapper}>
      {dotEl}
      <span className={s.label}>{label}</span>
    </span>
  );
}
