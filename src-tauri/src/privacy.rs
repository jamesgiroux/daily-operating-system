//! I430: Privacy controls — data summary, clear intelligence, delete all.

use serde::{Deserialize, Serialize};

use crate::db::ActionDb;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSummary {
    pub accounts: usize,
    pub people: usize,
    pub projects: usize,
    pub meetings: usize,
    pub actions: usize,
    pub insights: usize,
    pub signals: usize,
    pub emails: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClearReport {
    pub assessments_deleted: usize,
    pub feedback_deleted: usize,
    pub signals_deleted: usize,
    pub summaries_cleared: usize,
}

/// Return counts of user data across all domains.
pub fn get_data_summary(db: &ActionDb) -> Result<DataSummary, String> {
    let conn = db.conn_ref();

    let count = |table: &str| -> usize {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap_or(0)
    };

    let count_where = |table: &str, clause: &str| -> usize {
        conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE {clause}"),
            [],
            |row| row.get(0),
        )
        .unwrap_or(0)
    };

    Ok(DataSummary {
        accounts: count_where("accounts", "archived = 0"),
        people: count_where("people", "archived = 0"),
        projects: count_where("projects", "archived = 0"),
        meetings: count("meetings"),
        actions: count("actions"),
        insights: count("entity_assessment"),
        signals: count("signal_events"),
        emails: count("emails"),
    })
}

/// Delete all AI-generated intelligence data while preserving user content.
pub fn clear_intelligence(db: &ActionDb) -> Result<ClearReport, String> {
    let conn = db.conn_ref();

    let assessments_deleted: usize = conn
        .execute("DELETE FROM entity_assessment", [])
        .map_err(|e| e.to_string())?;

    let feedback_deleted: usize = conn
        .execute("DELETE FROM intelligence_feedback", [])
        .map_err(|e| e.to_string())?;

    let signals_deleted: usize = conn
        .execute("DELETE FROM signal_events", [])
        .map_err(|e| e.to_string())?;

    // NULL out contextual summaries on emails
    let summaries_cleared: usize = conn
        .execute(
            "UPDATE emails SET contextual_summary = NULL WHERE contextual_summary IS NOT NULL",
            [],
        )
        .map_err(|e| e.to_string())?;

    // NULL out entity_quality health columns
    let _ = conn.execute(
        "UPDATE entity_quality SET health_score = NULL, health_trend = NULL WHERE health_score IS NOT NULL",
        [],
    );

    Ok(ClearReport {
        assessments_deleted,
        feedback_deleted,
        signals_deleted,
        summaries_cleared,
    })
}
