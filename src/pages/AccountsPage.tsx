import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import { ListRow, ListColumn } from "@/components/ui/list-row";
import { PageError } from "@/components/PageState";
import { Building2, ChevronDown, ChevronRight, Plus, RefreshCw } from "lucide-react";
import { cn, formatArr } from "@/lib/utils";
import type { AccountListItem } from "@/types";

/** Lightweight shape returned by get_archived_accounts (DbAccount from Rust). */
interface ArchivedAccount {
  id: string;
  name: string;
  lifecycle?: string;
  arr?: number;
  health?: string;
  csm?: string;
  archived: boolean;
}

type ArchiveTab = "active" | "archived";
type ScopeTab = "all" | "internal" | "external";

type HealthTab = "all" | "green" | "yellow" | "red";

const healthTabs: { key: HealthTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "green", label: "Green" },
  { key: "yellow", label: "Yellow" },
  { key: "red", label: "Red" },
];

const archiveTabs: { key: ArchiveTab; label: string }[] = [
  { key: "active", label: "Active" },
  { key: "archived", label: "Archived" },
];

const scopeTabs: { key: ScopeTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "internal", label: "Internal" },
  { key: "external", label: "External" },
];

export default function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<HealthTab>("all");
  const [scopeTab, setScopeTab] = useState<ScopeTab>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  // I114: expanded parent tracking + cached children
  const [expandedParents, setExpandedParents] = useState<Set<string>>(new Set());
  const [childrenCache, setChildrenCache] = useState<Record<string, AccountListItem[]>>({});
  // I176: archive tab
  const [archiveTab, setArchiveTab] = useState<ArchiveTab>("active");
  const [archivedAccounts, setArchivedAccounts] = useState<ArchivedAccount[]>([]);
  // I162: bulk create mode
  const [bulkMode, setBulkMode] = useState(false);
  const [bulkValue, setBulkValue] = useState("");

  const loadAccounts = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const result = await invoke<AccountListItem[]>("get_accounts_list");
      setAccounts(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // I176: load archived accounts
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

  // I162: bulk create
  async function handleBulkCreate() {
    const names = bulkValue
      .split("\n")
      .map((s) => s.trim())
      .filter((s) => s.length > 0);
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
      // Fetch children if not cached
      if (!childrenCache[parentId]) {
        try {
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

  const healthFiltered =
    tab === "all"
      ? accounts
      : accounts.filter((a) => {
          // Show parent if it matches OR if any cached child matches
          if (a.health === tab) return true;
          if (a.isParent) {
            const children = childrenCache[a.id];
            if (children?.some((c) => c.health === tab)) return true;
          }
          return false;
        });

  const scopeFiltered = healthFiltered.filter((a) => {
    if (scopeTab === "internal") return a.isInternal;
    if (scopeTab === "external") return !a.isInternal;
    return true;
  });

  const filtered = searchQuery
    ? scopeFiltered.filter(
        (a) =>
          a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (a.csm ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : scopeFiltered;

  const tabCounts: Record<HealthTab, number> = {
    all: accounts.length,
    green: accounts.filter((a) => a.health === "green").length,
    yellow: accounts.filter((a) => a.health === "yellow").length,
    red: accounts.filter((a) => a.health === "red").length,
  };
  const scopeCounts: Record<ScopeTab, number> = {
    all: accounts.length,
    internal: accounts.filter((a) => a.isInternal).length,
    external: accounts.filter((a) => !a.isInternal).length,
  };

  // I176: filter archived accounts by search query
  const filteredArchived = searchQuery
    ? archivedAccounts.filter(
        (a) =>
          a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (a.csm ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : archivedAccounts;

  const isArchived = archiveTab === "archived";

  if (loading && (isArchived ? archivedAccounts.length === 0 : accounts.length === 0)) {
    return (
      <main className="flex-1 overflow-hidden p-6">
        <div className="mb-6 space-y-2">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-4 w-64" />
        </div>
        <div className="space-y-4">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-20 w-full" />
          ))}
        </div>
      </main>
    );
  }

  if (error) {
    return (
      <main className="flex-1 overflow-hidden">
        <PageError message={error} onRetry={loadAccounts} />
      </main>
    );
  }

  if (!isArchived && accounts.length === 0) {
    return (
      <main className="flex-1 overflow-hidden">
        <div className="flex h-full flex-col items-center justify-center gap-4 p-6">
          <Building2 className="size-16 text-muted-foreground/40" />
          <div className="text-center">
            <h2 className="text-lg font-semibold">No accounts yet</h2>
            <p className="text-sm text-muted-foreground">
              Create your first account to get started.
            </p>
          </div>
          {creating ? (
            <InlineCreateForm
              value={newName}
              onChange={setNewName}
              onCreate={handleCreate}
              onCancel={() => setCreating(false)}
              placeholder="Account name"
            />
          ) : (
            <Button onClick={() => setCreating(true)}>
              <Plus className="mr-1 size-4" />
              New Account
            </Button>
          )}
        </div>
      </main>
    );
  }

  return (
    <main className="flex-1 overflow-hidden">
      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="mb-6 flex items-start justify-between">
            <div>
              <h1 className="text-2xl font-semibold tracking-tight">
                Accounts
                <span className="ml-2 text-base font-normal text-muted-foreground">
                  {isArchived ? filteredArchived.length : filtered.length}
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                {isArchived
                  ? "Previously tracked accounts"
                  : "Account health, engagement signals, and renewal tracking"}
              </p>
            </div>
            <div className="flex items-center gap-2">
              {!isArchived && (
                <>
                  {creating ? (
                    <>
                      {bulkMode ? (
                        <div className="flex flex-col gap-2">
                          <textarea
                            autoFocus
                            value={bulkValue}
                            onChange={(e) => setBulkValue(e.target.value)}
                            onKeyDown={(e) => {
                              if (e.key === "Escape") {
                                setBulkMode(false);
                                setBulkValue("");
                                setCreating(false);
                              }
                            }}
                            placeholder="One account name per line"
                            rows={5}
                            className="w-64 rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                          />
                          <div className="flex items-center gap-2">
                            <Button size="sm" onClick={handleBulkCreate}>
                              Create{" "}
                              {bulkValue.split("\n").filter((s) => s.trim()).length || ""}
                            </Button>
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => {
                                setBulkMode(false);
                                setBulkValue("");
                              }}
                            >
                              Single
                            </Button>
                            <Button
                              size="sm"
                              variant="ghost"
                              onClick={() => {
                                setCreating(false);
                                setBulkMode(false);
                                setBulkValue("");
                                setNewName("");
                              }}
                            >
                              Cancel
                            </Button>
                          </div>
                        </div>
                      ) : (
                        <div className="flex items-center gap-2">
                          <InlineCreateForm
                            value={newName}
                            onChange={setNewName}
                            onCreate={handleCreate}
                            onCancel={() => {
                              setCreating(false);
                              setNewName("");
                            }}
                            placeholder="Account name"
                          />
                          <Button
                            size="sm"
                            variant="ghost"
                            onClick={() => setBulkMode(true)}
                          >
                            Bulk
                          </Button>
                        </div>
                      )}
                    </>
                  ) : (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setCreating(true)}
                    >
                      <Plus className="mr-1 size-4" />
                      New Account
                    </Button>
                  )}
                </>
              )}
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={isArchived ? loadArchivedAccounts : loadAccounts}
              >
                <RefreshCw className="size-4" />
              </Button>
            </div>
          </div>

          <TabFilter
            tabs={archiveTabs}
            active={archiveTab}
            onChange={setArchiveTab}
            className="mb-4"
          />

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search accounts..."
            className="mb-4"
          />

          {!isArchived && (
            <TabFilter
              tabs={scopeTabs}
              active={scopeTab}
              onChange={setScopeTab}
              counts={scopeCounts}
              className="mb-4"
            />
          )}

          {!isArchived && (
            <TabFilter
              tabs={healthTabs}
              active={tab}
              onChange={setTab}
              counts={tabCounts}
              className="mb-6"
            />
          )}

          {/* Accounts list */}
          <div>
            {isArchived ? (
              filteredArchived.length === 0 ? (
                <Card>
                  <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                    <Building2 className="mb-4 size-12 text-muted-foreground/40" />
                    <p className="text-lg font-medium">No archived accounts</p>
                    <p className="text-sm text-muted-foreground">
                      Archived accounts will appear here.
                    </p>
                  </CardContent>
                </Card>
              ) : (
                filteredArchived.map((account) => (
                  <ArchivedAccountRow key={account.id} account={account} />
                ))
              )
            ) : filtered.length === 0 ? (
              <Card>
                <CardContent className="flex flex-col items-center justify-center py-12 text-center">
                  <Building2 className="mb-4 size-12 text-muted-foreground/40" />
                  <p className="text-lg font-medium">No matches</p>
                  <p className="text-sm text-muted-foreground">
                    Try a different search or filter.
                  </p>
                </CardContent>
              </Card>
            ) : (
              filtered.map((account) => (
                <div key={account.id}>
                  <AccountRow
                    account={account}
                    isExpanded={expandedParents.has(account.id)}
                    onToggleExpand={
                      account.isParent
                        ? () => toggleExpand(account.id)
                        : undefined
                    }
                  />
                  {/* Render children when expanded */}
                  {account.isParent &&
                    expandedParents.has(account.id) &&
                    childrenCache[account.id]?.map((child) => (
                      <AccountRow
                        key={child.id}
                        account={child}
                        isChild
                      />
                    ))}
                </div>
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function AccountRow({
  account,
  isExpanded,
  onToggleExpand,
  isChild,
}: {
  account: AccountListItem;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  isChild?: boolean;
}) {
  const healthDot: Record<string, string> = {
    green: "bg-success",
    yellow: "bg-primary",
    red: "bg-destructive",
  };

  const daysSince = account.daysSinceLastMeeting;
  const isStale = daysSince != null && daysSince > 14;

  const Chevron = isExpanded ? ChevronDown : ChevronRight;
  const hasInternalBadge = account.isInternal;

  return (
    <ListRow
      to="/accounts/$accountId"
      params={{ accountId: account.id }}
      signalColor={healthDot[account.health ?? ""] ?? "bg-muted-foreground/30"}
      name={account.name}
      subtitle={account.csm ? `CSM: ${account.csm}` : undefined}
      className={isChild ? "pl-8" : undefined}
      badges={
        <div className="inline-flex items-center gap-2">
          {hasInternalBadge && (
            <span className="rounded-full border border-primary/30 bg-primary/10 px-2 py-0.5 text-[10px] font-medium text-primary">
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
              className="inline-flex items-center gap-0.5 rounded px-1 py-0.5 text-xs text-muted-foreground hover:bg-muted"
            >
              <Chevron className="size-3.5" />
              <span>{account.childCount} BU{account.childCount !== 1 ? "s" : ""}</span>
            </button>
          )}
        </div>
      }
      columns={
        <>
          {account.arr != null && (
            <ListColumn
              value={<span className="font-mono">${formatArr(account.arr)}</span>}
              className="w-20"
            />
          )}
          {account.openActionCount > 0 && (
            <ListColumn
              value={account.openActionCount}
              label="actions"
              className="w-14"
            />
          )}
          {daysSince != null && (
            <ListColumn
              value={
                <span className={cn(isStale && "text-destructive")}>
                  {daysSince === 0 ? "Today" : `${daysSince}d`}
                </span>
              }
              label="last mtg"
              className="w-14"
            />
          )}
        </>
      }
    />
  );
}

/** I176: Simplified row for archived accounts (no active metrics). */
function ArchivedAccountRow({ account }: { account: ArchivedAccount }) {
  const healthDot: Record<string, string> = {
    green: "bg-success",
    yellow: "bg-primary",
    red: "bg-destructive",
  };

  return (
    <ListRow
      to="/accounts/$accountId"
      params={{ accountId: account.id }}
      signalColor={healthDot[account.health ?? ""] ?? "bg-muted-foreground/30"}
      name={account.name}
      subtitle={
        [account.csm ? `CSM: ${account.csm}` : "", account.lifecycle]
          .filter(Boolean)
          .join(" \u00B7 ") || undefined
      }
      columns={
        account.arr != null ? (
          <ListColumn
            value={<span className="font-mono">${formatArr(account.arr)}</span>}
            className="w-20"
          />
        ) : undefined
      }
    />
  );
}
