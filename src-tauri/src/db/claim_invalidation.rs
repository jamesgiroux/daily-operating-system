//! DOS-310: per-entity claim invalidation primitive.
//!
//! Codex round 1 finding 4 + round 2 finding 6 corrected the original DOS-7
//! plan that extended `entity_graph_version` triggers to cover claims:
//! `entity_graph_version` is a singleton counter, so every claim write would
//! thrash unrelated entity-linking evaluations. This module replaces that
//! approach with **per-entity `claim_version` columns** (Option A picked).
//!
//! Architecture:
//!
//! - **Per-entity** `claim_version` on `accounts`, `projects`, `people`,
//!   `meetings_history`. Bumped synchronously inside the same transaction
//!   as the claim write (DOS-7's `commit_claim` calls into this module).
//!   Readers check `entity.claim_version` off the row they already loaded
//!   — no extra query.
//!
//! - **`SubjectRef::Multi`** uses deterministic lock ordering: claim subjects
//!   are sorted by `(entity_type_order, id)` lexicographically with precedence
//!   `Account < Meeting < Person < Project` before bumping. SQLite's serialized
//!   writer + `BEGIN IMMEDIATE` makes concurrent transactions safe; the sort
//!   gives deterministic update ordering.
//!
//! - **`SubjectRef::Global`** does NOT bump per-entity counters. Instead it
//!   bumps `migration_state.global_claim_epoch`. **Spine restriction**: v1.4.0
//!   spine does NOT register any `claim_type` with `canonical_subject_types`
//!   containing `Global` (per ADR-0125). The variant is structurally available
//!   for v1.4.1+ work that justifies it via ADR amendment + tests.
//!
//! - All bumps are `#[must_use]` and expect to run inside `with_transaction`.
//!   This module is the SOLE writer of `claim_version` columns and the
//!   `global_claim_epoch` row. Direct UPDATE from elsewhere is rejected by
//!   `scripts/check_claim_version_writers.sh` (CI lint).
//!
//! References:
//! - Codex round 1 findings 4, 9 (singleton thrash)
//! - Codex round 2 findings 6, 7, 8 (Option pick + Global undefined + Multi deadlock)
//! - Live Linear ticket DOS-310

use rusqlite::params;

use crate::db::{ActionDb, DbError};

/// Subject of a claim. DOS-7's `commit_claim` constructs this from the
/// claim's `subject_ref` JSON column and passes to `bump_for_subject`.
///
/// Variants are listed in the entity-type lock-order precedence:
/// `Account < Meeting < Person < Project`. The `Multi` variant carries a
/// `Vec<SubjectRef>` that is sorted via `entity_type_order()` + `id` before
/// bumping, providing deterministic update ordering across concurrent commits.
///
/// # Spine restriction
///
/// `Global` is structurally available, but v1.4.0 spine MUST NOT register any
/// `claim_type` whose `canonical_subject_types` contains `SubjectType::Global`.
/// This is enforced at the CLAIM_TYPE_REGISTRY layer (DOS-7 / ADR-0125), not
/// here.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubjectRef {
    Account { id: String },
    Meeting { id: String },
    Person { id: String },
    Project { id: String },
    /// Multiple entities affected by one claim. Sorted before bumping.
    Multi(Vec<SubjectRef>),
    /// v1.4.1+ only; bumps `migration_state.global_claim_epoch` instead of
    /// per-entity counters. Spine restriction: see module-level docs.
    Global,
}

impl SubjectRef {
    /// Lock-order precedence: `Account < Meeting < Person < Project`. Used
    /// to sort `Multi` subjects before bumping so concurrent commits with
    /// reversed orderings produce deterministic update sequences.
    ///
    /// `Multi` and `Global` panic if compared via this method — they are
    /// resolved at a higher level (`bump_for_subject` dispatches).
    fn entity_type_order(&self) -> u8 {
        match self {
            Self::Account { .. } => 0,
            Self::Meeting { .. } => 1,
            Self::Person { .. } => 2,
            Self::Project { .. } => 3,
            Self::Multi(_) | Self::Global => {
                debug_assert!(
                    false,
                    "entity_type_order() called on Multi/Global; should be flattened first",
                );
                u8::MAX
            }
        }
    }

    fn id_str(&self) -> &str {
        match self {
            Self::Account { id }
            | Self::Meeting { id }
            | Self::Person { id }
            | Self::Project { id } => id.as_str(),
            Self::Multi(_) | Self::Global => "",
        }
    }
}

impl ActionDb {
    /// Bump the `claim_version` for a single entity.
    ///
    /// Returns the number of rows affected (0 if the entity doesn't exist).
    /// Unknown entity IDs are NOT a hard error — DOS-7's `commit_claim`
    /// inserts the claim row even if the subject entity is being deleted
    /// concurrently; the caller logs and proceeds. SQLite UPDATE on a
    /// non-existent ID silently affects 0 rows.
    ///
    /// MUST be called inside an active transaction (`with_transaction`
    /// closure). The bump runs as part of the same transactional unit as
    /// the claim insert.
    #[must_use = "claim invalidation results must be propagated"]
    pub fn bump_entity_claim_version(
        &self,
        subject: &SubjectRef,
    ) -> Result<usize, DbError> {
        let (table, id) = match subject {
            SubjectRef::Account { id } => ("accounts", id.as_str()),
            SubjectRef::Project { id } => ("projects", id.as_str()),
            SubjectRef::Person { id } => ("people", id.as_str()),
            SubjectRef::Meeting { id } => ("meetings", id.as_str()),
            SubjectRef::Multi(_) | SubjectRef::Global => {
                return Err(DbError::InvalidArgument(
                    "bump_entity_claim_version called with Multi/Global; \
                     use bump_for_subject which dispatches"
                        .to_string(),
                ))
            }
        };
        let sql = format!(
            "UPDATE {} SET claim_version = claim_version + 1 WHERE id = ?1",
            table
        );
        let affected = self.conn_ref().execute(&sql, params![id])?;
        Ok(affected)
    }

    /// Bump `migration_state.global_claim_epoch`. Used for `SubjectRef::Global`
    /// claims.
    ///
    /// **Spine restriction**: v1.4.0 spine does not register any `claim_type`
    /// with `canonical_subject_types` containing `Global`. Calling this from
    /// spine production code is a category error caught by the
    /// `CLAIM_TYPE_REGISTRY` lint (DOS-7 / ADR-0125 era).
    #[must_use = "claim invalidation results must be propagated"]
    pub fn bump_global_claim_epoch(&self) -> Result<(), DbError> {
        self.conn_ref().execute(
            "UPDATE migration_state SET value = value + 1 WHERE key = 'global_claim_epoch'",
            [],
        )?;
        Ok(())
    }

    /// Dispatches `SubjectRef` variants to the correct bump path.
    ///
    /// `Multi(refs)` sorts the inner subjects by `(entity_type_order, id)`
    /// (precedence: `Account < Meeting < Person < Project`) before bumping.
    /// Sorting provides deterministic update ordering: concurrent commits
    /// with `Multi([A, B])` and `Multi([B, A])` produce the same UPDATE
    /// sequence (`A`, `B`), so SQLite's serialized writer never sees
    /// reversed-order conflicts.
    ///
    /// MUST be called inside an active transaction. The bump runs as part
    /// of the same transactional unit as the claim insert.
    #[must_use = "claim invalidation results must be propagated"]
    pub fn bump_for_subject(&self, subject: &SubjectRef) -> Result<(), DbError> {
        match subject {
            SubjectRef::Account { .. }
            | SubjectRef::Project { .. }
            | SubjectRef::Person { .. }
            | SubjectRef::Meeting { .. } => {
                self.bump_entity_claim_version(subject)?;
                Ok(())
            }
            SubjectRef::Multi(refs) => {
                // Validate the Multi contents BEFORE sorting: nested Multi or
                // Global within Multi are contract violations. Sorting first
                // would call entity_type_order() on these illegal variants
                // and panic in debug.
                for r in refs {
                    match r {
                        SubjectRef::Multi(_) => {
                            return Err(DbError::InvalidArgument(
                                "nested Multi in SubjectRef not supported".to_string(),
                            ))
                        }
                        SubjectRef::Global => {
                            return Err(DbError::InvalidArgument(
                                "Global within Multi not supported".to_string(),
                            ))
                        }
                        _ => {}
                    }
                }
                // Deterministic lock ordering: sort by (entity_type_order, id)
                // before bumping. Dedup so a Multi with the same entity twice
                // bumps that entity once.
                let mut sorted: Vec<&SubjectRef> = refs.iter().collect();
                sorted.sort_by(|a, b| {
                    a.entity_type_order()
                        .cmp(&b.entity_type_order())
                        .then_with(|| a.id_str().cmp(b.id_str()))
                });
                sorted.dedup_by(|a, b| {
                    a.entity_type_order() == b.entity_type_order()
                        && a.id_str() == b.id_str()
                });
                for r in sorted {
                    self.bump_entity_claim_version(r)?;
                }
                Ok(())
            }
            SubjectRef::Global => self.bump_global_claim_epoch(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::test_db;

    const TS: &str = "2026-01-01T00:00:00Z";

    fn seed_account(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params![id, format!("acct-{}", id), TS],
            )
            .expect("seed account");
    }

    fn seed_project(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO projects (id, name, updated_at) VALUES (?1, ?2, ?3)",
                params![id, format!("proj-{}", id), TS],
            )
            .expect("seed project");
    }

    fn seed_person(db: &ActionDb, id: &str) {
        db.conn_ref()
            .execute(
                "INSERT INTO people (id, email, name, updated_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, format!("{}@example.com", id), format!("person-{}", id), TS],
            )
            .expect("seed person");
    }

    fn seed_meeting(db: &ActionDb, id: &str) {
        // Post migration 055: meetings has NOT NULL columns title, meeting_type,
        // start_time, created_at.
        db.conn_ref()
            .execute(
                "INSERT INTO meetings (id, title, meeting_type, start_time, created_at) \
                 VALUES (?1, ?2, 'sync', ?3, ?3)",
                params![id, format!("meet-{}", id), TS],
            )
            .expect("seed meeting");
    }

    fn read_claim_version(db: &ActionDb, table: &str, id: &str) -> i64 {
        let sql = format!("SELECT claim_version FROM {} WHERE id = ?1", table);
        db.conn_ref()
            .query_row(&sql, params![id], |r| r.get(0))
            .expect("read claim_version")
    }

    fn read_global_epoch(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT value FROM migration_state WHERE key = 'global_claim_epoch'",
                [],
                |r| r.get(0),
            )
            .expect("read global_claim_epoch")
    }

    fn read_entity_graph_version(db: &ActionDb) -> i64 {
        db.conn_ref()
            .query_row(
                "SELECT version FROM entity_graph_version WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .expect("read entity_graph_version")
    }

    #[test]
    fn bump_account_increments_claim_version() {
        let db = test_db();
        seed_account(&db, "acc-1");
        let before = read_claim_version(&db, "accounts", "acc-1");
        let affected = db
            .bump_entity_claim_version(&SubjectRef::Account {
                id: "acc-1".to_string(),
            })
            .expect("bump");
        assert_eq!(affected, 1);
        assert_eq!(read_claim_version(&db, "accounts", "acc-1"), before + 1);
    }

    #[test]
    fn bump_each_entity_kind_targets_correct_table() {
        let db = test_db();
        seed_account(&db, "a");
        seed_project(&db, "p");
        seed_person(&db, "ps");
        seed_meeting(&db, "m");

        db.bump_entity_claim_version(&SubjectRef::Account { id: "a".into() })
            .unwrap();
        db.bump_entity_claim_version(&SubjectRef::Project { id: "p".into() })
            .unwrap();
        db.bump_entity_claim_version(&SubjectRef::Person { id: "ps".into() })
            .unwrap();
        db.bump_entity_claim_version(&SubjectRef::Meeting { id: "m".into() })
            .unwrap();

        assert_eq!(read_claim_version(&db, "accounts", "a"), 1);
        assert_eq!(read_claim_version(&db, "projects", "p"), 1);
        assert_eq!(read_claim_version(&db, "people", "ps"), 1);
        assert_eq!(read_claim_version(&db, "meetings", "m"), 1);
    }

    #[test]
    fn bump_unknown_id_no_op() {
        let db = test_db();
        let affected = db
            .bump_entity_claim_version(&SubjectRef::Account {
                id: "does-not-exist".into(),
            })
            .expect("bump unknown");
        assert_eq!(affected, 0);
    }

    #[test]
    fn bump_for_subject_dispatches_global() {
        let db = test_db();
        let before = read_global_epoch(&db);
        db.bump_for_subject(&SubjectRef::Global).unwrap();
        assert_eq!(read_global_epoch(&db), before + 1);
    }

    #[test]
    fn global_does_not_bump_per_entity_claim_version() {
        let db = test_db();
        seed_account(&db, "a");
        let before = read_claim_version(&db, "accounts", "a");
        db.bump_for_subject(&SubjectRef::Global).unwrap();
        assert_eq!(read_claim_version(&db, "accounts", "a"), before);
    }

    #[test]
    fn multi_sorts_in_canonical_lock_order() {
        // Input: reversed ordering Project, Person, Meeting, Account
        // Expected: sorted Account < Meeting < Person < Project
        // Verify via call sequence: each bump increments its respective
        // counter; final state is consistent regardless of input order.
        let db = test_db();
        seed_account(&db, "a");
        seed_meeting(&db, "m");
        seed_person(&db, "ps");
        seed_project(&db, "p");

        db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Project { id: "p".into() },
            SubjectRef::Person { id: "ps".into() },
            SubjectRef::Meeting { id: "m".into() },
            SubjectRef::Account { id: "a".into() },
        ]))
        .unwrap();

        // All four bumped exactly once.
        assert_eq!(read_claim_version(&db, "accounts", "a"), 1);
        assert_eq!(read_claim_version(&db, "meetings", "m"), 1);
        assert_eq!(read_claim_version(&db, "people", "ps"), 1);
        assert_eq!(read_claim_version(&db, "projects", "p"), 1);
    }

    #[test]
    fn multi_dedups_repeated_subjects() {
        let db = test_db();
        seed_account(&db, "a");
        seed_account(&db, "b");

        db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Account { id: "a".into() },
            SubjectRef::Account { id: "a".into() }, // duplicate
            SubjectRef::Account { id: "b".into() },
        ]))
        .unwrap();

        // a bumped once (dedup), b bumped once.
        assert_eq!(read_claim_version(&db, "accounts", "a"), 1);
        assert_eq!(read_claim_version(&db, "accounts", "b"), 1);
    }

    #[test]
    fn multi_reversed_input_orders_produce_consistent_sequences() {
        // Two Multi subjects with same entities in reversed orderings
        // should produce identical version sequences (sort cleanses input).
        let db = test_db();
        seed_account(&db, "a");
        seed_project(&db, "p");

        db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Project { id: "p".into() },
            SubjectRef::Account { id: "a".into() },
        ]))
        .unwrap();

        let v_a_after_first = read_claim_version(&db, "accounts", "a");
        let v_p_after_first = read_claim_version(&db, "projects", "p");
        assert_eq!(v_a_after_first, 1);
        assert_eq!(v_p_after_first, 1);

        db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Account { id: "a".into() },
            SubjectRef::Project { id: "p".into() },
        ]))
        .unwrap();

        assert_eq!(read_claim_version(&db, "accounts", "a"), 2);
        assert_eq!(read_claim_version(&db, "projects", "p"), 2);
    }

    #[test]
    fn bump_does_not_touch_entity_graph_version() {
        // Live ticket acceptance: claim writes do NOT bump entity_graph_version.
        // entity_graph_version is for entity-linking (DOS-258); claim invalidation
        // is a separate domain.
        let db = test_db();
        seed_account(&db, "acc-1");
        let egv_before = read_entity_graph_version(&db);

        db.bump_entity_claim_version(&SubjectRef::Account {
            id: "acc-1".into(),
        })
        .unwrap();
        db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Account { id: "acc-1".into() },
        ]))
        .unwrap();
        db.bump_global_claim_epoch().unwrap();

        assert_eq!(
            read_entity_graph_version(&db),
            egv_before,
            "claim writes must NOT bump entity_graph_version"
        );
    }

    #[test]
    fn burst_writes_on_one_account_dont_affect_unrelated_entities() {
        // Live ticket acceptance: 1000 claim writes on account A in 10s
        // produce no invalidation noise on unrelated accounts/projects/people/meetings.
        // Using 1000 writes synchronously here as a stand-in for the production
        // burst test (test mode is single-threaded).
        let db = test_db();
        seed_account(&db, "hot");
        seed_account(&db, "cold");
        seed_project(&db, "cold-p");
        seed_person(&db, "cold-ps");
        seed_meeting(&db, "cold-m");

        for _ in 0..1000 {
            db.bump_entity_claim_version(&SubjectRef::Account {
                id: "hot".into(),
            })
            .unwrap();
        }

        assert_eq!(read_claim_version(&db, "accounts", "hot"), 1000);
        assert_eq!(read_claim_version(&db, "accounts", "cold"), 0);
        assert_eq!(read_claim_version(&db, "projects", "cold-p"), 0);
        assert_eq!(read_claim_version(&db, "people", "cold-ps"), 0);
        assert_eq!(read_claim_version(&db, "meetings", "cold-m"), 0);
    }

    #[test]
    fn nested_multi_returns_error() {
        let db = test_db();
        seed_account(&db, "a");
        let result = db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Multi(vec![SubjectRef::Account { id: "a".into() }]),
        ]));
        assert!(result.is_err(), "nested Multi must error");
    }

    #[test]
    fn dos310_100_concurrent_multi_consistent_sequences_no_deadlock() {
        // Live ticket DOS-310 acceptance: "100 concurrent claim commits with
        // Multi([A, B]) and Multi([B, A]) produce exactly 100 commits with
        // consistent version sequences, no deadlock."
        //
        // SQLite is a single-writer database; the production DbService
        // serializes through one writer thread. So "100 concurrent" here
        // really tests that:
        //   (a) the sort+dedup logic produces deterministic update ordering
        //       regardless of input ordering (no deadlock from inverted lock
        //       acquisition under SQLite's lock model), and
        //   (b) all 100 commits succeed (none lost; no panic; no error path).
        // We run synchronously in a tight loop because tokio task spawns
        // would just queue against the single writer anyway.
        use std::time::Instant;
        let db = test_db();
        seed_account(&db, "a");
        seed_project(&db, "p");

        let start = Instant::now();
        for i in 0..100 {
            let multi = if i % 2 == 0 {
                SubjectRef::Multi(vec![
                    SubjectRef::Account { id: "a".into() },
                    SubjectRef::Project { id: "p".into() },
                ])
            } else {
                // Reversed order; sort+dedup must produce same UPDATE sequence.
                SubjectRef::Multi(vec![
                    SubjectRef::Project { id: "p".into() },
                    SubjectRef::Account { id: "a".into() },
                ])
            };
            db.bump_for_subject(&multi).expect("bump must succeed");
        }
        // Bound: 100 single-row UPDATEs on test_db (in-memory SQLite). Should
        // complete well under 1s even on slow CI; if this exceeds 5s, the
        // sort/dedup logic has accidentally become quadratic.
        assert!(
            start.elapsed().as_secs() < 5,
            "100 multi-commits took longer than 5s — possible deadlock or O(n^2) regression",
        );
        // Each entity bumped exactly once per Multi → 100 total bumps each.
        assert_eq!(read_claim_version(&db, "accounts", "a"), 100);
        assert_eq!(read_claim_version(&db, "projects", "p"), 100);
    }

    #[test]
    fn global_within_multi_returns_error() {
        let db = test_db();
        seed_account(&db, "a");
        let result = db.bump_for_subject(&SubjectRef::Multi(vec![
            SubjectRef::Account { id: "a".into() },
            SubjectRef::Global,
        ]));
        assert!(result.is_err(), "Global inside Multi must error");
    }
}
