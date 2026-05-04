import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import styles from "./Segmented.module.css";

export type SegmentedTint = "turmeric" | "eucalyptus" | "larkspur";

export interface SegmentedOption<Value extends string | number = string> {
  value: Value;
  label: ReactNode;
  disabled?: boolean;
}

export interface SegmentedProps<Value extends string | number = string>
  extends Omit<ComponentPropsWithoutRef<"div">, "onChange"> {
  value: Value;
  options: readonly SegmentedOption<Value>[];
  onChange: (value: Value) => void;
  tint?: SegmentedTint;
  disabled?: boolean;
  "aria-label": string;
}

export function Segmented<Value extends string | number = string>({
  value,
  options,
  onChange,
  tint = "turmeric",
  disabled = false,
  className,
  "aria-label": ariaLabel,
  ...rest
}: SegmentedProps<Value>) {
  return (
    <div
      role="group"
      aria-label={ariaLabel}
      className={clsx(styles.group, styles[tint], className)}
      data-disabled={disabled}
      data-ds-name="Segmented"
      data-ds-spec="primitives/Segmented.md"
      {...rest}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            className={clsx(styles.option, selected && styles.selected)}
            aria-pressed={selected}
            disabled={disabled || option.disabled}
            onClick={() => onChange(option.value)}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
