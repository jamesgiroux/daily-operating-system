import clsx from "clsx";
import type {
  ButtonHTMLAttributes,
  ComponentPropsWithoutRef,
  CSSProperties,
  InputHTMLAttributes,
  ReactNode,
} from "react";
import styles from "./FormRow.module.css";

export type FormRowVariant = "default" | "dense" | "stacked" | "readonly";

export interface FormRowProps extends ComponentPropsWithoutRef<"div"> {
  label: ReactNode;
  help?: ReactNode;
  aux?: ReactNode;
  children: ReactNode;
  variant?: FormRowVariant;
  controlId?: string;
  noBorder?: boolean;
}

export function FormRow({
  label,
  help,
  aux,
  children,
  variant = "default",
  controlId,
  noBorder = false,
  className,
  ...rest
}: FormRowProps) {
  const labelNode = controlId ? (
    <label className={styles.label} htmlFor={controlId}>
      {label}
    </label>
  ) : (
    <span className={styles.label}>{label}</span>
  );

  return (
    <div
      className={clsx(
        styles.row,
        variant !== "default" && styles[variant],
        noBorder && styles.noBorder,
        className,
      )}
      data-ds-name="FormRow"
      data-ds-spec="patterns/FormRow.md"
      {...rest}
    >
      <div className={styles.copy}>
        {labelNode}
        {help ? <p className={styles.help}>{help}</p> : null}
      </div>
      <div className={styles.control}>{children}</div>
      {aux ? <div className={styles.aux}>{aux}</div> : null}
    </div>
  );
}

export interface SettingsSectionLabelProps extends ComponentPropsWithoutRef<"p"> {
  as?: "p" | "h3";
}

export function SettingsSectionLabel({
  as = "p",
  className,
  children,
  ...rest
}: SettingsSectionLabelProps) {
  const Component = as;

  return (
    <Component className={clsx(styles.subsectionLabel, className)} {...rest}>
      {children}
    </Component>
  );
}

export function SettingsDescription({
  className,
  children,
  ...rest
}: ComponentPropsWithoutRef<"p">) {
  return (
    <p className={clsx(styles.description, className)} {...rest}>
      {children}
    </p>
  );
}

export function SettingsMonoLabel({
  className,
  children,
  ...rest
}: ComponentPropsWithoutRef<"span">) {
  return (
    <span className={clsx(styles.monoLabel, className)} {...rest}>
      {children}
    </span>
  );
}

export function SettingsRule({
  className,
  ...rest
}: ComponentPropsWithoutRef<"hr">) {
  return <hr className={clsx(styles.thinRule, className)} {...rest} />;
}

export type SettingsButtonTone = "primary" | "ghost" | "danger" | "borderless";

export interface SettingsButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  tone?: SettingsButtonTone;
  muted?: boolean;
  compact?: boolean;
}

const buttonToneClass: Record<SettingsButtonTone, string> = {
  primary: styles.buttonPrimary,
  ghost: styles.buttonGhost,
  danger: styles.buttonDanger,
  borderless: styles.buttonBorderless,
};

export function SettingsButton({
  tone = "ghost",
  muted = false,
  compact = false,
  className,
  children,
  type = "button",
  ...rest
}: SettingsButtonProps) {
  return (
    <button
      type={type}
      className={clsx(
        buttonToneClass[tone],
        compact && styles.buttonCompact,
        muted && styles.isMuted,
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
}

export interface SettingsInputProps extends InputHTMLAttributes<HTMLInputElement> {
  width?: 200 | 240 | 260 | 300;
}

export function SettingsInput({
  width,
  className,
  ...rest
}: SettingsInputProps) {
  return (
    <input
      className={clsx(width && styles[`inputWidth${width}`], styles.input, className)}
      {...rest}
    />
  );
}

export interface SettingsStatusDotProps extends ComponentPropsWithoutRef<"span"> {
  color?: string;
}

export function SettingsStatusDot({
  color,
  className,
  style,
  ...rest
}: SettingsStatusDotProps) {
  const cssVars = color
    ? ({ "--settings-status-color": color, ...style } as CSSProperties)
    : style;

  return (
    <span
      aria-hidden="true"
      className={clsx(styles.statusDot, className)}
      style={cssVars}
      {...rest}
    />
  );
}

export { styles as formRowStyles };
