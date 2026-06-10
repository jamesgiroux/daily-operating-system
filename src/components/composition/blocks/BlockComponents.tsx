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

type Payload = Record<string, unknown>;

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
  // Single-claim blocks carry provenance_kind at the top level; the ActionList
  // (commitment) and RelationshipMap (relationship) blocks carry it on their
  // single item/node. Check both so the block-level fade covers every
  // claim-backed block.
  const candidate =
    text(payload.provenance_kind) ??
    text(array(payload.items)[0]?.provenance_kind) ??
    text(array(payload.nodes)[0]?.provenance_kind);
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

function feedbackRoute(block: ProjectedBlock): EditRoute | null {
  return block.edit_routes.find((route) => route.feedback_allowed && route.claim_refs.length > 0) ?? null;
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
        pageStyles.compositionBlock,
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
      <BlockFeedback accountId={accountId} entityType={entityType} block={block} payload={payload} />
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
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title={text(payload.title) ?? text(payload.intent)} empty={empty}>
      {text(payload.text) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/text" value={text(payload.text) ?? ""} as="p" className={pageStyles.compositionNarrative} />
      )}
      {text(payload.body) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/body" value={text(payload.body) ?? ""} as="p" className={pageStyles.compositionBodyText} />
      )}
    </BlockShell>
  );
}

function HealthSnapshotBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const band = text(payload.band) ?? text(payload.trust_band);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title="Health">
      <div className={pageStyles.compositionMetricGrid}>
        {typeof payload.score === "number" && (
          <div className={pageStyles.compositionMetric}>
            <span className={pageStyles.compositionMetricValue}>{payload.score}</span>
            <span className={pageStyles.compositionMetricLabel}>Score</span>
          </div>
        )}
        {band && (
          <div className={pageStyles.compositionMetric}>
            <span className={pageStyles.compositionMetricValue}>{band.replace(/_/g, " ")}</span>
            <span className={pageStyles.compositionMetricLabel}>Band</span>
          </div>
        )}
      </div>
      {text(payload.text) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/text" value={text(payload.text) ?? ""} as="p" className={pageStyles.compositionNarrative} />
      )}
    </BlockShell>
  );
}

function RiskCalloutBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const primary = text(payload.text) ?? text(payload.body);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title={text(payload.title) ?? "Risk"}>
      {primary && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath={text(payload.text) ? "/text" : "/body"} value={primary} as="p" className={pageStyles.compositionNarrative} />
      )}
      {text(payload.recommended_action) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/recommended_action" value={text(payload.recommended_action) ?? ""} as="p" className={pageStyles.compositionBodyText} />
      )}
    </BlockShell>
  );
}

function RelationshipMapBlock({ block, accountId, entityType, payload, renderedProvenance }: BlockComponentProps) {
  const nodes = array(payload.nodes);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title="Relationships">
      <div className={pageStyles.compositionRelationshipGrid}>
        {nodes.map((node, index) => (
          <div className={pageStyles.compositionPersonNode} key={`${text(node.claim_id) ?? text(node.label) ?? "node"}-${index}`}>
            <span className={pageStyles.compositionAvatar}>{(text(node.label) ?? text(node.text) ?? "?").slice(0, 1)}</span>
            <span>{text(node.label) ?? text(node.text)}</span>
          </div>
        ))}
      </div>
    </BlockShell>
  );
}

function ActionListBlock({ block, accountId, entityType, payload, renderedProvenance }: BlockComponentProps) {
  const items = array(payload.items);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title={text(payload.title) ?? "Actions"} empty={items.length === 0}>
      <div className={pageStyles.compositionBlockStack}>
        {items.map((item, index) => (
          <div className={pageStyles.compositionActionRow} key={`${text(item.claim_id) ?? text(item.title) ?? "action"}-${index}`}>
            <span className={pageStyles.compositionActionIndex}>{index + 1}</span>
            <div>
              <p className={pageStyles.compositionEvidenceTitle}>{text(item.title) ?? text(item.text)}</p>
              {text(item.status) && <p className={pageStyles.compositionEvidenceMeta}>{text(item.status)}</p>}
            </div>
          </div>
        ))}
      </div>
    </BlockShell>
  );
}

function EvidenceListBlock({ block, accountId, entityType, payload, renderedProvenance }: BlockComponentProps) {
  const items = array(payload.items);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title={text(payload.title) ?? "Evidence"} empty={items.length === 0}>
      <div className={pageStyles.compositionEvidenceList}>
        {items.map((item, index) => {
          const title = text(item.evidence_quote) ?? text(item.label);
          const assertion = text(item.assertion_text);
          const meta = [
            text(item.source_label),
            text(item.source_asof),
            text(item.workspace_file_kind),
            text(item.trust_band),
            text(item.sensitivity),
            text(item.redaction_state),
          ].filter(Boolean);
          return (
            <div className={pageStyles.compositionEvidenceRow} key={`${title ?? "evidence"}-${index}`}>
              <div className={pageStyles.compositionEvidenceMain}>
                <p className={pageStyles.compositionEvidenceTitle}>{title}</p>
                {assertion && assertion !== title && (
                  <p className={pageStyles.compositionEvidenceMeta}>{assertion}</p>
                )}
                {meta.length > 0 && (
                  <p className={pageStyles.compositionEvidenceMeta}>{meta.join(" · ")}</p>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </BlockShell>
  );
}

function MarkdownDocumentBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const sections = array(payload.sections);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title={text(payload.title)}>
      {text(payload.body) && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath="/body" value={text(payload.body) ?? ""} as="p" className={pageStyles.compositionBodyText} />
      )}
      {sections.map((section, index) => (
        <section key={`${text(section.heading) ?? "section"}-${index}`}>
          {text(section.heading) && <h4 className={pageStyles.compositionEvidenceTitle}>{text(section.heading)}</h4>}
          {text(section.body) && <p className={pageStyles.compositionBodyText}>{text(section.body)}</p>}
        </section>
      ))}
    </BlockShell>
  );
}

export function GenericTextBlock({ block, accountId, entityType, payload, renderedProvenance, editMode }: BlockComponentProps) {
  const title = text(payload.title) ?? text(payload.label);
  const body = text(payload.text) ?? text(payload.body);
  return (
    <BlockShell block={block} accountId={accountId} entityType={entityType} payload={payload} renderedProvenance={renderedProvenance} title={title}>
      {body && (
        <EditableBlockText accountId={accountId} entityType={entityType} block={block} editMode={editMode} fieldPath={text(payload.text) ? "/text" : "/body"} value={body} as="p" className={pageStyles.compositionBodyText} />
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
