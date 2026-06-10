import clsx from "clsx";
import type { ReactNode } from "react";
import { IntelligenceCorrection } from "@/components/ui/IntelligenceCorrection";
import { HealthBadge } from "@/components/shared/HealthBadge";
import { CompositionInlineEdit } from "@/components/composition/CompositionInlineEdit";
import { normalizeTrustBand } from "@/services/composition/contracts";
import type {
  CompositionFeedbackEntityType,
  EditRoute,
  KnownCompositionBlockType,
  ProjectedBlock,
  RenderedProvenance,
} from "@/services/composition/contracts";
import { IntelligenceQualityBadge } from "@/components/entity/IntelligenceQualityBadge";
import { TypeBadge } from "@/components/ui/TypeBadge";
import { TypeBadgeDisplay, type TypeBadgeValue } from "@/components/ui/TypeBadgeDisplay";
import { CompositionVitalsStrip, type CompositionVitalSpec } from "@/components/composition/blocks/CompositionVitalsStrip";
import pageStyles from "@/pages/AccountDetailPage.module.css";
import heroStyles from "@/components/composition/blocks/AccountHeroBlock.module.css";
import chapterStyles from "@/components/composition/blocks/CompositionChapters.module.css";
import { WatchList } from "@/components/entity/WatchList";
import { ValueCommitments } from "@/components/entity/ValueCommitments";
import { StrategicLandscape } from "@/components/entity/StrategicLandscape";
import { OutlookPanel, renewalCallVerdict } from "@/components/health/OutlookPanel";
import { OnTrackChapter } from "@/components/health/OnTrackChapter";
import { TriageSection } from "@/components/health/TriageSection";
import { DivergenceSection } from "@/components/health/DivergenceSection";
import { SupportingTension } from "@/components/health/SupportingTension";
import { AboutIntelligence } from "@/components/health/AboutIntelligence";
import { StakeholderGrid } from "@/components/entity/StakeholderGrid";
import { QuoteWall } from "@/components/editorial/QuoteWall";
import { AccountTechnicalFootprint } from "@/components/account/AccountTechnicalFootprint";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import type { ConsistencyFinding, HealthOutlookSignals, QuoteWallEntry } from "@/types";
import { useIntelligenceFieldUpdate } from "@/hooks/useIntelligenceFieldUpdate";
import type { EntityIntelligence } from "@/types";

type Payload = Record<string, unknown>;

/**
 * The chapter's enriched-intelligence payload — the production content
 * contract. The producer ships the chapter-relevant subset of the account's
 * EntityIntelligence at payload.intelligence (camelCase, same shape the
 * production account-detail chapters consume), so composition chapters render
 * through the SAME bespoke components as production.
 */
function chapterIntelligence(payload: Payload): EntityIntelligence | null {
  const value = object(payload.intelligence);
  return value ? (value as unknown as EntityIntelligence) : null;
}

/** Production-parity inline-edit wiring for intelligence-backed chapters.
 *  Same write path the production page uses; corrections persist through the
 *  entity intelligence field update and surface on the next projection.
 *  Deliberately NO thumbs feedback wiring — the helpful/not-helpful pattern
 *  is the legacy affordance the trust model retired; typed claim feedback
 *  arrives with the unified-writer track. */
function useChapterIntelligenceWiring(accountId?: string) {
  const { updateField } = useIntelligenceFieldUpdate("account", accountId, async () => {});
  return {
    onUpdateField: accountId ? updateField : undefined,
  };
}

/**
 * Production-block registry — payload.block names the main-branch component
 * this block renders through (the blocks model: producers shape content,
 * blocks are display-only). Returns null for unknown/locally-rendered keys
 * (outlook_panel, on_track) so type components fall through to their own
 * rendering.
 */
function renderProductionBlock(payload: Payload, accountId?: string): JSX.Element | null {
  const block = text(payload.block);
  if (!block) return null;
  const intelligence = chapterIntelligence(payload);
  const glean = (object(payload.gleanSignals) as unknown as HealthOutlookSignals | null) ?? null;
  switch (block) {
    case "triage":
      return (
        <TriageSection
          intelligence={intelligence}
          gleanSignals={glean}
          sentiment={(text(object(payload.sentiment)?.current) ?? null) as never}
          accountId={accountId}
        />
      );
    case "divergence":
      return (
        <DivergenceSection
          findings={(Array.isArray(payload.findings) ? payload.findings : []) as unknown as ConsistencyFinding[]}
          gleanSignals={glean}
          accountId={accountId}
        />
      );
    case "supporting_tension":
      return <SupportingTension intelligence={intelligence} gleanSignals={glean} />;
    case "about_intelligence":
      return <AboutIntelligence intelligence={intelligence} gleanSignals={glean} />;
    case "stakeholder_grid": {
      const stakeholders = object(payload.stakeholders);
      if (!stakeholders?.stakeholdersFull) return null;
      return (
        <StakeholderGrid
          stakeholders={stakeholders.stakeholdersFull as never}
          accountName={text(stakeholders.accountName) ?? undefined}
        />
      );
    }
    case "technical_footprint": {
      const footprint = object(payload.technicalFootprint);
      return footprint ? <AccountTechnicalFootprint footprint={footprint as never} /> : null;
    }
    case "quote_wall":
      return (
        <QuoteWall quotes={(Array.isArray(payload.quotes) ? payload.quotes : null) as unknown as QuoteWallEntry[] | null} />
      );
    default:
      return null;
  }
}

const TYPE_BADGE_VALUES: ReadonlySet<string> = new Set(["customer", "internal", "partner"]);
function asTypeBadgeValue(value: string | null): TypeBadgeValue | null {
  return value && TYPE_BADGE_VALUES.has(value) ? (value as TypeBadgeValue) : null;
}

// Map a headline vital's label to its editable account column. Labels are
// stable constants from the snapshot builder (context.rs push_account_vital_field);
// an unmapped label simply renders read-only. (Same label-keyed approach as
// VitalsStrip's matchVitalToSourceRef.)
const VITAL_FIELD_BY_LABEL: Record<string, string> = {
  arr: "arr",
  "contract end": "contract_end",
  nps: "nps",
  lifecycle: "lifecycle",
};
function vitalFieldFromLabel(label: string | null): string | null {
  return label ? VITAL_FIELD_BY_LABEL[label.toLowerCase().trim()] ?? null : null;
}
type BlockComponentProps = {
  block: ProjectedBlock;
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  payload: Payload;
  renderedProvenance?: RenderedProvenance | null;
  editMode?: boolean;
  /** Save a snapshot-derived account field (name/type/vitals) via the
   *  service-layer correction command; the page re-projects on success. */
  onSnapshotFieldSave?: (field: string, value: string) => Promise<void> | void;
};
type BlockComponent = (props: BlockComponentProps) => JSX.Element;

function text(value: unknown): string | null {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return null;
}

function array(value: unknown): Payload[] {
  return Array.isArray(value) ? value.filter((item): item is Payload => !!item && typeof item === "object" && !Array.isArray(item)) : [];
}

function object(value: unknown): Payload | null {
  return value && typeof value === "object" && !Array.isArray(value) ? (value as Payload) : null;
}

function pointerSegment(segment: string): string {
  return segment.replace(/~1/g, "/").replace(/~0/g, "~");
}

function pointerValue(payload: Payload, pointer: string): unknown {
  if (!pointer || pointer === "/") return payload;
  return pointer
    .split("/")
    .slice(1)
    .map(pointerSegment)
    .reduce<unknown>((current, segment) => {
      if (Array.isArray(current)) {
        const index = Number(segment);
        return Number.isInteger(index) ? current[index] : undefined;
      }
      if (current && typeof current === "object") {
        return (current as Record<string, unknown>)[segment];
      }
      return undefined;
    }, payload);
}

/**
 * Provenance class for trust rendering. "inferred" (enrichment, no source_ref)
 * renders faded with a tooltip + confirm/contest; "sourced" (hard fact) reads
 * at full presence. Trust surfaces as opacity, not chips — keyed off the
 * producer's provenance_kind, not the cold-start trust score (DOS-853).
 */
function provenanceKind(payload: Payload): "sourced" | "inferred" | null {
  // Single-claim blocks carry provenance_kind at the top level and fade as a
  // whole. Aggregate chapter blocks carry provenance_kind per item/node — the
  // item rows own their own fade, so the block stays at full presence.
  const candidate = text(payload.provenance_kind);
  return candidate === "sourced" || candidate === "inferred" ? candidate : null;
}

const INFERRED_TOOLTIP =
  "Inferred from enrichment — not yet confirmed by a source. Confirm or contest below.";

function renderedValue(renderedProvenance?: RenderedProvenance | null): Payload | null {
  return object(renderedProvenance?.value);
}

function provenanceFieldAttributions(renderedProvenance?: RenderedProvenance | null): Payload {
  const value = renderedValue(renderedProvenance);
  return (
    object(value?.field_attributions) ??
    object(object(object(value?.about_this)?.details)?.field_attributions) ??
    {}
  );
}

function provenanceSourceCount(renderedProvenance?: RenderedProvenance | null): number | null {
  const value = renderedValue(renderedProvenance);
  const summary = object(object(value?.about_this)?.summary);
  const count = summary?.source_count;
  if (typeof count === "number") return count;
  const sources = value?.sources;
  if (Array.isArray(sources)) return sources.length;
  return null;
}

function fieldPathCovers(candidate: string, target: string): boolean {
  return candidate === "" || candidate === target || target.startsWith(`${candidate}/`);
}

function fieldPathsOverlap(left: string, right: string): boolean {
  return fieldPathCovers(left, right) || fieldPathCovers(right, left);
}

function matchingFieldAttribution(block: ProjectedBlock, fieldAttributions: Payload): Payload | null {
  for (const ref of block.provenance) {
    const exact = object(fieldAttributions[ref.field_path]);
    if (exact) return exact;

    for (const [fieldPath, attribution] of Object.entries(fieldAttributions)) {
      const candidate = object(attribution);
      if (candidate && fieldPathsOverlap(fieldPath, ref.field_path)) return candidate;
    }
  }
  return null;
}

function attributionSourceCount(attribution: Payload): number | null {
  const sourceRefs = attribution.source_refs;
  return Array.isArray(sourceRefs) ? sourceRefs.length : null;
}

function provenanceWasTruncated(renderedProvenance?: RenderedProvenance | null): boolean {
  const value = renderedValue(renderedProvenance);
  const warnings = array(value?.warnings);
  return (
    warnings.some((warning) => text(warning.kind) === "truncated_for_render") ||
    object(value?.about_this)?.details_available === true
  );
}

function provenanceState(block: ProjectedBlock, renderedProvenance?: RenderedProvenance | null) {
  if (block.provenance.length === 0) return null;
  const value = renderedValue(renderedProvenance);
  if (!value) return { state: "missing", label: "Provenance unavailable" };
  if (value.kind === "provenance_masked" || value.status === "masked") {
    return { state: "masked", label: "Provenance masked" };
  }

  const fieldAttributions = provenanceFieldAttributions(renderedProvenance);
  const attribution = matchingFieldAttribution(block, fieldAttributions);
  if (!attribution) {
    if (provenanceWasTruncated(renderedProvenance)) {
      const sourceCount = provenanceSourceCount(renderedProvenance);
      return {
        state: "rendered",
        label:
          typeof sourceCount === "number" && sourceCount > 0
            ? `from ${sourceCount} ${sourceCount === 1 ? "source" : "sources"}`
            : "Provenance recorded",
      };
    }
    return { state: "unresolved", label: "Source pending" };
  }

  const sourceCount = attributionSourceCount(attribution);
  return {
    state: "rendered",
    label:
      typeof sourceCount === "number" && sourceCount > 0
        ? `from ${sourceCount} ${sourceCount === 1 ? "source" : "sources"}`
        : "Provenance recorded",
  };
}

/** Mono "Mar 12, 2026" date for the kicker line; null on unparseable input. */
function formatAsofDate(value: string | null): string | null {
  if (!value) return null;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return null;
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" });
}

/** Claim-row intent for kicker color + accent border. */
type ChapterIntent = string | null;

function chapterIntent(payload: Payload): ChapterIntent {
  return text(payload.intent);
}

const INTENT_KICKER_LABELS: Record<string, string> = {
  win: "Win",
  value: "Value",
  context: "Context",
  risk: "Risk",
  working: "Working",
  struggling: "Struggling",
};

/** Group labels for aggregate claim lists. Only the state-of-play split
 *  carries a visible label — other groups' chapters are already titled by
 *  the section header, so a repeated label would be redundant chrome. */
const INTENT_GROUP_LABELS: Record<string, string> = {
  working: "Working",
  struggling: "Struggling",
};

/**
 * KickerLine — the quiet mono label that opens each chapter claim row
 * (WatchList section-label treatment). Carries the block's title or intent
 * plus the claim's as-of date; intent tints the label (sage for win/value,
 * terracotta for risk). This replaces the bold sans BlockShell title for
 * spine-D chapter bodies.
 */
function KickerLine({ label, intent, asof }: { label: string | null; intent?: ChapterIntent; asof?: string | null }) {
  const date = formatAsofDate(asof ?? null);
  if (!label && !date) return null;
  return (
    <div className={chapterStyles.kickerLine} data-intent={intent ?? undefined}>
      {label && <span className={chapterStyles.kicker}>{label}</span>}
      {date && <span className={chapterStyles.kickerDate}>{date}</span>}
    </div>
  );
}

/** Item-level routes (aggregate chapter payloads) are owned by the item rows,
 *  not the block-level feedback prompt. */
function isItemRoute(route: EditRoute): boolean {
  return route.field_path.startsWith("/items/") || route.field_path.startsWith("/nodes/");
}

function feedbackRoute(block: ProjectedBlock): EditRoute | null {
  return (
    block.edit_routes.find(
      (route) => route.feedback_allowed && route.claim_refs.length > 0 && !isItemRoute(route),
    ) ?? null
  );
}

function feedbackRouteForPath(block: ProjectedBlock, fieldPath: string): EditRoute | null {
  return block.edit_routes.find((route) => route.field_path === fieldPath && route.feedback_allowed && route.claim_refs.length > 0) ?? null;
}

function feedbackField(route: EditRoute): string {
  const normalized = route.field_path
    .split("/")
    .filter(Boolean)
    .map(pointerSegment)
    .join(".");
  return normalized ? `composition:${normalized}` : "composition:block";
}

function feedbackCurrentValue(payload: Payload, route: EditRoute): string | null {
  return text(pointerValue(payload, route.field_path));
}

function BlockFeedback({
  accountId,
  entityType = "account",
  block,
  payload,
}: {
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  block: ProjectedBlock;
  payload: Payload;
}) {
  const route = feedbackRoute(block);
  const claimRef = route?.claim_refs[0];
  if (!accountId || !route || !claimRef) return null;

  const currentValue = feedbackCurrentValue(payload, route);
  return (
    <div className={pageStyles.compositionFeedbackRow}>
      <IntelligenceCorrection
        entityId={accountId}
        entityType={entityType}
        field={feedbackField(route)}
        itemKey={claimRef.claim_id}
        currentValue={currentValue}
        variant={currentValue ? "correct" : "dismiss"}
      />
    </div>
  );
}

/** Per-item confirm/contest for aggregate chapter rows. Same
 *  IntelligenceCorrection affordance as the block-level prompt, routed
 *  through the item's own `/items/N/text` claim ref; hover-revealed so a
 *  chapter of N claims doesn't render N standing prompts. */
function ItemFeedback({
  accountId,
  entityType = "account",
  block,
  fieldPath,
  value,
}: {
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  block: ProjectedBlock;
  fieldPath: string;
  value: string | null;
}) {
  const route = feedbackRouteForPath(block, fieldPath);
  const claimRef = route?.claim_refs[0];
  if (!accountId || !route || !claimRef) return null;
  return (
    <div className={clsx(pageStyles.compositionFeedbackRow, chapterStyles.itemFeedback)}>
      <IntelligenceCorrection
        entityId={accountId}
        entityType={entityType}
        field={feedbackField(route)}
        itemKey={claimRef.claim_id}
        currentValue={value}
        variant={value ? "correct" : "dismiss"}
      />
    </div>
  );
}

/** Aggregate claim items — the shared row renderer for chapter lists.
 *  Each row: editable claim text, per-item trust fade (provenance_kind),
 *  hover-revealed confirm/contest. */
function AggregateClaimItems({
  accountId,
  entityType,
  block,
  editMode,
  items,
  itemKey = "items",
  indexOffset = 0,
  textClassName,
}: {
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  block: ProjectedBlock;
  editMode?: boolean;
  items: Payload[];
  itemKey?: string;
  /** Payload index of items[0] — nonzero when a lead item renders separately. */
  indexOffset?: number;
  textClassName?: string;
}) {
  return (
    <>
      {items.map((item, index) => {
        const fieldPath = `/${itemKey}/${index + indexOffset}/text`;
        const value = text(item.text) ?? "";
        return (
          <div
            className={chapterStyles.itemRow}
            data-provenance-kind={text(item.provenance_kind) ?? undefined}
            title={text(item.provenance_kind) === "inferred" ? INFERRED_TOOLTIP : undefined}
            key={`${text(item.claim_id) ?? "item"}-${index}`}
          >
            <EditableBlockText
              accountId={accountId}
              entityType={entityType}
              block={block}
              editMode={editMode}
              fieldPath={fieldPath}
              value={value}
              as="p"
              className={textClassName ?? chapterStyles.itemText}
            />
            <ItemFeedback
              accountId={accountId}
              entityType={entityType}
              block={block}
              fieldPath={fieldPath}
              value={value}
            />
          </div>
        );
      })}
    </>
  );
}

function BlockShell({
  block,
  accountId,
  entityType,
  payload,
  renderedProvenance,
  title,
  children,
  featured = false,
  empty = false,
  quiet = false,
}: {
  block: ProjectedBlock;
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  payload: Payload;
  renderedProvenance?: RenderedProvenance | null;
  title?: string | null;
  children: ReactNode;
  featured?: boolean;
  empty?: boolean;
  /** Chrome-free shell for intelligence-backed chapters: the production
   *  components own their rules/headers, so the block adds no border or
   *  block-level feedback row (safety provenance states still render). */
  quiet?: boolean;
}) {
  const provenance = provenanceState(block, renderedProvenance);
  // Trust surfaces as opacity, not chips. Only the safety states (unavailable
  // / masked / pending) get a visible status; the routine "from N sources"
  // resolution stays quiet — sources live in the sources chapter, and inferred
  // content fades with a tooltip rather than carrying a loud trust band.
  const safetyProvenance = provenance && provenance.state !== "rendered" ? provenance : null;
  const inferred = provenanceKind(payload) === "inferred";
  return (
    <article
      className={clsx(
        quiet ? pageStyles.compositionQuietBlock : pageStyles.compositionBlock,
        featured && pageStyles.compositionFeaturedBlock,
        empty && pageStyles.compositionEmptyState,
        block.banner && pageStyles.compositionFallbackState,
        inferred && pageStyles.compositionInferred,
      )}
      data-block-type={block.selected_known_type_id}
      data-trust-band={normalizeTrustBand(block.trust_band)}
      data-provenance-kind={provenanceKind(payload) ?? undefined}
      title={inferred ? INFERRED_TOOLTIP : undefined}
    >
      {block.banner && (
        <div className={pageStyles.compositionFallbackState} role="note">
          <p className={pageStyles.compositionStateLabel}>Fallback</p>
          <p className={pageStyles.compositionStateText}>{block.banner}</p>
        </div>
      )}
      {(title || safetyProvenance) && (
        <header className={pageStyles.compositionBlockHeader}>
          {title && <h3 className={pageStyles.compositionBlockTitle}>{title}</h3>}
          {safetyProvenance && (
            <div className={pageStyles.compositionBlockMeta}>
              <span
                className={pageStyles.compositionProvenanceStatus}
                data-provenance-state={safetyProvenance.state}
              >
                {safetyProvenance.label}
              </span>
            </div>
          )}
        </header>
      )}
      {children}
      {!quiet && <BlockFeedback accountId={accountId} entityType={entityType} block={block} payload={payload} />}
    </article>
  );
}

function EditableBlockText({
  accountId,
  entityType,
  block,
  editMode,
  fieldPath,
  value,
  as,
  className,
  multiline = true,
}: {
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  block: ProjectedBlock;
  editMode?: boolean;
  fieldPath: string;
  value: string;
  as: "p" | "span" | "h3" | "div";
  className: string;
  multiline?: boolean;
}) {
  const fallback = <>{as === "p" ? <p className={className}>{value}</p> : as === "h3" ? <h3 className={className}>{value}</h3> : as === "div" ? <div className={className}>{value}</div> : <span className={className}>{value}</span>}</>;
  if (!editMode) return fallback;
  return (
    <CompositionInlineEdit
      accountId={accountId}
      entityType={entityType}
      route={feedbackRouteForPath(block, fieldPath)}
      value={value}
      as={as}
      multiline={multiline}
      className={className}
      fallback={fallback}
    />
  );
}

/**
 * AccountOverviewBlock — the headline hero. Chrome-free (no BlockShell card):
 * the headline chapter is full-bleed. Renders the v1.5.0 claim-backed
 * account_overview payload with the curated editorial look (mono uppercase
 * meta row, 76px serif name, provenance-bearing vitals strip). Identity-only:
 * no claim lede in the hero (James, 2026-06-08). Inline edits route to the
 * block's claim-feedback edit_routes — never to account-field writes.
 */
function AccountOverviewBlock({ payload, onSnapshotFieldSave }: BlockComponentProps) {
  const account = object(payload.account) ?? {};
  const vitals = array(payload.vitals);
  const displayName = text(account.display_name) ?? text(payload.title) ?? "Account";
  const accountType = asTypeBadgeValue(text(account.type));
  const snapshotDegraded = Boolean(text(payload.snapshot_degraded));
  const heroAsof = text(vitals[0]?.source_asof) ?? undefined;

  // Map the claim-backed vitals into the curated dot-separated strip. Each
  // cell shows the producer's display_value ("ARR $185,400") but edits the raw
  // value (185400) via the snapshot-field correction path. Provenance/source
  // detail deliberately stays OFF the hero — the ambient freshness dot is the
  // only trust signal here; sources live in the sources chapter.
  const vitalSpecs: CompositionVitalSpec[] = vitals.flatMap((item) => {
    const label = text(item.label);
    const raw = text(item.value);
    const display = text(item.display_value) ?? raw;
    if (!display) return [];
    const composed = label ? `${label} ${display}` : display;
    return [{ label, display: composed, raw: raw ?? display, field: vitalFieldFromLabel(label) }];
  });

  return (
    <div className={heroStyles.hero}>
      <div className={heroStyles.metaRow}>
        <IntelligenceQualityBadge enrichedAt={heroAsof} showLabel />
        {accountType &&
          (onSnapshotFieldSave ? (
            <TypeBadge
              value={accountType}
              onChange={(value) => void onSnapshotFieldSave("account_type", value)}
            />
          ) : (
            <TypeBadgeDisplay value={accountType} />
          ))}
      </div>

      <h1 className={heroStyles.name}>{displayName}</h1>

      {vitalSpecs.length > 0 && (
        <CompositionVitalsStrip vitals={vitalSpecs} onSave={onSnapshotFieldSave} />
      )}

      {snapshotDegraded && (
        <div className={heroStyles.degraded} role="status">
          <p className={heroStyles.degradedLabel}>Account details unavailable</p>
          <p className={heroStyles.degradedText}>Some sourced account details could not be loaded for this view.</p>
        </div>
      )}
    </div>
  );
}

function ClaimSummaryBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const empty = payload.empty_state === true;
  const intent = chapterIntent(payload);
  const items = array(payload.items);
  const intelligence = chapterIntelligence(payload);
  const wiring = useChapterIntelligenceWiring(accountId);

  const production = renderProductionBlock(payload, accountId);
  if (production) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        {production}
      </BlockShell>
    );
  }

  // Production content path: the chapter renders through the SAME bespoke
  // components the production account page uses, fed by the producer's
  // intelligence subset. Claims remain attached on the block for provenance.
  if (intelligence) {
    const isState = !!intelligence.currentState || !!intelligence.executiveAssessment;
    const isValue =
      !!intelligence.valueDelivered?.length ||
      !!intelligence.successMetrics?.length ||
      !!intelligence.openCommitments?.length;
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        {isState ? (
          <>
            <OnTrackChapter intelligence={intelligence} />
            {accountId && intelligence.executiveAssessment && (
              <IntelligenceCorrection
                entityId={accountId}
                entityType={entityType ?? "account"}
                field="executiveAssessment"
                variant="correct"
                currentValue={intelligence.executiveAssessment}
              />
            )}
          </>
        ) : isValue ? (
          <ValueCommitments intelligence={intelligence} onUpdateField={wiring.onUpdateField} />
        ) : (
          <StrategicLandscape intelligence={intelligence} onUpdateField={wiring.onUpdateField} />
        )}
      </BlockShell>
    );
  }

  // Aggregate chapter payload: a grouped claim list (StateBlock treatment —
  // mono group label + accent-bordered rows). Working/Struggling carry their
  // split labels; other groups are already titled by the section header.
  if (items.length > 0) {
    const label = intent ? INTENT_GROUP_LABELS[intent] ?? null : null;
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
        <div className={chapterStyles.group} data-intent={intent ?? undefined}>
          {label && <div className={chapterStyles.groupLabel}>{label}</div>}
          <div className={chapterStyles.groupItems}>
            <AggregateClaimItems
              accountId={accountId}
              entityType={entityType}
              block={block}
              editMode={editMode}
              items={items}
            />
          </div>
        </div>
      </BlockShell>
    );
  }

  // Single-claim / empty-state / fallback payload.
  const kicker = text(payload.title) ?? (intent ? INTENT_KICKER_LABELS[intent] : null);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} empty={empty}>
      <div className={chapterStyles.claimBody} data-intent={intent ?? undefined}>
        <KickerLine label={kicker} intent={intent} asof={text(payload.source_asof)} />
        {text(payload.text) && (
          <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/text" value={text(payload.text) ?? ""} as="p" className={chapterStyles.claimValue} />
        )}
        {text(payload.body) && (
          <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/body" value={text(payload.body) ?? ""} as="p" className={chapterStyles.claimSupport} />
        )}
      </div>
    </BlockShell>
  );
}

function HealthSnapshotBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const items = array(payload.items);
  const intelligence = chapterIntelligence(payload);

  const production = renderProductionBlock(payload, accountId);
  if (production) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        {production}
      </BlockShell>
    );
  }

  // Production content path: the outlook chapter IS the production
  // OutlookPanel (main retired the legacy AccountOutlook), fed the
  // producer's intelligence subset (agreementOutlook + contractContext).
  if (intelligence?.agreementOutlook) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        <ChapterHeading title={`The Call: ${renewalCallVerdict(intelligence.agreementOutlook)}`} />
        <OutlookPanel intelligence={intelligence} />
        {accountId && (
          <IntelligenceCorrection
            entityId={accountId}
            entityType={entityType ?? "account"}
            field="agreementOutlook.renewalNarrative"
            variant="correct"
            currentValue={intelligence.agreementOutlook.renewalNarrative ?? intelligence.agreementOutlook.expansionPotential ?? null}
          />
        )}
      </BlockShell>
    );
  }

  // Aggregate outlook payload: the lead claim reads as the AccountOutlook
  // editorial statement; supporting claims follow as quieter rows. Trust
  // stays quiet (opacity per item) — trust_band never renders as a metric.
  if (items.length > 0) {
    const lead = items[0];
    const rest = items.slice(1);
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
        <div
          className={chapterStyles.itemRow}
          data-provenance-kind={text(lead.provenance_kind) ?? undefined}
          title={text(lead.provenance_kind) === "inferred" ? INFERRED_TOOLTIP : undefined}
        >
          <EditableBlockText
            accountId={accountId}
            entityType={entityType}
            block={block}
            editMode={editMode}
            fieldPath="/items/0/text"
            value={text(lead.text) ?? ""}
            as="p"
            className={chapterStyles.statement}
          />
          <ItemFeedback accountId={accountId} entityType={entityType} block={block} fieldPath="/items/0/text" value={text(lead.text)} />
        </div>
        {rest.length > 0 && (
          <AggregateClaimItems
            accountId={accountId}
            entityType={entityType}
            block={block}
            editMode={editMode}
            items={rest}
            indexOffset={1}
          />
        )}
      </BlockShell>
    );
  }

  // Legacy single-claim / snapshot payload.
  const band = text(payload.band);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
      <KickerLine label={text(payload.title) ?? "Health"} asof={text(payload.source_asof)} />
      {text(payload.text) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/text" value={text(payload.text) ?? ""} as="p" className={chapterStyles.statement} />
      )}
      {(typeof payload.score === "number" || band) && (
        <div className={chapterStyles.statementMetrics}>
          {typeof payload.score === "number" && (
            <div className={chapterStyles.statementMetric}>
              <span className={chapterStyles.statementMetricValue}>{payload.score}</span>
              <span className={chapterStyles.statementMetricLabel}>Score</span>
            </div>
          )}
          {band && (
            <div className={chapterStyles.statementMetric}>
              <span className={chapterStyles.statementMetricValue}>{band.replace(/_/g, " ")}</span>
              <span className={chapterStyles.statementMetricLabel}>Band</span>
            </div>
          )}
        </div>
      )}
    </BlockShell>
  );
}

function RiskCalloutBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const items = array(payload.items);
  const intelligence = chapterIntelligence(payload);
  const wiring = useChapterIntelligenceWiring(accountId);

  const production = renderProductionBlock(payload, accountId);
  if (production) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        {production}
      </BlockShell>
    );
  }

  // Production content path: the watch-list chapter IS the production
  // WatchList component, fed the producer's intelligence subset.
  if (intelligence) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        <WatchList intelligence={intelligence} sectionId="" onUpdateField={wiring.onUpdateField} />
      </BlockShell>
    );
  }

  // Aggregate watch-list payload: terracotta-flagged claim rows (WatchList
  // grouped treatment) with per-item trust fade + confirm/contest.
  if (items.length > 0) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
        <div className={chapterStyles.group} data-intent="risk">
          <div className={chapterStyles.groupItems}>
            <AggregateClaimItems
              accountId={accountId}
              entityType={entityType}
              block={block}
              editMode={editMode}
              items={items}
            />
          </div>
        </div>
      </BlockShell>
    );
  }

  // Legacy single-claim payload.
  const primary = text(payload.text) ?? text(payload.body);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
      <div className={chapterStyles.claimBody} data-intent="risk">
        <KickerLine label={text(payload.title) ?? "Risk"} intent="risk" asof={text(payload.source_asof)} />
        {primary && (
          <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath={text(payload.text) ? "/text" : "/body"} value={primary} as="p" className={chapterStyles.claimValue} />
        )}
        {text(payload.recommended_action) && (
          <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/recommended_action" value={text(payload.recommended_action) ?? ""} as="p" className={chapterStyles.claimSupport} />
        )}
      </div>
    </BlockShell>
  );
}

function RelationshipMapBlock({ block, accountId, entityType, payload, renderedProvenance }: BlockComponentProps) {
  const nodes = array(payload.nodes);
  const production = renderProductionBlock(payload, accountId);
  if (production) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        {production}
      </BlockShell>
    );
  }
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
      <div className={chapterStyles.personGrid}>
        {nodes.map((node, index) => {
          const label = text(node.label) ?? text(node.text);
          return (
            <div
              className={chapterStyles.personRow}
              data-provenance-kind={text(node.provenance_kind) ?? undefined}
              title={text(node.provenance_kind) === "inferred" ? INFERRED_TOOLTIP : undefined}
              key={`${text(node.claim_id) ?? label ?? "node"}-${index}`}
            >
              <span className={chapterStyles.personAvatar} aria-hidden="true">{(label ?? "?").slice(0, 1)}</span>
              <p className={chapterStyles.personText}>{label}</p>
              <ItemFeedback
                accountId={accountId}
                entityType={entityType}
                block={block}
                fieldPath={`/nodes/${index}/text`}
                value={text(node.text)}
              />
            </div>
          );
        })}
      </div>
    </BlockShell>
  );
}

function ActionListBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const items = array(payload.items);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} empty={items.length === 0}>
      <div className={chapterStyles.actionList}>
        {items.map((item, index) => {
          const fieldPath = `/items/${index}/text`;
          const value = text(item.title) ?? text(item.text) ?? "";
          const meta = [text(item.status), formatAsofDate(text(item.source_asof))].filter(Boolean);
          return (
            <div
              className={chapterStyles.actionRow}
              data-provenance-kind={text(item.provenance_kind) ?? undefined}
              title={text(item.provenance_kind) === "inferred" ? INFERRED_TOOLTIP : undefined}
              key={`${text(item.claim_id) ?? value ?? "action"}-${index}`}
            >
              <span className={chapterStyles.actionDot} aria-hidden="true" />
              {meta.length > 0 && <p className={chapterStyles.actionMeta}>{meta.join(" · ")}</p>}
              <EditableBlockText
                accountId={accountId}
                entityType={entityType}
                block={block}
                editMode={editMode}
                fieldPath={fieldPath}
                value={value}
                as="p"
                className={chapterStyles.actionText}
              />
              <ItemFeedback
                accountId={accountId}
                entityType={entityType}
                block={block}
                fieldPath={fieldPath}
                value={text(item.text)}
              />
            </div>
          );
        })}
      </div>
    </BlockShell>
  );
}

function EvidenceListBlock({ block, accountId, entityType, payload, renderedProvenance }: BlockComponentProps) {
  const items = array(payload.items);
  const production = renderProductionBlock(payload, accountId);
  if (production) {
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} quiet>
        {production}
      </BlockShell>
    );
  }
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} empty={items.length === 0}>
      <KickerLine label={text(payload.title)} />
      {items.map((item, index) => {
        const quote = text(item.evidence_quote);
        const title = quote ?? text(item.label);
        const assertion = text(item.assertion_text);
        const meta = [
          text(item.source_label),
          formatAsofDate(text(item.source_asof)) ?? text(item.source_asof),
          text(item.workspace_file_kind),
          text(item.sensitivity),
          text(item.redaction_state),
        ].filter(Boolean);
        return (
          <div className={chapterStyles.evidenceItem} key={`${title ?? "evidence"}-${index}`}>
            {title && (
              <p className={quote ? chapterStyles.evidenceQuote : chapterStyles.evidenceAssertion}>
                {quote ? `“${quote}”` : title}
              </p>
            )}
            {assertion && assertion !== title && <p className={chapterStyles.evidenceAssertion}>{assertion}</p>}
            {meta.length > 0 && <p className={chapterStyles.evidenceMeta}>{meta.join(" · ")}</p>}
          </div>
        );
      })}
    </BlockShell>
  );
}

function MarkdownDocumentBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const sections = array(payload.sections);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
      <KickerLine label={text(payload.title)} asof={text(payload.source_asof)} />
      {text(payload.body) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/body" value={text(payload.body) ?? ""} as="p" className={chapterStyles.docBody} />
      )}
      {sections.map((section, index) => (
        <section className={chapterStyles.docSection} key={`${text(section.heading) ?? "section"}-${index}`}>
          {text(section.heading) && <h4 className={chapterStyles.docHeading}>{text(section.heading)}</h4>}
          {text(section.body) && <p className={chapterStyles.docBody}>{text(section.body)}</p>}
        </section>
      ))}
    </BlockShell>
  );
}

export function GenericTextBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const title = text(payload.title) ?? text(payload.label);
  const body = text(payload.text) ?? text(payload.body);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
      <KickerLine label={title} />
      {body && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath={text(payload.text) ? "/text" : "/body"} value={body} as="p" className={chapterStyles.docBody} />
      )}
    </BlockShell>
  );
}

function PrimitiveBlock({ block, accountId, entityType, payload, renderedProvenance }: BlockComponentProps) {
  const label = text(payload.label) ?? text(object(payload.payload)?.text) ?? text(payload.text) ?? block.selected_known_type_id;
  if (block.selected_known_type_id === "dailyos/health-badge") {
    const rawBand = text(payload.band);
    const band = rawBand === "green" || rawBand === "yellow" || rawBand === "red" ? rawBand : "yellow";
    return (
      <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
        <HealthBadge
          band={band}
          score={typeof payload.score === "number" ? payload.score : 0}
          trend={{ direction: "stable" }}
          sufficientData={typeof payload.score === "number"}
          size="compact"
        />
      </BlockShell>
    );
  }
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance}>
      <span className={pageStyles.compositionButton}>{label}</span>
    </BlockShell>
  );
}

export const BLOCK_RENDERERS: Record<KnownCompositionBlockType, BlockComponent> = {
  account_overview: AccountOverviewBlock,
  claim_summary: ClaimSummaryBlock,
  evidence_list: EvidenceListBlock,
  health_snapshot: HealthSnapshotBlock,
  relationship_map: RelationshipMapBlock,
  risk_callout: RiskCalloutBlock,
  action_list: ActionListBlock,
  markdown_document: MarkdownDocumentBlock,
  "dailyos/pill": PrimitiveBlock,
  "dailyos/status-dot": PrimitiveBlock,
  "dailyos/provenance-tag": PrimitiveBlock,
  "dailyos/health-badge": PrimitiveBlock,
  "dailyos/avatar": PrimitiveBlock,
  "dailyos/freshness-indicator": PrimitiveBlock,
  "dailyos/trust-band-badge": PrimitiveBlock,
  "dailyos/intelligence-quality-badge": PrimitiveBlock,
  "dailyos/entity-chip": PrimitiveBlock,
  "dailyos/type-badge": PrimitiveBlock,
  "dailyos/score-band": PrimitiveBlock,
};
