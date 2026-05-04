import clsx from "clsx";
import type { ComponentPropsWithoutRef, MouseEvent } from "react";
import styles from "./ThreadMark.module.css";

export const THREAD_MARK_SEED_EVENT = "dailyos:threadmark-seed";

export interface ThreadMarkSeedDetail {
  context: string;
  threadId?: string;
}

export interface ThreadMarkProps
  extends Omit<ComponentPropsWithoutRef<"button">, "children" | "onClick"> {
  context?: string;
  label?: string;
  persistent?: boolean;
  threadId?: string;
  stopPropagation?: boolean;
  onClick?: (context: string, event: MouseEvent<HTMLButtonElement>) => void;
}

function dispatchThreadMarkSeed(detail: ThreadMarkSeedDetail) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent<ThreadMarkSeedDetail>(THREAD_MARK_SEED_EVENT, { detail }));
}

export function ThreadMark({
  context = "",
  label = "talk",
  persistent = false,
  threadId,
  stopPropagation = true,
  className,
  onClick,
  type = "button",
  ...rest
}: ThreadMarkProps) {
  function handleClick(event: MouseEvent<HTMLButtonElement>) {
    if (stopPropagation) {
      event.stopPropagation();
    }

    const detail = { context, threadId };
    dispatchThreadMarkSeed(detail);
    onClick?.(context, event);
  }

  return (
    <button
      type={type}
      className={clsx(styles.threadMark, persistent && styles.persistent, className)}
      data-ds-name="ThreadMark"
      data-ds-spec="patterns/ThreadMark.md"
      data-thread-mark=""
      data-thread-id={threadId}
      data-persistent={persistent || undefined}
      aria-label={context ? `Talk about ${context}` : "Talk about this"}
      onClick={handleClick}
      {...rest}
    >
      {label}
    </button>
  );
}
