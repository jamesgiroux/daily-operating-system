import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { Pill, type PillSize, type PillTone } from "./Pill";
import styles from "./RemovableChip.module.css";

export interface RemovableChipProps
  extends Omit<ComponentPropsWithoutRef<"span">, "children" | "aria-label"> {
  label: ReactNode;
  tone?: PillTone;
  size?: PillSize;
  onRemove: () => void;
  removeLabel?: string;
  disabled?: boolean;
  removing?: boolean;
  "aria-label"?: string;
}

export function RemovableChip({
  label,
  tone = "neutral",
  size = "standard",
  onRemove,
  removeLabel,
  disabled = false,
  removing = false,
  className,
  "aria-label": ariaLabel,
  ...rest
}: RemovableChipProps) {
  return (
    <Pill
      tone={tone}
      size={size}
      className={clsx(
        styles.chip,
        size === "compact" && styles.compact,
        removing && styles.removing,
        disabled && styles.disabled,
        className,
      )}
      data-ds-name="RemovableChip"
      data-ds-spec="primitives/RemovableChip.md"
      {...rest}
    >
      <span className={styles.label}>{label}</span>
      <button
        type="button"
        className={styles.remove}
        onClick={(event) => {
          event.stopPropagation();
          event.preventDefault();
          onRemove();
        }}
        disabled={disabled || removing}
        aria-label={ariaLabel ?? removeLabel ?? "Remove"}
      >
        <span aria-hidden="true">&times;</span>
      </button>
    </Pill>
  );
}
