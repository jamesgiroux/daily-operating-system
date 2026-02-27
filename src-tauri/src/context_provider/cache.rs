//! Glean document cache — in-memory (`DashMap`) with SQLite persistence.
//!
//! TTLs:
//! - Documents: 1 hour
//! - Person profiles: 24 hours
//! - Org graph: 4 hours
//!
//! Manual refresh bypasses cache.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::db::ActionDb;

// ---------------------------------------------------------------------------
// TTL constants
// ---------------------------------------------------------------------------

const DOCUMENT_TTL: Duration = Duration::from_secs(3600); // 1 hour
const PERSON_PROFILE_TTL: Duration = Duration::from_secs(86400); // 24 hours
const ORG_GRAPH_TTL: Duration = Duration::from_secs(14400); // 4 hours

// ---------------------------------------------------------------------------
// Cache entry types
// ---------------------------------------------------------------------------

/// The kind of cached item, which determines its TTL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheKind {
    Document,
    PersonProfile,
    OrgGraph,
}

impl CacheKind {
    pub fn ttl(&self) -> Duration {
        match self {
            Self::Document => DOCUMENT_TTL,
            Self::PersonProfile => PERSON_PROFILE_TTL,
            Self::OrgGraph => ORG_GRAPH_TTL,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Document => "document",
            Self::PersonProfile => "person_profile",
            Self::OrgGraph => "org_graph",
        }
    }
}

/// A cached item with metadata.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached content (JSON string or rendered text).
    content: String,
    /// When this entry was inserted or last refreshed.
    inserted_at: Instant,
    /// Kind of cache entry (determines TTL).
    kind: CacheKind,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        self.inserted_at.elapsed() > self.kind.ttl()
    }
}

// ---------------------------------------------------------------------------
// GleanCache
// ---------------------------------------------------------------------------

/// In-memory + DB-backed cache for Glean API responses.
///
/// The in-memory layer (`DashMap`) is the hot path. On cache miss, we check
/// the DB table. On Glean API response, we write to both.
pub struct GleanCache {
    /// In-memory cache: key → entry.
    /// Key format: `"{kind}:{entity_id}"` or `"{kind}:{query}"`.
    memory: DashMap<String, CacheEntry>,
}

impl Default for GleanCache {
    fn default() -> Self {
        Self::new()
    }
}

impl GleanCache {
    pub fn new() -> Self {
        Self {
            memory: DashMap::new(),
        }
    }

    /// Build a cache key from kind and identifier.
    fn cache_key(kind: CacheKind, id: &str) -> String {
        format!("{}:{}", kind.as_str(), id)
    }

    /// Get a cached value, returning `None` if missing or expired.
    pub fn get(&self, kind: CacheKind, id: &str) -> Option<String> {
        let key = Self::cache_key(kind, id);
        if let Some(entry) = self.memory.get(&key) {
            if !entry.is_expired() {
                return Some(entry.content.clone());
            }
            // Expired — remove it
            drop(entry);
            self.memory.remove(&key);
        }
        None
    }

    /// Get from memory first, then fall back to DB.
    pub fn get_with_db(&self, kind: CacheKind, id: &str, db: &ActionDb) -> Option<String> {
        // Try memory first
        if let Some(content) = self.get(kind, id) {
            return Some(content);
        }

        // Try DB
        let key = Self::cache_key(kind, id);
        let ttl_secs = kind.ttl().as_secs() as i64;
        let result: Option<String> = db
            .conn_ref()
            .query_row(
                "SELECT content FROM glean_document_cache
                 WHERE cache_key = ?1
                 AND datetime(cached_at, '+' || ?2 || ' seconds') > datetime('now')",
                rusqlite::params![key, ttl_secs],
                |row| row.get(0),
            )
            .ok();

        if let Some(ref content) = result {
            // Populate memory cache from DB hit
            self.memory.insert(
                key,
                CacheEntry {
                    content: content.clone(),
                    inserted_at: Instant::now(),
                    kind,
                },
            );
        }

        result
    }

    /// Insert a value into both memory and DB cache.
    pub fn put(&self, kind: CacheKind, id: &str, content: &str, db: &ActionDb) {
        let key = Self::cache_key(kind, id);

        // Memory
        self.memory.insert(
            key.clone(),
            CacheEntry {
                content: content.to_string(),
                inserted_at: Instant::now(),
                kind,
            },
        );

        // DB (best-effort — cache misses are not fatal)
        let _ = db.conn_ref().execute(
            "INSERT OR REPLACE INTO glean_document_cache (cache_key, kind, content, cached_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            rusqlite::params![key, kind.as_str(), content],
        );
    }

    /// Insert into memory only (no DB write). Used when no DB is available.
    pub fn put_memory_only(&self, kind: CacheKind, id: &str, content: &str) {
        let key = Self::cache_key(kind, id);
        self.memory.insert(
            key,
            CacheEntry {
                content: content.to_string(),
                inserted_at: Instant::now(),
                kind,
            },
        );
    }

    /// Invalidate a specific cache entry.
    pub fn invalidate(&self, kind: CacheKind, id: &str, db: Option<&ActionDb>) {
        let key = Self::cache_key(kind, id);
        self.memory.remove(&key);
        if let Some(db) = db {
            let _ = db.conn_ref().execute(
                "DELETE FROM glean_document_cache WHERE cache_key = ?1",
                [&key],
            );
        }
    }

    /// Purge all expired entries from memory.
    pub fn purge_expired(&self) {
        self.memory.retain(|_, entry| !entry.is_expired());
    }

    /// Purge expired entries from DB.
    pub fn purge_expired_db(&self, db: &ActionDb) {
        let _ = db.conn_ref().execute(
            "DELETE FROM glean_document_cache
             WHERE (kind = 'document' AND datetime(cached_at, '+3600 seconds') < datetime('now'))
             OR (kind = 'person_profile' AND datetime(cached_at, '+86400 seconds') < datetime('now'))
             OR (kind = 'org_graph' AND datetime(cached_at, '+14400 seconds') < datetime('now'))",
            [],
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_cache_hit_miss() {
        let cache = GleanCache::new();

        // Miss
        assert!(cache.get(CacheKind::Document, "doc-1").is_none());

        // Put + Hit
        cache.put_memory_only(CacheKind::Document, "doc-1", "hello world");
        assert_eq!(
            cache.get(CacheKind::Document, "doc-1"),
            Some("hello world".to_string())
        );

        // Different kind = miss
        assert!(cache.get(CacheKind::PersonProfile, "doc-1").is_none());
    }

    #[test]
    fn test_invalidate() {
        let cache = GleanCache::new();
        cache.put_memory_only(CacheKind::Document, "doc-2", "content");
        assert!(cache.get(CacheKind::Document, "doc-2").is_some());

        cache.invalidate(CacheKind::Document, "doc-2", None);
        assert!(cache.get(CacheKind::Document, "doc-2").is_none());
    }

    #[test]
    fn test_purge_expired() {
        let cache = GleanCache::new();
        // Insert with a very short TTL (manually expired)
        let key = GleanCache::cache_key(CacheKind::Document, "old");
        cache.memory.insert(
            key.clone(),
            CacheEntry {
                content: "stale".to_string(),
                inserted_at: Instant::now() - Duration::from_secs(7200), // 2 hours ago
                kind: CacheKind::Document,
            },
        );

        assert!(cache.memory.contains_key(&key));
        cache.purge_expired();
        assert!(!cache.memory.contains_key(&key));
    }
}
