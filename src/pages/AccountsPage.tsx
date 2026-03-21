import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
import { toast } from "sonner";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import {
  BulkCreateForm,
  parseBulkCreateInput,
} from "@/components/ui/bulk-create-form";
import {
  EntityListSkeleton,
  EntityListError,
  EntityListEmpty,
  EntityListHeader,
  EntityListEndMark,
  FilterTabs,
} from "@/components/entity/EntityListShell";
import { EntityRow } from "@/components/entity/EntityRow";
import { ChapterHeading } from "@/components/editorial/ChapterHeading";
import { EmptyState } from "@/components/editorial/EmptyState";
import { EphemeralBriefing } from "@/components/editorial/EphemeralBriefing";
import { usePersonality } from "@/hooks/usePersonality";
import { getPersonalityCopy } from "@/lib/personality";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { formatArr } from "@/lib/utils";
import type { AccountListItem, DiscoveredAccount, EphemeralBriefing as EphemeralBriefingType, FeatureFlags } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import { HealthBadge } from "@/components/shared/HealthBadge";
import styles from "./AccountsPage.module.css";

/** Lightweight shape returned by get_archived_accounts (DbAccount from Rust). */
interface ArchivedAccount {
  id: string;
  name: string;
  lifecycle?: string;
  arr?: number;
  health?: string;
  archived: boolean;
}

type ArchiveTab = "active" | "archived";

const healthDotColor: Record<string, string> = {
  green: "var(--color-garden-sage)",
  yellow: "var(--color-spice-saffron)",
  red: "var(--color-spice-terracotta)",
};

/** Section configuration for the three account type groups (I383). */
const ACCOUNT_SECTIONS: {
  type: AccountListItem["accountType"];
  title: string;
}[] = [
  { type: "customer", title: "Your Book" },
  { type: "internal", title: "Your Team" },
  { type: "partner", title: "Your Partners" },
];

export default function AccountsPage() {
  const { personality } = usePersonality();
  const navigate = useNavigate();
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lifecycleFilter, setLifecycleFilter] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [newAccountType, setNewAccountType] = useState<"customer" | "internal" | "partner">("customer");
  const [newParentId, setNewParentId] = useState<string | null>(null);
  const [expandedParents, setExpandedParents] = useState<Set<string>>(new Set());
  const [childrenCache, setChildrenCache] = useState<Record<string, AccountListItem[]>>({});
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedAccounts, setArchivedAccounts] = useState<ArchivedAccount[]>([]);
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkValue, setBulkValue] = useState("");

  // I494/I495: Glean discovery and ephemeral query state
  const [gleanConnected, setGleanConnected] = useState(false);
  const [discoveryOpen, setDiscoveryOpen] = useState(false);
  const [discoveryLoading, setDiscoveryLoading] = useState(false);
  const [discoveredAccounts, setDiscoveredAccounts] = useState<DiscoveredAccount[]>([]);
  const [discoveryFilter, setDiscoveryFilter] = useState("");
  const [addedNames, setAddedNames] = useState<Set<string>>(new Set());
  const [ephemeralQuery, setEphemeralQuery] = useState("");
  const [ephemeralLoading, setEphemeralLoading] = useState(false);
  const [ephemeralBriefing, setEphemeralBriefing] = useState<EphemeralBriefingType | null>(null);
  const [ephemeralAdded, setEphemeralAdded] = useState(false);

  const loadAccounts = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AccountListItem[]>("get_accounts_list");
      setAccounts(result);

      // Auto-expand all parents recursively and pre-fetch the full tree
      const expanded = new Set<string>();
      const cache: Record<string, AccountListItem[]> = {};

      async function expandRecursive(items: AccountListItem[]) {
        const parents = items.filter((a) => a.isParent);
        if (parents.length === 0) return;
        await Promise.all(
          parents.map(async (p) => {
            expanded.add(p.id);
            if (!cache[p.id]) {
              try {
                const children = await invoke<AccountListItem[]>("get_child_accounts_list", { parentId: p.id });
                cache[p.id] = children;
                await expandRecursive(children);
              } catch { /* ignore */ }
            }
          }),
        );
      }

      await expandRecursive(result);
      if (expanded.size > 0) {
        setExpandedParents(expanded);
        setChildrenCache((prev) => ({ ...prev, ...cache }));
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadArchivedAccounts = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<ArchivedAccount[]>("get_archived_accounts");
      setArchivedAccounts(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (archiveTab === "active") {
      loadAccounts();
    } else {
      loadArchivedAccounts();
    }
  }, [archiveTab, loadAccounts, loadArchivedAccounts]);

  // I494: Check Glean connection status + feature flag on mount
  const [discoveryEnabled, setDiscoveryEnabled] = useState(false);
  useEffect(() => {
    invoke<FeatureFlags>("get_feature_flags")
      .then((flags) => {
        if (flags.glean_discovery_enabled) {
          setDiscoveryEnabled(true);
          invoke<{ status: string }>("get_glean_auth_status")
            .then((result) => setGleanConnected(result.status === "authenticated"))
            .catch(() => setGleanConnected(false));
        }
      })
      .catch(() => setDiscoveryEnabled(false)); // Expected: feature flag check on init
  }, []);

  // I494: Discover accounts from Glean
  async function handleDiscoverAccounts() {
    setDiscoveryOpen(true);
    setDiscoveryLoading(true);
    setDiscoveredAccounts([]);
    try {
      const result = await invoke<DiscoveredAccount[]>("discover_accounts_from_glean");
      setDiscoveredAccounts(result);
    } catch (e) {
      console.error("discover_accounts_from_glean failed:", e);
      toast.error("Failed to discover accounts");
      setError(String(e));
    } finally {
      setDiscoveryLoading(false);
    }
  }

  // I494: Add a discovered account
  async function handleAddDiscovered(account: DiscoveredAccount) {
    try {
      await invoke<string>("import_account_from_glean", {
        request: {
          name: account.name,
          myRole: account.myRole,
          evidence: account.evidence,
          source: account.source,
          domain: account.domain,
          industry: account.industry,
          contextPreview: account.contextPreview,
          sections: [],
          summary: null,
        },
      });
      setAddedNames((prev) => new Set(prev).add(account.name.toLowerCase()));
      await loadAccounts();
    } catch (e) {
      console.error("import_account_from_glean failed:", e);
      toast.error("Failed to add account");
      setError(String(e));
    }
  }

  // I495: Ephemeral account query
  async function handleEphemeralQuery(e: React.FormEvent) {
    e.preventDefault();
    if (!ephemeralQuery.trim()) return;
    setEphemeralLoading(true);
    setEphemeralBriefing(null);
    setEphemeralAdded(false);
    try {
      const result = await invoke<EphemeralBriefingType>("query_ephemeral_account", {
        name: ephemeralQuery.trim(),
      });
      setEphemeralBriefing(result);
    } catch (e) {
      console.error("query_ephemeral_account failed:", e);
      toast.error("Account lookup failed");
      setError(String(e));
    } finally {
      setEphemeralLoading(false);
    }
  }

  // I495: Add account from ephemeral briefing
  async function handleAddFromBriefing() {
    if (!ephemeralBriefing) return;
    try {
      await invoke<string>("import_account_from_glean", {
        request: {
          name: ephemeralBriefing.name,
          summary: ephemeralBriefing.summary,
          sections: ephemeralBriefing.sections,
          contextPreview: ephemeralBriefing.summary,
          myRole: null,
          evidence: null,
          source: "Glean briefing",
          domain: null,
          industry: null,
        },
      });
      setEphemeralAdded(true);
      await loadAccounts();
    } catch (e) {
      console.error("import_account_from_glean (briefing) failed:", e);
      toast.error("Failed to add account");
      setError(String(e));
    }
  }

  // Load role preset for portfolio report label

  async function handleCreate() {
    if (!newName.trim()) return;
    try {
      await invoke<string>("create_account", {
        name: newName.trim(),
        accountType: newAccountType,
        parentId: newParentId,
      });
      setNewName("");
      setNewAccountType("customer");
      setNewParentId(null);
      setCreating(false);
      await loadAccounts();
    } catch (e) {
      console.error("create_account failed:", e);
      toast.error("Failed to create account");
      setError(String(e));
    }
  }

  // Potential parent accounts: same type, not archived.
  // Recursively flattens the full hierarchy from childrenCache so deeply
  // nested accounts (e.g. Globex > Enterprise > Sales > CS > Key Accounts)
  // all appear as selectable parents. Each entry carries its depth for
  // visual indentation in the dropdown.
  const parentOptions = useMemo(() => {
    if (newAccountType === "partner") return [];
    const result: (AccountListItem & { _depth: number })[] = [];

    function walk(items: AccountListItem[], depth: number) {
      for (const acct of items) {
        if (acct.archived) continue;
        result.push({ ...acct, _depth: depth });
        const children = childrenCache[acct.id];
        if (children) walk(children, depth + 1);
      }
    }

    const topLevel = accounts.filter(
      (a) => a.accountType === newAccountType && !a.archived,
    );
    walk(topLevel, 0);
    return result;
  }, [accounts, childrenCache, newAccountType]);

  async function handleBulkCreate() {
    const names = parseBulkCreateInput(bulkValue);
    if (names.length === 0) return;
    try {
      await invoke<string[]>("bulk_create_accounts", { names });
      setBulkValue("");
      setBulkMode(false);
      await loadAccounts();
    } catch (e) {
      console.error("bulk_create_accounts failed:", e);
      toast.error("Failed to create accounts");
      setError(String(e));
    }
  }

  async function toggleExpand(parentId: string) {
    const next = new Set(expandedParents);
    if (next.has(parentId)) {
      next.delete(parentId);
    } else {
      next.add(parentId);
      if (!childrenCache[parentId]) {
        try {
          // I316: Use get_descendant_accounts for n-level nesting support
          const children = await invoke<AccountListItem[]>(
            "get_child_accounts_list",
            { parentId }
          );
          setChildrenCache((prev) => ({ ...prev, [parentId]: children }));
        } catch (e) {
          console.error("get_child_accounts_list failed:", e);
          toast.error("Failed to load sub-accounts");
          setError(String(e));
          return;
        }
      }
    }
    setExpandedParents(next);
  }

  // Lifecycle values derived from data
  const lifecycleValues = useMemo(() => {
    const values = new Set<string>();
    for (const a of accounts) {
      if (a.lifecycle) values.add(a.lifecycle);
    }
    return Array.from(values).sort();
  }, [accounts]);

  const lifecycleTabs = useMemo(() => ["all", ...lifecycleValues] as const, [lifecycleValues]);

  // Filters
  const lifecycleFiltered =
    lifecycleFilter === "all"
      ? accounts
      : accounts.filter((a) => a.lifecycle === lifecycleFilter);

  const filtered = useMemo(() => {
    if (!searchQuery) return lifecycleFiltered;
    const q = searchQuery.toLowerCase();
    return lifecycleFiltered.filter((a) => {
      if (a.name.toLowerCase().includes(q)) {
        return true;
      }
      const children = childrenCache[a.id];
      if (children && children.some((c) => c.name.toLowerCase().includes(q))) {
        return true;
      }
      return false;
    });
  }, [searchQuery, lifecycleFiltered, childrenCache]);

  // D3: Auto-expand parent accounts when a child matches the search query
  useEffect(() => {
    if (!searchQuery) return;
    const q = searchQuery.toLowerCase();
    const toExpand = new Set<string>();
    for (const a of filtered) {
      if (childrenCache[a.id]?.some((c) => c.name.toLowerCase().includes(q))) {
        toExpand.add(a.id);
      }
    }
    if (toExpand.size > 0) {
      setExpandedParents((prev) => {
        const next = new Set(prev);
        let changed = false;
        for (const id of toExpand) {
          if (!next.has(id)) { next.add(id); changed = true; }
        }
        return changed ? next : prev;
      });
    }
  }, [searchQuery, filtered, childrenCache]);

  const filteredArchived = searchQuery
    ? archivedAccounts.filter(
        (a) => a.name.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedAccounts;

  // I383: Group filtered accounts by accountType for three-section layout
  const groupedAccounts = useMemo(() => {
    const groups: Record<string, AccountListItem[]> = {
      customer: [],
      internal: [],
      partner: [],
    };
    for (const account of filtered) {
      const type = account.accountType ?? "customer";
      if (groups[type]) {
        groups[type].push(account);
      } else {
        groups.customer.push(account);
      }
    }
    return groups;
  }, [filtered]);

  const isArchived = archiveTab === "archived";
  const displayList = isArchived ? filteredArchived : filtered;
  const activeCount = accounts.filter((a) => !a.archived).length;

  const formattedDate = new Date().toLocaleDateString("en-US", {
    weekday: "long",
    month: "long",
    day: "numeric",
    year: "numeric",
  }).toUpperCase();

  // FolioBar stats
  const folioStats = useMemo((): ReadinessStat[] => {
    const stats: ReadinessStat[] = [];
    if (activeCount > 0) stats.push({ label: `${activeCount} active`, color: "sage" });
    return stats;
  }, [activeCount]);

  // Folio actions: archive toggle + new button + Glean buttons
  const folioActions = useMemo(() => {
    return (
      <>
        <button
          onClick={() => setArchiveTab(isArchived ? "active" : "archived")}
          className={isArchived ? styles.folioButtonArchiveActive : styles.folioButtonArchive}
        >
          {isArchived ? "\u2190 Active" : "Archive"}
        </button>
        {!isArchived && discoveryEnabled && gleanConnected && (
          <button
            onClick={handleDiscoverAccounts}
            className={styles.folioButtonDiscover}
          >
            Discover
          </button>
        )}
        {!isArchived && (
          <button
            onClick={() => setCreating(true)}
            className={styles.folioButtonNew}
          >
            + New
          </button>
        )}
      </>
    );
  }, [isArchived, gleanConnected]);

  // Register magazine shell
  const shellConfig = useMemo(
    () => ({
      folioLabel: "Accounts",
      atmosphereColor: "turmeric" as const,
      activePage: "accounts" as const,
      folioDateText: formattedDate,
      folioReadinessStats: folioStats,
      folioActions: folioActions,
    }),
    [formattedDate, folioStats, folioActions],
  );
  useRegisterMagazineShell(shellConfig);

  // Loading state
  if (loading && (isArchived ? archivedAccounts.length === 0 : accounts.length === 0)) {
    return <EntityListSkeleton />;
  }

  // Error state
  if (error) {
    return <EntityListError error={error} onRetry={loadAccounts} />;
  }

  // Empty state
  if (!isArchived && accounts.length === 0) {
    return (
      <div className={styles.emptyContainer}>
        <h1 className={styles.emptyTitle}>Your Book</h1>
        {(() => {
          const copy = getPersonalityCopy("accounts-empty", personality);
          return (
            <EmptyState
              headline={copy.title}
              explanation={copy.explanation ?? copy.message ?? ""}
              benefit={copy.benefit}
              action={!creating ? { label: "Create your first account", onClick: () => setCreating(true) } : undefined}
            >
              {creating && (
                <div className={styles.emptyCreateForm}>
                  <AccountTypeSelector
                    value={newAccountType}
                    onChange={(v) => { setNewAccountType(v); setNewParentId(null); }}
                  />
                  {newAccountType !== "partner" && parentOptions.length > 0 && (
                    <ParentSelector
                      value={newParentId}
                      onChange={setNewParentId}
                      options={parentOptions}
                    />
                  )}
                  <InlineCreateForm
                    value={newName}
                    onChange={setNewName}
                    onCreate={handleCreate}
                    onCancel={() => { setCreating(false); setNewAccountType("customer"); setNewParentId(null); }}
                    placeholder={newParentId ? "Business unit name" : "Account name"}
                  />
                </div>
              )}
            </EmptyState>
          );
        })()}
      </div>
    );
  }

  return (
    <div className={styles.pageContainer}>
      <EntityListHeader
        headline="Your Book"
        count={isArchived ? filteredArchived.length : filtered.length}
        countLabel={isArchived ? "archived" : "active"}
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder="⌘  Search accounts..."
      >
        {/* Lifecycle filter (active only, only when lifecycle values exist) */}
        {!isArchived && lifecycleValues.length > 0 && (
          <FilterTabs
            tabs={lifecycleTabs}
            active={lifecycleFilter}
            onChange={setLifecycleFilter}
          />
        )}
      </EntityListHeader>

      {/* I494: Discovery panel */}
      {discoveryOpen && !isArchived && (
        <div className={styles.discoveryPanel}>
          <div className={styles.discoveryHeader}>
            <h3 className={styles.discoveryTitle}>
              Discovered Accounts
            </h3>
            <button
              onClick={() => { setDiscoveryOpen(false); setDiscoveredAccounts([]); setDiscoveryFilter(""); }}
              className={styles.discoveryCloseButton}
            >
              Close
            </button>
          </div>

          {discoveryLoading ? (
            <p className={styles.discoveryLoading}>
              Searching your organization...
            </p>
          ) : discoveredAccounts.length === 0 ? (
            <p className={styles.discoveryEmpty}>
              No accounts discovered. Try the search below for a specific account.
            </p>
          ) : (
            <>
              {/* Filter input */}
              <input
                type="text"
                value={discoveryFilter}
                onChange={(e) => setDiscoveryFilter(e.target.value)}
                placeholder="Filter discovered accounts..."
                className={styles.discoveryFilterInput}
              />
              <div className={styles.discoveryList}>
                {discoveredAccounts
                  .filter((a) => {
                    if (!discoveryFilter) return true;
                    const q = discoveryFilter.toLowerCase();
                    return (
                      a.name.toLowerCase().includes(q) ||
                      (a.domain ?? "").toLowerCase().includes(q) ||
                      (a.industry ?? "").toLowerCase().includes(q)
                    );
                  })
                  .slice(0, 50)
                  .map((account, i, arr) => {
                    const isAdded = account.alreadyInDailyos || addedNames.has(account.name.toLowerCase());
                    return (
                      <div
                        key={account.name + i}
                        className={`${styles.discoveryRow} ${i < arr.length - 1 ? styles.discoveryRowBorder : ""}`}
                      >
                        <div className={styles.discoveryRowContent}>
                          <div className={styles.discoveryRowHeader}>
                            <span className={styles.discoveryAccountName}>
                              {account.name}
                            </span>
                            {account.domain && (
                              <span className={styles.discoveryDomain}>
                                {account.domain}
                              </span>
                            )}
                            {account.industry && (
                              <span className={styles.discoveryIndustry}>
                                {account.industry}
                              </span>
                            )}
                          </div>
                          {account.contextPreview && (
                            <p className={styles.discoveryContextPreview}>
                              {account.contextPreview}
                            </p>
                          )}
                        </div>
                        <div className={styles.discoveryRowActions}>
                          {isAdded ? (
                            <span className={styles.discoveryAddedLabel}>
                              Added
                            </span>
                          ) : (
                            <button
                              onClick={() => handleAddDiscovered(account)}
                              className={styles.discoveryAddButton}
                            >
                              Add
                            </button>
                          )}
                        </div>
                      </div>
                    );
                  })}
              </div>
              {discoveredAccounts.filter((a) => {
                if (!discoveryFilter) return true;
                const q = discoveryFilter.toLowerCase();
                return (
                  a.name.toLowerCase().includes(q) ||
                  (a.domain ?? "").toLowerCase().includes(q) ||
                  (a.industry ?? "").toLowerCase().includes(q)
                );
              }).length > 50 && (
                <p className={styles.discoveryOverflowHint}>
                  Showing first 50. Use search to narrow the list.
                </p>
              )}
            </>
          )}

          <hr className={styles.discoveryDivider} />
        </div>
      )}

      {/* I495: Ephemeral account query */}
      {discoveryEnabled && gleanConnected && !isArchived && (
        <div className={styles.ephemeralContainer}>
          <form onSubmit={handleEphemeralQuery} className={styles.ephemeralForm}>
            <input
              type="text"
              value={ephemeralQuery}
              onChange={(e) => setEphemeralQuery(e.target.value)}
              placeholder="Tell me about..."
              className={styles.ephemeralInput}
            />
            <button
              type="submit"
              disabled={ephemeralLoading || !ephemeralQuery.trim()}
              className={`${styles.ephemeralSubmitButton} ${ephemeralLoading || !ephemeralQuery.trim() ? styles.ephemeralSubmitDisabled : styles.ephemeralSubmitActive}`}
            >
              {ephemeralLoading ? "Searching..." : "Look up"}
            </button>
          </form>

          {/* Ephemeral briefing result */}
          {ephemeralBriefing && (
            <EphemeralBriefing
              briefing={ephemeralBriefing}
              onAdd={ephemeralAdded ? undefined : handleAddFromBriefing}
              onNavigate={(entityId) => navigate({ to: "/accounts/$accountId", params: { accountId: entityId } })}
            />
          )}

          {ephemeralBriefing && (
            <hr className={styles.ephemeralDivider} />
          )}
        </div>
      )}

      {/* Create form */}
      {creating && !isArchived && (
        <div className={styles.createForm}>
          {bulkMode ? (
            <BulkCreateForm
              value={bulkValue}
              onChange={setBulkValue}
              onCreate={handleBulkCreate}
              onSingleMode={() => { setBulkMode(false); setBulkValue(""); }}
              onCancel={() => { setCreating(false); setBulkMode(false); setBulkValue(""); setNewName(""); setNewParentId(null); }}
              placeholder="One account name per line"
            />
          ) : (
            <div className={styles.createFormRow}>
              <AccountTypeSelector
                value={newAccountType}
                onChange={(v) => { setNewAccountType(v); setNewParentId(null); }}
              />
              {newAccountType !== "partner" && parentOptions.length > 0 && (
                <ParentSelector
                  value={newParentId}
                  onChange={setNewParentId}
                  options={parentOptions}
                />
              )}
              <div className={styles.createFormInlineRow}>
                <InlineCreateForm
                  value={newName}
                  onChange={setNewName}
                  onCreate={handleCreate}
                  onCancel={() => { setCreating(false); setNewName(""); setNewAccountType("customer"); setNewParentId(null); }}
                  placeholder={newParentId ? "Business unit name" : "Account name"}
                />
                <button
                  onClick={() => setBulkMode(true)}
                  className={styles.bulkButton}
                >
                  Bulk
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Account rows — grouped by account type (I383) or flat for archived */}
      {isArchived ? (
        <section>
          {filteredArchived.length === 0 ? (
            <EntityListEmpty
              title="No archived accounts"
              message="Archived accounts will appear here."
            />
          ) : (
            <div className={styles.archivedList}>
              {filteredArchived.map((account, i) => (
                <EntityRow
                  key={account.id}
                  to="/accounts/$accountId"
                  params={{ accountId: account.id }}
                  dotColor={healthDotColor[account.health ?? ""] ?? "var(--color-paper-linen)"}
                  name={account.name}
                  showBorder={i < filteredArchived.length - 1}
                  subtitle={account.lifecycle}
                >
                  {account.arr != null && (
                    <span className={styles.archivedArrLabel}>
                      ${formatArr(account.arr)}
                    </span>
                  )}
                </EntityRow>
              ))}
            </div>
          )}
        </section>
      ) : filtered.length === 0 ? (
        <section>
          <EntityListEmpty
            title="No matches"
            message="Try a different search or filter."
          />
        </section>
      ) : (
        /* Three-group layout: render each non-empty section with ChapterHeading */
        ACCOUNT_SECTIONS.map(({ type, title }) => {
          const sectionAccounts = groupedAccounts[type] ?? [];
          if (sectionAccounts.length === 0) return null;

          return (
            <section key={type} className={styles.accountSection}>
              <ChapterHeading title={title} />
              <div className={styles.accountSectionList}>
                {sectionAccounts.map((account, i) => (
                  <AccountTreeNode
                    key={account.id}
                    account={account}
                    depth={0}
                    expandedParents={expandedParents}
                    childrenCache={childrenCache}
                    toggleExpand={toggleExpand}
                    isLastSibling={i === sectionAccounts.length - 1}
                  />
                ))}
              </div>
            </section>
          );
        })
      )}

      {displayList.length > 0 && <EntityListEndMark />}
    </div>
  );
}

// --- Recursive Account Tree Node ---

function AccountTreeNode({
  account,
  depth,
  expandedParents,
  childrenCache,
  toggleExpand,
  isLastSibling,
}: {
  account: AccountListItem;
  depth: number;
  expandedParents: Set<string>;
  childrenCache: Record<string, AccountListItem[]>;
  toggleExpand: (id: string) => void;
  isLastSibling: boolean;
}) {
  const isExpanded = expandedParents.has(account.id);
  const children = childrenCache[account.id] ?? [];
  const hasExpandedChildren = account.isParent && isExpanded && children.length > 0;

  return (
    <div>
      <AccountRow
        account={account}
        isChild={depth > 0}
        depth={depth}
        isExpanded={isExpanded}
        onToggleExpand={account.isParent ? () => toggleExpand(account.id) : undefined}
        showBorder={!isLastSibling || hasExpandedChildren}
      />
      {hasExpandedChildren &&
        children.map((child, ci) => (
          <AccountTreeNode
            key={child.id}
            account={child}
            depth={depth + 1}
            expandedParents={expandedParents}
            childrenCache={childrenCache}
            toggleExpand={toggleExpand}
            isLastSibling={ci === children.length - 1 && isLastSibling}
          />
        ))}
    </div>
  );
}

// --- Account Row ---

function AccountRow({
  account,
  isExpanded,
  onToggleExpand,
  depth = 0,
  showBorder,
}: {
  account: AccountListItem;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  isChild?: boolean; // kept for call-site compat
  depth?: number;
  showBorder: boolean;
}) {
  const nameSuffix = (
    <>
      {account.accountType !== "customer" && (
        <span className={`${styles.accountTypeBadge} ${account.accountType === "partner" ? styles.accountTypeBadgePartner : styles.accountTypeBadgeInternal}`}>
          {account.accountType === "partner" ? "Partner" : "Internal"}
        </span>
      )}
      {onToggleExpand && (
        <button
          onClick={(e) => {
            e.preventDefault();
            e.stopPropagation();
            onToggleExpand();
          }}
          className={styles.expandToggle}
        >
          {isExpanded ? "\u25BE" : "\u25B8"} {account.childCount} BU{account.childCount !== 1 ? "s" : ""}
        </button>
      )}
    </>
  );

  const ih = account.intelligenceHealth;
  const healthAvatar = ih ? (
    <HealthBadge score={ih.score} band={ih.band} trend={ih.trend} size="compact" />
  ) : undefined;

  return (
    <EntityRow
      to="/accounts/$accountId"
      params={{ accountId: account.id }}
      dotColor={account.accountType === "internal" ? "var(--color-garden-larkspur)" : account.accountType === "partner" ? "var(--color-garden-rosemary)" : (healthDotColor[account.health ?? ""] ?? "var(--color-paper-linen)")}
      name={account.name}
      showBorder={showBorder}
      paddingLeft={depth > 0 ? depth * 28 : 0}
      nameSuffix={nameSuffix}
      subtitle={undefined}
      avatar={healthAvatar}
    >
      {account.arr != null && (
        <span className={styles.archivedArrLabel}>
          ${formatArr(account.arr)}
        </span>
      )}
    </EntityRow>
  );
}

// --- Account Type Selector ---

const TYPE_OPTIONS: { value: "customer" | "internal" | "partner"; label: string; color: string }[] = [
  { value: "customer", label: "Customer", color: "var(--color-spice-turmeric)" },
  { value: "internal", label: "Internal", color: "var(--color-garden-larkspur)" },
  { value: "partner", label: "Partner", color: "var(--color-garden-rosemary)" },
];

function AccountTypeSelector({
  value,
  onChange,
}: {
  value: "customer" | "internal" | "partner";
  onChange: (v: "customer" | "internal" | "partner") => void;
}) {
  return (
    <div className={styles.typeSelectorContainer}>
      {TYPE_OPTIONS.map((opt, idx) => {
        const isActive = value === opt.value;
        return (
          <button
            key={opt.value}
            onClick={() => onChange(opt.value)}
            className={`${styles.typeSelectorButton} ${isActive ? styles.typeSelectorButtonActive : styles.typeSelectorButtonInactive} ${idx < TYPE_OPTIONS.length - 1 ? styles.typeSelectorDivider : ""}`}
            style={{ color: isActive ? opt.color : undefined }}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

// --- Parent Account Selector ---

function ParentSelector({
  value,
  onChange,
  options,
}: {
  value: string | null;
  onChange: (v: string | null) => void;
  options: (AccountListItem & { _depth: number })[];
}) {
  return (
    <div className={styles.parentSelectorRow}>
      <label className={styles.parentSelectorLabel}>
        Parent
      </label>
      <Select value={value ?? "__none__"} onValueChange={(v) => onChange(v === "__none__" ? null : v)}>
        <SelectTrigger
          className={`${styles.parentSelectorTrigger} ${value ? styles.parentSelectorTriggerActive : styles.parentSelectorTriggerPlaceholder}`}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent
          position="popper"
          className={styles.parentSelectorContent}
        >
          <SelectItem
            value="__none__"
            className={styles.parentSelectorItemNone}
          >
            None (top-level)
          </SelectItem>
          {options.map((acct) => {
            const indent = acct._depth > 0 ? 12 + acct._depth * 16 : undefined;
            const prefix = acct._depth > 0 ? "\u2514 " : "";
            return (
              <SelectItem
                key={acct.id}
                value={acct.id}
                className={acct._depth > 0 ? styles.parentSelectorItemChild : styles.parentSelectorItem}
                style={indent ? { paddingLeft: indent } : undefined}
              >
                {prefix}{acct.name}
              </SelectItem>
            );
          })}
        </SelectContent>
      </Select>
    </div>
  );
}
