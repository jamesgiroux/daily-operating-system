import { useState, useEffect, useCallback, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Link } from "@tanstack/react-router";
import { useRegisterMagazineShell } from "@/hooks/useMagazineShell";
import { InlineCreateForm } from "@/components/ui/inline-create-form";
import {
  BulkCreateForm,
  parseBulkCreateInput,
} from "@/components/ui/bulk-create-form";
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
          {isArchived ? "← Active" : "Archive"}
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
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
        {[1, 2, 3, 4].map((i) => (
          <div
            key={i}
            style={{
              height: 52,
              background: "var(--color-rule-light)",
              borderRadius: 8,
              marginBottom: 12,
            }}
          />
        ))}
      </div>
    );
  }

  // Error state
  if (error) {
    return (
      <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80, textAlign: "center" }}>
        <p style={{ fontFamily: "var(--font-sans)", fontSize: 15, color: "var(--color-spice-terracotta)" }}>{error}</p>
        <button
          onClick={loadAccounts}
          style={{
            fontFamily: "var(--font-mono)",
            fontSize: 12,
            color: "var(--color-text-tertiary)",
            background: "none",
            border: "1px solid var(--color-rule-heavy)",
            borderRadius: 4,
            padding: "4px 12px",
            cursor: "pointer",
            marginTop: 12,
          }}
        >
          Retry
        </button>
      </div>
    );
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
        <div style={{ textAlign: "center", padding: "64px 0" }}>
          <p style={{ fontFamily: "var(--font-serif)", fontSize: 18, fontStyle: "italic", color: "var(--color-text-tertiary)" }}>
            No accounts yet
          </p>
          <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 300, color: "var(--color-text-tertiary)", marginTop: 8 }}>
            Create your first account to get started.
          </p>
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
        </div>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto" }}>
      {/* ═══ PAGE HEADER ═══ */}
      <section style={{ paddingTop: 80, paddingBottom: 24 }}>
        <div style={{ display: "flex", alignItems: "baseline", justifyContent: "space-between" }}>
          <h1
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 36,
              fontWeight: 400,
              letterSpacing: "-0.02em",
              color: "var(--color-text-primary)",
              margin: 0,
            }}
          >
            Your Book
          </h1>
          <span
            style={{
              fontFamily: "var(--font-mono)",
              fontSize: 13,
              color: "var(--color-text-tertiary)",
            }}
          >
            {isArchived ? filteredArchived.length : filtered.length} {isArchived ? "archived" : "active"}
          </span>
        </div>

        {/* Section rule */}
        <div style={{ height: 1, background: "var(--color-rule-heavy)", marginTop: 16, marginBottom: 16 }} />

        {/* Lifecycle filter (active only, only when lifecycle values exist) */}
        {!isArchived && lifecycleValues.length > 0 && (
          <div style={{ display: "flex", gap: 20, marginBottom: 16 }}>
            <button
              onClick={() => setLifecycleFilter("all")}
              style={{
                fontFamily: "var(--font-mono)",
                fontSize: 12,
                fontWeight: 500,
                letterSpacing: "0.06em",
                textTransform: "uppercase",
                color: lifecycleFilter === "all" ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
                textDecoration: lifecycleFilter === "all" ? "underline" : "none",
                textUnderlineOffset: "4px",
                background: "none",
                border: "none",
                padding: 0,
                cursor: "pointer",
              }}
            >
              all
            </button>
            {lifecycleValues.map((lc) => (
              <button
                key={lc}
                onClick={() => setLifecycleFilter(lc)}
                style={{
                  fontFamily: "var(--font-mono)",
                  fontSize: 12,
                  fontWeight: 500,
                  letterSpacing: "0.06em",
                  textTransform: "uppercase",
                  color: lifecycleFilter === lc ? "var(--color-text-primary)" : "var(--color-text-tertiary)",
                  textDecoration: lifecycleFilter === lc ? "underline" : "none",
                  textUnderlineOffset: "4px",
                  background: "none",
                  border: "none",
                  padding: 0,
                  cursor: "pointer",
                }}
              >
                {lc}
              </button>
            ))}
          </div>
        )}

        {/* Search */}
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="⌘  Search accounts..."
          style={{
            width: "100%",
            fontFamily: "var(--font-sans)",
            fontSize: 14,
            color: "var(--color-text-primary)",
            background: "none",
            border: "none",
            borderBottom: "1px solid var(--color-rule-light)",
            padding: "8px 0",
            outline: "none",
          }}
        />
      </section>

      {/* ═══ CREATE FORM ═══ */}
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

      {/* ═══ ACCOUNT ROWS ═══ */}
      <section>
        {displayList.length === 0 ? (
          <div style={{ textAlign: "center", padding: "64px 0" }}>
            <p style={{ fontFamily: "var(--font-serif)", fontSize: 18, fontStyle: "italic", color: "var(--color-text-tertiary)" }}>
              {isArchived ? "No archived accounts" : "No matches"}
            </p>
            <p style={{ fontFamily: "var(--font-sans)", fontSize: 13, fontWeight: 300, color: "var(--color-text-tertiary)", marginTop: 8 }}>
              {isArchived ? "Archived accounts will appear here." : "Try a different search or filter."}
            </p>
          </div>
        ) : (
          <div style={{ display: "flex", flexDirection: "column" }}>
            {isArchived
              ? filteredArchived.map((account, i) => (
                  <ArchivedAccountRow
                    key={account.id}
                    account={account}
                    showBorder={i < filteredArchived.length - 1}
                  />
                ))
              : filtered.map((account, i) => (
                  <div key={account.id}>
                    <AccountRow
                      account={account}
                      isExpanded={expandedParents.has(account.id)}
                      onToggleExpand={account.isParent ? () => toggleExpand(account.id) : undefined}
                      showBorder={i < filtered.length - 1 || (expandedParents.has(account.id) && !!childrenCache[account.id]?.length)}
                    />
                    {account.isParent && expandedParents.has(account.id) &&
                      childrenCache[account.id]?.map((child, ci) => (
                        <AccountRow
                          key={child.id}
                          account={child}
                          isChild
                          showBorder={ci < (childrenCache[account.id]?.length ?? 0) - 1 || i < filtered.length - 1}
                        />
                      ))}
                  </div>
                ))}
          </div>
        )}
      </section>

      {/* ═══ END MARK ═══ */}
      {displayList.length > 0 && (
        <div
          style={{
            borderTop: "1px solid var(--color-rule-heavy)",
            marginTop: 48,
            paddingTop: 32,
            paddingBottom: 120,
            textAlign: "center",
          }}
        >
          <div
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 14,
              fontStyle: "italic",
              color: "var(--color-text-tertiary)",
            }}
          >
            That's everything.
          </div>
        </div>
      )}
    </div>
  );
}

// ─── Account Row ────────────────────────────────────────────────────────────

function AccountRow({
  account,
  isExpanded,
  onToggleExpand,
  isChild,
  showBorder,
}: {
  account: AccountListItem;
  isExpanded?: boolean;
  onToggleExpand?: () => void;
  isChild?: boolean;
  showBorder: boolean;
}) {
  const daysSince = account.daysSinceLastMeeting;
  const isStale = daysSince != null && daysSince > 14;

  return (
    <Link
      to="/accounts/$accountId"
      params={{ accountId: account.id }}
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "14px 0",
        paddingLeft: isChild ? 28 : 0,
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        textDecoration: "none",
        transition: "background 0.12s ease",
      }}
    >
      {/* Health dot */}
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: healthDotColor[account.health ?? ""] ?? "var(--color-paper-linen)",
          flexShrink: 0,
          marginTop: 8,
        }}
      />

      {/* Content */}
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <span
            style={{
              fontFamily: "var(--font-serif)",
              fontSize: 17,
              fontWeight: 400,
              color: "var(--color-text-primary)",
            }}
          >
            {account.name}
          </span>
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
              {isExpanded ? "▾" : "▸"} {account.childCount} BU{account.childCount !== 1 ? "s" : ""}
            </button>
          )}
        </div>
        {account.teamSummary && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              marginTop: 2,
            }}
          >
            {account.teamSummary}
            {account.openActionCount > 0 && (
              <span> · {account.openActionCount} action{account.openActionCount !== 1 ? "s" : ""}</span>
            )}
          </div>
        )}
      </div>

      {/* Right metrics */}
      <div style={{ display: "flex", alignItems: "baseline", gap: 16, flexShrink: 0 }}>
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
      </div>
    </Link>
  );
}

// ─── Archived Account Row ───────────────────────────────────────────────────

function ArchivedAccountRow({
  account,
  showBorder,
}: {
  account: ArchivedAccount;
  showBorder: boolean;
}) {
  return (
    <Link
      to="/accounts/$accountId"
      params={{ accountId: account.id }}
      style={{
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        padding: "14px 0",
        borderBottom: showBorder ? "1px solid var(--color-rule-light)" : "none",
        textDecoration: "none",
      }}
    >
      <div
        style={{
          width: 8,
          height: 8,
          borderRadius: 4,
          background: healthDotColor[account.health ?? ""] ?? "var(--color-paper-linen)",
          flexShrink: 0,
          marginTop: 8,
        }}
      />
      <div style={{ flex: 1, minWidth: 0 }}>
        <span
          style={{
            fontFamily: "var(--font-serif)",
            fontSize: 17,
            fontWeight: 400,
            color: "var(--color-text-primary)",
          }}
        >
          {account.name}
        </span>
        {account.lifecycle && (
          <div
            style={{
              fontFamily: "var(--font-sans)",
              fontSize: 13,
              fontWeight: 300,
              color: "var(--color-text-tertiary)",
              marginTop: 2,
            }}
          >
            {account.lifecycle}
          </div>
        )}
      </div>
      {account.arr != null && (
        <span style={{ fontFamily: "var(--font-mono)", fontSize: 13, color: "var(--color-text-secondary)", flexShrink: 0 }}>
          ${formatArr(account.arr)}
        </span>
      )}
    </Link>
  );
}
