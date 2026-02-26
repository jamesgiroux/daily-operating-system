//! Async database service using tokio-rusqlite.
//!
//! Provides read/write separation for SQLite in WAL mode:
//! - 1 writer connection (serialized via tokio-rusqlite's internal channel)
//! - 2 reader connections (concurrent under WAL, round-robin dispatched)
//!
//! All SQLite I/O runs on dedicated OS threads, never blocking the Tokio
//! runtime. This eliminates the beachball caused by the old
//! `Mutex<Option<ActionDb>>` pattern where background tasks holding the
//! lock for seconds would block every Tauri command handler.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio_rusqlite::Connection;

use crate::db::DbError;

/// Number of read connections in the pool. 2 is plenty for a desktop app —
/// WAL mode allows concurrent readers so this gives us two parallel reads
/// while a write is in progress.
const NUM_READERS: usize = 2;

/// Async database connection pool with read/write separation.
///
/// The writer connection is serialized: all closures submitted via `writer()`
/// execute sequentially on a single dedicated thread. Reader connections
/// share WAL snapshots and can execute concurrently with the writer and
/// each other.
pub struct DbService {
    writer: Connection,
    readers: Vec<Connection>,
    read_idx: AtomicUsize,
}

/// Apply standard pragmas to a connection. Both readers and writers get
/// busy_timeout and WAL mode; readers additionally get query_only.
/// PRAGMA key is set first for SQLCipher (ADR-0092).
fn apply_pragmas(
    conn: &rusqlite::Connection,
    read_only: bool,
    hex_key: &str,
) -> Result<(), rusqlite::Error> {
    // PRAGMA key MUST be first — before any other PRAGMA (ADR-0092)
    conn.execute_batch(&crate::db::encryption::key_to_pragma(hex_key))?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
    conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    if read_only {
        conn.execute_batch("PRAGMA query_only = ON;")?;
    }
    Ok(())
}

impl DbService {
    /// Open a new DbService at the standard database path.
    ///
    /// Runs migrations on the writer connection, then opens reader connections.
    /// Must be called from an async context (Tokio runtime).
    pub async fn open() -> Result<Self, DbError> {
        let path = crate::db::ActionDb::db_path_public()?;
        Self::open_at(path).await
    }

    /// Open a DbService at an explicit path. Used for testing.
    pub async fn open_at(path: PathBuf) -> Result<Self, DbError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(DbError::CreateDir)?;
            }
        }

        // Get or create encryption key from Keychain (ADR-0092)
        let hex_key =
            crate::db::encryption::get_or_create_db_key(&path).map_err(DbError::Encryption)?;

        // Migrate plaintext DB if it exists
        if path.exists() && crate::db::encryption::is_database_plaintext(&path) {
            log::info!("DbService: Detected plaintext database, migrating to encrypted...");
            crate::db::encryption::migrate_to_encrypted(&path, &hex_key)
                .map_err(DbError::Encryption)?;
        }

        let path_str = path.to_string_lossy().to_string();
        let writer_key = hex_key.clone();

        // Open the writer connection — this is where migrations run.
        let writer = Connection::open(&path_str)
            .await
            .map_err(|e| DbError::Migration(format!("Failed to open writer: {e}")))?;

        // Apply pragmas and run migrations on the writer.
        writer
            .call(move |conn| {
                apply_pragmas(conn, false, &writer_key)?;
                crate::migrations::run_migrations(conn)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

        // Open reader connections — no migrations, just pragmas.
        let mut readers = Vec::with_capacity(NUM_READERS);
        for _ in 0..NUM_READERS {
            let r = Connection::open(&path_str)
                .await
                .map_err(|e| DbError::Migration(format!("Failed to open reader: {e}")))?;

            let reader_key = hex_key.clone();
            r.call(move |conn| {
                apply_pragmas(conn, true, &reader_key)?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::Migration(e.to_string()))?;

            readers.push(r);
        }

        Ok(Self {
            writer,
            readers,
            read_idx: AtomicUsize::new(0),
        })
    }

    /// Get a reader connection (round-robin). Use for SELECT-only queries
    /// from Tauri command handlers. Never blocks the writer.
    pub fn reader(&self) -> &Connection {
        let idx = self.read_idx.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        &self.readers[idx]
    }

    /// Get the writer connection. Use for INSERT/UPDATE/DELETE operations.
    /// All writes are serialized through this single connection, preventing
    /// WAL contention and SQLITE_BUSY retry storms.
    pub fn writer(&self) -> &Connection {
        &self.writer
    }
}
