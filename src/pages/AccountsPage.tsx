import { useState, useEffect, useCallback } from "react";
import { Link } from "@tanstack/react-router";
import { invoke } from "@tauri-apps/api/core";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { ScrollArea } from "@/components/ui/scroll-area";
import { PageError } from "@/components/PageState";
import { cn } from "@/lib/utils";
import { Building2, Plus, RefreshCw, Search } from "lucide-react";
import type { AccountListItem, AccountHealth } from "@/types";

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
            <div className="flex items-center gap-2">
              <input
                type="text"
                autoFocus
                value={newName}
                onChange={(e) => setNewName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleCreate();
                  if (e.key === "Escape") setCreating(false);
                }}
                placeholder="Account name"
                className="rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
              />
              <Button size="sm" onClick={handleCreate}>
                Create
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setCreating(false)}
              >
                Cancel
              </Button>
            </div>
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
                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    autoFocus
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") handleCreate();
                      if (e.key === "Escape") {
                        setCreating(false);
                        setNewName("");
                      }
                    }}
                    placeholder="Account name"
                    className="rounded-md border bg-background px-3 py-2 text-sm outline-none focus:ring-1 focus:ring-ring"
                  />
                  <Button size="sm" onClick={handleCreate}>
                    Create
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => {
                      setCreating(false);
                      setNewName("");
                    }}
                  >
                    Cancel
                  </Button>
                </div>
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

          {/* Search */}
          <div className="relative mb-4">
            <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search accounts..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full rounded-md border bg-background py-2 pl-10 pr-4 text-sm outline-none focus:ring-1 focus:ring-ring"
            />
          </div>

          {/* Health tabs */}
          <div className="mb-6 flex gap-2">
            {tabs.map((t) => (
              <button
                key={t.key}
                onClick={() => setTab(t.key)}
                className={cn(
                  "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                  tab === t.key
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted hover:bg-muted/80"
                )}
              >
                {t.label}
                {tabCounts[t.key] > 0 && (
                  <span
                    className={cn(
                      "ml-1.5 inline-flex size-5 items-center justify-center rounded-full text-xs",
                      tab === t.key
                        ? "bg-primary-foreground/20 text-primary-foreground"
                        : "bg-muted-foreground/15 text-muted-foreground"
                    )}
                  >
                    {tabCounts[t.key]}
                  </span>
                )}
              </button>
            ))}
          </div>

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
              <HealthBadge health={account.health} />
              {account.ring && (
                <Badge variant="outline" className="text-xs">
                  Ring {account.ring}
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

function HealthBadge({ health }: { health?: AccountHealth }) {
  if (!health) return null;
  const styles: Record<AccountHealth, string> = {
    green: "bg-green-100 text-green-800 border-green-300 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700",
    yellow: "bg-yellow-100 text-yellow-800 border-yellow-300 dark:bg-yellow-900/30 dark:text-yellow-400 dark:border-yellow-700",
    red: "bg-red-100 text-red-800 border-red-300 dark:bg-red-900/30 dark:text-red-400 dark:border-red-700",
  };
  return (
    <Badge variant="outline" className={cn("text-xs", styles[health])}>
      {health}
    </Badge>
  );
}

function formatArr(arr: number): string {
  if (arr >= 1_000_000) return `${(arr / 1_000_000).toFixed(1)}M`;
  if (arr >= 1_000) return `${(arr / 1_000).toFixed(0)}K`;
  return arr.toFixed(0);
}
