import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { ChevronDown } from "lucide-react";
import type { AccountProduct } from "@/types";
import { formatProvenanceSource } from "@/components/ui/ProvenanceLabel";
import { IntelligenceFeedback } from "@/components/ui/IntelligenceFeedback";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";

import shared from "@/styles/entity-detail.module.css";
import styles from "@/pages/AccountDetailEditorial.module.css";

interface AccountProductsSectionProps {
  accountId: string;
  products: AccountProduct[];
  getFeedback: (fieldPath: string) => "positive" | "negative" | null;
  onFeedback: (fieldPath: string, type: "positive" | "negative") => void;
  onRefresh: () => void;
  silentRefresh: () => void;
}

export function AccountProductsSection({
  accountId,
  products,
  getFeedback,
  onFeedback,
  onRefresh,
  silentRefresh,
}: AccountProductsSectionProps) {
  const [editingProductId, setEditingProductId] = useState<number | null>(null);
  const [editProductName, setEditProductName] = useState("");
  const [editProductStatus, setEditProductStatus] = useState("");
  const [statusDropdownProductId, setStatusDropdownProductId] = useState<number | null>(null);

  // Close product status dropdown on outside click
  useEffect(() => {
    if (statusDropdownProductId === null) return;
    const handler = () => setStatusDropdownProductId(null);
    const id = setTimeout(() => document.addEventListener("click", handler), 0);
    return () => { clearTimeout(id); document.removeEventListener("click", handler); };
  }, [statusDropdownProductId]);

  const handleCorrectProduct = async (product: AccountProduct) => {
    try {
      await invoke("correct_account_product", {
        accountId,
        productId: product.id,
        name: editProductName,
        status: editProductStatus || null,
        sourceToPenalize: product.source,
      });
      setEditingProductId(null);
      await onRefresh();
      toast.success(`Product "${editProductName}" updated`);
    } catch (err) {
      console.error("correct_account_product failed:", err);
      toast.error("Failed to update product");
    }
  };

  return (
    <div className={`editorial-reveal ${shared.marginLabelSection}`}>
      <div className={shared.marginLabel}>Products</div>
      <div className={shared.marginContent}>
        <ChapterHeading title="Products & Entitlements" />
        {products && products.length > 0 ? (
          <div className={styles.productList}>
            {products.map((product) => {
              const confidencePct = Math.round(product.confidence * 100);
              const sourceLabel = formatProvenanceSource(product.source);
              const tooltipText = `${sourceLabel ?? "Unknown source"} \u00b7 ${confidencePct}% confidence`;

              const setProductStatus = (nextStatus: string) => {
                setStatusDropdownProductId(null);
                void invoke("correct_account_product", {
                  accountId,
                  productId: product.id,
                  name: product.name,
                  status: nextStatus,
                  sourceToPenalize: product.source,
                }).then(() => {
                  void silentRefresh();
                }).catch((err: unknown) => {
                  console.error("correct_account_product failed:", err);
                  toast.error("Failed to update product status");
                });
              };

              return (
                <div key={`${product.id}-${product.name}`} className={styles.productRow}>
                  {/* Left: name (inline editable) + source provenance */}
                  <div
                    className={styles.productInfo}
                    onClick={() => {
                      if (editingProductId !== product.id) {
                        setEditingProductId(product.id);
                        setEditProductName(product.name);
                        setEditProductStatus(product.status);
                      }
                    }}
                  >
                    {editingProductId === product.id ? (
                      <input
                        className={styles.productEditInput}
                        value={editProductName}
                        onChange={(e) => setEditProductName(e.target.value)}
                        onBlur={() => void handleCorrectProduct(product)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") void handleCorrectProduct(product);
                          if (e.key === "Escape") setEditingProductId(null);
                        }}
                        autoFocus
                      />
                    ) : (
                      <div className={styles.productName}>{product.name}</div>
                    )}
                    </div>

                  {/* Center: status badge with dropdown */}
                  {product.status && (
                    <div className={styles.productStatusWrapper}>
                      <button
                        type="button"
                        className={styles.productStatusBadge}
                        data-status={product.status}
                        onClick={() => setStatusDropdownProductId(
                          statusDropdownProductId === product.id ? null : product.id
                        )}
                      >
                        {product.status}
                        <ChevronDown size={8} strokeWidth={2} />
                      </button>
                      {statusDropdownProductId === product.id && (
                        <div className={styles.productStatusDropdown}>
                          {["active", "trial", "churned"].map((s) => (
                            <button
                              key={s}
                              type="button"
                              className={`${styles.productStatusOption} ${s === product.status ? styles.productStatusOptionActive : ""}`}
                              data-status={s}
                              onClick={() => setProductStatus(s)}
                            >
                              {s}
                            </button>
                          ))}
                        </div>
                      )}
                    </div>
                  )}

                  {/* Right: feedback + dismiss */}
                  <div className={styles.productRight}>
                    <span className={styles.productFeedback}>
                      <IntelligenceFeedback
                        value={getFeedback(`products[${product.id}]`)}
                        onFeedback={(type) => {
                          onFeedback(`products[${product.id}]`, type);
                        }}
                      />
                    </span>
                    <button
                      type="button"
                      className={styles.productDismiss}
                      onClick={(e) => {
                        e.stopPropagation();
                        onFeedback(`products[${product.id}]`, "negative");
                        void handleCorrectProduct({
                          ...product,
                          status: "churned",
                        });
                      }}
                      title={`Remove — ${tooltipText}`}
                    >
                      ×
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        ) : (
          <p className={styles.sectionEmpty}>No products captured yet.</p>
        )}
      </div>
    </div>
  );
}
