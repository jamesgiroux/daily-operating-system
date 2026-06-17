/**
 * DailyBriefing.tsx — Magazine editorial daily briefing
 *
 * A morning document, not a dashboard. You read it top to bottom.
 * When you reach the end, you're briefed.
 *
 * Sections: Day Frame > Schedule (Up Next) > Attention > Finis
 *
 * Design reference: design/mockups/daily-briefing-reimagined-v2.html
 * Layout: margin grid (100px label | content), section rules, no cards.
 */

import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { useCalendar } from "@/hooks/useCalendar";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";

import type { WorkflowStatus } from "@/hooks/useWorkflow";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import type {
  DashboardData,
  DataFreshness,
  Meeting,
  Action,
} from "@/types";
import { EditableBlockText, ItemFeedback } from "@/components/composition/blocks/BlockComponents";
import { normalizeTrustBand, type ProjectedBlock, type ProjectedComposition } from "@/services/composition/contracts";
import { DayChart, type DayChartMeeting, type DayChartMeetingState, type DayChartMeetingType } from "./DayChart";
import { DayStrip } from "./DayStrip";
import {
  MeetingSpineItem,
  type MeetingSpinePrepState,
  type MeetingSpineState,
  type MeetingSpineType,
} from "./MeetingSpineItem";
import s from "@/styles/editorial-briefing.module.css";
import briefingStyles from "./DailyBriefing.module.css";

// ─── Types ───────────────────────────────────────────────────────────────────

interface DailyBriefingProps {
  data: DashboardData;
  freshness: DataFreshness;
  onRunBriefing?: () => void;
  isRunning?: boolean;
  workflowStatus?: WorkflowStatus;
  onRefresh?: () => void;
}

// ─── Readiness Computation ───────────────────────────────────────────────────

function computeReadiness(meetings: Meeting[], actions: Action[]) {
  const externalMeetings = meetings.filter((m) =>
    ["customer", "qbr", "partnership", "external"].includes(m.type) &&
    m.overlayStatus !== "cancelled"
  );
  const preppedCount = externalMeetings.filter((m) =>
    m.intelligenceQuality && m.intelligenceQuality.level !== "sparse"
  ).length;
  const totalExternal = externalMeetings.length;
  const overdueActions = actions.filter((a) => a.isOverdue && a.status !== "completed");
  return { preppedCount, totalExternal, overdueCount: overdueActions.length };
}

function localDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function dateFromLocalDateKey(key: string): Date {
  const [year, month, day] = key.split("-").map((part) => Number(part));
  if (!year || !month || !day) return new Date();
  return new Date(year, month - 1, day);
}

type PayloadRecord = Record<string, unknown>;

const D_SPINE_SECTION_TOKENS = {
  lead: ["lead", "hero"],
  schedule: ["schedule", "today"],
  moving: ["moving", "attention"],
  watch: ["watch", "follow", "action"],
} as const;

function payloadText(value: unknown): string | null {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return null;
}

function payloadObject(value: unknown): PayloadRecord | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as PayloadRecord)
    : null;
}

function payloadArray(value: unknown): PayloadRecord[] {
  return Array.isArray(value)
    ? value.filter((item): item is PayloadRecord => Boolean(payloadObject(item)))
    : [];
}

function projectionBlocksForSection(
  projection: ProjectedComposition,
  tokens: readonly string[],
): ProjectedBlock[] {
  const blocksById = new Map(projection.blocks.map((block) => [block.block_id, block]));
  return projection.sections
    .filter((section) => {
      const haystack = `${section.section_id} ${section.label ?? ""}`.toLowerCase();
      return tokens.some((token) => haystack.includes(token));
    })
    .flatMap((section) =>
      section.block_ids
        .map((blockId) => blocksById.get(blockId))
        .filter((block): block is ProjectedBlock => Boolean(block)),
    );
}

function blockItems(block?: ProjectedBlock | null): PayloadRecord[] {
  return payloadArray(block?.payload.items);
}

function firstBlockWithItems(
  blocks: ProjectedBlock[],
  predicate: (item: PayloadRecord) => boolean,
): ProjectedBlock | null {
  return blocks.find((block) => blockItems(block).some(predicate)) ?? null;
}

function firstTextBlock(blocks: ProjectedBlock[]): ProjectedBlock | null {
  return blocks.find((block) =>
    Boolean(
      payloadText(block.payload.headline) ??
      payloadText(block.payload.text) ??
      payloadText(block.payload.summary) ??
      payloadText(block.payload.body),
    ),
  ) ?? null;
}

function readinessLabels(projection: ProjectedComposition): string[] {
  const block = projection.blocks.find((candidate) =>
    Boolean(
      payloadText(candidate.payload.availability_label) ||
      payloadText(candidate.payload.freshness_label) ||
      payloadText(candidate.payload.integrity_label),
    ),
  );
  if (!block) return [];
  return [
    payloadText(block.payload.availability_label),
    payloadText(block.payload.freshness_label),
    payloadText(block.payload.integrity_label),
  ].filter((label): label is string => Boolean(label));
}

function humanizeLabel(value: string | null): string | null {
  if (!value) return null;
  const normalized = value.replace(/[_-]+/g, " ").trim();
  if (!normalized) return null;
  return normalized.replace(/\b\w/g, (letter) => letter.toUpperCase());
}

function formatAsofLabel(value: string | null): string | null {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  const hasTime = /t\d{2}:\d{2}/i.test(value);
  const formatted = hasTime
    ? date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })
    : date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  return `as of ${formatted}`;
}

function parseDate(value: string | null): Date | null {
  if (!value) return null;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function formatMeetingTime(value: string | null): string {
  const date = parseDate(value);
  if (!date) return "TBD";
  return date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}

function formatMeetingDuration(startsAt: string | null, endsAt: string | null): string | undefined {
  const start = parseDate(startsAt);
  const end = parseDate(endsAt);
  if (!start || !end) return undefined;
  const minutes = Math.round((end.getTime() - start.getTime()) / 60000);
  return minutes > 0 ? formatMinutes(minutes) : undefined;
}

function meetingState(item: PayloadRecord, now: number): MeetingSpineState {
  const status = payloadText(item.status)?.toLowerCase() ?? "";
  if (status.includes("cancel")) return "cancelled";
  // Derive past/now/upcoming from the actual clock, not the producer's bucket
  // label — a meeting in the "upcoming" bucket can still be in the past.
  const start = parseDate(payloadText(item.starts_at));
  if (start) {
    const end = parseDate(payloadText(item.ends_at));
    const endMs = end && end > start ? end.getTime() : start.getTime() + 30 * 60000;
    if (now >= endMs) return "past";
    if (now >= start.getTime()) return "in-progress";
    return "upcoming";
  }
  const label = payloadText(item.label)?.toLowerCase() ?? "";
  if (status.includes("progress") || label.includes("current")) return "in-progress";
  if (status.includes("past") || status.includes("ended") || status.includes("complete")) return "past";
  return "upcoming";
}

function meetingType(item: PayloadRecord): MeetingSpineType {
  const entityType = payloadText(item.linked_entity_type)?.toLowerCase();
  if (entityType === "account" || entityType === "customer" || entityType === "company") {
    return "customer";
  }
  if (entityType === "person" || entityType === "contact") return "one_on_one";
  if (entityType === "project") return "project";
  if (entityType === "partner") return "partner";
  return "internal";
}

function prepStateForStatus(item: PayloadRecord, state: MeetingSpineState): MeetingSpinePrepState {
  // A past meeting carries its captured notes/outcomes, not a prep status.
  if (state === "past") return "captured";
  const lookup = `${payloadText(item.status) ?? ""} ${payloadText(item.status_label) ?? ""}`.toLowerCase();
  if (/build|prepar|pending|running|queued/.test(lookup)) return "building";
  // A present prep-context narrative means a briefing already exists — show it
  // as fresh rather than "needs prep" just because the pipeline status is stale.
  if (payloadText(item.context)) return "ready";
  if (/ready|fresh|current|available|clean/.test(lookup)) return "ready";
  if (/need|missing|unavailable|no briefing|prep needed|blocked/.test(lookup)) return "needs";
  return "none";
}

function chartType(item: PayloadRecord, now: number): DayChartMeetingType {
  if (meetingState(item, now) === "cancelled") return "cancel";
  const type = meetingType(item);
  if (type === "customer") return "customer";
  if (type === "partner" || type === "project") return "partner";
  if (type === "one_on_one") return "oo";
  return "internal";
}

function chartState(item: PayloadRecord, now: number): DayChartMeetingState {
  const state = meetingState(item, now);
  if (state === "in-progress") return "now";
  if (state === "past") return "past";
  if (state === "cancelled") return "cancelled";
  return "upcoming";
}

function chartPercent(date: Date): number {
  const workdayStart = 7;
  const workdayHours = 10;
  const hour = date.getHours() + date.getMinutes() / 60;
  return ((hour - workdayStart) / workdayHours) * 100;
}

function dayChartMeeting(item: PayloadRecord, index: number, now: number): DayChartMeeting | null {
  const start = parseDate(payloadText(item.starts_at));
  if (!start) return null;
  const end = parseDate(payloadText(item.ends_at));
  const fallbackEnd = new Date(start);
  fallbackEnd.setMinutes(fallbackEnd.getMinutes() + 30);
  const resolvedEnd = end && end > start ? end : fallbackEnd;
  const title = payloadText(item.text) ?? "Untitled meeting";
  const time = formatMeetingTime(payloadText(item.starts_at));
  const durationMinutes = Math.max(15, Math.round((resolvedEnd.getTime() - start.getTime()) / 60000));
  return {
    id: payloadText(item.linked_entity_id) ?? `${title}-${index}`,
    type: chartType(item, now),
    state: chartState(item, now),
    startPct: chartPercent(start),
    durationPct: (durationMinutes / (10 * 60)) * 100,
    title,
    time,
    tooltip: `${title} · ${time}`,
    ariaLabel: `${title}, ${time}`,
  };
}

function nowPositionForDate(selectedDate: Date, now: number): number | null {
  const current = new Date(now);
  if (localDateKey(selectedDate) !== localDateKey(current)) return null;
  const position = chartPercent(current);
  return position >= 0 && position <= 100 ? position : null;
}

function movingKind(item: PayloadRecord): "customer" | "person" | undefined {
  const entityType = payloadText(item.entity_type)?.toLowerCase();
  if (entityType === "person" || entityType === "contact") return "person";
  if (entityType === "account" || entityType === "customer" || entityType === "company") {
    return "customer";
  }
  return undefined;
}

function countLabel(count: number, singular: string, plural = `${singular}s`): string {
  return `${count} ${count === 1 ? singular : plural}`;
}

function ProjectedBriefingChapter({
  projection,
  briefingEntityId,
  selectedDate,
  now,
}: {
  projection: ProjectedComposition | null;
  briefingEntityId: string;
  selectedDate: Date;
  now: number;
}) {
  if (!projection || projection.blocks.length === 0) {
    return null;
  }

  const leadBlocks = projectionBlocksForSection(projection, D_SPINE_SECTION_TOKENS.lead);
  const scheduleBlocks = projectionBlocksForSection(projection, D_SPINE_SECTION_TOKENS.schedule);
  const movingBlocks = projectionBlocksForSection(projection, D_SPINE_SECTION_TOKENS.moving);
  const watchBlocks = projectionBlocksForSection(projection, D_SPINE_SECTION_TOKENS.watch);
  const leadBlock = firstTextBlock(leadBlocks);
  const scheduleBlock =
    firstBlockWithItems(scheduleBlocks, (item) => Boolean(payloadText(item.starts_at) || payloadText(item.ends_at))) ??
    firstBlockWithItems(projection.blocks, (item) => Boolean(payloadText(item.starts_at) || payloadText(item.ends_at)));
  const movingBlock =
    firstBlockWithItems(movingBlocks, (item) => Boolean(payloadText(item.body) || payloadText(item.entity_name))) ??
    firstBlockWithItems(projection.blocks, (item) => Boolean(payloadText(item.body) || payloadText(item.entity_name)));
  const watchBlock =
    watchBlocks.find((block) => block.selected_known_type_id === "action_list") ??
    firstBlockWithItems(watchBlocks, (item) => Boolean(payloadText(item.status_label) || payloadText(item.text))) ??
    projection.blocks.find((block) => block.selected_known_type_id === "action_list") ??
    null;
  const scheduleItems = [...blockItems(scheduleBlock)].sort((a, b) => {
    const ta = parseDate(payloadText(a.starts_at))?.getTime() ?? Number.MAX_SAFE_INTEGER;
    const tb = parseDate(payloadText(b.starts_at))?.getTime() ?? Number.MAX_SAFE_INTEGER;
    return ta - tb;
  });
  const movingItems = blockItems(movingBlock);
  const watchItems = blockItems(watchBlock);
  const readiness = readinessLabels(projection);
  const chartMeetings = scheduleItems
    .map((item, index) => dayChartMeeting(item, index, now))
    .filter((meeting): meeting is DayChartMeeting => Boolean(meeting));
  const hasDSpineContent =
    leadBlock || scheduleBlock || movingBlock || watchBlock || readiness.length > 0;

  if (!hasDSpineContent) return null;

  return (
    <section
      className={briefingStyles.projectedCompositionSection}
      data-composition-id={projection.composition_id}
      data-composition-version={projection.composition_version ?? 0}
    >
      <div
        data-ds-tier="surface"
        data-ds-name="DailyBriefingDSpine"
        data-ds-spec="surfaces/DailyBriefingDSpine.md"
        data-ds-state="proposed"
      >
        {scheduleBlock ? (
          <section id="schedule" className={s.scheduleSection}>
            <div className={s.marginGrid} data-ds-tier="pattern" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">
              <div className={s.marginLabel}>
                Today
                <span className={s.marginLabelCount}>
                  {countLabel(scheduleItems.length, "meeting")}
                </span>
              </div>
              <div className={s.marginContent}>
                <div className={s.sectionRule} />
                <h2 className={briefingStyles["dspine-section-heading"]}>Today's schedule</h2>

                {chartMeetings.length > 0 ? (
                  <DayChart
                    meetings={chartMeetings}
                    nowPosition={nowPositionForDate(selectedDate, now)}
                    chartHeight={118}
                  />
                ) : null}

                <div className={briefingStyles["dspine-stack"]}>
                  {scheduleItems.length > 0 ? (
                    scheduleItems.map((item, index) => {
                      const title = payloadText(item.text) ?? "Untitled meeting";
                      const entityLabel =
                        payloadText(item.linked_entity_name) ??
                        humanizeLabel(payloadText(item.linked_entity_type)) ??
                        "Unlinked";
                      const kindLabel = humanizeLabel(payloadText(item.kind));
                      const entityName = [entityLabel, kindLabel]
                        .filter(Boolean)
                        .join(" · ");
                      const state = meetingState(item, now);
                      const timeLabel = formatMeetingTime(payloadText(item.starts_at));
                      const meetingId = payloadText(item.meeting_id) ?? undefined;
                      const isUpNext = state === "upcoming" && payloadText(item.label) === "Next";
                      return (
                        <MeetingSpineItem
                          key={`${meetingId ?? payloadText(item.linked_entity_id) ?? title}-${index}`}
                          time={timeLabel}
                          duration={formatMeetingDuration(payloadText(item.starts_at), payloadText(item.ends_at))}
                          state={state}
                          type={meetingType(item)}
                          entityName={entityName}
                          title={title}
                          context={payloadText(item.context) ?? undefined}
                          attendees={payloadText(item.attendees) ?? undefined}
                          prepState={prepStateForStatus(item, state)}
                          showStatus={state === "in-progress" || isUpNext}
                          meetingId={meetingId}
                          data-trust-band={normalizeTrustBand(scheduleBlock.trust_band)}
                        />
                      );
                    })
                  ) : (
                    <p className={briefingStyles["dspine-section-summary"]}>
                      {payloadText(scheduleBlock.payload.empty_state_text) ?? "No meetings in this briefing."}
                    </p>
                  )}
                </div>
              </div>
            </div>
          </section>
        ) : null}

        <section id="moving" className={s.prioritiesSection}>
            <div className={s.marginGrid} data-ds-tier="pattern" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">
              <div className={s.marginLabel}>
                Moving
                <span className={s.marginLabelCount}>
                  {countLabel(movingItems.length, "change")}
                </span>
              </div>
              <div className={s.marginContent}>
                <div className={s.sectionRule} />
                <h2 className={briefingStyles["dspine-section-heading"]}>What's moving</h2>
                <div
                  className={briefingStyles["dspine-moving-list"]}
                  data-ds-tier="pattern"
                  data-ds-name="DailyBriefingAttentionSection"
                  data-ds-spec="patterns/DailyBriefingAttentionSection.md"
                  data-ds-variant="moving"
                >
                  {movingBlock && movingItems.length > 0 ? (
                    movingItems.map((item, index) => {
                      const title = payloadText(item.text) ?? "Untitled signal";
                      const body = payloadText(item.body);
                      const fieldPath = `/items/${index}/text`;
                      return (
                        <article
                          key={`${payloadText(item.claim_id) ?? title}-${index}`}
                          className={briefingStyles["dspine-moving-row"]}
                          data-kind={movingKind(item)}
                          data-trust-band={normalizeTrustBand(movingBlock.trust_band)}
                        >
                          <div className={briefingStyles["dspine-moving-name"]}>
                            {payloadText(item.entity_name) ?? humanizeLabel(payloadText(item.entity_type)) ?? "Signal"}
                          </div>
                          <div className={briefingStyles["dspine-moving-copy"]}>
                            <EditableBlockText
                              accountId={briefingEntityId}
                              entityType="briefing"
                              block={movingBlock}
                              fieldPath={fieldPath}
                              value={title}
                              as="div"
                              className={briefingStyles["dspine-moving-title"]}
                            />
                            {body ? (
                              <EditableBlockText
                                accountId={briefingEntityId}
                                entityType="briefing"
                                block={movingBlock}
                                fieldPath={`/items/${index}/body`}
                                value={body}
                                as="p"
                                className={briefingStyles["dspine-moving-context"]}
                              />
                            ) : null}
                            <ItemFeedback
                              accountId={briefingEntityId}
                              entityType="briefing"
                              block={movingBlock}
                              fieldPath={fieldPath}
                              value={title}
                            />
                          </div>
                          {formatAsofLabel(payloadText(item.source_asof)) ? (
                            <div className={briefingStyles["dspine-moving-meta"]}>
                              {formatAsofLabel(payloadText(item.source_asof))}
                            </div>
                          ) : null}
                        </article>
                      );
                    })
                  ) : (
                    <article className={briefingStyles["dspine-moving-row"]}>
                      <div className={briefingStyles["dspine-moving-name"]}>Moving</div>
                      <div className={briefingStyles["dspine-moving-copy"]}>
                        <div className={briefingStyles["dspine-moving-title"]}>
                          {payloadText(movingBlock?.payload?.empty_state_text) ?? "Nothing moving right now."}
                        </div>
                      </div>
                    </article>
                  )}
                </div>
              </div>
            </div>
          </section>

        <section
            id="watch"
            className={s.prioritiesSection}
            data-ds-tier="pattern"
            data-ds-name="DailyBriefingAttentionSection"
            data-ds-spec="patterns/DailyBriefingAttentionSection.md"
            data-ds-variant="watch"
          >
            <div className={s.marginGrid} data-ds-tier="pattern" data-ds-name="MarginGrid" data-ds-spec="patterns/MarginGrid.md">
              <div className={s.marginLabel}>
                Watch
                <span className={s.marginLabelCount}>
                  {countLabel(watchItems.length, "quiet")}
                </span>
              </div>
              <div className={s.marginContent}>
                <div className={s.sectionRule} />
                <h2 className={briefingStyles["dspine-section-heading"]}>Watch</h2>
                <div className={briefingStyles["dspine-watch-list"]}>
                  {watchBlock && watchItems.length > 0 ? (
                    watchItems.map((item, index) => {
                      const value = payloadText(item.text) ?? "Untitled follow-through";
                      const fieldPath = `/items/${index}/text`;
                      return (
                        <div
                          key={`${payloadText(item.claim_id) ?? value}-${index}`}
                          className={briefingStyles["dspine-watch-row"]}
                          data-trust-band={normalizeTrustBand(watchBlock.trust_band)}
                        >
                          <span className={briefingStyles["dspine-watch-who"]}>
                            {payloadText(item.status_label) ?? "Tracked"}
                          </span>
                          <span>
                            <EditableBlockText
                              accountId={briefingEntityId}
                              entityType="briefing"
                              block={watchBlock}
                              fieldPath={fieldPath}
                              value={value}
                              as="span"
                              className={briefingStyles["dspine-watch-what"]}
                            />
                            <ItemFeedback
                              accountId={briefingEntityId}
                              entityType="briefing"
                              block={watchBlock}
                              fieldPath={fieldPath}
                              value={value}
                            />
                          </span>
                          {formatAsofLabel(payloadText(item.source_asof)) ? (
                            <span className={briefingStyles["dspine-watch-quiet"]}>
                              {formatAsofLabel(payloadText(item.source_asof))}
                            </span>
                          ) : null}
                        </div>
                      );
                    })
                  ) : (
                    <div className={briefingStyles["dspine-watch-row"]}>
                      <span className={briefingStyles["dspine-watch-who"]}>Watch</span>
                      <span className={briefingStyles["dspine-watch-what"]}>
                        {payloadText(watchBlock?.payload?.empty_state_text) ?? "Nothing on watch right now."}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </section>

        <FinisMarker />
      </div>
    </section>
  );
}

// ─── Capacity Formatting ─────────────────────────────────────────────────────

function formatMinutes(minutes: number): string {
  if (minutes < 60) return `${minutes}m`;
  const hrs = Math.floor(minutes / 60);
  const rem = minutes % 60;
  return rem > 0 ? `${hrs}h ${rem}m` : `${hrs}h`;
}

// ─── Component ───────────────────────────────────────────────────────────────

export function DailyBriefing({ data, freshness, onRunBriefing, isRunning, workflowStatus }: DailyBriefingProps) {
  const { now } = useCalendar();
  const [projectedBriefingDateKey, setProjectedBriefingDateKey] = useState(() =>
    localDateKey(new Date(now)),
  );
  const projectedBriefingDate = useMemo(
    () => dateFromLocalDateKey(projectedBriefingDateKey),
    [projectedBriefingDateKey],
  );
  const projectedBriefingEntityId = `local~${projectedBriefingDateKey}`;
  const projectedBriefing = useProjectedComposition({
    entityType: "briefing",
    entityId: projectedBriefingEntityId,
  });
  const handleProjectedBriefingDateSelect = useCallback((date: Date) => {
    setProjectedBriefingDateKey(localDateKey(date));
  }, []);
  const hasMountedProjectedBriefingRefreshRef = useRef(false);
  const freshnessRevision = freshness.freshness === "unknown" ? "unknown" : freshness.generatedAt;
  const refreshProjectedBriefing = useCallback(() => {
    void projectedBriefing.refetch({ forceRefresh: true });
  }, [projectedBriefing.refetch]);
  useEffect(() => {
    if (!hasMountedProjectedBriefingRefreshRef.current) {
      hasMountedProjectedBriefingRefreshRef.current = true;
      return;
    }
    refreshProjectedBriefing();
  }, [data, freshnessRevision, refreshProjectedBriefing]);
  const handleRunBriefing = useCallback(() => {
    onRunBriefing?.();
    refreshProjectedBriefing();
  }, [onRunBriefing, refreshProjectedBriefing]);
  // Data
  const meetings = data.meetings;
  const actions = data.actions;

  // Readiness
  const readiness = computeReadiness(meetings, actions);

  // Date formatting
  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // Register magazine shell with folio bar
  const folioReadinessStats = useMemo(() => {
    const stats: ReadinessStat[] = [];
    if (readiness.totalExternal > 0) {
      // Use intelligence quality levels when available, fall back to hasPrep count
      const externalMeetings = meetings.filter((m) =>
        ["customer", "qbr", "partnership", "external"].includes(m.type) &&
        m.overlayStatus !== "cancelled"
      );
      const hasQualityData = externalMeetings.some((m) => m.intelligenceQuality);

      if (hasQualityData) {
        const readyCount = externalMeetings.filter(
          (m) => m.intelligenceQuality?.level === "ready" || m.intelligenceQuality?.level === "fresh"
        ).length;
        const buildingCount = externalMeetings.filter(
          (m) => m.intelligenceQuality?.level === "developing"
        ).length;

        if (readyCount === readiness.totalExternal) {
          stats.push({ label: `${readyCount}/${readiness.totalExternal} briefings ready`, color: "sage" });
        } else if (buildingCount > 0) {
          stats.push({ label: `${readyCount} briefings ready, ${buildingCount} limited`, color: "sage" });
        } else {
          stats.push({ label: `${readyCount}/${readiness.totalExternal} briefings ready`, color: "sage" });
        }
      } else {
        stats.push({ label: `${readiness.preppedCount}/${readiness.totalExternal} briefings ready`, color: "sage" });
      }
    }
    if (readiness.overdueCount > 0) {
      stats.push({ label: `${readiness.overdueCount} overdue`, color: "terracotta" });
    }
    return stats;
  }, [meetings, readiness.preppedCount, readiness.totalExternal, readiness.overdueCount]);

  const folioActions = useMemo(() => {
    if (!onRunBriefing) return undefined;
    const phaseLabel = isRunning && workflowStatus?.status === "running"
      ? { preparing: "Preparing…", enriching: "AI Processing…", delivering: "Delivering…" }[workflowStatus.phase]
      : null;
    return (
      <FolioRefreshButton
        onClick={handleRunBriefing}
        loading={!!isRunning}
        loadingLabel={phaseLabel ?? "Running\u2026"}
        title={isRunning ? "Briefing in progress" : "Refresh emails, actions, and context"}
      />
    );
  }, [handleRunBriefing, onRunBriefing, isRunning, workflowStatus]);

  const shellConfig = useMemo(
    () => ({
      folioLabel: "Daily Briefing",
      atmosphereColor: "turmeric" as const,
      activePage: "today" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioReadinessStats,
      folioActions,
    }),
    [formattedDate, folioReadinessStats, folioActions],
  );
  useRegisterMagazineShell(shellConfig);

  // Schedule stats
  const activeMeetings = meetings.filter((m) => m.overlayStatus !== "cancelled");

  // Build hero narrative from overview
  const heroHeadline = data.overview.summary || (activeMeetings.length === 0
    ? "A clear day. Nothing needs you."
    : "Your day is ready.");

  return (
    <div>
      <DayStrip
        selectedDate={projectedBriefingDate}
        today={new Date(now)}
        onSelectDate={handleProjectedBriefingDateSelect}
      />

      {/* ═══ DAY FRAME (Hero + Focus) ═══ */}
      <section className={s.hero}>
        <h1 className={s.heroHeadline}>{heroHeadline}</h1>

        {/* Capacity + focus directive */}
        {data.focus && (() => {
          const deepWorkBlocks = data.focus.availableBlocks.filter((b) => b.durationMinutes >= 60).length;
          return (
            <div className={s.focusCapacity}>
              {formatMinutes(data.focus.availableMinutes)} available
              {deepWorkBlocks > 0 && (
                <> &middot; {deepWorkBlocks} deep work block{deepWorkBlocks !== 1 ? "s" : ""}</>
              )}
              {" "}&middot; {data.focus.meetingCount} meeting{data.focus.meetingCount !== 1 ? "s" : ""}
            </div>
          );
        })()}

        {data.overview.focus && (
          <div className={s.focusBlock}>
            <div className={s.focusText}>{data.overview.focus}</div>
          </div>
        )}
      </section>

      <ProjectedBriefingChapter
        projection={projectedBriefing.data?.projection ?? null}
        briefingEntityId={projectedBriefingEntityId}
        selectedDate={projectedBriefingDate}
        now={now}
      />

    </div>
  );
}
