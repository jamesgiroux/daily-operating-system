use std::sync::Arc;

use chrono::{Duration, Utc};
use rusqlite::params;

use crate::db::{ActionDb, LocalKeychain};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct WatermarkDoctorReport {
    pub claims_below_floor: i64,
    pub compositions_below_floor: i64,
    pub zombie_attempts: i64,
    pub claims_missing_outbox: i64,
    pub compositions_missing_outbox: i64,
}

impl WatermarkDoctorReport {
    pub fn is_clean(&self) -> bool {
        self.claims_below_floor == 0
            && self.compositions_below_floor == 0
            && self.zombie_attempts == 0
            && self.claims_missing_outbox == 0
            && self.compositions_missing_outbox == 0
    }
}

pub fn run_from_args<I>(args: I) -> Option<i32>
where
    I: IntoIterator<Item = String>,
{
    let args = args.into_iter().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) != Some("doctor") {
        return None;
    }

    let subcommand = args.get(2).map(String::as_str).unwrap_or("watermarks");
    if subcommand != "watermarks" {
        eprintln!("unknown doctor subcommand `{subcommand}`; expected `watermarks`");
        return Some(2);
    }

    match run_watermark_doctor() {
        Ok(report) if report.is_clean() => {
            println!("dailyos doctor watermarks: ok");
            Some(0)
        }
        Ok(report) => {
            println!("dailyos doctor watermarks: failed");
            println!("claims_below_floor={}", report.claims_below_floor);
            println!(
                "compositions_below_floor={}",
                report.compositions_below_floor
            );
            println!("zombie_attempts={}", report.zombie_attempts);
            println!("claims_missing_outbox={}", report.claims_missing_outbox);
            println!(
                "compositions_missing_outbox={}",
                report.compositions_missing_outbox
            );
            Some(1)
        }
        Err(error) => {
            eprintln!("dailyos doctor watermarks failed to run: {error}");
            Some(1)
        }
    }
}

pub fn run_watermark_doctor() -> Result<WatermarkDoctorReport, String> {
    let db = ActionDb::open(Arc::new(LocalKeychain::new())).map_err(|error| error.to_string())?;
    inspect_watermarks(&db)
}

pub fn inspect_watermarks(db: &ActionDb) -> Result<WatermarkDoctorReport, String> {
    let cutoff = (Utc::now() - Duration::seconds(60)).to_rfc3339();
    Ok(WatermarkDoctorReport {
        claims_below_floor: count_i64(
            db,
            "SELECT COUNT(*) FROM intelligence_claims WHERE claim_version < 1",
            [],
        )?,
        compositions_below_floor: count_i64(
            db,
            "SELECT COUNT(*) FROM composition_versions WHERE composition_version < 1",
            [],
        )?,
        zombie_attempts: count_i64(
            db,
            "SELECT COUNT(*) FROM mutation_attempts
             WHERE status = 'in_flight' AND started_at < ?1",
            params![cutoff],
        )?,
        claims_missing_outbox: count_i64(
            db,
            "SELECT COUNT(*)
             FROM intelligence_claims c
             WHERE c.claim_version >= 1
               AND NOT EXISTS (
                 SELECT 1
                 FROM version_events ve
                 JOIN mutation_attempts ma ON ma.mutation_id = ve.mutation_id
                 WHERE ve.claim_id = c.id
                   AND ve.current_version = c.claim_version
                   AND ve.cursor = ma.cursor
                   AND ma.status IN ('committed', 'aborted')
               )",
            [],
        )?,
        compositions_missing_outbox: count_i64(
            db,
            "SELECT COUNT(*)
             FROM composition_versions cv
             WHERE cv.composition_version >= 1
               AND NOT EXISTS (
                 SELECT 1
                 FROM version_events ve
                 JOIN mutation_attempts ma ON ma.mutation_id = ve.mutation_id
                 WHERE ve.composition_id = cv.composition_id
                   AND ve.current_version = cv.composition_version
                   AND ve.cursor = ma.cursor
                   AND ma.status IN ('committed', 'aborted')
               )",
            [],
        )?,
    })
}

fn count_i64<P>(db: &ActionDb, sql: &str, params: P) -> Result<i64, String>
where
    P: rusqlite::Params,
{
    db.conn_ref()
        .query_row(sql, params, |row| row.get(0))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_empty_watermark_schema_passes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = ActionDb::open_at_unencrypted(dir.path().join("doctor.sqlite")).expect("db");
        let report = inspect_watermarks(&db).expect("inspect");
        assert!(report.is_clean(), "unexpected report: {report:?}");
    }
}
