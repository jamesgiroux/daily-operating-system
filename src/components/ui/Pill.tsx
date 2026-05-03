import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import styles from "./Pill.module.css";

export type PillTone =
  | "sage"
  | "turmeric"
  | "terracotta"
  | "larkspur"
  | "olive"
  | "eucalyptus"
  | "neutral";

export type PillSize = "standard" | "compact";

interface PillBaseProps {
  tone?: PillTone;
  size?: PillSize;
  dot?: boolean;
  interactive?: boolean;
  children: ReactNode;
  className?: string;
}

type SpanPillProps = PillBaseProps
  & Omit<ComponentPropsWithoutRef<"span">, keyof PillBaseProps>
  & { as?: "span" };

type AnchorPillProps = PillBaseProps
  & Omit<ComponentPropsWithoutRef<"a">, keyof PillBaseProps>
  & { as: "a" };

type ButtonPillProps = PillBaseProps
  & Omit<ComponentPropsWithoutRef<"button">, keyof PillBaseProps>
  & { as: "button" };

export type PillProps = SpanPillProps | AnchorPillProps | ButtonPillProps;

function pillContent(dot: boolean, children: ReactNode) {
  return (
    <>
      {dot ? <span className={styles.dot} aria-hidden="true" /> : null}
      {children}
    </>
  );
}

export function Pill(props: PillProps) {
  const {
    as = "span",
    tone = "neutral",
    size = "standard",
    dot = false,
    interactive = false,
    className,
    children,
    ...rest
  } = props;

  const pillClassName = clsx(
    styles.pill,
    styles[tone],
    styles[size],
    interactive && styles.interactive,
    as === "button" && styles.asButton,
    className,
  );

  const dataAttrs = {
    "data-tone": tone,
    "data-ds-name": "Pill",
    "data-ds-spec": "primitives/Pill.md",
  };

  if (as === "a") {
    return (
      <a
        className={pillClassName}
        {...dataAttrs}
        {...(rest as Omit<ComponentPropsWithoutRef<"a">, keyof PillBaseProps>)}
      >
        {pillContent(dot, children)}
      </a>
    );
  }

  if (as === "button") {
    const { type = "button", ...buttonRest } = rest as Omit<
      ComponentPropsWithoutRef<"button">,
      keyof PillBaseProps
    >;
    return (
      <button
        type={type}
        className={pillClassName}
        {...dataAttrs}
        {...buttonRest}
      >
        {pillContent(dot, children)}
      </button>
    );
  }

  return (
    <span
      className={pillClassName}
      {...dataAttrs}
      {...(rest as Omit<ComponentPropsWithoutRef<"span">, keyof PillBaseProps>)}
    >
      {pillContent(dot, children)}
    </span>
  );
}
