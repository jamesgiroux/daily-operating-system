import clsx from "clsx";
import { useBriefingViewModel } from "@/hooks/useBriefingViewModel";
import {
  BriefingEmptyState,
  BriefingErrorState,
  BriefingLoadingState,
  DayStrip,
  Lead,
  MovingRow,
  PredictionsSection,
  WatchRow,
} from "@/components/dashboard";
import {
  AtmosphereLayer,
  FloatingNavIsland,
  FolioBar,
  type FolioBreadcrumbItem,
  type ReadinessStat,
} from "@/components/layout";
import type {
  BriefingLoadState,
  BriefingViewModel,
  MovingEntityViewModel,
  ScheduleMeeting,
  TrustBandWire,
  WatchRowViewModel,
} from "@/types/briefing";
import { TrustBandBadge, type TrustBand } from "@/components/ui/TrustBandBadge";
import styles from "./DailyBriefingRedesign.module.css";

const NAV_ROUTES: Record<string, string> = {
  today: "/",
  emails: "/emails",
  dropbox: "/inbox",
  actions: "/actions",
  me: "/me",
  people: "/people",
  accounts: "/accounts",
  projects: "/projects",
  settings: "/settings",
};

function asBadgeBand(band: TrustBandWire): TrustBand | null {
  return band === "unscored" ? null : band;
}

export function DailyBriefingRedesign(): JSX.Element {
  const { state, refresh } = useBriefingViewModel();
  const model = state.status === "success" ? state.model : null;

  return (
    <div
      className={styles.root}
      data-ds-name="DailyBriefingRedesign"
      data-ds-tier="surface"
      data-ds-spec="surfaces/DailyBriefingRedesign.md"
    >
      <AtmosphereLayer color="turmeric" />
      <div className={styles.chrome}>
        <SurfaceFolio model={model} />
        {model ? <DayStrip {...model.dayStrip} /> : null}
        <FloatingNavIsland
          activePage="today"
          activeColor="turmeric"
          onHome={() => navigateToHref("/")}
          onNavigate={handleNavNavigate}
        />
      </div>
      <main
        className={clsx(
          styles.main,
          state.status !== "success" && styles.stateMain,
        )}
      >
        <StateBranch state={state} refresh={refresh} />
      </main>
    </div>
  );
}

function SurfaceFolio({ model }: { model: BriefingViewModel | null }) {
  if (!model) {
    return <FolioBar publicationLabel="Daily Briefing" />;
  }

  const breadcrumbs = toBreadcrumbs(model.folio.crumbs);

  return (
    <FolioBar
      publicationLabel={model.folio.label}
      breadcrumbs={breadcrumbs.length > 0 ? breadcrumbs : undefined}
      dateText={model.folio.dateLabel}
      readinessStats={toReadinessStats(model)}
      statusText={model.folio.status}
    />
  );
}

function StateBranch({
  state,
  refresh,
}: {
  state: BriefingLoadState;
  refresh: () => void;
}) {
  switch (state.status) {
    case "loading":
      return (
        <BriefingLoadingState
          eyebrow="Daily Briefing"
          headline="Assembling your briefing."
        />
      );

    case "error":
      return (
        <BriefingErrorState
          eyebrow="Briefing unavailable"
          message={state.message}
          detailMessage={state.detailMessage}
          code={state.code}
          service={state.service}
          onRetry={refresh}
        />
      );

    case "empty":
      return (
        <BriefingEmptyState
          eyebrow="Daily Briefing"
          headline="Your day, when DailyOS can read it."
          lede={state.message}
          checklistItems={state.checklistItems}
          cta={{ label: "Check again", onClick: refresh }}
        />
      );

    case "success":
      return <SuccessBranch model={state.model} />;

    default: {
      const exhaustive: never = state;
      return exhaustive;
    }
  }
}

function SuccessBranch({ model }: { model: BriefingViewModel }) {
  return (
    <div className={styles.content}>
      <Lead lead={model.lead} />
      <ScheduleSection model={model} />
      <section
        className={styles.section}
        aria-labelledby="predictions-heading"
      >
        <div className={styles.sectionLabel}>{model.predictions.label}</div>
        <div className={styles.sectionBody}>
          <div className={styles.sectionHeader}>
            <div className={styles.sectionTitleRow}>
              <h2 id="predictions-heading" className={styles.sectionTitle}>
                {model.predictions.label}
              </h2>
              <span className={styles.countLabel}>
                {model.predictions.countLabel}
              </span>
            </div>
          </div>
          <PredictionsSection predictions={model.predictions} />
        </div>
      </section>
      <MovingSection model={model} />
      <WatchSection model={model} />
    </div>
  );
}

function ScheduleSection({ model }: { model: BriefingViewModel }) {
  return (
    <section className={styles.section} aria-labelledby="schedule-heading">
      <div className={styles.sectionLabel}>{model.schedule.label}</div>
      <div className={styles.sectionBody}>
        <div className={styles.sectionHeader}>
          <div className={styles.sectionTitleRow}>
            <h2 id="schedule-heading" className={styles.sectionTitle}>
              {model.schedule.heading}
            </h2>
            <span className={styles.countLabel}>
              {model.schedule.countLabel}
            </span>
          </div>
          <p className={styles.sectionSummary}>{model.schedule.summary}</p>
        </div>
        <ul className={styles.meetingList}>
          {model.schedule.meetings.map((meeting) => (
            <ScheduleMeetingRow key={meeting.id} meeting={meeting} />
          ))}
        </ul>
      </div>
    </section>
  );
}

function ScheduleMeetingRow({
  meeting,
}: {
  meeting: ScheduleMeeting;
}): JSX.Element {
  const badgeBand = asBadgeBand(meeting.trustBand);
  const href = scheduleMeetingHref(meeting);
  const content = (
    <>
      <time
        className={styles.meetingTime}
        dateTime={meeting.time.startsAtIso}
      >
        {meeting.time.startLabel}
      </time>
      <span className={styles.meetingMain}>
        <span className={styles.meetingTitle}>{meeting.title}</span>
        {badgeBand && (
          <TrustBandBadge
            band={badgeBand}
            compact
            className={styles.meetingTrust}
          />
        )}
      </span>
    </>
  );

  return (
    <li className={styles.meetingItem}>
      {href ? (
        <a
          className={styles.meetingLink}
          href={href}
          onClick={(event) => {
            event.preventDefault();
            navigateToHref(href);
          }}
        >
          {content}
        </a>
      ) : (
        <div className={styles.meetingItemContent}>{content}</div>
      )}
    </li>
  );
}

function scheduleMeetingHref(meeting: ScheduleMeeting): string | undefined {
  if (meeting.state === "cancelled") return undefined;
  if (meeting.href) return meeting.href;
  if (meeting.briefingAction.kind === "link") return meeting.briefingAction.href;
  return `/meeting/${meeting.id}`;
}

function MovingSection({ model }: { model: BriefingViewModel }) {
  return (
    <section className={styles.section} aria-labelledby="moving-heading">
      <div className={styles.sectionLabel}>{model.moving.label}</div>
      <div className={styles.sectionBody}>
        <div className={styles.sectionHeader}>
          <div className={styles.sectionTitleRow}>
            <h2 id="moving-heading" className={styles.sectionTitle}>
              {model.moving.heading}
            </h2>
            <span className={styles.countLabel}>{model.moving.countLabel}</span>
          </div>
          <p className={styles.sectionSummary}>{model.moving.summary}</p>
        </div>
        <div className={styles.rowStack}>
          {model.moving.entities.map((entity) => (
            <MovingRow
              key={entity.entity.id}
              {...entity}
              onNavigate={handleMovingNavigate}
            />
          ))}
        </div>
      </div>
    </section>
  );
}

function WatchSection({ model }: { model: BriefingViewModel }) {
  return (
    <section className={styles.section} aria-labelledby="watch-heading">
      <div className={styles.sectionLabel}>{model.watch.label}</div>
      <div className={styles.sectionBody}>
        <div className={styles.sectionHeader}>
          <div className={styles.sectionTitleRow}>
            <h2 id="watch-heading" className={styles.sectionTitle}>
              {model.watch.heading}
            </h2>
            <span className={styles.countLabel}>{model.watch.countLabel}</span>
          </div>
          <p className={styles.sectionSummary}>{model.watch.summary}</p>
        </div>
        <div className={styles.rowStack}>
          {model.watch.rows.map((row, index) => (
            <WatchRow
              key={watchRowKey(row, index)}
              {...row}
              onSelectorOption={handleSelectorOption}
              onMarkComplete={handleMarkComplete}
              onAgingAction={handleAgingAction}
            />
          ))}
        </div>
      </div>
    </section>
  );
}

function toBreadcrumbs(crumbs: string[]): FolioBreadcrumbItem[] {
  return crumbs.map((label) => ({ label }));
}

function toReadinessStats(model: BriefingViewModel): ReadinessStat[] {
  return model.folio.readiness.map((item) => ({
    label: item.label,
    color:
      item.semantic === "blocked" || item.semantic === "needs_attention"
        ? "terracotta"
        : "sage",
  }));
}

function handleMovingNavigate(href: MovingEntityViewModel["href"]) {
  navigateToHref(href);
}

function handleNavNavigate(page: string) {
  const href = NAV_ROUTES[page];
  if (href) navigateToHref(href);
}

function handleSelectorOption(actionId: string, optionId: string) {
  logWatchAction("suggestedAction", actionId, optionId);
}

function handleMarkComplete(actionId: string) {
  logWatchAction("openAction", actionId);
}

function handleAgingAction(
  actionId: string,
  optionId: "restore" | "archive",
) {
  logWatchAction("aging", actionId, optionId);
}

function logWatchAction(
  kind: WatchRowViewModel["kind"],
  actionId: string,
  optionId?: string,
) {
  window.console.debug("DailyBriefingRedesign watch action", {
    kind,
    actionId,
    optionId,
  });
}

function watchRowKey(row: WatchRowViewModel, index: number): string {
  switch (row.kind) {
    case "parked":
      return `parked-${row.who}-${index}`;
    case "suggestedAction":
    case "openAction":
    case "aging":
      return `${row.kind}-${row.actionId}`;
    default: {
      const exhaustive: never = row;
      return exhaustive;
    }
  }
}

function navigateToHref(href: string) {
  if (!href) return;
  if (href.startsWith("/")) {
    window.history.pushState(null, "", href);
    return;
  }
  window.location.assign(href);
}

export default DailyBriefingRedesign;
