/**
 * TypeBadge — editable account-type badge with inline dropdown.
 * Composes TypeBadgeDisplay's badge classes and overlays dropdown state.
 *
 * v1.4.3 W2 L0 Packet D §5.10 (DOS-682) split: editable variant
 * extracted from AccountHero.tsx:113-172 into the canonical primitive
 * location. Display-only variant lives in TypeBadgeDisplay.tsx and is
 * what the W2 PR-D4 WP block translates.
 */
import { useState, useRef, useEffect } from "react";
import { ChevronDown } from "lucide-react";
import { TYPE_BADGE_OPTIONS, type TypeBadgeValue } from "./TypeBadgeDisplay";
import styles from "./TypeBadge.module.css";

export interface TypeBadgeProps {
  value: TypeBadgeValue;
  onChange: (value: TypeBadgeValue) => void;
}

export function TypeBadge({ value, onChange }: TypeBadgeProps) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [open]);

  const current = TYPE_BADGE_OPTIONS.find((t) => t.value === value) ?? TYPE_BADGE_OPTIONS[0];

  return (
    <div
      ref={ref}
      className={styles.typeBadgeWrapper}
      data-ds-name="TypeBadge"
      data-ds-tier="primitive"
      data-ds-spec="primitives/TypeBadge.md"
    >
      <button
        className={`${styles.typeBadge} ${styles[current.badgeClass]} ${styles.typeBadgeButton}`}
        onClick={() => setOpen(!open)}
      >
        {current.label}
        <ChevronDown size={10} strokeWidth={2} className={styles.typeBadgeChevron} />
      </button>
      {open && (
        <div className={styles.typeBadgeDropdown}>
          {TYPE_BADGE_OPTIONS.map((opt) => {
            const activeClass =
              opt.value === value
                ? `${styles.typeBadgeOptionActive} ${
                    opt.value === "customer"
                      ? styles.typeBadgeOptionCustomer
                      : opt.value === "internal"
                        ? styles.typeBadgeOptionInternal
                        : styles.typeBadgeOptionPartner
                  }`
                : "";
            return (
              <button
                key={opt.value}
                onClick={() => {
                  onChange(opt.value);
                  setOpen(false);
                }}
                className={`${styles.typeBadgeOption} ${activeClass}`}
              >
                {opt.label}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
