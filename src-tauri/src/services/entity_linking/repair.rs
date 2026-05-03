use chrono::Utc;
use rusqlite::params;
use serde_json::json;
use uuid::Uuid;

use crate::db::ActionDb;

const REPAIR_SOURCE: &str = "repair:dos345";

#[derive(Debug, Clone)]
pub struct RepairOptions {
    pub min_batch_size: i64,
    pub min_coattendees: i64,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self {
            min_batch_size: 3,
            min_coattendees: 3,
        }
    }
}

impl RepairOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.min_batch_size < 2 {
            return Err("--min-batch must be at least 2".to_string());
        }
        if self.min_coattendees < 2 {
            return Err("--min-coattendees must be at least 2".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct RepairReport {
    pub applied: bool,
    pub repair_id: Option<String>,
    pub candidate_stakeholder_rows: i64,
    pub candidate_batches: i64,
    pub touched_accounts: i64,
    pub touched_people: i64,
    pub max_batch_size: i64,
    pub max_coattendees: i64,
    pub multi_account_people: i64,
    pub roles_to_dismiss: i64,
    pub unsupported_inferred_domains: i64,
    pub auto_meeting_links_to_clear: i64,
    pub affected_meetings: i64,
    pub ledger_items: i64,
}

impl RepairReport {
    pub fn has_changes(&self) -> bool {
        self.candidate_stakeholder_rows > 0
            || self.unsupported_inferred_domains > 0
            || self.auto_meeting_links_to_clear > 0
    }

    pub fn to_operator_summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!(
            "repair_id={}",
            self.repair_id.as_deref().unwrap_or("none")
        ));
        lines.push(format!(
            "mode={}",
            if self.applied { "apply" } else { "dry-run" }
        ));
        lines.push(format!(
            "candidate_stakeholder_rows={}",
            self.candidate_stakeholder_rows
        ));
        lines.push(format!("candidate_batches={}", self.candidate_batches));
        lines.push(format!("touched_accounts={}", self.touched_accounts));
        lines.push(format!("touched_people={}", self.touched_people));
        lines.push(format!("max_batch_size={}", self.max_batch_size));
        lines.push(format!("max_coattendees={}", self.max_coattendees));
        lines.push(format!(
            "multi_account_people={}",
            self.multi_account_people
        ));
        lines.push(format!("roles_to_dismiss={}", self.roles_to_dismiss));
        lines.push(format!(
            "unsupported_inferred_domains={}",
            self.unsupported_inferred_domains
        ));
        lines.push(format!(
            "auto_meeting_links_to_clear={}",
            self.auto_meeting_links_to_clear
        ));
        lines.push(format!("affected_meetings={}", self.affected_meetings));
        lines.push(format!("ledger_items={}", self.ledger_items));
        lines.join("\n")
    }
}

pub fn build_report(db: &ActionDb, opts: &RepairOptions) -> Result<RepairReport, String> {
    opts.validate()?;
    prepare_temp_tables(db, opts)?;
    gather_report(db, false, None, 0)
}

pub fn apply_repair(db: &ActionDb, opts: &RepairOptions) -> Result<RepairReport, String> {
    opts.validate()?;
    db.with_transaction(|tx| {
        prepare_temp_tables(tx, opts)?;
        let preview = gather_report(tx, false, None, 0)?;
        if !preview.has_changes() {
            return Ok(RepairReport {
                applied: true,
                ..preview
            });
        }

        ensure_ledger(tx)?;
        let repair_id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let ledger_items = write_ledger(tx, &repair_id, &now)?;

        tx.conn_ref()
            .execute(
                "UPDATE account_stakeholders \
                 SET status = 'dismissed', data_source = ?1, confidence = 0.0 \
                 WHERE EXISTS ( \
                   SELECT 1 FROM dos345_candidates c \
                   WHERE c.account_id = account_stakeholders.account_id \
                     AND c.person_id = account_stakeholders.person_id \
                 )",
                params![REPAIR_SOURCE],
            )
            .map_err(|e| format!("quarantine stakeholders: {e}"))?;

        let affected_accounts = repair_candidate_account_ids(tx)?;
        let clock = crate::services::context::SystemClock;
        let rng = crate::services::context::SystemRng;
        let ext = crate::services::context::ExternalClients::default();
        let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &ext);
        for account_id in affected_accounts {
            crate::services::derived_state::rebuild_stakeholder_insights_cache_for_entity(
                &ctx,
                tx,
                &account_id,
                "account",
            )
            .map_err(|e| format!("stakeholder cache rebuild failed: {}", e.as_str()))?;
        }

        // L2 cycle-22 fix: snapshot the (person_id, role) tuples
        // we're about to dismiss BEFORE the UPDATE so we can write
        // matching shadow tombstone claims after. The repair sets
        // dismissed_at on legacy rows but the substrate also needs
        // matching m2 stakeholder_role tombstones, otherwise
        // commit_claim PRE-GATE misses the dismissal and a future
        // enrichment can re-surface the role despite the operator
        // repair.
        let dismissed_role_subjects: Vec<(String, String)> = {
            let mut stmt = tx
                .conn_ref()
                .prepare(
                    "SELECT person_id, role FROM account_stakeholder_roles \
                     WHERE role = 'associated' \
                       AND data_source = 'ai' \
                       AND dismissed_at IS NULL \
                       AND EXISTS ( \
                         SELECT 1 FROM dos345_candidates c \
                         WHERE c.account_id = account_stakeholder_roles.account_id \
                           AND c.person_id = account_stakeholder_roles.person_id \
                       )",
                )
                .map_err(|e| format!("snapshot dismissed roles prepare: {e}"))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(|e| format!("snapshot dismissed roles query: {e}"))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        tx.conn_ref()
            .execute(
                "UPDATE account_stakeholder_roles \
                 SET data_source = ?1, dismissed_at = ?2 \
                 WHERE role = 'associated' \
                   AND data_source = 'ai' \
                   AND dismissed_at IS NULL \
                   AND EXISTS ( \
                     SELECT 1 FROM dos345_candidates c \
                     WHERE c.account_id = account_stakeholder_roles.account_id \
                       AND c.person_id = account_stakeholder_roles.person_id \
                   )",
                params![REPAIR_SOURCE, now],
            )
            .map_err(|e| format!("dismiss stakeholder roles: {e}"))?;

        // L2 cycle-22 fix: shadow-write a stakeholder_role tombstone
        // claim for each dismissed (person_id, role) pair, in the
        // same transaction. Failures are tolerated (best-effort
        // claim write); the legacy DB write is authoritative for
        // this repair path until the substrate becomes the primary
        // read surface.
        for (person_id, role) in &dismissed_role_subjects {
            let _ = crate::services::claims::shadow_write_tombstone_claim(
                tx,
                crate::services::claims::ShadowTombstoneClaim {
                    subject_kind: "Person",
                    subject_id: person_id,
                    claim_type: "stakeholder_role",
                    field_path: None,
                    text: role,
                    actor: "system",
                    source_scope: Some("repair:dos345"),
                    observed_at: &now,
                    expires_at: None,
                },
            );
        }

        tx.conn_ref()
            .execute(
                "DELETE FROM account_domains \
                 WHERE source = 'inferred' \
                   AND EXISTS ( \
                     SELECT 1 FROM dos345_domains d \
                     WHERE d.account_id = account_domains.account_id \
                       AND lower(d.domain) = lower(account_domains.domain) \
                   )",
                [],
            )
            .map_err(|e| format!("delete unsupported domains: {e}"))?;

        tx.conn_ref()
            .execute(
                "DELETE FROM linked_entities_raw \
                 WHERE EXISTS ( \
                   SELECT 1 FROM dos345_links l \
                   WHERE l.owner_type = linked_entities_raw.owner_type \
                     AND l.owner_id = linked_entities_raw.owner_id \
                     AND l.entity_id = linked_entities_raw.entity_id \
                     AND l.entity_type = linked_entities_raw.entity_type \
                 )",
                [],
            )
            .map_err(|e| format!("clear stale links: {e}"))?;

        Ok(RepairReport {
            applied: true,
            repair_id: Some(repair_id),
            ledger_items,
            ..preview
        })
    })
}

fn repair_candidate_account_ids(db: &ActionDb) -> Result<Vec<String>, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT DISTINCT account_id
             FROM dos345_candidates
             ORDER BY account_id",
        )
        .map_err(|e| format!("prepare repair affected accounts: {e}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query repair affected accounts: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("read repair affected account: {e}"))
}

fn prepare_temp_tables(db: &ActionDb, opts: &RepairOptions) -> Result<(), String> {
    let conn = db.conn_ref();
    conn.execute_batch(
        "DROP TABLE IF EXISTS dos345_candidates;
         DROP TABLE IF EXISTS dos345_domains;
         DROP TABLE IF EXISTS dos345_links;",
    )
    .map_err(|e| format!("drop temp repair tables: {e}"))?;

    conn.execute(
        "CREATE TEMP TABLE dos345_candidates AS
         WITH base AS (
             SELECT
                 s.account_id,
                 s.person_id,
                 s.created_at,
                 lower(substr(p.email, instr(p.email, '@') + 1)) AS person_domain
             FROM account_stakeholders s
             JOIN accounts a ON a.id = s.account_id
             JOIN people p ON p.id = s.person_id
             WHERE s.status = 'active'
               AND s.data_source = 'user'
               AND s.confidence IS NULL
               AND COALESCE(a.archived, 0) = 0
               AND p.email IS NOT NULL
               AND instr(p.email, '@') > 0
               AND EXISTS (
                   SELECT 1 FROM account_stakeholder_roles r
                   WHERE r.account_id = s.account_id
                     AND r.person_id = s.person_id
                     AND r.role = 'associated'
                     AND r.data_source = 'ai'
                     AND r.dismissed_at IS NULL
               )
               AND NOT EXISTS (
                   SELECT 1 FROM account_stakeholder_roles r
                   WHERE r.account_id = s.account_id
                     AND r.person_id = s.person_id
                     AND r.role <> 'associated'
                     AND r.dismissed_at IS NULL
               )
         ),
         batched AS (
             SELECT
                 base.*,
                 COUNT(*) OVER (PARTITION BY account_id, created_at) AS batch_size
             FROM base
         ),
         batch_meetings AS (
             SELECT
                 b.account_id,
                 b.created_at,
                 ma.meeting_id,
                 COUNT(DISTINCT b.person_id) AS coattendee_count
             FROM batched b
             JOIN meeting_attendees ma ON ma.person_id = b.person_id
             WHERE b.batch_size >= ?1
             GROUP BY b.account_id, b.created_at, ma.meeting_id
         ),
         qualified_batches AS (
             SELECT
                 account_id,
                 created_at,
                 MAX(coattendee_count) AS max_coattendees
             FROM batch_meetings
             WHERE coattendee_count >= ?2
             GROUP BY account_id, created_at
         )
         SELECT
             b.account_id,
             b.person_id,
             b.created_at,
             b.person_domain,
             b.batch_size,
             q.max_coattendees
         FROM batched b
         JOIN qualified_batches q
           ON q.account_id = b.account_id
          AND q.created_at = b.created_at
         WHERE b.batch_size >= ?1",
        params![opts.min_batch_size, opts.min_coattendees],
    )
    .map_err(|e| format!("build candidate table: {e}"))?;

    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_dos345_candidates_pair
             ON dos345_candidates(account_id, person_id);
         CREATE INDEX IF NOT EXISTS idx_dos345_candidates_person
             ON dos345_candidates(person_id);",
    )
    .map_err(|e| format!("index candidate table: {e}"))?;

    conn.execute(
        "CREATE TEMP TABLE dos345_domains AS
         SELECT d.account_id, d.domain, d.source
         FROM account_domains d
         WHERE d.source = 'inferred'
           AND d.account_id IN (SELECT DISTINCT account_id FROM dos345_candidates)
           AND NOT EXISTS (
               SELECT 1
               FROM account_stakeholders s
               JOIN people p ON p.id = s.person_id
               WHERE s.account_id = d.account_id
                 AND s.status = 'active'
                 AND instr(p.email, '@') > 0
                 AND lower(substr(p.email, instr(p.email, '@') + 1)) = lower(d.domain)
                 AND NOT EXISTS (
                     SELECT 1 FROM dos345_candidates c
                     WHERE c.account_id = s.account_id
                       AND c.person_id = s.person_id
                 )
           )",
        [],
    )
    .map_err(|e| format!("build domain table: {e}"))?;

    conn.execute(
        "CREATE TEMP TABLE dos345_links AS
         SELECT
             ler.owner_type,
             ler.owner_id,
             ler.entity_id,
             ler.entity_type,
             ler.role,
             ler.source,
             ler.rule_id,
             ler.confidence,
             ler.evidence_json,
             ler.graph_version,
             ler.created_at
         FROM linked_entities_raw ler
         WHERE ler.owner_type = 'meeting'
           AND ler.entity_type = 'account'
           AND ler.source NOT IN ('user', 'user_dismissed')
           AND EXISTS (
               SELECT 1 FROM dos345_candidates c
               JOIN meeting_attendees ma ON ma.person_id = c.person_id
               WHERE c.account_id = ler.entity_id
                 AND ma.meeting_id = ler.owner_id
           )",
        [],
    )
    .map_err(|e| format!("build link table: {e}"))?;

    Ok(())
}

fn gather_report(
    db: &ActionDb,
    applied: bool,
    repair_id: Option<String>,
    ledger_items: i64,
) -> Result<RepairReport, String> {
    let conn = db.conn_ref();
    let (
        candidate_stakeholder_rows,
        touched_accounts,
        touched_people,
        candidate_batches,
        max_batch_size,
        max_coattendees,
    ): (i64, i64, i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                COUNT(*),
                COUNT(DISTINCT account_id),
                COUNT(DISTINCT person_id),
                COUNT(DISTINCT account_id || char(31) || created_at),
                COALESCE(MAX(batch_size), 0),
                COALESCE(MAX(max_coattendees), 0)
             FROM dos345_candidates",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .map_err(|e| format!("candidate summary: {e}"))?;

    let roles_to_dismiss = count_one(
        conn,
        "SELECT COUNT(*)
         FROM account_stakeholder_roles r
         WHERE r.role = 'associated'
           AND r.data_source = 'ai'
           AND r.dismissed_at IS NULL
           AND EXISTS (
               SELECT 1 FROM dos345_candidates c
               WHERE c.account_id = r.account_id
                 AND c.person_id = r.person_id
           )",
    )?;
    let unsupported_inferred_domains = count_one(conn, "SELECT COUNT(*) FROM dos345_domains")?;
    let auto_meeting_links_to_clear = count_one(conn, "SELECT COUNT(*) FROM dos345_links")?;
    let affected_meetings = count_one(
        conn,
        "SELECT COUNT(DISTINCT owner_id) FROM dos345_links WHERE owner_type = 'meeting'",
    )?;
    let multi_account_people = count_one(
        conn,
        "SELECT COUNT(*) FROM (
             SELECT c.person_id
             FROM dos345_candidates c
             JOIN account_stakeholders s
               ON s.person_id = c.person_id
              AND s.status = 'active'
             GROUP BY c.person_id
             HAVING COUNT(DISTINCT s.account_id) > 1
         )",
    )?;

    Ok(RepairReport {
        applied,
        repair_id,
        candidate_stakeholder_rows,
        candidate_batches,
        touched_accounts,
        touched_people,
        max_batch_size,
        max_coattendees,
        multi_account_people,
        roles_to_dismiss,
        unsupported_inferred_domains,
        auto_meeting_links_to_clear,
        affected_meetings,
        ledger_items,
    })
}

fn count_one(conn: &rusqlite::Connection, sql: &str) -> Result<i64, String> {
    conn.query_row(sql, [], |row| row.get(0))
        .map_err(|e| format!("count query failed: {e}"))
}

fn ensure_ledger(db: &ActionDb) -> Result<(), String> {
    db.conn_ref()
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS entity_linking_repair_ledger (
                id INTEGER PRIMARY KEY,
                repair_id TEXT NOT NULL,
                item_type TEXT NOT NULL,
                item_key TEXT NOT NULL,
                before_json TEXT NOT NULL,
                created_at TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_entity_linking_repair_ledger_repair
                ON entity_linking_repair_ledger(repair_id);",
        )
        .map_err(|e| format!("ensure repair ledger: {e}"))
}

fn write_ledger(db: &ActionDb, repair_id: &str, now: &str) -> Result<i64, String> {
    let mut count = 0i64;
    count += write_stakeholder_ledger(db, repair_id, now)?;
    count += write_role_ledger(db, repair_id, now)?;
    count += write_domain_ledger(db, repair_id, now)?;
    count += write_link_ledger(db, repair_id, now)?;
    Ok(count)
}

fn insert_ledger_item(
    db: &ActionDb,
    repair_id: &str,
    item_type: &str,
    item_key: &str,
    before_json: &str,
    now: &str,
) -> Result<(), String> {
    db.conn_ref()
        .execute(
            "INSERT INTO entity_linking_repair_ledger
             (repair_id, item_type, item_key, before_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![repair_id, item_type, item_key, before_json, now],
        )
        .map(|_| ())
        .map_err(|e| format!("insert repair ledger item: {e}"))
}

fn write_stakeholder_ledger(db: &ActionDb, repair_id: &str, now: &str) -> Result<i64, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT s.account_id, s.person_id, s.data_source, s.status,
                    s.confidence, s.created_at
             FROM account_stakeholders s
             JOIN dos345_candidates c
               ON c.account_id = s.account_id
              AND c.person_id = s.person_id",
        )
        .map_err(|e| format!("prepare stakeholder ledger: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let account_id: String = row.get(0)?;
            let person_id: String = row.get(1)?;
            let data_source: String = row.get(2)?;
            let status: String = row.get(3)?;
            let confidence: Option<f64> = row.get(4)?;
            let created_at: String = row.get(5)?;
            let before = json!({
                "account_id": account_id,
                "person_id": person_id,
                "data_source": data_source,
                "status": status,
                "confidence": confidence,
                "created_at": created_at,
            });
            Ok((format!("{account_id}:{person_id}"), before.to_string()))
        })
        .map_err(|e| format!("query stakeholder ledger: {e}"))?;

    let mut count = 0;
    for row in rows {
        let (key, before_json) = row.map_err(|e| format!("stakeholder ledger row: {e}"))?;
        insert_ledger_item(
            db,
            repair_id,
            "account_stakeholder",
            &key,
            &before_json,
            now,
        )?;
        count += 1;
    }
    Ok(count)
}

fn write_role_ledger(db: &ActionDb, repair_id: &str, now: &str) -> Result<i64, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT r.account_id, r.person_id, r.role, r.data_source,
                    r.created_at, r.dismissed_at
             FROM account_stakeholder_roles r
             JOIN dos345_candidates c
               ON c.account_id = r.account_id
              AND c.person_id = r.person_id
             WHERE r.role = 'associated'
               AND r.data_source = 'ai'
               AND r.dismissed_at IS NULL",
        )
        .map_err(|e| format!("prepare role ledger: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let account_id: String = row.get(0)?;
            let person_id: String = row.get(1)?;
            let role: String = row.get(2)?;
            let data_source: String = row.get(3)?;
            let created_at: String = row.get(4)?;
            let dismissed_at: Option<String> = row.get(5)?;
            let before = json!({
                "account_id": account_id,
                "person_id": person_id,
                "role": role,
                "data_source": data_source,
                "created_at": created_at,
                "dismissed_at": dismissed_at,
            });
            Ok((
                format!("{account_id}:{person_id}:{role}"),
                before.to_string(),
            ))
        })
        .map_err(|e| format!("query role ledger: {e}"))?;

    let mut count = 0;
    for row in rows {
        let (key, before_json) = row.map_err(|e| format!("role ledger row: {e}"))?;
        insert_ledger_item(
            db,
            repair_id,
            "account_stakeholder_role",
            &key,
            &before_json,
            now,
        )?;
        count += 1;
    }
    Ok(count)
}

fn write_domain_ledger(db: &ActionDb, repair_id: &str, now: &str) -> Result<i64, String> {
    let mut stmt = db
        .conn_ref()
        .prepare("SELECT account_id, domain, source FROM dos345_domains")
        .map_err(|e| format!("prepare domain ledger: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let account_id: String = row.get(0)?;
            let domain: String = row.get(1)?;
            let source: String = row.get(2)?;
            let before = json!({
                "account_id": account_id,
                "domain": domain,
                "source": source,
            });
            Ok((format!("{account_id}:{domain}"), before.to_string()))
        })
        .map_err(|e| format!("query domain ledger: {e}"))?;

    let mut count = 0;
    for row in rows {
        let (key, before_json) = row.map_err(|e| format!("domain ledger row: {e}"))?;
        insert_ledger_item(db, repair_id, "account_domain", &key, &before_json, now)?;
        count += 1;
    }
    Ok(count)
}

fn write_link_ledger(db: &ActionDb, repair_id: &str, now: &str) -> Result<i64, String> {
    let mut stmt = db
        .conn_ref()
        .prepare(
            "SELECT owner_type, owner_id, entity_id, entity_type, role, source,
                    rule_id, confidence, evidence_json, graph_version, created_at
             FROM dos345_links",
        )
        .map_err(|e| format!("prepare link ledger: {e}"))?;
    let rows = stmt
        .query_map([], |row| {
            let owner_type: String = row.get(0)?;
            let owner_id: String = row.get(1)?;
            let entity_id: String = row.get(2)?;
            let entity_type: String = row.get(3)?;
            let role: String = row.get(4)?;
            let source: String = row.get(5)?;
            let rule_id: Option<String> = row.get(6)?;
            let confidence: Option<f64> = row.get(7)?;
            let evidence_json: Option<String> = row.get(8)?;
            let graph_version: i64 = row.get(9)?;
            let created_at: String = row.get(10)?;
            let before = json!({
                "owner_type": owner_type,
                "owner_id": owner_id,
                "entity_id": entity_id,
                "entity_type": entity_type,
                "role": role,
                "source": source,
                "rule_id": rule_id,
                "confidence": confidence,
                "evidence_json": evidence_json,
                "graph_version": graph_version,
                "created_at": created_at,
            });
            Ok((
                format!("{owner_type}:{owner_id}:{entity_id}:{entity_type}"),
                before.to_string(),
            ))
        })
        .map_err(|e| format!("query link ledger: {e}"))?;

    let mut count = 0;
    for row in rows {
        let (key, before_json) = row.map_err(|e| format!("link ledger row: {e}"))?;
        insert_ledger_item(db, repair_id, "linked_entity_raw", &key, &before_json, now)?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    fn seed_account(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived)
                 VALUES (?1, ?1, '2026-01-01', 0)",
                params![id],
            )
            .expect("insert account");
    }

    fn seed_person(db: &ActionDb, id: &str, email: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, name, email, relationship, updated_at)
                 VALUES (?1, ?1, ?2, 'external', '2026-01-01')",
                params![id, email],
            )
            .expect("insert person");
    }

    fn seed_meeting(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at)
                 VALUES (?1, 'Working Session', 'external', '2026-01-01T10:00:00Z', '2026-01-01')",
                params![id],
            )
            .expect("insert meeting");
    }

    fn seed_candidate_pair(db: &ActionDb, account_id: &str, person_id: &str, created_at: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholders
                 (account_id, person_id, data_source, status, confidence, created_at)
                 VALUES (?1, ?2, 'user', 'active', NULL, ?3)",
                params![account_id, person_id, created_at],
            )
            .expect("insert stakeholder");
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles
                 (account_id, person_id, role, data_source, created_at)
                 VALUES (?1, ?2, 'associated', 'ai', ?3)",
                params![account_id, person_id, created_at],
            )
            .expect("insert role");
    }

    fn seed_fixture() -> ActionDb {
        let db = test_db();
        seed_account(&db, "acc-a");
        seed_person(&db, "person-1", "one@example-a.test");
        seed_person(&db, "person-2", "two@example-a.test");
        seed_person(&db, "person-3", "three@example-a.test");
        seed_meeting(&db, "meeting-1");
        for person_id in ["person-1", "person-2", "person-3"] {
            seed_candidate_pair(&db, "acc-a", person_id, "2026-01-01 12:00:00");
            db.conn_ref()
                .execute(
                    "INSERT INTO meeting_attendees (meeting_id, person_id) VALUES ('meeting-1', ?1)",
                    params![person_id],
                )
                .expect("insert attendee");
        }
        db.conn_ref()
            .execute(
                "INSERT INTO account_domains (account_id, domain, source)
                 VALUES ('acc-a', 'example-a.test', 'inferred')",
                [],
            )
            .expect("insert domain");
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw
                 (owner_type, owner_id, entity_id, entity_type, role, source,
                  rule_id, confidence, graph_version, created_at)
                 VALUES ('meeting', 'meeting-1', 'acc-a', 'account', 'primary',
                         'rule:P4a', 'P4a', 0.93, 0, '2026-01-01')",
                [],
            )
            .expect("insert link");
        db
    }

    #[test]
    fn dry_run_detects_batch_contamination_signature() {
        let db = seed_fixture();
        let report = build_report(&db, &RepairOptions::default()).expect("report");

        assert_eq!(report.candidate_stakeholder_rows, 3);
        assert_eq!(report.candidate_batches, 1);
        assert_eq!(report.touched_accounts, 1);
        assert_eq!(report.touched_people, 3);
        assert_eq!(report.max_batch_size, 3);
        assert_eq!(report.max_coattendees, 3);
        assert_eq!(report.roles_to_dismiss, 3);
        assert_eq!(report.unsupported_inferred_domains, 1);
        assert_eq!(report.auto_meeting_links_to_clear, 1);
        assert_eq!(report.affected_meetings, 1);
    }

    #[test]
    fn apply_quarantines_rows_and_writes_ledger() {
        let db = seed_fixture();
        let report = apply_repair(&db, &RepairOptions::default()).expect("apply");
        assert!(report.applied);
        assert_eq!(report.ledger_items, 8);

        let active_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholders WHERE status = 'active'",
                [],
                |row| row.get(0),
            )
            .expect("active count");
        assert_eq!(active_count, 0);

        let role_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM account_stakeholder_roles
                 WHERE data_source = ?1 AND dismissed_at IS NOT NULL",
                params![REPAIR_SOURCE],
                |row| row.get(0),
            )
            .expect("role count");
        assert_eq!(role_count, 3);

        let domain_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM account_domains", [], |row| row.get(0))
            .expect("domain count");
        assert_eq!(domain_count, 0);

        let link_count: i64 = db
            .conn_ref()
            .query_row("SELECT COUNT(*) FROM linked_entities_raw", [], |row| {
                row.get(0)
            })
            .expect("link count");
        assert_eq!(link_count, 0);

        let ledger_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM entity_linking_repair_ledger",
                [],
                |row| row.get(0),
            )
            .expect("ledger count");
        assert_eq!(ledger_count, 8);
    }

    #[test]
    fn dry_run_skips_rows_with_non_default_roles() {
        let db = seed_fixture();
        db.conn_ref()
            .execute(
                "INSERT INTO account_stakeholder_roles
                 (account_id, person_id, role, data_source, created_at)
                 VALUES ('acc-a', 'person-1', 'champion', 'user', '2026-01-01')",
                [],
            )
            .expect("insert non-default role");

        let report = build_report(&db, &RepairOptions::default()).expect("report");
        assert_eq!(report.candidate_stakeholder_rows, 0);
    }

    #[test]
    fn apply_preserves_user_override_links() {
        let db = seed_fixture();
        db.conn_ref()
            .execute(
                "INSERT INTO linked_entities_raw
                 (owner_type, owner_id, entity_id, entity_type, role, source,
                  rule_id, confidence, graph_version, created_at)
                 VALUES ('meeting', 'meeting-user', 'acc-a', 'account', 'primary',
                         'user', 'P1', 1.0, 0, '2026-01-01')",
                [],
            )
            .expect("insert user link");

        apply_repair(&db, &RepairOptions::default()).expect("apply");
        let user_link_count: i64 = db
            .conn_ref()
            .query_row(
                "SELECT COUNT(*) FROM linked_entities_raw
                 WHERE owner_id = 'meeting-user' AND source = 'user'",
                [],
                |row| row.get(0),
            )
            .expect("user link count");
        assert_eq!(user_link_count, 1);
    }
}
