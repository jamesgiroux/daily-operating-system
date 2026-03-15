import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "@tanstack/react-router";
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
import type { AccountListItem, DiscoveredAccount, EphemeralBriefing as EphemeralBriefingType } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";
import { HealthBadge } from "@/components/shared/HealthBadge";

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

  // I494: Check Glean connection status on mount
  useEffect(() => {
    invoke<{ status: string }>("get_glean_auth_status")
      .then((result) => setGleanConnected(result.status === "authenticated"))
      .catch(() => setGleanConnected(false));
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
    const archiveButtonStyle = {
      fontFamily: "var(--font-mono)",
      fontSize: 11,
      fontWeight: 500,
      letterSpacing: "0.06em",
      background: "none",
      border: "none",
      padding: 0,
      cursor: "pointer",
    } as const;

    return (
      <>
        <button
          onClick={() => setArchiveTab(isArchived ? "active" : "archived")}
          style={{
            ...archiveButtonStyle,
            color: isArchived ? "var(--color-spice-turmeric)" : "var(--color-text-tertiary)",
          }}
        >
          {isArchived ? "\u2190 Active" : "Archive"}
        </button>
        {!isArchived && gleanConnected && (
          <button
            onClick={handleDiscoverAccounts}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 500,
              letterSpacing: "0.06em",
              color: "var(--color-text-tertiary)",
              background: "none",
              border: "none",
              padding: 0,
              cursor: "pointer",
            }}
          >
            Discover
          </button>
        )}
        {!isArchived && (
          <button
            onClick={() => setCreating(true)}
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: "0.06em",
              textTransform: "uppercase" as const,
              color: "var(--color-spice-turmeric)",
              background: "none",
              border: "1px solid var(--color-spice-turmeric)",
              borderRadius: 4,
              padding: "2px 10px",
              cursor: "pointer",
            }}
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
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        <h1
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 36,
            fontWeight: 400,
            letterSpacing: "-0.02em",
            color: "var(--color-text-primary)",
            margin: "0 0 24px 0",
          }}
        >
          Your Book
        </h1>
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
                <div style={{ maxWidth: 400, margin: "0 auto", display: "flex", flexDirection: "column", gap: 12, textAlign: "left" }}>
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
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
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
        <div style={{ marginBottom: 24 }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: 12 }}>
            <h3
              style={{
                fontFamily: "var(--font-serif)",
                fontSize: 20,
                fontWeight: 400,
                color: "var(--color-text-primary)",
                margin: 0,
              }}
            >
              Discovered Accounts
            </h3>
            <button
              onClick={() => { setDiscoveryOpen(false); setDiscoveredAccounts([]); setDiscoveryFilter(""); }}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                color: "var(--color-text-tertiary)",
                background: "none",
                border: "none",
                cursor: "pointer",
                padding: 0,
              }}
            >
              Close
            </button>
          </div>

          {discoveryLoading ? (
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-text-tertiary)",
                fontStyle: "italic",
              }}
            >
              Searching your organization...
            </p>
          ) : discoveredAccounts.length === 0 ? (
            <p
              style={{
                fontFamily: "var(--font-sans)",
                fontSize: 14,
                color: "var(--color-text-tertiary)",
              }}
            >
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
                style={{
                  width: "100%",
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  color: "var(--color-text-primary)",
                  background: "var(--color-paper-warm-white)",
                  border: "1px solid var(--color-paper-linen)",
                  borderRadius: 4,
                  padding: "6px 10px",
                  marginBottom: 12,
                  boxSizing: "border-box",
                  outline: "none",
                }}
              />
              <div style={{ display: "flex", flexDirection: "column" }}>
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
                        style={{
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                          padding: "10px 0",
                          borderBottom: i < arr.length - 1 ? "1px solid var(--color-rule-light)" : "none",
                        }}
                      >
                        <div style={{ flex: 1, minWidth: 0 }}>
                          <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
                            <span
                              style={{
                                fontFamily: "var(--font-sans)",
                                fontSize: 14,
                                fontWeight: 500,
                                color: "var(--color-text-primary)",
                              }}
                            >
                              {account.name}
                            </span>
                            {account.domain && (
                              <span
                                style={{
                                  fontFamily: "var(--font-mono)",
                                  fontSize: 11,
                                  color: "var(--color-text-tertiary)",
                                }}
                              >
                                {account.domain}
                              </span>
                            )}
                            {account.industry && (
                              <span
                                style={{
                                  fontFamily: "var(--font-mono)",
                                  fontSize: 10,
                                  letterSpacing: "0.06em",
                                  textTransform: "uppercase",
                                  color: "var(--color-text-tertiary)",
                                }}
                              >
                                {account.industry}
                              </span>
                            )}
                          </div>
                          {account.contextPreview && (
                            <p
                              style={{
                                fontFamily: "var(--font-sans)",
                                fontSize: 13,
                                lineHeight: 1.5,
                                color: "var(--color-text-secondary)",
                                margin: "4px 0 0 0",
                              }}
                            >
                              {account.contextPreview}
                            </p>
                          )}
                        </div>
                        <div style={{ marginLeft: 16, flexShrink: 0 }}>
                          {isAdded ? (
                            <span
                              style={{
                                fontFamily: "var(--font-mono)",
                                fontSize: 11,
                                fontWeight: 500,
                                color: "var(--color-garden-sage)",
                              }}
                            >
                              Added
                            </span>
                          ) : (
                            <button
                              onClick={() => handleAddDiscovered(account)}
                              style={{
                                fontFamily: "var(--font-mono)",
                                fontSize: 11,
                                fontWeight: 600,
                                letterSpacing: "0.06em",
                                textTransform: "uppercase",
                                color: "var(--color-spice-turmeric)",
                                background: "none",
                                border: "1px solid var(--color-spice-turmeric)",
                                borderRadius: 4,
                                padding: "3px 10px",
                                cursor: "pointer",
                              }}
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
                <p
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                    margin: "10px 0 0 0",
                  }}
                >
                  Showing first 50. Use search to narrow the list.
                </p>
              )}
            </>
          )}

          <hr style={{ border: "none", borderTop: "1px solid var(--color-rule-heavy)", margin: "20px 0" }} />
        </div>
      )}

      {/* I495: Ephemeral account query */}
      {gleanConnected && !isArchived && (
        <div style={{ marginBottom: 20 }}>
          <form onSubmit={handleEphemeralQuery} style={{ display: "flex", gap: 8, alignItems: "center" }}>
            <input
              type="text"
              value={ephemeralQuery}
              onChange={(e) => setEphemeralQuery(e.target.value)}
              placeholder="Tell me about..."
              style={{
                flex: 1,
                fontFamily: "var(--font-sans)",
                fontSize: 13,
                color: "var(--color-text-primary)",
                background: "var(--color-paper-warm-white)",
                border: "1px solid var(--color-paper-linen)",
                borderRadius: 4,
                padding: "6px 10px",
                outline: "none",
              }}
            />
            <button
              type="submit"
              disabled={ephemeralLoading || !ephemeralQuery.trim()}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: ephemeralLoading || !ephemeralQuery.trim() ? "var(--color-text-tertiary)" : "var(--color-spice-turmeric)",
                background: "none",
                border: "1px solid",
                borderColor: ephemeralLoading || !ephemeralQuery.trim() ? "var(--color-paper-linen)" : "var(--color-spice-turmeric)",
                borderRadius: 4,
                padding: "5px 12px",
                cursor: ephemeralLoading || !ephemeralQuery.trim() ? "default" : "pointer",
                whiteSpace: "nowrap",
              }}
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
            <hr style={{ border: "none", borderTop: "1px solid var(--color-rule-heavy)", margin: "0 0 16px 0" }} />
          )}
        </div>
      )}

      {/* Create form */}
      {creating && !isArchived && (
        <div style={{ marginBottom: 16 }}>
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
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
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
              <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <InlineCreateForm
                  value={newName}
                  onChange={setNewName}
                  onCreate={handleCreate}
                  onCancel={() => { setCreating(false); setNewName(""); setNewAccountType("customer"); setNewParentId(null); }}
                  placeholder={newParentId ? "Business unit name" : "Account name"}
                />
                <button
                  onClick={() => setBulkMode(true)}
                  style={{
                    fontFamily: "var(--font-mono)",
                    fontSize: 11,
                    color: "var(--color-text-tertiary)",
                    background: "none",
                    border: "none",
                    cursor: "pointer",
                  }}
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
            <div style={{ display: "flex", flexDirection: "column" }}>
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
                    <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-secondary)" }}>
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
            <section key={type} style={{ marginBottom: "var(--space-2xl)" }}>
              <ChapterHeading title={title} />
              <div style={{ display: "flex", flexDirection: "column" }}>
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

// ─── Recursive Account Tree Node ─────────────────────────────────────────────

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

// ─── Account Row ────────────────────────────────────────────────────────────

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
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: account.accountType === "partner" ? "var(--color-garden-rosemary)" : "var(--color-text-tertiary)",
          }}
        >
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
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 11,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "none",
            cursor: "pointer",
            padding: 0,
          }}
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
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-secondary)" }}>
          ${formatArr(account.arr)}
        </span>
      )}
    </EntityRow>
  );
}

// ─── Account Type Selector ──────────────────────────────────────────────────

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
    <div style={{ display: "flex", gap: 0, borderRadius: 4, overflow: "hidden", border: "1px solid var(--color-paper-linen)" }}>
      {TYPE_OPTIONS.map((opt) => {
        const isActive = value === opt.value;
        return (
          <button
            key={opt.value}
            onClick={() => onChange(opt.value)}
            style={{
              flex: 1,
              padding: "6px 12px",
              fontFamily: "var(--font-mono)",
              fontSize: 10,
              fontWeight: isActive ? 600 : 400,
              letterSpacing: "0.08em",
              textTransform: "uppercase",
              color: isActive ? opt.color : "var(--color-text-tertiary)",
              background: isActive ? "var(--color-desk-charcoal-4)" : "transparent",
              border: "none",
              borderRight: opt.value !== "partner" ? "1px solid var(--color-paper-linen)" : "none",
              cursor: "pointer",
              transition: "all 0.15s ease",
            }}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

// ─── Parent Account Selector ────────────────────────────────────────────────

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
    <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
      <label
        style={{
          fontFamily: "var(--font-mono)",
          fontSize: 10,
          fontWeight: 500,
          letterSpacing: "0.08em",
          textTransform: "uppercase",
          color: "var(--color-text-tertiary)",
          whiteSpace: "nowrap",
        }}
      >
        Parent
      </label>
      <Select value={value ?? "__none__"} onValueChange={(v) => onChange(v === "__none__" ? null : v)}>
        <SelectTrigger
          className=""
          style={{
            flex: 1,
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            color: value ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
            background: "var(--color-paper-warm-white)",
            border: "1px solid var(--color-paper-linen)",
            borderRadius: 4,
            padding: "5px 8px",
            height: "auto",
            boxShadow: "none",
          }}
        >
          <SelectValue />
        </SelectTrigger>
        <SelectContent
          position="popper"
          style={{
            background: "var(--color-paper-warm-white)",
            border: "1px solid var(--color-paper-linen)",
            borderRadius: 6,
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            maxHeight: 240,
          }}
        >
          <SelectItem
            value="__none__"
            style={{ color: "var(--color-text-tertiary)", fontFamily: "var(--font-sans)", fontSize: 13 }}
          >
            None (top-level)
          </SelectItem>
          {options.map((acct) => {
            const indent = acct._depth > 0 ? 12 + acct._depth * 16 : undefined;
            const prefix = acct._depth > 0 ? "└ " : "";
            return (
              <SelectItem
                key={acct.id}
                value={acct.id}
                style={{
                  fontFamily: "var(--font-sans)",
                  fontSize: 13,
                  paddingLeft: indent,
                  color: acct._depth > 0 ? "var(--color-text-secondary)" : undefined,
                }}
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
