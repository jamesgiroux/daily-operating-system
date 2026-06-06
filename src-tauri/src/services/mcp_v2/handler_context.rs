//! Request-scoped context threaded into every `McpToolHandler::invoke`.
//!
//! # Why this exists
//!
//! The MCP v2 server runs as a **separate out-of-process sidecar**
//! (`dailyos-mcp`) with no in-process `DbService` — `db_service::try_global()`
//! returns `None` there. Before this type, every handler and the audit path
//! reached for the DB by self-opening its own `ActionDb` (e.g.
//! `tool_account_status.rs` `resolve_account_subject` and `audit.rs`
//! `insert_outbox` fallback). N independent connections per process is the
//! db-lock-storm class (`docs/solutions/architecture-patterns/db-lock-storm-class-2026-05-27.md`):
//! each one races the app writer's WAL cross-process.
//!
//! `McpHandlerContext` gives the sidecar **one owned connection** for the
//! process lifetime, threaded by reference into each `invoke`. Handlers borrow
//! it; they never open their own. This mirrors ADR-0102 §5's `AbilityContext`
//! pattern — handlers *receive* DB access, they do not open it.
//!
//! ## Scope boundary (L0 packet §2, §4)
//!
//! This closes the **direct** self-opens in the `mcp_v2` handler + audit path.
//! It does NOT rewrite the `abilities-runtime` workspace readers
//! (`attach_live_workspace_readers`), which open lazily across the crate
//! boundary — that porosity is covered by the bounded CI gate, not this type.
//! It also does NOT route writes to the app's single writer across processes
//! (that needs a cross-process transport/IPC layer, tracked separately) and
//! installs **no second `DbService`** in the sidecar.
//!
//! ## Connection shape
//!
//! The owned connection is an [`ActionDb`] opened **writable** through
//! `ActionDb::open_with_audit_tagger`, so the same startup key fetch also
//! captures the audit tagger registered write paths need without re-entering
//! [`LocalKeychain`] during a request. The same handle serves handler reads and
//! the audit-outbox write (`audit.rs` needs only `&Connection` — verified
//! `conn.execute`). `Connection` is `!Sync` and the `Gateway` is shared as
//! `Arc<Gateway>` across the rmcp service, so the connection is wrapped in
//! `Arc<Mutex<…>>` and locked per call — which *is* the single-writer discipline
//! for the sidecar process.

use std::sync::{Arc, Mutex};

use crate::db::{ActionDb, DbError, LocalDbAuditTagger, LocalKeychain};

/// Open the sidecar's single owned connection.
///
/// Lives in `services/` so the DB open stays behind the ADR-0101 service
/// boundary — the sidecar binary calls this rather than opening a connection
/// itself. Writable so the one handle serves both handler reads and the
/// audit-outbox write.
pub fn open_sidecar_connection() -> Result<OwnedSidecarConnection, DbError> {
    let (db, audit_tagger) = ActionDb::open_with_audit_tagger(Arc::new(LocalKeychain::new()))?;
    Ok(OwnedSidecarConnection {
        connection: Arc::new(Mutex::new(db)),
        audit_tagger: Arc::new(audit_tagger),
    })
}

/// One process-lifetime DB connection, shared per-call across handlers.
///
/// `Mutex` because [`rusqlite::Connection`] is `!Sync` and the owning
/// [`Gateway`](super::gateway::Gateway) is shared as `Arc<Gateway>`. Locking
/// per call serializes the sidecar's DB access through a single connection.
pub type OwnedConnection = Arc<Mutex<ActionDb>>;

/// Sidecar-owned DB capabilities captured at process startup.
#[derive(Clone)]
pub struct OwnedSidecarConnection {
    connection: OwnedConnection,
    audit_tagger: Arc<LocalDbAuditTagger>,
}

impl OwnedSidecarConnection {
    #[cfg(test)]
    pub(crate) fn for_tests(connection: OwnedConnection) -> Self {
        Self {
            connection,
            audit_tagger: Arc::new(LocalDbAuditTagger::for_tests(
                "mcp-owned-sidecar-test-audit-key",
            )),
        }
    }

    pub(crate) fn connection(&self) -> OwnedConnection {
        Arc::clone(&self.connection)
    }

    pub(crate) fn audit_tagger(&self) -> Arc<LocalDbAuditTagger> {
        Arc::clone(&self.audit_tagger)
    }
}

/// Request-scoped dependency bundle handed to `McpToolHandler::invoke`.
///
/// Constructed once per dispatch by the gateway, borrowing the gateway's one
/// owned connection (when present). Handlers obtain DB access through
/// [`Self::with_conn`] instead of opening their own.
#[derive(Clone)]
pub struct McpHandlerContext {
    connection: Option<OwnedConnection>,
    audit_tagger: Option<Arc<LocalDbAuditTagger>>,
}

impl McpHandlerContext {
    /// Context backed by the gateway's single owned connection.
    pub fn with_owned_connection(connection: OwnedConnection) -> Self {
        Self {
            connection: Some(connection),
            audit_tagger: None,
        }
    }

    /// Context backed by the sidecar's single owned connection and startup
    /// audit tagger.
    pub fn with_sidecar_connection(connection: OwnedSidecarConnection) -> Self {
        Self {
            connection: Some(connection.connection()),
            audit_tagger: Some(connection.audit_tagger()),
        }
    }

    /// Context with no owned connection.
    ///
    /// Used by tests and any dispatch path that has not adopted the owned
    /// connection yet. Handlers fall back to their prior self-open behavior
    /// when [`Self::with_conn`] returns `None` — preserving existing behavior
    /// rather than failing closed mid-migration.
    pub fn without_connection() -> Self {
        Self {
            connection: None,
            audit_tagger: None,
        }
    }

    /// Run `f` against the one owned connection, if this context carries one.
    ///
    /// Returns `None` when no owned connection is present, signalling the
    /// caller to use its fallback path. The lock is held only for the duration
    /// of `f`, serializing access to the single sidecar connection.
    pub fn with_conn<T>(&self, f: impl FnOnce(&ActionDb) -> T) -> Option<T> {
        let owned = self.connection.as_ref()?;
        let guard = owned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Some(f(&guard))
    }

    /// Clone the owned connection handle for request-scoped service adapters.
    pub(crate) fn owned_connection(&self) -> Option<OwnedConnection> {
        self.connection.as_ref().map(Arc::clone)
    }

    /// Clone the startup-captured audit tagger for request-scoped service adapters.
    pub(crate) fn audit_tagger(&self) -> Option<Arc<LocalDbAuditTagger>> {
        self.audit_tagger.as_ref().map(Arc::clone)
    }

    /// Whether this context carries an owned connection.
    pub fn has_owned_connection(&self) -> bool {
        self.connection.is_some()
    }
}

impl std::fmt::Debug for McpHandlerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpHandlerContext")
            .field("has_owned_connection", &self.connection.is_some())
            .field("has_audit_tagger", &self.audit_tagger.is_some())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ActionDb, DbAccount};

    /// Build a migrated temp-file [`ActionDb`] and wrap it as the one owned
    /// connection a context would carry.
    fn owned_db(path: std::path::PathBuf) -> (ActionDb, OwnedConnection) {
        let seed = ActionDb::open_at_unencrypted(path.clone()).expect("seed db open");
        let owned = ActionDb::open_at_unencrypted(path).expect("owned db open");
        let owned = Arc::new(Mutex::new(owned));
        (seed, owned)
    }

    fn seed_account(db: &ActionDb, id: &str, name: &str) {
        db.upsert_account(&DbAccount {
            id: id.to_string(),
            name: name.to_string(),
            account_type: crate::db::types::AccountType::default(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            ..Default::default()
        })
        .expect("seed account");
    }

    /// Load-bearing: the context routes handler DB access through the ONE
    /// owned connection. Proven observably — the account is seeded only
    /// into the owned connection's DB (a temp path no self-open via
    /// `LocalKeychain` would ever resolve), so a successful read through
    /// `with_conn` can only have come from the owned connection.
    #[test]
    fn with_conn_routes_reads_through_the_owned_connection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("owned.db");
        let (seed, owned) = owned_db(path);
        seed_account(&seed, "acct-owned-only", "Owned Only Co");

        let ctx = McpHandlerContext::with_owned_connection(owned);
        assert!(ctx.has_owned_connection());

        let found = ctx
            .with_conn(|db| db.get_account("acct-owned-only").expect("get_account"))
            .expect("with_conn must run against the owned connection");
        assert_eq!(
            found.map(|a| a.name),
            Some("Owned Only Co".to_string()),
            "read must resolve via the single owned connection, not a self-open"
        );
    }

    /// A multi-tool dispatch reuses the SAME owned connection. Two
    /// sequential `with_conn` calls (standing in for two tool invocations on
    /// the shared gateway connection) both see the same seeded state through
    /// one handle — no per-call reopen.
    #[test]
    fn multi_call_dispatch_reuses_one_owned_connection() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("owned.db");
        let (seed, owned) = owned_db(path);
        seed_account(&seed, "acct-1", "First Co");

        let ctx = McpHandlerContext::with_owned_connection(Arc::clone(&owned));

        // First "tool call".
        let first = ctx
            .with_conn(|db| db.get_account("acct-1").expect("get acct-1"))
            .flatten();
        assert_eq!(first.map(|a| a.name), Some("First Co".to_string()));

        // A write through the very same owned handle, then a second "tool
        // call" observes it — proving one connection, not N reopens.
        ctx.with_conn(|db| seed_account(db, "acct-2", "Second Co"))
            .expect("write through owned conn");
        let second = ctx
            .with_conn(|db| db.get_account("acct-2").expect("get acct-2"))
            .flatten();
        assert_eq!(second.map(|a| a.name), Some("Second Co".to_string()));

        // The Arc is shared (the gateway holds one, the context borrows it);
        // strong_count proves no clone-per-call duplication of the handle.
        assert_eq!(Arc::strong_count(&owned), 2, "one shared owned connection");
    }

    /// Without an owned connection, `with_conn` returns `None` so handlers take
    /// their documented self-open fallback rather than failing closed.
    #[test]
    fn without_connection_signals_fallback() {
        let ctx = McpHandlerContext::without_connection();
        assert!(!ctx.has_owned_connection());
        let ran: Option<()> = ctx.with_conn(|_db| ());
        assert!(ran.is_none(), "no owned connection => caller falls back");
    }
}
