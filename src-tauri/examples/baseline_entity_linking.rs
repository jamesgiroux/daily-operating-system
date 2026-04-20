//! DOS-258 baseline capture.
//!
//! Dumps the current (legacy) entity linking state to JSON so Lane D/E can
//! diff the new deterministic service output against it.
//!
//! Run:
//!   cargo run --example baseline_entity_linking -- \
//!     --db ~/.dailyos/dailyos.db \
//!     --days-meetings 90 --days-emails 30 \
//!     --output .docs/migrations/entity-linking-baseline.json
//!
//! The DB is opened using the macOS Keychain key (same path as the app).
//! Output JSON schema:
//!   { "captured_at": "...", "meetings": [...], "emails": [...] }
//! Each entry: { "id", "primary_entity_id", "primary_entity_type", "source" }

use std::path::PathBuf;

use dailyos_lib::db::encryption;
use rusqlite::Connection;
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
struct EntityRow {
    id: String,
    primary_entity_id: Option<String>,
    primary_entity_type: Option<String>,
    source: String,
}

fn open_db(db_path: &PathBuf) -> Result<Connection, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("Failed to open DB at {}: {e}", db_path.display()))?;

    if !encryption::is_database_plaintext(db_path) {
        let hex_key = encryption::get_or_create_db_key(db_path)
            .map_err(|e| format!("Failed to get DB encryption key: {e}"))?;
        conn.execute_batch(&encryption::key_to_pragma(&hex_key))
            .map_err(|e| format!("Failed to apply encryption key: {e}"))?;
    }

    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|e| format!("Failed to configure DB connection: {e}"))?;

    Ok(conn)
}

fn query_meetings(conn: &Connection, days: u32) -> Result<Vec<EntityRow>, String> {
    let sql = "
        SELECT m.id, me.entity_id, me.entity_type
        FROM meetings m
        LEFT JOIN meeting_entities me
            ON me.meeting_id = m.id AND me.is_primary = 1
        WHERE m.start_time >= datetime('now', ?1)
        ORDER BY m.start_time DESC
    ";
    let cutoff = format!("-{days} days");

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Failed to prepare meeting query: {e}"))?;

    let rows = stmt
        .query_map([&cutoff], |row| {
            Ok(EntityRow {
                id: row.get(0)?,
                primary_entity_id: row.get(1)?,
                primary_entity_type: row.get(2)?,
                source: "legacy".to_string(),
            })
        })
        .map_err(|e| format!("Failed to query meetings: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

fn query_emails(conn: &Connection, days: u32) -> Result<Vec<EntityRow>, String> {
    let sql = "
        SELECT email_id, entity_id, entity_type
        FROM emails
        WHERE received_at >= datetime('now', ?1)
        ORDER BY received_at DESC
    ";
    let cutoff = format!("-{days} days");

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("Failed to prepare email query: {e}"))?;

    let rows = stmt
        .query_map([&cutoff], |row| {
            Ok(EntityRow {
                id: row.get(0)?,
                primary_entity_id: row.get(1)?,
                primary_entity_type: row.get(2)?,
                source: "legacy".to_string(),
            })
        })
        .map_err(|e| format!("Failed to query emails: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(rows)
}

fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == flag)
        .map(|w| w[1].clone())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let db_path = PathBuf::from(
        parse_arg(&args, "--db")
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_default();
                format!("{home}/.dailyos/dailyos.db")
            }),
    );
    let days_meetings: u32 = parse_arg(&args, "--days-meetings")
        .and_then(|v| v.parse().ok())
        .unwrap_or(90);
    let days_emails: u32 = parse_arg(&args, "--days-emails")
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);
    let output_path = parse_arg(&args, "--output")
        .unwrap_or_else(|| ".docs/migrations/entity-linking-baseline.json".to_string());

    let conn = match open_db(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error opening DB: {e}");
            std::process::exit(1);
        }
    };

    let meetings = match query_meetings(&conn, days_meetings) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error querying meetings: {e}");
            std::process::exit(1);
        }
    };

    let emails = match query_emails(&conn, days_emails) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("Error querying emails: {e}");
            std::process::exit(1);
        }
    };

    let output = json!({
        "captured_at": chrono::Utc::now().to_rfc3339(),
        "days_meetings": days_meetings,
        "days_emails": days_emails,
        "meeting_count": meetings.len(),
        "email_count": emails.len(),
        "meetings": meetings,
        "emails": emails,
    });

    let json_str = serde_json::to_string_pretty(&output).expect("JSON serialization failed");

    std::fs::write(&output_path, &json_str)
        .unwrap_or_else(|e| eprintln!("Warning: could not write to {output_path}: {e}"));

    println!(
        "Baseline captured: {} meetings, {} emails → {}",
        meetings.len(),
        emails.len(),
        output_path
    );
}
