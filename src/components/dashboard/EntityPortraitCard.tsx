import clsx from "clsx";
import type { ComponentPropsWithoutRef, ReactNode } from "react";
import { ThreadMark } from "@/components/ui/ThreadMark";
import styles from "./EntityPortraitCard.module.css";

export type EntityPortraitType = "account" | "project" | "person";
export type EntityPortraitAccent = "turmeric" | "terracotta" | "olive" | "larkspur" | "rosemary";
export type EntityPortraitStatDirection = "up" | "down" | "neutral";
export type EntityPortraitThreadType = "meeting" | "action" | "mail" | "lifecycle";

export interface EntityPortraitState {
  label: string;
  overrideColor?: EntityPortraitAccent;
}

export interface EntityPortraitStat {
  label: ReactNode;
  value: ReactNode;
  direction?: EntityPortraitStatDirection;
}

export interface EntityPortraitThreadItem {
  type: EntityPortraitThreadType;
  when: string;
  what: ReactNode;
  overdue?: boolean;
  showThreadMark?: boolean;
  threadId?: string;
  threadMarkContext?: string;
}

export interface EntityPortraitCardProps
  extends Omit<ComponentPropsWithoutRef<"article">, "title"> {
  entityType: EntityPortraitType;
  state: EntityPortraitState;
  name: string;
  glyph?: ReactNode;
  asideStats: EntityPortraitStat[];
  lede: ReactNode;
  thread: EntityPortraitThreadItem[];
}

const DEFAULT_ACCENT_BY_TYPE: Record<EntityPortraitType, EntityPortraitAccent> = {
  account: "turmeric",
  project: "olive",
  person: "larkspur",
};

const ACCENT_CLASS: Record<EntityPortraitAccent, string> = {
  turmeric: styles.accentTurmeric,
  terracotta: styles.accentTerracotta,
  olive: styles.accentOlive,
  larkspur: styles.accentLarkspur,
  rosemary: styles.accentRosemary,
};

const THREAD_TYPE_CLASS: Record<EntityPortraitThreadType, string> = {
  meeting: styles.eventMeeting,
  action: styles.eventAction,
  mail: styles.eventMail,
  lifecycle: styles.eventLifecycle,
};

const STAT_DIRECTION_CLASS: Record<EntityPortraitStatDirection, string | undefined> = {
  up: styles.directionUp,
  down: styles.directionDown,
  neutral: undefined,
};

function fallbackGlyph(name: string): string {
  return name.trim().charAt(0).toUpperCase() || "?";
}

function fallbackThreadContext(item: EntityPortraitThreadItem): string {
  if (item.threadMarkContext) return item.threadMarkContext;
  return typeof item.what === "string" ? item.what : "";
}

export function EntityPortraitCard({
  entityType,
  state,
  name,
  glyph,
  asideStats,
  lede,
  thread,
  className,
  ...rest
}: EntityPortraitCardProps) {
  const accent = state.overrideColor ?? DEFAULT_ACCENT_BY_TYPE[entityType];

  return (
    <article
      className={clsx(styles.card, ACCENT_CLASS[accent], className)}
      data-entity-type={entityType}
      data-ds-name="EntityPortraitCard"
      data-ds-spec="patterns/EntityPortraitCard.md"
      {...rest}
    >
      <aside className={styles.aside} aria-label={`${name} summary`}>
        <div>
          <div className={styles.stateLabel}>{state.label}</div>
          <h3 className={styles.name}>{name}</h3>
        </div>

        {asideStats.length > 0 ? (
          <dl className={styles.asideStats}>
            {asideStats.map((stat, index) => (
              <div className={styles.statRow} key={index}>
                <dt className={styles.statLabel}>{stat.label}</dt>
                <dd className={clsx(styles.statValue, STAT_DIRECTION_CLASS[stat.direction ?? "neutral"])}>
                  {stat.value}
                </dd>
              </div>
            ))}
          </dl>
        ) : null}
      </aside>

      <div className={styles.main}>
        <span className={styles.glyph} aria-hidden="true">
          {glyph ?? fallbackGlyph(name)}
        </span>
        <p className={styles.lede}>{lede}</p>

        {thread.length > 0 ? (
          <ul className={styles.thread}>
            {thread.map((item, index) => (
              <li
                className={clsx(styles.threadItem, item.overdue && styles.overdue)}
                data-addressable={item.showThreadMark || undefined}
                data-thread-id={item.threadId}
                key={`${item.type}-${item.when}-${index}`}
              >
                <span className={clsx(styles.typeDot, THREAD_TYPE_CLASS[item.type])} aria-hidden="true" />
                <span className={styles.when}>{item.when}</span>
                <span className={styles.what}>{item.what}</span>
                {item.showThreadMark ? (
                  <ThreadMark context={fallbackThreadContext(item)} threadId={item.threadId} />
                ) : null}
              </li>
            ))}
          </ul>
        ) : null}
      </div>
    </article>
  );
}
