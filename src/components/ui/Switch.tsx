import clsx from "clsx";
import type { ComponentPropsWithoutRef, MouseEvent } from "react";
import styles from "./Switch.module.css";

export interface SwitchProps
  extends Omit<ComponentPropsWithoutRef<"button">, "onChange" | "role"> {
  checked: boolean;
  onCheckedChange?: (checked: boolean) => void;
  size?: "sm";
}

export function Switch({
  checked,
  onCheckedChange,
  size = "sm",
  className,
  disabled,
  onClick,
  ...rest
}: SwitchProps) {
  function handleClick(event: MouseEvent<HTMLButtonElement>) {
    onClick?.(event);
    if (event.defaultPrevented || disabled) return;
    onCheckedChange?.(!checked);
  }

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      className={clsx(styles.switch, className)}
      data-checked={checked}
      data-size={size}
      data-ds-name="Switch"
      data-ds-spec="primitives/Switch.md"
      disabled={disabled}
      onClick={handleClick}
      {...rest}
    >
      <span className={styles.thumb} aria-hidden="true" />
    </button>
  );
}
