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
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useSuggestedActions } from "@/hooks/useSuggestedActions";
// SuggestedActionRow removed from briefing — suggestions live on /actions page
import clsx from "clsx";
import { useCalendar } from "@/hooks/useCalendar";
import { useDailyBriefingAbility } from "@/hooks/useDailyBriefingAbility";
import { useProjectedComposition } from "@/hooks/useProjectedComposition";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import {
  BriefingMeetingCard,
  getTemporalState,
} from "./BriefingMeetingCard";
import { FolioRefreshButton } from "@/components/ui/folio-refresh-button";

import type { WorkflowStatus } from "@/hooks/useWorkflow";
import { FinisMarker } from "@/components/editorial/FinisMarker";
import { formatShortDate, stripMarkdown } from "@/lib/utils";
import { EmailEntityChip } from "@/components/ui/email-entity-chip";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  BriefingCallout,
  DashboardData,
  DashboardLifecycleUpdate,
  DataFreshness,
  RenderedProvenanceSummary,
  Meeting,
  Action,
  Email,
  PrioritizedAction,
} from "@/types";
import { HealthBadge } from "@/components/shared/HealthBadge";
import { TrustBandIndicator } from "@/components/ui/TrustBandIndicator";
import { Pill } from "@/components/ui/Pill";
import { EditableBlockText, ItemFeedback } from "@/components/composition/blocks/BlockComponents";
import { compareEmailRank } from "@/lib/email-ranking";
import type { DailyBriefingOutput } from "@/services/daily-briefing/contracts";
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

// ─── Up Next Selection ──────────────────────────────────────────────────────

function parseDisplayTimeMs(timeStr: string | undefined): number | null {
  if (!timeStr) return null;
  const match = timeStr.match(/^(\d{1,2}):(\d{2})\s*(AM|PM)$/i);
  if (!match) return null;
  let hours = parseInt(match[1], 10);
  const minutes = parseInt(match[2], 10);
  const period = match[3].toUpperCase();
  if (period === "PM" && hours !== 12) hours += 12;
  if (period === "AM" && hours === 12) hours = 0;
  const d = new Date();
  d.setHours(hours, minutes, 0, 0);
  return d.getTime();
}

function getMeetingStartMs(meeting: Meeting): number | null {
  return parseDisplayTimeMs(meeting.time);
}

/** Find the first upcoming (not past, not cancelled) meeting. */
function findUpNextMeeting(meetings: Meeting[], now: number): Meeting | null {
  const upcoming = meetings.filter((m) => {
    const state = getTemporalState(m, now);
    return state !== "past" && state !== "cancelled";
  });
  if (upcoming.length === 0) return null;
  // Sort by start time, return earliest
  return upcoming.sort((a, b) => {
    const ta = getMeetingStartMs(a) ?? Infinity;
    const tb = getMeetingStartMs(b) ?? Infinity;
    return ta - tb;
  })[0];
}

/** Find high-stakes meetings (QBR/customer) that lack prep — for prep flags. */
function findUnpreppedHighStakes(meetings: Meeting[], now: number, upNextId?: string): Meeting[] {
  return meetings.filter((m) => {
    if (m.id === upNextId) return false;
    const state = getTemporalState(m, now);
    if (state === "past" || state === "cancelled") return false;
    const isHighStakes = ["qbr", "customer"].includes(m.type);
    const needsPrep = !m.intelligenceQuality || m.intelligenceQuality.level === "sparse";
    return isHighStakes && needsPrep;
  });
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

function renderedProvenanceSourceCount(renderedProvenance?: RenderedProvenanceSummary | null): number {
  const value = renderedProvenance?.value;
  const sources = value?.sources;
  if (Array.isArray(sources)) {
    return sources.length;
  }
  const aboutThis = value?.about_this;
  if (aboutThis && typeof aboutThis === "object") {
    const summary = (aboutThis as { summary?: unknown }).summary;
    if (summary && typeof summary === "object") {
      const sourceCount = (summary as { source_count?: unknown; sourceCount?: unknown }).source_count
        ?? (summary as { source_count?: unknown; sourceCount?: unknown }).sourceCount;
      if (typeof sourceCount === "number") {
        return sourceCount;
      }
    }
  }
  return 0;
}

function briefingFreshnessLabel(state: DailyBriefingOutput["state"]): string {
  switch (state.freshness.kind) {
    case "fresh":
      return "Current";
    case "stale":
      return "Context may be stale";
    case "needs_preparation": {
      const count = state.freshness.meetingIds.length;
      return `${count} briefing${count === 1 ? "" : "s"} need prep`;
    }
    default:
      return "Current";
  }
}

function briefingAdvisoryLabel(state: DailyBriefingOutput["state"]): string | null {
  const advisory = state.advisories[0];
  if (!advisory) return null;
  switch (advisory.kind) {
    case "unlinked_meetings":
      return `Link ${advisory.meetingIds.length} meeting${advisory.meetingIds.length === 1 ? "" : "s"} for fuller context`;
    case "partial_read_failure":
      return "Some sources are unavailable";
    case "watch_proposal":
      return advisory.summary;
    default:
      return null;
  }
}

function DailyBriefingAbilityStrip({
  output,
  renderedProvenance,
  error,
}: {
  output?: DailyBriefingOutput | null;
  renderedProvenance?: RenderedProvenanceSummary | null;
  error?: string | null;
}) {
  if (!output) {
    return error ? (
      <div className={briefingStyles.abilityStrip} data-testid="daily-briefing-ability-strip">
        <span className={briefingStyles.abilityStripLabel}>Briefing context unavailable</span>
      </div>
    ) : null;
  }

  const sourceCount = renderedProvenanceSourceCount(renderedProvenance);
  const advisory = briefingAdvisoryLabel(output.state);
  const trustBand = output.trustSummary.aggregateBand;

  return (
    <div className={briefingStyles.abilityStrip} data-testid="daily-briefing-ability-strip">
      <span className={briefingStyles.abilityStripLabel}>
        {briefingFreshnessLabel(output.state)}
      </span>
      <span className={briefingStyles.abilityStripMeta}>
        Trust
        <TrustBandIndicator band={trustBand} />
        <span>{trustBand.replace(/_/g, " ")}</span>
      </span>
      {sourceCount > 0 && (
        <span className={briefingStyles.abilityStripMeta}>
          {sourceCount} source{sourceCount === 1 ? "" : "s"}
        </span>
      )}
      {advisory && (
        <span className={briefingStyles.abilityStripAdvisory}>{advisory}</span>
      )}
    </div>
  );
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

function projectedLeadText(block: ProjectedBlock | null): string | null {
  if (!block) return null;
  return (
    payloadText(block.payload.headline) ??
    payloadText(block.payload.text) ??
    payloadText(block.payload.summary) ??
    payloadText(block.payload.body)
  );
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

function meetingState(item: PayloadRecord): MeetingSpineState {
  const status = payloadText(item.status)?.toLowerCase() ?? "";
  const label = payloadText(item.label)?.toLowerCase() ?? "";
  if (status.includes("cancel")) return "cancelled";
  if (status.includes("progress") || status.includes("current") || label.includes("current")) {
    return "in-progress";
  }
  if (status.includes("past") || status.includes("ended") || status.includes("complete")) {
    return "past";
  }
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

function prepStateForStatus(item: PayloadRecord): MeetingSpinePrepState {
  const statusLabel = payloadText(item.status_label);
  if (!statusLabel) return "none";
  const lookup = `${payloadText(item.status) ?? ""} ${statusLabel}`.toLowerCase();
  if (/need|missing|unavailable|no briefing/.test(lookup)) return "needs";
  if (/build|prepar|pending|running/.test(lookup)) return "building";
  if (/captur|complete|done|notes/.test(lookup)) return "captured";
  if (/ready|fresh|current|available|clean/.test(lookup)) return "ready";
  return "none";
}

function chartType(item: PayloadRecord): DayChartMeetingType {
  if (meetingState(item) === "cancelled") return "cancel";
  const type = meetingType(item);
  if (type === "customer") return "customer";
  if (type === "partner" || type === "project") return "partner";
  if (type === "one_on_one") return "oo";
  return "internal";
}

function chartState(item: PayloadRecord): DayChartMeetingState {
  const state = meetingState(item);
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

function dayChartMeeting(item: PayloadRecord, index: number): DayChartMeeting | null {
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
    type: chartType(item),
    state: chartState(item),
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
  const scheduleItems = blockItems(scheduleBlock);
  const movingItems = blockItems(movingBlock);
  const watchItems = blockItems(watchBlock);
  const leadText = projectedLeadText(leadBlock) ?? "Your day is ready.";
  const readiness = readinessLabels(projection);
  const chartMeetings = scheduleItems
    .map((item, index) => dayChartMeeting(item, index))
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
        <section
          id="lead"
          className={s.hero}
          data-ds-tier="pattern"
          data-ds-name="Lead"
          data-ds-spec="patterns/Lead.md"
          data-ds-state="proposed"
          data-trust-band={leadBlock ? normalizeTrustBand(leadBlock.trust_band) : undefined}
        >
          <h1 className={s.heroHeadline}>{leadText}</h1>
          {readiness.length > 0 ? (
            <div className={briefingStyles.abilityStrip} aria-label="Briefing readiness">
              {readiness.map((label) => (
                <Pill key={label} tone="neutral" size="compact">
                  {label}
                </Pill>
              ))}
            </div>
          ) : null}
        </section>

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
                      const entityName =
                        payloadText(item.linked_entity_name) ??
                        humanizeLabel(payloadText(item.linked_entity_type)) ??
                        "Unlinked";
                      const state = meetingState(item);
                      const timeLabel = formatMeetingTime(payloadText(item.starts_at));
                      const statusLabel = payloadText(item.status_label);
                      return (
                        <MeetingSpineItem
                          key={`${payloadText(item.linked_entity_id) ?? title}-${index}`}
                          time={timeLabel}
                          duration={formatMeetingDuration(payloadText(item.starts_at), payloadText(item.ends_at))}
                          state={state}
                          type={meetingType(item)}
                          entityName={entityName}
                          title={title}
                          attendees={formatAsofLabel(payloadText(item.source_asof))}
                          prepState={prepStateForStatus(item)}
                          prepLabel={statusLabel}
                          statusLabel={payloadText(item.label)}
                          showStatus={Boolean(payloadText(item.label)) || state === "in-progress"}
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

        {movingBlock ? (
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
                  {movingItems.length > 0 ? (
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
                          {payloadText(movingBlock.payload.empty_state_text) ?? "Nothing to surface."}
                        </div>
                      </div>
                    </article>
                  )}
                </div>
              </div>
            </div>
          </section>
        ) : null}

        {watchBlock ? (
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
                  {watchItems.length > 0 ? (
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
                        {payloadText(watchBlock.payload.empty_state_text) ?? "Nothing parked for later."}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          </section>
        ) : null}

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

export function DailyBriefing({ data, freshness, onRunBriefing, isRunning, workflowStatus, onRefresh }: DailyBriefingProps) {
  const { now, currentMeeting } = useCalendar();
  const dailyBriefingAbility = useDailyBriefingAbility();
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
  const refreshBriefingSurface = useCallback(() => {
    onRefresh?.();
    refreshProjectedBriefing();
  }, [onRefresh, refreshProjectedBriefing]);
  const [completedIds, setCompletedIds] = useState<Set<string>>(new Set());
  const [pendingLifecycleChangeId, setPendingLifecycleChangeId] = useState<number | null>(null);
  const [correctionTarget, setCorrectionTarget] = useState<DashboardLifecycleUpdate | null>(null);
  const [correctedLifecycle, setCorrectedLifecycle] = useState("");
  const [correctedStage, setCorrectedStage] = useState("");
  const [correctionNotes, setCorrectionNotes] = useState("");
  // Data
  const meetings = data.meetings;
  const actions = data.actions;
  const emails = data.emails ?? [];
  const lifecycleUpdates = data.lifecycleUpdates ?? [];

  // Score-based email selection from metadata-linked inbox rows.
  // Shows up to 5 emails, high-scored first.
  // that didn't meet the score threshold (avoids hiding useful intelligence).
  // Cached emails shown immediately even when briefing is stale — background
  // reconciliation will remove archived ones within seconds.
  const briefingEmails = (() => {
    if (emails.length === 0) return [];
    // Only consider emails linked to a known entity (account/person/project).
    // Unlinked emails (newsletters, marketing, stock alerts) should never
    // appear on the daily briefing — the briefing is about your portfolio.
    const entityLinked = emails.filter((e) => e.entityId);
    const ranked = [...entityLinked].sort(compareEmailRank);
    const scored = ranked
      .filter((e) => (e.relevanceScore ?? 0) >= 0.15)
      .slice(0, 5);
    const scoredIds = new Set(scored.map((e) => e.id));
    // Fill remaining slots with lower-scored metadata-linked emails.
    const enrichedFill = ranked
      .filter((e) => !scoredIds.has(e.id) && e.summary && e.summary.trim().length > 0)
      .slice(0, Math.max(0, 5 - scored.length));
    const selected = [...scored, ...enrichedFill].slice(0, 5);
    // No fallback — if no entity-linked emails pass filters, show nothing.
    // Showing unfiltered emails by recency defeats the purpose of the briefing.
    return selected;
  })();
  const emailSectionLabel = briefingEmails.length > 0 ? "WORTH YOUR ATTENTION" : "";

  // Up Next meeting (first upcoming, not past/cancelled)
  const upNext = findUpNextMeeting(meetings, now);
  const unpreppedHighStakes = findUnpreppedHighStakes(meetings, now, upNext?.id);
  // Only show meetings with invitees — filter out personal/solo calendar blocks
  // (classified as "personal" by google_api/classify.rs rule 2: 0-1 attendees)
  const scheduleMeetings = meetings.filter((m) => m.type !== "personal");

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

  // Pending actions (sorted by urgency)
  const pendingActions = actions
    .filter((a) => a.status !== "completed" && !completedIds.has(a.id))
    .sort((a, b) => {
      if (a.isOverdue && !b.isOverdue) return -1;
      if (!a.isOverdue && b.isOverdue) return 1;
      return (a.priority ?? 3) - (b.priority ?? 3);
    });

  // Action completion
  const handleComplete = useCallback((id: string) => {
    setCompletedIds((prev) => new Set(prev).add(id));
    invoke("complete_action", { id })
      .then(refreshProjectedBriefing)
      .catch((err) => {
        console.error("complete_action failed:", err);
        toast.error("Failed to complete action");
      });
  }, [refreshProjectedBriefing]);

  const handleConfirmLifecycle = useCallback(async (update: DashboardLifecycleUpdate) => {
    setPendingLifecycleChangeId(update.changeId);
    try {
      await invoke("confirm_lifecycle_change", { changeId: update.changeId });
      toast.success(`${update.accountName} marked confirmed`);
      refreshBriefingSurface();
    } catch (err) {
      console.error("confirm_lifecycle_change failed:", err);
      toast.error("Failed to confirm lifecycle change");
    } finally {
      setPendingLifecycleChangeId(null);
    }
  }, [refreshBriefingSurface]);

  const openCorrection = useCallback((update: DashboardLifecycleUpdate) => {
    setCorrectionTarget(update);
    setCorrectedLifecycle(update.newLifecycle);
    setCorrectedStage(update.renewalStage ?? "");
    setCorrectionNotes(update.evidence ?? "");
  }, []);

  const closeCorrection = useCallback((open: boolean) => {
    if (open) return;
    setCorrectionTarget(null);
    setCorrectedLifecycle("");
    setCorrectedStage("");
    setCorrectionNotes("");
  }, []);

  const handleSubmitCorrection = useCallback(async () => {
    if (!correctionTarget) return;
    setPendingLifecycleChangeId(correctionTarget.changeId);
    try {
      await invoke("correct_lifecycle_change", {
        changeId: correctionTarget.changeId,
        correctedLifecycle,
        correctedStage: correctedStage || null,
        notes: correctionNotes.trim() || null,
      });
      toast.success(`${correctionTarget.accountName} updated`);
      setCorrectionTarget(null);
      setCorrectedLifecycle("");
      setCorrectedStage("");
      setCorrectionNotes("");
      refreshBriefingSurface();
    } catch (err) {
      console.error("correct_lifecycle_change failed:", err);
      toast.error("Failed to correct lifecycle change");
    } finally {
      setPendingLifecycleChangeId(null);
    }
  }, [correctionNotes, correctedLifecycle, correctedStage, correctionTarget, refreshBriefingSurface]);

  // Proposed actions for triage
  const { suggestedActions, acceptAction, rejectAction } = useSuggestedActions();

  // Meeting actions helper: find actions related to a specific meeting
  const getActionsForMeeting = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId && a.status !== "completed");
  }, [actions]);

  // Meeting outcomes counts for past meeting summary lines
  const getCapturedActionCount = useCallback((meetingId: string) => {
    return actions.filter((a) => a.source === meetingId).length;
  }, [actions]);

  const getSuggestedActionCount = useCallback((meetingId: string) => {
    return suggestedActions.filter((a) => a.sourceId === meetingId).length;
  }, [suggestedActions]);

  // Schedule stats
  const activeMeetings = meetings.filter((m) => m.overlayStatus !== "cancelled");
  const hasSchedule = scheduleMeetings.some((m) => m.overlayStatus !== "cancelled");
  const scheduleCount = scheduleMeetings.filter((m) => m.overlayStatus !== "cancelled").length;

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
      <ProjectedBriefingChapter
        projection={projectedBriefing.data?.projection ?? null}
        briefingEntityId={projectedBriefingEntityId}
        selectedDate={projectedBriefingDate}
        now={now}
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

        <DailyBriefingAbilityStrip
          output={dailyBriefingAbility.response?.data}
          renderedProvenance={dailyBriefingAbility.response?.rendered_provenance}
          error={dailyBriefingAbility.error}
        />

        {/* Staleness indicator removed — orphaned "Last updated" with no date
            was confusing. The hero headline already communicates state. */}
      </section>

      {/* ═══ SCHEDULE (with Up Next) ═══ */}
      {hasSchedule && (
        <section className={s.scheduleSection}>
          <div className={s.marginGrid}>
            <div className={s.marginLabel}>
              Schedule
              <span className={s.marginLabelCount}>{scheduleCount} meetings</span>
            </div>
            <div className={s.marginContent}>
              <div className={s.sectionRule} />

              {/* Prep flags for high-stakes meetings without prep */}
              {unpreppedHighStakes.map((m) => (
                <div key={m.id} className={s.prepFlag}>
                  <Link
                    to="/meeting/$meetingId"
                    params={{ meetingId: m.id }}
                    className={briefingStyles.linkUnstyled}
                  >
                    &#9888; {m.title} at {m.time} — no briefing yet
                  </Link>
                </div>
              ))}

              <div className={s.scheduleRows}>
                {scheduleMeetings.map((meeting) => {
                  // Find health data for first linked account
                  const healthMap = data.entityHealthMap;
                  const linkedAccountHealth = healthMap && meeting.linkedEntities
                    ? meeting.linkedEntities
                        .filter((e) => e.entityType === "account" && healthMap[e.id])
                        .map((e) => ({ entity: e, health: healthMap[e.id] }))[0]
                    : undefined;

                  return (
                    <div key={meeting.id}>
                      <BriefingMeetingCard
                        meeting={meeting}
                        now={now}
                        currentMeeting={currentMeeting}
                        meetingActions={getActionsForMeeting(meeting.id)}
                        onComplete={handleComplete}
                        completedIds={completedIds}
                        onEntitiesChanged={refreshBriefingSurface}
                        capturedActionCount={getCapturedActionCount(meeting.id)}
                        suggestedActionCount={getSuggestedActionCount(meeting.id)}
                        isUpNext={upNext?.id === meeting.id}
                        userDomain={data.userDomains?.[0]}
                      />
                      {linkedAccountHealth && (
                        <div className={briefingStyles.healthBadgeRow}>
                          <HealthBadge
                            score={linkedAccountHealth.health.score}
                            band={linkedAccountHealth.health.band}
                            trend={linkedAccountHealth.health.trend}
                            sufficientData={linkedAccountHealth.health.sufficientData}
                            size="compact"
                          />
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </section>
      )}

      {/* ═══ ATTENTION ═══ */}
      {/* Cached emails shown even when stale — background reconciliation updates them */}
      <AttentionSection
        lifecycleUpdates={lifecycleUpdates}
        briefingCallouts={data.briefingCallouts ?? []}
        onConfirmLifecycle={handleConfirmLifecycle}
        onOpenLifecycleCorrection={openCorrection}
        pendingLifecycleChangeId={pendingLifecycleChangeId}
        suggestedActions={suggestedActions}
        acceptAction={acceptAction}
        rejectAction={rejectAction}
        focus={data.focus}
        pendingActions={pendingActions}
        completedIds={completedIds}
        onComplete={handleComplete}
        briefingEmails={briefingEmails}
        emailSectionLabel={emailSectionLabel}
        todayMeetingIds={new Set(meetings.map((m) => m.id))}
        emailSyncTimestamp={data.emailSync?.lastSuccessAt}
        agingActionCount={data.agingActionCount}
      />

      <Dialog open={!!correctionTarget} onOpenChange={closeCorrection}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Fix lifecycle change</DialogTitle>
            <DialogDescription>
              Update the lifecycle call for {correctionTarget?.accountName ?? "this account"}.
            </DialogDescription>
          </DialogHeader>
          <div className={briefingStyles.correctionFormGrid}>
            <label className={briefingStyles.correctionFieldLabel}>
              <span className={briefingStyles.correctionMonoLabel}>
                Lifecycle
              </span>
              <select
                value={correctedLifecycle}
                onChange={(event) => setCorrectedLifecycle(event.target.value)}
                className={briefingStyles.correctionSelect}
              >
                {["onboarding", "active", "renewing", "at_risk", "churned"].map((value) => (
                  <option key={value} value={value}>
                    {value.replace(/_/g, " ")}
                  </option>
                ))}
              </select>
            </label>
            <label className={briefingStyles.correctionFieldLabel}>
              <span className={briefingStyles.correctionMonoLabel}>
                Renewal stage
              </span>
              <select
                value={correctedStage}
                onChange={(event) => setCorrectedStage(event.target.value)}
                className={briefingStyles.correctionSelect}
              >
                <option value="">No stage</option>
                {["approaching", "negotiating", "contract_sent", "processed"].map((value) => (
                  <option key={value} value={value}>
                    {value.replace(/_/g, " ")}
                  </option>
                ))}
              </select>
            </label>
            <label className={briefingStyles.correctionFieldLabel}>
              <span className={briefingStyles.correctionMonoLabel}>
                Notes
              </span>
              <textarea
                value={correctionNotes}
                onChange={(event) => setCorrectionNotes(event.target.value)}
                rows={4}
                className={briefingStyles.correctionTextarea}
              />
            </label>
            <div className={briefingStyles.correctionButtonRow}>
              <button
                type="button"
                onClick={() => closeCorrection(false)}
                className={briefingStyles.correctionCancelBtn}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => { void handleSubmitCorrection(); }}
                disabled={!correctedLifecycle || pendingLifecycleChangeId === correctionTarget?.changeId}
                className={briefingStyles.correctionSubmitBtn}
              >
                Save correction
              </button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {/* ═══ FINIS ═══ */}
      <FinisMarker />
    </div>
  );
}

// ─── Attention Section (unified: suggested + actions + emails) ─────────────────

function AttentionSection({
  lifecycleUpdates,
  briefingCallouts,
  onConfirmLifecycle,
  onOpenLifecycleCorrection,
  pendingLifecycleChangeId,
  suggestedActions: _suggestedActions,
  acceptAction: _acceptAction,
  rejectAction: _rejectAction,
  focus,
  pendingActions,
  completedIds,
  onComplete,
  briefingEmails,
  emailSectionLabel,
  todayMeetingIds,
  emailSyncTimestamp,
  agingActionCount,
}: {
  lifecycleUpdates: DashboardLifecycleUpdate[];
  briefingCallouts: BriefingCallout[];
  onConfirmLifecycle: (update: DashboardLifecycleUpdate) => void;
  onOpenLifecycleCorrection: (update: DashboardLifecycleUpdate) => void;
  pendingLifecycleChangeId: number | null;
  suggestedActions: Array<{ id: string; title: string; sourceLabel?: string; sourceId?: string }>;
  acceptAction: (id: string) => void;
  rejectAction: (
    id: string,
    source?: "actions_page" | "daily_briefing" | "meeting_detail" | "account_detail_work"
  ) => void;
  focus: DashboardData["focus"];
  pendingActions: Action[];
  completedIds: Set<string>;
  onComplete: (id: string) => void;
  briefingEmails: Email[];
  emailSectionLabel: string;
  todayMeetingIds: Set<string>;
  emailSyncTimestamp?: string;
  agingActionCount?: number;
}) {
  // Filter attention-worthy actions: meeting-relevant for today OR overdue (max 3)
  const attentionActions = useMemo(() => {
    const prioritized = focus?.prioritizedActions ?? [];

    // Meeting-relevant: actions whose sourceId matches a today meeting
    const meetingRelevant = prioritized.filter(
      (pa) => pa.action.status !== "completed" && pa.action.sourceId && todayMeetingIds.has(pa.action.sourceId)
    );
    // Overdue/at-risk
    const atRisk = prioritized.filter(
      (pa) => pa.action.status !== "completed" && pa.atRisk && !meetingRelevant.includes(pa)
    );

    // If we have prioritized actions, use them
    if (meetingRelevant.length > 0 || atRisk.length > 0) {
      return [...meetingRelevant, ...atRisk].slice(0, 3);
    }

    // Fallback: use raw pending actions (overdue first, then meeting-relevant)
    const overdueRaw = pendingActions.filter((a) => a.isOverdue);
    const meetingRaw = pendingActions.filter(
      (a) => !a.isOverdue && a.source && todayMeetingIds.has(a.source)
    );
    return [...overdueRaw, ...meetingRaw].slice(0, 3);
  }, [focus, pendingActions, todayMeetingIds]);

  // Suggested actions removed from briefing — too noisy. Live on /actions page.
  const hasActions = attentionActions.length > 0;
  const hasEmails = briefingEmails.length > 0;
  // Filter lifecycle to today only — stale updates belong on the account detail page
  const todayLifecycle = useMemo(() => {
    const today = new Date().toISOString().slice(0, 10);
    return lifecycleUpdates.filter((u) => u.createdAt?.startsWith(today));
  }, [lifecycleUpdates]);
  const hasLifecycle = todayLifecycle.length > 0;
  // Callouts disabled: raw signal data violates ADR-0083 vocabulary rules.
  const hasCallouts = false;
  const hasAging = (agingActionCount ?? 0) > 0;
  const hasAnything = hasLifecycle || hasActions || hasEmails || hasAging;

  if (!hasAnything) return null;

  // Determine if attentionActions are PrioritizedAction or raw Action.
  // Check the actual items in attentionActions, not the source array — the useMemo
  // may have fallen through to raw actions even when prioritizedActions exists.
  const hasPrioritizedActions = attentionActions.length > 0
    && attentionActions.every((item) => "action" in item && (item).action?.id);

  return (
    <section className={s.prioritiesSection}>
      <div className={s.marginGrid}>
        <div className={s.marginLabel}>Attention</div>
        <div className={s.marginContent}>
          <div className={s.sectionRule} />


          {hasLifecycle && (
            <div className={briefingStyles.lifecycleGroup}>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>
                Lifecycle
              </div>
              <div className={s.priorityItems}>
                {todayLifecycle.slice(0, 3).map((update) => (
                  <LifecycleUpdateItem
                    key={update.changeId}
                    update={update}
                    pending={pendingLifecycleChangeId === update.changeId}
                    onConfirm={onConfirmLifecycle}
                    onCorrect={onOpenLifecycleCorrection}
                  />
                ))}
              </div>
            </div>
          )}

          {hasCallouts && (
            <div className={hasLifecycle ? briefingStyles.calloutsGroupSpaced : briefingStyles.calloutsGroupFlush}>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelOverdue)}>
                Signals
              </div>
              <div className={s.priorityItems}>
                {briefingCallouts.slice(0, 5).map((callout) => (
                  <div key={callout.id} className={briefingStyles.calloutItem}>
                    <div className={briefingStyles.calloutSeverity} data-severity={callout.severity} />
                    <div className={briefingStyles.calloutContent}>
                      <span className={briefingStyles.calloutHeadline}>{callout.headline}</span>
                      {callout.entityName && (
                        <span className={briefingStyles.calloutEntity}>{callout.entityName}</span>
                      )}
                      {callout.detail && (
                        <span className={briefingStyles.calloutDetail}>{callout.detail}</span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Suggested actions removed — live on /actions page */}

          {/* Actions: meeting-relevant + overdue (max 3) */}
          {hasActions && (
            <div className={hasLifecycle ? briefingStyles.actionsGroupSpaced : briefingStyles.actionsGroupFlush}>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelOverdue)}>
                Actions
              </div>
              <div className={s.priorityItems}>
                {hasPrioritizedActions ? (
                  // Render as PrioritizedActionItem
                  (attentionActions as PrioritizedAction[]).map((pa) => (
                    <PrioritizedActionItem
                      key={pa.action.id}
                      pa={pa}
                      urgency={pa.atRisk ? "overdue" : "today"}
                      isCompleted={completedIds.has(pa.action.id)}
                      onComplete={onComplete}
                    />
                  ))
                ) : (
                  // Render raw actions
                  (attentionActions as Action[]).map((action) => {
                    const done = action.status === "completed" || completedIds.has(action.id);
                    const isOverdue = action.isOverdue;
                    return (
                      <div
                        key={action.id}
                        className={clsx(
                          s.priorityItem,
                          done && s.priorityItemCompleted,
                          isOverdue ? s.priorityItemOverdue : s.priorityItemToday,
                          action.account && s.priorityItemAccount,
                        )}
                      >
                        <button
                          className={clsx(
                            s.priorityCheck,
                            done && s.priorityCheckChecked,
                            isOverdue && !done && s.priorityCheckOverdue,
                          )}
                          onClick={() => !done && onComplete(action.id)}
                          disabled={done}
                        >
                          {done && (
                            <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
                              <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                            </svg>
                          )}
                        </button>
                        <div className={s.priorityContent}>
                          <Link
                            to="/actions/$actionId"
                            params={{ actionId: action.id }}
                            className={clsx(s.priorityTitle, done ? briefingStyles.linkLineThrough : briefingStyles.linkNoDecoration)}
                          >
                            {stripMarkdown(action.title)}
                          </Link>
                          {(isOverdue || action.dueDate || action.account) && (
                            <div className={s.priorityContext}>
                              {isOverdue && action.daysOverdue
                                ? `${action.daysOverdue} day${action.daysOverdue !== 1 ? "s" : ""} overdue`
                                : action.dueDate ?? ""}
                              {action.account && ` \u00B7 ${action.account}`}
                            </div>
                          )}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>
          )}

          {/* Aging awareness — subtle line when actions approach auto-archive */}
          {hasAging && (
            <div className={briefingStyles.agingNotice}>
              {agingActionCount} {agingActionCount === 1 ? "item" : "items"} aging toward auto-archive
            </div>
          )}

          {/* Emails: show scored/intelligence emails first, never raw repliesNeeded */}
          {hasEmails ? (
            <>
              <div className={clsx(s.priorityGroupLabel, s.priorityGroupLabelToday)}>
                {emailSectionLabel}
                {emailSyncTimestamp && (
                  <span className={briefingStyles.emailSyncTimestamp}>
                    as of {formatAsOfTime(emailSyncTimestamp)}
                  </span>
                )}
              </div>
              <div className={s.priorityItems}>
                {briefingEmails.map((email) => (
                  <PriorityEmailItem key={email.id} email={email} />
                ))}
              </div>
            </>
          ) : null}

          {/* View all links */}
          <div className={s.prioritiesViewAll}>
            {pendingActions.length > 3 && (
              <Link to="/actions" search={{ search: undefined }} className={s.viewAllLink}>
                View all {pendingActions.length} actions &rarr;
              </Link>
            )}
          </div>
        </div>
      </div>
    </section>
  );
}

// ─── Prioritized Action Item ─────────────────────────────────────────────────

function PrioritizedActionItem({
  pa,
  urgency,
  isCompleted,
  onComplete,
}: {
  pa: PrioritizedAction;
  urgency: "overdue" | "today" | "upcoming";
  isCompleted: boolean;
  onComplete: (id: string) => void;
}) {
  const action = pa.action;
  const done = action.status === "completed" || isCompleted;

  const contextParts: string[] = [];
  if (urgency === "overdue") {
    contextParts.push("Overdue");
  }
  if (action.accountName) contextParts.push(action.accountName);
  else if (action.accountId) contextParts.push(action.accountId);
  contextParts.push(`~${formatMinutes(pa.effortMinutes)}`);

  const urgencyClass = {
    overdue: s.priorityItemOverdue,
    today: s.priorityItemToday,
    upcoming: s.priorityItemUpcoming,
  }[urgency];

  return (
    <Link
      to="/actions/$actionId"
      params={{ actionId: action.id }}
      className={clsx(
        s.priorityItem,
        urgencyClass,
        done && s.priorityItemCompleted,
        action.accountName && s.priorityItemAccount,
        briefingStyles.linkNoDecoration,
      )}
    >
      <button
        className={clsx(
          s.priorityCheck,
          done && s.priorityCheckChecked,
          urgency === "overdue" && !done && s.priorityCheckOverdue,
        )}
        onClick={(e) => { e.preventDefault(); !done && onComplete(action.id); }}
        disabled={done}
      >
        {done && (
          <svg width="10" height="10" viewBox="0 0 12 12" fill="none">
            <path d="M2.5 6L5 8.5L9.5 4" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        )}
      </button>
      <div className={s.priorityContent}>
        <div className={s.priorityTitle}>{stripMarkdown(action.title)}</div>
        <div className={s.priorityContext}>{contextParts.join(" \u00B7 ")}</div>
        {urgency === "overdue" && pa.reason && (
          <div className={s.priorityWhy}>{pa.reason}</div>
        )}
      </div>
    </Link>
  );
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/** Format ISO timestamp as "X:XX AM" for the "as of" label. */
function formatAsOfTime(isoString: string): string {
  try {
    const date = new Date(isoString);
    return date.toLocaleTimeString(undefined, {
      hour: "numeric",
      minute: "2-digit",
      hour12: true,
    });
  } catch {
    return "";
  }
}

// ─── Priority Email Item ─────────────────────────────────────────────────────

function PriorityEmailItem({ email }: { email: Email }) {
  return (
    <div
      className={clsx(s.priorityItem, s.priorityItemEmailType, briefingStyles.linkNoDecoration)}
    >
      <div
        className={clsx(
          s.priorityDot,
          email.priority === "high" ? s.priorityDotTerracotta : s.priorityDotTurmeric,
        )}
      />
      <div className={s.priorityContent}>
        {email.summary ? (
          <>
            <div className={s.priorityTitle}>{email.summary}</div>
            <div className={clsx(s.replyMeta, briefingStyles.emailMetaRow)}>
              {email.entityName && (
                <EmailEntityChip
                  entityType={email.entityType}
                  entityName={email.entityName}
                />
              )}
              {/* Only show sender when it adds info beyond entity name */}
              {(!email.entityName || !email.sender.includes(email.entityName)) && (
                <span>{email.sender}</span>
              )}
              {email.summaryContextTrustBand && (
                <TrustBandIndicator band={email.summaryContextTrustBand} />
              )}
              {email.summaryContextSourceCount && email.summaryContextSourceCount > 0 && (
                <span>
                  claim context · {email.summaryContextSourceCount} source{email.summaryContextSourceCount === 1 ? "" : "s"}
                </span>
              )}
              {email.scoreReason && (() => {
                // Strip entity name from reason when chip already shows it
                const reason = email.entityName
                  ? email.scoreReason.replace(email.entityName, "").replace(/^[\s·]+|[\s·]+$/g, "")
                  : email.scoreReason;
                return reason ? <span className={s.emailScoreReason}>{reason}</span> : null;
              })()}
            </div>
          </>
        ) : (
          <div className={s.priorityTitle}>
            <span className={s.prioritySender}>{email.sender}</span>
            <span className={s.prioritySubjectSep}>&mdash;</span>
            <span>{email.subject}</span>
          </div>
        )}
      </div>
    </div>
  );
}

function formatLifecycleLabel(value?: string | null) {
  return value ? value.replace(/_/g, " ") : "";
}

// ─── Lifecycle Update Item ──────────────────────────────────────────────────

function LifecycleUpdateItem({
  update,
  pending,
  onConfirm,
  onCorrect,
}: {
  update: DashboardLifecycleUpdate;
  pending: boolean;
  onConfirm: (update: DashboardLifecycleUpdate) => void;
  onCorrect: (update: DashboardLifecycleUpdate) => void;
}) {
  const transitionLabel = update.previousLifecycle
    ? `${formatLifecycleLabel(update.previousLifecycle)} → ${formatLifecycleLabel(update.newLifecycle)}`
    : formatLifecycleLabel(update.newLifecycle);
  const healthDelta = update.healthScoreBefore != null && update.healthScoreAfter != null
    ? `${Math.round(update.healthScoreBefore)} → ${Math.round(update.healthScoreAfter)}`
    : null;
  const contextBits = [
    update.renewalStage ? `Stage: ${update.renewalStage.replace(/_/g, " ")}` : null,
    healthDelta ? `Health ${healthDelta}` : null,
    update.actionState !== "pending"
      ? update.actionState.charAt(0).toUpperCase() + update.actionState.slice(1)
      : null,
    `${Math.round(update.confidence * 100)}% confidence`,
    formatShortDate(update.createdAt),
  ].filter(Boolean);

  return (
    <div
      className={clsx(s.priorityItem, s.priorityItemToday, s.priorityItemAccount)}
    >
      <div className={clsx(s.priorityDot, s.priorityDotTurmeric)} />
      <div className={s.priorityContent}>
        <Link
          to="/accounts/$accountId"
          params={{ accountId: update.accountId }}
          className={clsx(s.priorityTitle, briefingStyles.linkNoDecoration)}
        >
          {update.accountName}: {transitionLabel}
        </Link>
        <div className={s.priorityContext}>{contextBits.join(" · ")}</div>
        {update.evidence && (
          <div className={s.priorityWhy}>{update.evidence}</div>
        )}
        {update.actionState === "pending" ? (
          <div className={briefingStyles.lifecycleButtonRow}>
            <button
              type="button"
              onClick={() => onConfirm(update)}
              disabled={pending}
              className={briefingStyles.lifecycleConfirmBtn}
            >
              Looks good
            </button>
            <button
              type="button"
              onClick={() => onCorrect(update)}
              disabled={pending}
              className={briefingStyles.lifecycleCorrectBtn}
            >
              Fix something
            </button>
          </div>
        ) : null}
      </div>
    </div>
  );
}
