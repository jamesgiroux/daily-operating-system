import { useState } from "react";
import { EditableText } from "@/components/ui/EditableText";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { formatShortDate } from "@/lib/utils";
import type { AccountProduct, AccountTechnicalFootprint as TechnicalFootprintData } from "@/types";

import inlineStyles from "@/pages/AccountDetailEditorial.module.css";
import refCss from "@/components/context/ReferenceGrid.module.css";
import styles from "./AccountTechnicalFootprint.module.css";

interface AccountTechnicalFootprintProps {
  /**
   * DOS-18: chapter variant accepts null so the chapter can always render
   * with all-gap rows when the account has no captured technical footprint.
   * Inline variant still requires a populated footprint (guarded below).
   */
  footprint: TechnicalFootprintData | null;
  /** DOS-18: render as a full chapter surface — ref-grid with gap rows + feature list. */
  variant?: "inline" | "chapter";
  /** DOS-18: feature list from productAdoption.featureAdoption (chapter variant only). */
  featureAdoption?: string[];
  /**
   * Products owned by the account, rendered as a dotted list alongside
   * Feature adoption. Dot color reflects status (active / trial / churned).
   * Chapter variant only. Full edit UX (status dropdown, product-level
   * Bayesian feedback) is tracked in DOS-251 for v1.2.2.
   */
  products?: AccountProduct[];
  onCaptureGap?: (field: string) => void;
  onUpdateField?: (field: string, value: string) => Promise<void> | void;
  onUpdateMetadata?: (key: string, value: string) => Promise<void> | void;
  metadataValues?: Record<string, string>;
}

function productDotClass(status: string): string {
  switch (status.toLowerCase()) {
    case "active":
      return refCss.featureDot;
    case "trial":
      return `${refCss.featureDot} ${refCss.featureDotTrial}`;
    case "churned":
      return `${refCss.featureDot} ${refCss.featureDotChurned}`;
    default:
      return refCss.featureDot;
  }
}

const FEATURE_CAP = 6;
const ACRONYMS = new Set(["ai", "api", "bi", "cdn", "cms", "crm", "erp", "gdpr", "hipaa", "sso"]);

interface ProductGroup {
  key: string;
  label: string;
  status: string;
  products: AccountProduct[];
  features: AccountProduct[];
}

function normalizeKey(value: string): string {
  return value.trim().toLowerCase().replace(/[^a-z0-9]+/g, " ").trim();
}

function formatProductLabel(value: string): string {
  return value
    .trim()
    .replace(/[_-]+/g, " ")
    .split(/\s+/)
    .filter(Boolean)
    .map((word) => {
      const lower = word.toLowerCase();
      if (ACRONYMS.has(lower)) return lower.toUpperCase();
      return `${lower.charAt(0).toUpperCase()}${lower.slice(1)}`;
    })
    .join(" ");
}

function isPrimaryProductName(name: string): boolean {
  const normalized = normalizeKey(name);
  const words = normalized.split(/\s+/).filter(Boolean);
  return Boolean(normalized) && name.length <= 32 && words.length <= 3;
}

function aggregateStatus(products: AccountProduct[]): string {
  if (products.some((product) => product.status.toLowerCase() === "active")) return "active";
  if (products.some((product) => product.status.toLowerCase() === "trial")) return "trial";
  if (products.some((product) => product.status.toLowerCase() === "churned")) return "churned";
  return products[0]?.status ?? "active";
}

function buildProductGroups(products: AccountProduct[]): ProductGroup[] {
  const groups = new Map<string, ProductGroup>();

  products.forEach((product) => {
    const name = product.name.trim();
    if (!name) return;

    const category = product.category?.trim();
    const hasCategory = Boolean(category);
    const label = hasCategory
      ? formatProductLabel(category as string)
      : isPrimaryProductName(name)
        ? formatProductLabel(name)
        : "Captured products";
    const key = hasCategory
      ? `category:${normalizeKey(category as string)}`
      : isPrimaryProductName(name)
        ? `product:${normalizeKey(name)}`
        : "uncategorized";

    const group = groups.get(key) ?? {
      key,
      label,
      status: product.status,
      products: [],
      features: [],
    };

    group.products.push(product);

    const nameMatchesLabel = normalizeKey(name) === normalizeKey(label);
    if ((hasCategory && !nameMatchesLabel) || (!hasCategory && !isPrimaryProductName(name))) {
      group.features.push(product);
    }

    group.status = aggregateStatus(group.products);
    groups.set(key, group);
  });

  return Array.from(groups.values()).sort((a, b) => {
    if (a.label === "Captured products") return 1;
    if (b.label === "Captured products") return -1;
    return a.label.localeCompare(b.label);
  });
}

interface RefRow {
  label: string;
  field: string;
  value: string;
  gap?: boolean;
  editableField?: string;
  metadataKey?: string;
  editableValue?: string;
  options?: string[];
}

const CLEAR_SELECT_VALUE = "__clear__";

function EditableSelect({
  value,
  options,
  onChange,
}: {
  value: string;
  options: string[];
  onChange: (value: string) => void;
}) {
  return (
    <Select
      value={value || undefined}
      onValueChange={(nextValue) => onChange(nextValue === CLEAR_SELECT_VALUE ? "" : nextValue)}
    >
      <SelectTrigger size="sm">
        <SelectValue placeholder="Set value..." />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value={CLEAR_SELECT_VALUE}>Clear</SelectItem>
        {options.map((option) => (
          <SelectItem key={option} value={option}>
            {option}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

const USAGE_TIER_OPTIONS = ["starter", "professional", "growth", "enterprise"];
const SERVICES_STAGE_OPTIONS = ["onboarding", "implementation", "optimization", "steady-state"];
const SUPPORT_TIER_OPTIONS = ["basic", "standard", "premium", "premier"];

export function AccountTechnicalFootprint({
  footprint,
  variant = "inline",
  featureAdoption,
  products,
  onUpdateField,
  onUpdateMetadata,
  metadataValues = {},
}: AccountTechnicalFootprintProps) {
  const tf = footprint;
  const [expandedProducts, setExpandedProducts] = useState<Set<string>>(() => new Set());

  if (variant === "chapter") {
    const productGroups = products && products.length > 0 ? buildProductGroups(products) : [];
    const metadataText = (key: string) => metadataValues[key]?.trim() ?? "";
    const technicalRow = (
      label: string,
      field: string,
      fallbackValue: string,
      editableFallback: string,
      sentinel: string,
      options?: string[],
    ): RefRow => {
      const metadataKey = `technical_shape:${field}`;
      const captured = metadataText(metadataKey);
      return {
        label,
        field,
        value: captured || fallbackValue || sentinel,
        gap: !(captured || fallbackValue),
        metadataKey,
        editableValue: metadataValues[metadataKey] ?? editableFallback,
        options,
      };
    };
    const rows: RefRow[] = [
      technicalRow(
        "Usage tier",
        "usage_tier",
        tf?.usageTier ?? "",
        tf?.usageTier ?? "",
        "— not captured",
        USAGE_TIER_OPTIONS,
      ),
      technicalRow(
        "Active users",
        "active_users",
        tf?.activeUsers != null && tf.activeUsers > 0 ? tf.activeUsers.toLocaleString() : "",
        tf?.activeUsers != null && tf.activeUsers > 0 ? tf.activeUsers.toLocaleString() : "",
        "— not captured",
      ),
      technicalRow(
        "Services stage",
        "services_stage",
        tf?.servicesStage ?? "",
        tf?.servicesStage ?? "",
        "— not captured",
        SERVICES_STAGE_OPTIONS,
      ),
      technicalRow(
        "Support tier",
        "support_tier",
        tf?.supportTier ?? "",
        tf?.supportTier ?? "",
        "— not captured",
        SUPPORT_TIER_OPTIONS,
      ),
      technicalRow(
        "Open tickets",
        "open_tickets",
        tf?.openTickets != null ? String(tf.openTickets) : "",
        tf?.openTickets != null ? String(tf.openTickets) : "",
        "— not captured",
      ),
      technicalRow(
        "CSAT",
        "csat_score",
        tf?.csatScore != null && tf.csatScore > 0 ? `${tf.csatScore.toFixed(1)}/5` : "",
        tf?.csatScore != null && tf.csatScore > 0 ? `${tf.csatScore.toFixed(1)}/5` : "",
        "— not captured",
      ),
      technicalRow(
        "Adoption score",
        "adoption_score",
        tf?.adoptionScore != null && tf.adoptionScore > 0 ? `${Math.round(tf.adoptionScore * 100)}%` : "",
        tf?.adoptionScore != null && tf.adoptionScore > 0 ? `${Math.round(tf.adoptionScore * 100)}%` : "",
        "— not computed",
      ),
      technicalRow("Integrations", "integrations", "", "", "— not captured"),
    ];

    return (
      <div>
        <div className={refCss.grid}>
          {rows.map((row) => {
            const canEdit = Boolean(
              (onUpdateField && row.editableField) || (onUpdateMetadata && row.metadataKey),
            );
            return (
              <div key={row.label} className={refCss.row}>
                <span className={refCss.label}>{row.label}</span>
                <span className={row.gap && !canEdit ? `${refCss.value} ${refCss.valueGap}` : refCss.value}>
                  {canEdit && row.options ? (
                    <EditableSelect
                      value={row.editableValue ?? ""}
                      options={row.options}
                      onChange={(v) => {
                        if (row.editableField) onUpdateField?.(row.editableField, v.trim());
                        else if (row.metadataKey) onUpdateMetadata?.(row.metadataKey, v.trim());
                      }}
                    />
                  ) : canEdit ? (
                    <EditableText
                      value={row.editableValue ?? ""}
                      placeholder={row.gap ? "Capture →" : row.value}
                      onChange={(v) => {
                        if (row.editableField) onUpdateField?.(row.editableField, v.trim());
                        else if (row.metadataKey) onUpdateMetadata?.(row.metadataKey, v.trim());
                      }}
                      as="span"
                      multiline={false}
                    />
                  ) : (
                    row.value
                  )}
                </span>
              </div>
            );
          })}
        </div>

        {productGroups.length > 0 && (
          <>
            <div className={refCss.featureHeading}>
              Products · {products?.length ?? 0}
            </div>
            <div className={styles.productGroups}>
              {productGroups.map((group) => {
                const isExpanded = expandedProducts.has(group.key);
                const visibleFeatures = isExpanded ? group.features : group.features.slice(0, FEATURE_CAP);
                const hiddenFeatureCount = group.features.length - visibleFeatures.length;

                return (
                  <div
                    key={group.key}
                    className={styles.productGroup}
                  >
                    <div className={`${refCss.featureItem} ${styles.productHeader}`}>
                      <span aria-hidden className={productDotClass(group.status)} />
                      <span className={styles.productLabel}>
                        {group.label}
                      </span>
                      <span className={styles.productMeta}>
                        {group.features.length > 0
                          ? `${group.features.length} feature${group.features.length === 1 ? "" : "s"}`
                          : group.status}
                      </span>
                    </div>

                    {visibleFeatures.length > 0 && (
                      <div
                        className={`${refCss.featureList} ${styles.productFeatureList}`}
                      >
                        {visibleFeatures.map((feature) => (
                          <div key={`${feature.id}-${feature.name}`} className={refCss.featureItem}>
                            <span aria-hidden className={productDotClass(feature.status)} />
                            {feature.name}
                          </div>
                        ))}
                      </div>
                    )}

                    {hiddenFeatureCount > 0 && (
                      <button
                        type="button"
                        onClick={() => {
                          setExpandedProducts((current) => {
                            const next = new Set(current);
                            if (next.has(group.key)) {
                              next.delete(group.key);
                            } else {
                              next.add(group.key);
                            }
                            return next;
                          });
                        }}
                        className={styles.showMoreButton}
                      >
                        {isExpanded ? "Show fewer" : `Show ${hiddenFeatureCount} more`}
                      </button>
                    )}
                  </div>
                );
              })}
            </div>
          </>
        )}

        {featureAdoption && featureAdoption.length > 0 && (
          <>
            <div className={refCss.featureHeading}>
              Feature adoption · {featureAdoption.length} active
            </div>
            <div className={refCss.featureList}>
              {featureAdoption.map((feature) => (
                <div key={feature} className={refCss.featureItem}>
                  <span aria-hidden className={refCss.featureDot} />
                  {feature}
                </div>
              ))}
            </div>
            {tf?.sourcedAt && (
              <div className={refCss.featureSource}>
                All {featureAdoption.length} features active as of {formatShortDate(tf.sourcedAt)}
                {tf.source ? ` (${tf.source})` : ""}
              </div>
            )}
          </>
        )}
      </div>
    );
  }

  // Inline variant bails out when the account has no footprint at all.
  if (!tf) return null;

  const items: { label: string; value: string }[] = [];
  if (tf.supportTier) items.push({ label: "Support", value: tf.supportTier });
  if (tf.csatScore != null && tf.csatScore > 0) items.push({ label: "CSAT", value: `${tf.csatScore.toFixed(1)}/5` });
  if (tf.openTickets != null && tf.openTickets > 0) items.push({ label: "Open Tickets", value: String(tf.openTickets) });
  if (tf.usageTier) items.push({ label: "Usage Tier", value: tf.usageTier });
  if (tf.activeUsers != null && tf.activeUsers > 0) items.push({ label: "Active Users", value: tf.activeUsers.toLocaleString() });
  if (tf.adoptionScore != null && tf.adoptionScore > 0) items.push({ label: "Adoption", value: `${Math.round(tf.adoptionScore * 100)}%` });
  if (tf.servicesStage) items.push({ label: "Services", value: tf.servicesStage });

  if (items.length === 0) return null;

  return (
    <div className={inlineStyles.technicalFootprint}>
      <div className={inlineStyles.technicalFootprintLabel}>Technical Footprint</div>
      <div className={inlineStyles.technicalFootprintGrid}>
        {items.map((item) => (
          <div key={item.label} className={inlineStyles.technicalFootprintItem}>
            <span className={inlineStyles.technicalFootprintItemLabel}>{item.label}</span>
            <span className={inlineStyles.technicalFootprintItemValue}>{item.value}</span>
          </div>
        ))}
      </div>
      <div className={inlineStyles.technicalFootprintSource}>
        Source: {tf.source} &middot; {formatShortDate(tf.sourcedAt)}
      </div>
    </div>
  );
}
