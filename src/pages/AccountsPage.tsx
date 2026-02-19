import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
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
import { formatArr } from "@/lib/utils";
import type { AccountListItem } from "@/types";
import type { ReadinessStat } from "@/components/layout/FolioBar";

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

export default function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [lifecycleFilter, setLifecycleFilter] = useState<string>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [expandedParents, setExpandedParents] = useState<Set<string>>(new Set());
  const [childrenCache, setChildrenCache] = useState<Record<string, AccountListItem[]>>({});
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedAccounts, setArchivedAccounts] = useState<ArchivedAccount[]>([]);
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkValue, setBulkValue] = useState("");

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

  async function handleCreate() {
    if (!newName.trim()) return;
    try {
      await invoke<string>("create_account", { name: newName.trim() });
      setNewName("");
      setCreating(false);
      await loadAccounts();
    } catch (e) {
      setError(String(e));
    }
  }

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

  const filtered = searchQuery
    ? lifecycleFiltered.filter(
        (a) =>
          a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (a.teamSummary ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : lifecycleFiltered;

  const filteredArchived = searchQuery
    ? archivedAccounts.filter(
        (a) => a.name.toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedAccounts;

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

  // Folio actions: archive toggle + new button
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
  }, [isArchived]);

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
        <EntityListEmpty title="No accounts yet" message="Create your first account to get started.">
          {creating ? (
            <div style={{ maxWidth: 400, margin: "24px auto 0" }}>
              <InlineCreateForm
                value={newName}
                onChange={setNewName}
                onCreate={handleCreate}
                onCancel={() => setCreating(false)}
                placeholder="Account name"
              />
            </div>
          ) : (
            <button
              onClick={() => setCreating(true)}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 600,
                color: "var(--color-spice-turmeric)",
                background: "none",
                border: "1px solid var(--color-spice-turmeric)",
                borderRadius: 4,
                padding: "6px 16px",
                cursor: "pointer",
                marginTop: 24,
              }}
            >
              + New Account
            </button>
          )}
        </EntityListEmpty>
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

      {/* Create form */}
      {creating && !isArchived && (
        <div style={{ marginBottom: 16 }}>
          {bulkMode ? (
            <BulkCreateForm
              value={bulkValue}
              onChange={setBulkValue}
              onCreate={handleBulkCreate}
              onSingleMode={() => { setBulkMode(false); setBulkValue(""); }}
              onCancel={() => { setCreating(false); setBulkMode(false); setBulkValue(""); setNewName(""); }}
              placeholder="One account name per line"
            />
          ) : (
            <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
              <InlineCreateForm
                value={newName}
                onChange={setNewName}
                onCreate={handleCreate}
                onCancel={() => { setCreating(false); setNewName(""); }}
                placeholder="Account name"
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
          )}
        </div>
      )}

      {/* Account rows */}
      <section>
        {displayList.length === 0 ? (
          <EntityListEmpty
            title={isArchived ? "No archived accounts" : "No matches"}
            message={isArchived ? "Archived accounts will appear here." : "Try a different search or filter."}
          />
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {isArchived
              ? filteredArchived.map((account, i) => (
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
                ))
              : filtered.map((account, i) => (
                  <AccountTreeNode
                    key={account.id}
                    account={account}
                    depth={0}
                    expandedParents={expandedParents}
                    childrenCache={childrenCache}
                    toggleExpand={toggleExpand}
                    isLastSibling={i === filtered.length - 1}
                  />
                ))}
          </div>
        )}
      </section>

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
  const daysSince = account.daysSinceLastMeeting;
  const isStale = daysSince != null && daysSince > 14;

  const subtitle = account.teamSummary ? (
    <>
      {account.teamSummary}
      {account.openActionCount > 0 && (
        <span> &middot; {account.openActionCount} action{account.openActionCount !== 1 ? "s" : ""}</span>
      )}
    </>
  ) : undefined;

  const nameSuffix = (
    <>
      {account.isInternal && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 10,
            fontWeight: 600,
            letterSpacing: "0.06em",
            textTransform: "uppercase",
            color: "var(--color-text-tertiary)",
          }}
        >
          Internal
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

  return (
    <EntityRow
      to="/accounts/$accountId"
      params={{ accountId: account.id }}
      dotColor={account.isInternal ? "var(--color-garden-larkspur)" : (healthDotColor[account.health ?? ""] ?? "var(--color-paper-linen)")}
      name={account.name}
      showBorder={showBorder}
      paddingLeft={depth > 0 ? depth * 28 : 0}
      nameSuffix={nameSuffix}
      subtitle={subtitle}
    >
      {account.arr != null && (
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-secondary)" }}>
          ${formatArr(account.arr)}
        </span>
      )}
      {daysSince != null && (
        <span
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 13,
            color: isStale ? "var(--color-spice-terracotta)" : "var(--color-text-tertiary)",
          }}
        >
          {daysSince === 0 ? "Today" : `${daysSince}d`}
        </span>
      )}
    </EntityRow>
  );
}
