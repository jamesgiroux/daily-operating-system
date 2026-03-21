//! Production demo data for first-run experience (I56).
//!
//! Seeds a curated dataset into the live database with `is_demo = 1` markers
//! so it can be cleanly removed when the user connects real data.
//! Unlike devtools, this runs in release builds and writes to the real workspace.
//! I633: health_score_history seeds + email_signals populated via enrichment pipeline.

use std::path::Path;

use chrono::{Local, TimeZone};

use crate::db::ActionDb;

/// Install demo data into the database and write fixture files to the workspace.
///
/// All rows are marked `is_demo = 1`. Sets `app_state.demo_mode_active = 1`.
pub fn install_demo(db: &ActionDb, workspace: Option<&Path>) -> Result<(), String> {
    let conn = db.conn_ref();
    let now = chrono::Utc::now();
    let today = now.to_rfc3339();

    let date_only = |n: i64| -> String {
        (chrono::Local::now() + chrono::Duration::days(n))
            .format("%Y-%m-%d")
            .to_string()
    };
    let days_ago = |n: i64| -> String { (now - chrono::Duration::days(n)).to_rfc3339() };

    // --- Accounts (3 with varied health/stage) ---
    let accounts: Vec<(&str, &str, &str, f64, &str, Option<&str>)> = vec![
        (
            "demo-acme",
            "Acme Corp",
            "nurture",
            1_200_000.0,
            "green",
            None,
        ),
        (
            "demo-globex",
            "Globex Industries",
            "renewal",
            800_000.0,
            "yellow",
            Some("2026-04-30"),
        ),
        (
            "demo-initech",
            "Initech",
            "onboarding",
            350_000.0,
            "green",
            None,
        ),
    ];

    for (id, name, lifecycle, arr, health, contract_end) in &accounts {
        conn.execute(
            "INSERT OR REPLACE INTO accounts (id, name, lifecycle, arr, health, contract_end, updated_at, is_demo) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
            rusqlite::params![id, name, lifecycle, arr, health, contract_end, &today],
        )
        .map_err(|e| format!("Demo account insert: {}", e))?;

        // Mirror in entities table
        conn.execute(
            "INSERT OR REPLACE INTO entities (id, name, entity_type, updated_at) VALUES (?1, ?2, 'account', ?3)",
            rusqlite::params![id, name, &today],
        )
        .map_err(|e| format!("Demo entity insert: {}", e))?;
    }

    // --- People (associated with accounts) ---
    let people: Vec<(&str, &str, &str, &str, Option<&str>)> = vec![
        (
            "demo-sarah",
            "sarah@acme.com",
            "Sarah Chen",
            "external",
            Some("demo-acme"),
        ),
        (
            "demo-jamie",
            "jamie@globex.com",
            "Jamie Morrison",
            "external",
            Some("demo-globex"),
        ),
        (
            "demo-dana",
            "dana@initech.com",
            "Dana Patel",
            "external",
            Some("demo-initech"),
        ),
        (
            "demo-priya",
            "priya@initech.com",
            "Priya Sharma",
            "external",
            Some("demo-initech"),
        ),
    ];

    for (id, email, name, relationship, account_id) in &people {
        conn.execute(
            "INSERT OR REPLACE INTO people (id, email, name, relationship, last_seen, is_demo) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1)",
            rusqlite::params![id, email, name, relationship, &today],
        )
        .map_err(|e| format!("Demo person insert: {}", e))?;

        // Link to account if specified
        if let Some(acct) = account_id {
            conn.execute(
                "INSERT INTO account_stakeholders (account_id, person_id, role, relationship_type) \
                 VALUES (?1, ?2, 'associated', 'stakeholder')
                 ON CONFLICT(account_id, person_id) DO NOTHING",
                rusqlite::params![acct, id],
            )
            .map_err(|e| format!("Demo account-stakeholder link: {}", e))?;
        }
    }

    // --- Actions (5 with varied priorities) ---
    let actions: Vec<(&str, &str, &str, &str, Option<&str>, Option<String>, Option<&str>)> = vec![
        (
            "demo-act-1",
            "Send updated SOW to Acme legal team",
            "P1",
            "pending",
            Some("demo-acme"),
            Some(date_only(-1)),
            Some("Sarah Chen confirmed Phase 2 executive sponsorship. Legal needs the updated SOW before scoping."),
        ),
        (
            "demo-act-2",
            "Review Globex QBR deck with AE",
            "P1",
            "pending",
            Some("demo-globex"),
            Some(date_only(0)),
            Some("QBR is the highest-stakes meeting this quarter. Renewal decision expected."),
        ),
        (
            "demo-act-3",
            "Schedule Phase 2 kickoff with Initech",
            "P2",
            "pending",
            Some("demo-initech"),
            Some(date_only(1)),
            Some("Phase 1 wrapped successfully. Dana expressed interest in Phase 2 but budget approval pending."),
        ),
        (
            "demo-act-4",
            "Follow up on NPS survey responses",
            "P2",
            "pending",
            Some("demo-acme"),
            Some(date_only(-7)),
            Some("3 detractors identified in the latest NPS survey. Scores trending down."),
        ),
        (
            "demo-act-5",
            "Draft quarterly impact summary",
            "P3",
            "pending",
            None,
            Some(date_only(7)),
            Some("End-of-quarter impact summary for leadership."),
        ),
    ];

    for (id, title, priority, status, account_id, due_date, context) in &actions {
        conn.execute(
            "INSERT OR REPLACE INTO actions (id, title, priority, status, created_at, due_date, \
             account_id, context, updated_at, is_demo) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1)",
            rusqlite::params![
                id, title, priority, status, &today, due_date, account_id, context, &today
            ],
        )
        .map_err(|e| format!("Demo action insert: {}", e))?;
    }

    // --- Meetings history (4 with prep content) ---
    let today_local = Local::now();
    let make_iso = |hour: u32, min: u32| -> String {
        today_local
            .date_naive()
            .and_hms_opt(hour, min, 0)
            .map(|naive| {
                Local
                    .from_local_datetime(&naive)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };

    let meetings: Vec<(&str, &str, &str, String, Option<&str>, Option<&str>)> = vec![
        (
            "demo-mtg-acme",
            "Acme Corp Weekly Sync",
            "customer",
            make_iso(9, 0),
            Some("demo-acme"),
            Some("Reviewed Phase 1 benchmarks. Performance exceeded targets by 15%. Phase 2 scoping on track."),
        ),
        (
            "demo-mtg-globex",
            "Globex Industries QBR",
            "qbr",
            make_iso(11, 0),
            Some("demo-globex"),
            Some("Expansion to 3 teams going well. Key stakeholder departing Q2. Team B usage declining."),
        ),
        (
            "demo-mtg-initech",
            "Initech Phase 2 Kickoff",
            "customer",
            make_iso(14, 0),
            Some("demo-initech"),
            Some("Phase 1 delivered on time. Dana interested in Phase 2. Budget approval pending."),
        ),
        (
            "demo-mtg-standup",
            "Engineering Standup",
            "team_sync",
            make_iso(10, 0),
            None,
            None,
        ),
    ];

    // Build prep_frozen_json for meetings with prep content
    let build_prep_json = |account_name: Option<&str>, summary: Option<&str>| -> Option<String> {
        let acct = account_name?;
        let sum = summary?;
        Some(
            serde_json::json!({
                "account_context": format!("{} — active customer relationship", acct),
                "meeting_narrative": sum,
                "recommended_actions": ["Review recent activity", "Prepare discussion points"],
                "attendee_context": format!("Key stakeholders from {}", acct)
            })
            .to_string(),
        )
    };

    let account_names: std::collections::HashMap<&str, &str> = [
        ("demo-acme", "Acme Corp"),
        ("demo-globex", "Globex Industries"),
        ("demo-initech", "Initech"),
    ]
    .into_iter()
    .collect();

    for (id, title, mtype, start_time, account_id, summary) in &meetings {
        let prep_json = account_id.and_then(|acct| {
            let acct_name = account_names.get(acct).copied();
            build_prep_json(acct_name, *summary)
        });

        conn.execute(
            "INSERT OR REPLACE INTO meetings (id, title, meeting_type, start_time, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, mtype, start_time, &today],
        )
        .map_err(|e| format!("Demo meeting insert: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO meeting_transcripts (meeting_id, summary) VALUES (?1, ?2)",
            rusqlite::params![id, summary],
        )
        .map_err(|e| format!("Demo meeting transcript insert: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO meeting_prep (meeting_id, prep_frozen_json, prep_frozen_at) \
             VALUES (?1, ?2, ?3)",
            rusqlite::params![id, prep_json, &today],
        )
        .map_err(|e| format!("Demo meeting prep insert: {}", e))?;

        // Link customer meetings to their account entity
        if let Some(acct) = account_id {
            conn.execute(
                "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) \
                 VALUES (?1, ?2, 'account')",
                rusqlite::params![id, acct],
            )
            .map_err(|e| format!("Demo meeting-entity link: {}", e))?;
        }
    }

    // --- Email signals (2 to show email triage) ---
    conn.execute(
        "INSERT OR REPLACE INTO email_signals (email_id, sender_email, entity_id, entity_type, \
         signal_type, signal_text) \
         VALUES ('demo-email-1', 'sarah@acme.com', 'demo-acme', 'account', 'timeline', \
                 'Acme requesting revised launch date for Phase 2')",
        [],
    )
    .map_err(|e| format!("Demo email signal 1: {}", e))?;

    conn.execute(
        "INSERT OR REPLACE INTO email_signals (email_id, sender_email, entity_id, entity_type, \
         signal_type, signal_text) \
         VALUES ('demo-email-2', 'jamie@globex.com', 'demo-globex', 'account', 'risk', \
                 'Globex budget review — potential scope reduction for Q3')",
        [],
    )
    .map_err(|e| format!("Demo email signal 2: {}", e))?;

    // --- Historical meetings for richer demo experience ---
    let historical: Vec<(&str, &str, &str, String, Option<&str>, Option<&str>)> = vec![
        (
            "demo-mh-acme-7d",
            "Acme Corp Status Call",
            "customer",
            days_ago(7),
            Some("demo-acme"),
            Some("Phase 1 migration completed ahead of schedule. NPS trending down with 3 detractors."),
        ),
        (
            "demo-mh-globex-14d",
            "Globex Sprint Demo",
            "customer",
            days_ago(14),
            Some("demo-globex"),
            Some("New dashboard features demoed. Jamie enthusiastic. Casey raised Team B engagement concerns."),
        ),
        (
            "demo-mh-initech-21d",
            "Initech Phase 1 Review",
            "customer",
            days_ago(21),
            Some("demo-initech"),
            Some("Phase 1 on track. Integration testing completed. Dana confirmed go-live date."),
        ),
    ];

    for (id, title, mtype, start_time, account_id, summary) in &historical {
        conn.execute(
            "INSERT OR REPLACE INTO meetings (id, title, meeting_type, start_time, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, title, mtype, start_time, &today],
        )
        .map_err(|e| format!("Demo historical meeting: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO meeting_transcripts (meeting_id, summary) VALUES (?1, ?2)",
            rusqlite::params![id, summary],
        )
        .map_err(|e| format!("Demo historical meeting transcript: {}", e))?;
        conn.execute(
            "INSERT OR IGNORE INTO meeting_prep (meeting_id) VALUES (?1)",
            rusqlite::params![id],
        )
        .map_err(|e| format!("Demo historical meeting prep: {}", e))?;

        if let Some(acct) = account_id {
            conn.execute(
                "INSERT OR IGNORE INTO meeting_entities (meeting_id, entity_id, entity_type) \
                 VALUES (?1, ?2, 'account')",
                rusqlite::params![id, acct],
            )
            .map_err(|e| format!("Demo historical meeting-entity link: {}", e))?;
        }
    }

    // I633: Seed health score history for trend computation demo
    for (acct, scores) in &[
        ("demo-acme", vec![(72.0, "green"), (68.0, "yellow"), (75.0, "green")]),
        ("demo-globex", vec![(45.0, "yellow"), (42.0, "yellow"), (38.0, "red")]),
    ] {
        for (i, (score, band)) in scores.iter().enumerate() {
            conn.execute(
                "INSERT OR IGNORE INTO health_score_history (account_id, score, band, confidence, computed_at) \
                 VALUES (?1, ?2, ?3, 0.75, datetime('now', ?4))",
                rusqlite::params![acct, score, band, format!("-{} days", (scores.len() - i) * 7)],
            )
            .map_err(|e| format!("Demo health history: {}", e))?;
        }
    }

    // Set demo mode active
    conn.execute("UPDATE app_state SET demo_mode_active = 1 WHERE id = 1", [])
        .map_err(|e| format!("Set demo_mode_active: {}", e))?;

    // Write fixture files if workspace provided
    if let Some(ws) = workspace {
        write_demo_fixtures(ws)?;
    }

    log::info!("Demo data installed (3 accounts, 4 people, 5 actions, 7 meetings)");
    Ok(())
}

/// Remove all demo data and reset demo mode.
pub fn clear_demo(db: &ActionDb, workspace: Option<&Path>) -> Result<(), String> {
    let conn = db.conn_ref();

    // Delete demo rows from all marked tables
    conn.execute("DELETE FROM actions WHERE is_demo = 1", [])
        .map_err(|e| format!("Clear demo actions: {}", e))?;

    // Clean up meeting-entity links for demo meetings
    conn.execute(
        "DELETE FROM meeting_entities WHERE meeting_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo meeting-entity links: {}", e))?;

    // Clean up health score history for demo accounts
    conn.execute(
        "DELETE FROM health_score_history WHERE account_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo health history: {}", e))?;

    // Clean up account_stakeholders links for demo people
    conn.execute(
        "DELETE FROM account_stakeholders WHERE person_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo account-stakeholder links: {}", e))?;

    // Clean up entity_members links for demo people
    conn.execute(
        "DELETE FROM entity_members WHERE person_id LIKE 'demo-%'",
        [],
    )
    .map_err(|e| format!("Clear demo entity-member links: {}", e))?;

    // Clean up meetings for demo meetings (CASCADE handles meeting_prep + meeting_transcripts)
    conn.execute("DELETE FROM meetings WHERE id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo meetings: {}", e))?;

    // Clean up email_signals for demo
    conn.execute("DELETE FROM email_signals WHERE email_id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo email signals: {}", e))?;

    // Clean up entities for demo accounts
    conn.execute("DELETE FROM entities WHERE id LIKE 'demo-%'", [])
        .map_err(|e| format!("Clear demo entities: {}", e))?;

    conn.execute("DELETE FROM people WHERE is_demo = 1", [])
        .map_err(|e| format!("Clear demo people: {}", e))?;
    conn.execute("DELETE FROM accounts WHERE is_demo = 1", [])
        .map_err(|e| format!("Clear demo accounts: {}", e))?;

    // Reset demo mode
    conn.execute("UPDATE app_state SET demo_mode_active = 0 WHERE id = 1", [])
        .map_err(|e| format!("Clear demo_mode_active: {}", e))?;

    // Remove demo fixture files (best-effort — don't fail if files don't exist)
    if let Some(ws) = workspace {
        let data_dir = ws.join("_today").join("data");
        let _ = std::fs::remove_file(data_dir.join("schedule.json"));
        let _ = std::fs::remove_dir_all(data_dir.join("preps"));
    }

    log::info!("Demo data cleared");
    Ok(())
}

/// Get app_state row values.
pub fn get_app_state(db: &ActionDb) -> Result<AppStateRow, String> {
    let conn = db.conn_ref();
    conn.query_row(
        "SELECT demo_mode_active, has_completed_tour, wizard_completed_at, wizard_last_step \
         FROM app_state WHERE id = 1",
        [],
        |row| {
            Ok(AppStateRow {
                demo_mode_active: row.get::<_, i32>(0)? != 0,
                has_completed_tour: row.get::<_, i32>(1)? != 0,
                wizard_completed_at: row.get(2)?,
                wizard_last_step: row.get(3)?,
            })
        },
    )
    .map_err(|e| format!("Read app_state: {}", e))
}

/// Set tour as completed.
pub fn set_tour_completed(db: &ActionDb) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE app_state SET has_completed_tour = 1 WHERE id = 1",
        [],
    )
    .map_err(|e| format!("Set tour completed: {}", e))?;
    Ok(())
}

/// Set wizard as completed with current timestamp.
pub fn set_wizard_completed(db: &ActionDb) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE app_state SET wizard_completed_at = datetime('now') WHERE id = 1",
        [],
    )
    .map_err(|e| format!("Set wizard completed: {}", e))?;
    Ok(())
}

/// Set wizard last step for resume.
pub fn set_wizard_step(db: &ActionDb, step: &str) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute(
        "UPDATE app_state SET wizard_last_step = ?1 WHERE id = 1",
        rusqlite::params![step],
    )
    .map_err(|e| format!("Set wizard step: {}", e))?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateRow {
    pub demo_mode_active: bool,
    pub has_completed_tour: bool,
    pub wizard_completed_at: Option<String>,
    pub wizard_last_step: Option<String>,
}

/// Write demo fixture JSON files to the workspace data directory.
fn write_demo_fixtures(workspace: &Path) -> Result<(), String> {
    let data_dir = workspace.join("_today").join("data");
    let preps_dir = data_dir.join("preps");

    std::fs::create_dir_all(&preps_dir)
        .map_err(|e| format!("Failed to create demo preps dir: {}", e))?;

    // Write a minimal schedule.json with today's demo meetings
    let today_local = Local::now();
    let today_str = today_local.format("%Y-%m-%d").to_string();
    let make_time = |hour: u32, min: u32| -> String {
        today_local
            .date_naive()
            .and_hms_opt(hour, min, 0)
            .map(|naive| {
                Local
                    .from_local_datetime(&naive)
                    .single()
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_default()
            })
            .unwrap_or_default()
    };

    let schedule = format!(
        r#"{{
  "date": "{}",
  "meetings": [
    {{
      "id": "demo-mtg-acme",
      "title": "Acme Corp Weekly Sync",
      "start": "{}",
      "end": "{}",
      "meeting_type": "customer",
      "attendees": ["sarah@acme.com"],
      "account_id": "demo-acme",
      "has_prep": true
    }},
    {{
      "id": "demo-mtg-standup",
      "title": "Engineering Standup",
      "start": "{}",
      "end": "{}",
      "meeting_type": "team_sync",
      "attendees": [],
      "account_id": null,
      "has_prep": false
    }},
    {{
      "id": "demo-mtg-globex",
      "title": "Globex Industries QBR",
      "start": "{}",
      "end": "{}",
      "meeting_type": "qbr",
      "attendees": ["jamie@globex.com"],
      "account_id": "demo-globex",
      "has_prep": true
    }},
    {{
      "id": "demo-mtg-initech",
      "title": "Initech Phase 2 Kickoff",
      "start": "{}",
      "end": "{}",
      "meeting_type": "customer",
      "attendees": ["dana@initech.com", "priya@initech.com"],
      "account_id": "demo-initech",
      "has_prep": true
    }}
  ]
}}"#,
        today_str,
        make_time(9, 0),
        make_time(9, 30),
        make_time(10, 0),
        make_time(10, 15),
        make_time(11, 0),
        make_time(12, 0),
        make_time(14, 0),
        make_time(14, 45),
    );

    std::fs::write(data_dir.join("schedule.json"), schedule)
        .map_err(|e| format!("Failed to write demo schedule: {}", e))?;

    Ok(())
}
