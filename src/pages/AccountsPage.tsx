import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { SearchInput } from "@/components/ui/search-input";
import { TabFilter } from "@/components/ui/tab-filter";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import { StatusBadge, healthStyles } from "@/components/ui/status-badge";
import { PageError } from "@/components/PageState";
import { Badge } from "@/components/ui/badge";
import { Building2, Plus, RefreshCw } from "lucide-react";
import type { AccountListItem } from "@/types";

type HealthTab = "all" | "green" | "yellow" | "red";

const tabs: { key: HealthTab; label: string }[] = [
  { key: "all", label: "All" },
  { key: "green", label: "Green" },
  { key: "yellow", label: "Yellow" },
  { key: "red", label: "Red" },
];

export default function AccountsPage() {
  const [accounts, setAccounts] = useState<AccountListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<HealthTab>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");

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

  useEffect(() => {
    loadAccounts();
  }, [loadAccounts]);

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

  const healthFiltered =
    tab === "all" ? accounts : accounts.filter((a) => a.health === tab);

  const filtered = searchQuery
    ? healthFiltered.filter(
        (a) =>
          a.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
          (a.csm ?? "").toLowerCase().includes(searchQuery.toLowerCase())
      )
    : healthFiltered;

  const tabCounts: Record<HealthTab, number> = {
    all: accounts.length,
    green: accounts.filter((a) => a.health === "green").length,
    yellow: accounts.filter((a) => a.health === "yellow").length,
    red: accounts.filter((a) => a.health === "red").length,
  };

  if (loading && accounts.length === 0) {
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

  if (accounts.length === 0) {
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
                  {filtered.length}
                </span>
              </h1>
              <p className="text-sm text-muted-foreground">
                Account health, engagement signals, and renewal tracking
              </p>
            </div>
            <div className="flex items-center gap-2">
              {creating ? (
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
              <Button
                variant="ghost"
                size="icon"
                className="size-8"
                onClick={loadAccounts}
              >
                <RefreshCw className="size-4" />
              </Button>
            </div>
          </div>

          <SearchInput
            value={searchQuery}
            onChange={setSearchQuery}
            placeholder="Search accounts..."
            className="mb-4"
          />

          <TabFilter
            tabs={tabs}
            active={tab}
            onChange={setTab}
            counts={tabCounts}
            className="mb-6"
          />

          {/* Accounts list */}
          <div className="space-y-2">
            {filtered.length === 0 ? (
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
                <AccountRow key={account.id} account={account} />
              ))
            )}
          </div>
        </div>
      </ScrollArea>
    </main>
  );
}

function AccountRow({ account }: { account: AccountListItem }) {
  return (
    <Link to="/accounts/$accountId" params={{ accountId: account.id }}>
      <Card className="cursor-pointer transition-all hover:-translate-y-0.5 hover:shadow-md">
        <CardContent className="flex items-center gap-4 p-4">
          {/* Avatar initial */}
          <div className="flex size-10 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold text-primary">
            {account.name.charAt(0).toUpperCase()}
          </div>

          {/* Name + badges */}
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <span className="truncate font-medium">{account.name}</span>
              {account.health && (
                <StatusBadge value={account.health} styles={healthStyles} />
              )}
              {account.lifecycle && (
                <Badge variant="outline" className="text-xs capitalize">
                  {account.lifecycle}
                </Badge>
              )}
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              {account.csm && <span>CSM: {account.csm}</span>}
              {account.csm && account.arr != null && (
                <span className="text-muted-foreground/40">&middot;</span>
              )}
              {account.arr != null && (
                <span>${formatArr(account.arr)}</span>
              )}
            </div>
          </div>

          {/* Open actions count */}
          {account.openActionCount > 0 && (
            <div className="shrink-0 text-right">
              <div className="text-sm font-medium">
                {account.openActionCount}
              </div>
              <div className="text-xs text-muted-foreground">actions</div>
            </div>
          )}

          {/* Days since last meeting */}
          {account.daysSinceLastMeeting != null && (
            <div className="w-16 shrink-0 text-right">
              <div className="text-xs text-muted-foreground">
                {account.daysSinceLastMeeting === 0
                  ? "Today"
                  : `${account.daysSinceLastMeeting}d ago`}
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </Link>
  );
}


function formatArr(arr: number): string {
  if (arr >= 1_000_000) return `${(arr / 1_000_000).toFixed(1)}M`;
  if (arr >= 1_000) return `${(arr / 1_000).toFixed(0)}K`;
  return arr.toFixed(0);
}
