/**
 * TypeBadgeDisplay — display-only TypeBadge primitive.
 * No state, no events. Renders the labeled account-type badge for the
 * given value. Editable variant (with dropdown) lives in TypeBadge.tsx.
 *
 * v1.4.3 W2 L0 Packet D §5.10 (DOS-682) split.
 */
import styles from "./TypeBadge.module.css";

export type TypeBadgeValue = "customer" | "internal" | "partner";

export const TYPE_BADGE_OPTIONS: { value: TypeBadgeValue; label: string; badgeClass: string }[] = [
  { value: "customer", label: "Customer", badgeClass: "customerBadge" },
  { value: "internal", label: "Internal", badgeClass: "internalBadge" },
  { value: "partner", label: "Partner", badgeClass: "partnerBadge" },
];

export interface TypeBadgeDisplayProps {
  value: TypeBadgeValue;
}

export function TypeBadgeDisplay({ value }: TypeBadgeDisplayProps) {
  const current = TYPE_BADGE_OPTIONS.find((t) => t.value === value) ?? TYPE_BADGE_OPTIONS[0];
  return (
    <span
      className={`${styles.typeBadge} ${styles[current.badgeClass]}`}
      data-ds-name="TypeBadge"
      data-ds-tier="primitive"
      data-ds-spec="primitives/TypeBadge.md"
    >
      {current.label}
    </span>
  );
}
