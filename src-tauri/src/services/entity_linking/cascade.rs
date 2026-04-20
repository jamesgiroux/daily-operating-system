//! Phase 4 cascade — C1 through C5.

use crate::db::ActionDb;

use super::types::{Candidate, EntityRef, LinkTier, LinkOutcome, LinkingContext};

/// Run all cascade steps and return the final LinkOutcome.
pub fn run_cascade(
    ctx: &LinkingContext,
    primary_candidate: &Option<Candidate>,
    phase3_related: &[Candidate],
    db: &ActionDb,
) -> Result<LinkOutcome, String> {
    let mut related: Vec<EntityRef> = phase3_related
        .iter()
        .map(|c| c.entity.clone())
        .collect();

    let primary_entity = primary_candidate
        .as_ref()
        .map(|c| c.entity.clone());

    // C1 — Related chips already written to DB in phases.rs; just collect for outcome.

    // C2 — Stakeholder review queue (sole post-migration writer to account_stakeholders).
    if let Some(ref primary) = primary_entity {
        if primary.entity_type == "account" {
            let auto_resolved = primary_candidate
                .as_ref()
                .map(|c| !matches!(c.rule_id.as_str(), "P1" | "P3"))
                .unwrap_or(false);

            if auto_resolved {
                c2_suggest_stakeholders(ctx, &primary.entity_id, db);
            } else {
                // C3 — User-set primary: promote domain-matching attendees directly.
                c3_promote_trusted_stakeholders(ctx, &primary.entity_id, db);
            }
        }
    }

    // C4 — If primary is a project, surface its account as related.
    if let Some(ref primary) = primary_entity {
        if primary.entity_type == "project" {
            if let Some(account_id) = get_project_account_id(db, &primary.entity_id) {
                let account_ref = EntityRef {
                    entity_id: account_id.clone(),
                    entity_type: "account".to_string(),
                };
                if !related.iter().any(|r| r.entity_id == account_id) {
                    related.push(account_ref);
                    // Write to DB
                    let _ = db.upsert_linked_entity_raw(
                        &crate::db::entity_linking::LinkedEntityRawWrite {
                            owner_type: ctx.owner.owner_type.as_str().to_string(),
                            owner_id: ctx.owner.owner_id.clone(),
                            entity_id: account_id,
                            entity_type: "account".to_string(),
                            role: "related".to_string(),
                            source: "rule:C4".to_string(),
                            rule_id: Some("C4".to_string()),
                            confidence: None,
                            evidence_json: None,
                            graph_version: ctx.graph_version,
                        },
                    );
                }
            }
        }
    }

    // C5 — Tier mapping.
    let tier = c5_tier(ctx, &primary_entity);

    let applied_rule = primary_candidate
        .as_ref()
        .map(|c| c.rule_id.clone());

    Ok(LinkOutcome {
        owner: ctx.owner.clone(),
        primary: primary_entity,
        related,
        tier,
        applied_rule,
    })
}

// ---------------------------------------------------------------------------
// C2 helpers
// ---------------------------------------------------------------------------

fn c2_suggest_stakeholders(ctx: &LinkingContext, account_id: &str, db: &ActionDb) {
    // Get account domains to identify domain-matching external participants.
    let account_domains = get_account_domains(db, account_id);

    for p in ctx.external_participants() {
        let p_domain = p
            .email
            .rsplit_once('@')
            .map(|(_, d)| d.to_lowercase());

        let domain_matches = p_domain
            .as_deref()
            .map(|d| account_domains.iter().any(|ad| ad.eq_ignore_ascii_case(d)))
            .unwrap_or(false);

        if !domain_matches {
            continue;
        }

        let person_id = match &p.person_id {
            Some(id) => id,
            None => continue,
        };

        // Skip if already an active/pending stakeholder on this account.
        let already = db
            .is_stakeholder_on_account(account_id, person_id)
            .unwrap_or(false);
        if already {
            continue;
        }

        let data_source = match ctx.owner.owner_type {
            super::types::OwnerType::Meeting => "calendar_attendance",
            super::types::OwnerType::Email | super::types::OwnerType::EmailThread => {
                "email_correspondence"
            }
        };

        let _ = db.suggest_stakeholder_pending(account_id, person_id, data_source, 0.75);
    }
}

fn c3_promote_trusted_stakeholders(ctx: &LinkingContext, account_id: &str, db: &ActionDb) {
    // User explicitly chose this account — domain-matching attendees become active stakeholders.
    let account_domains = get_account_domains(db, account_id);

    for p in ctx.external_participants() {
        let domain = p
            .email
            .rsplit_once('@')
            .map(|(_, d)| d.to_lowercase());
        let domain_matches = domain
            .as_deref()
            .map(|d| account_domains.iter().any(|ad| ad.eq_ignore_ascii_case(d)))
            .unwrap_or(false);

        if !domain_matches {
            continue;
        }

        let person_id = match &p.person_id {
            Some(id) => id,
            None => continue,
        };

        let already = db
            .is_stakeholder_on_account(account_id, person_id)
            .unwrap_or(false);
        if already {
            continue;
        }

        // Insert directly as active (trusted because user chose the account).
        let _ = db.suggest_stakeholder_pending(account_id, person_id, "user_set_primary", 1.0);
        let _ = db.confirm_stakeholder(account_id, person_id);
    }
}

// ---------------------------------------------------------------------------
// C5 — Tier mapping
// ---------------------------------------------------------------------------

fn c5_tier(ctx: &LinkingContext, primary: &Option<EntityRef>) -> LinkTier {
    match primary {
        None => {
            if ctx.is_one_on_one() {
                LinkTier::Person
            } else {
                LinkTier::Minimal
            }
        }
        Some(e) => match e.entity_type.as_str() {
            "account" => LinkTier::Entity,
            "person" => LinkTier::Person,
            "project" => LinkTier::Entity,
            _ => LinkTier::Minimal,
        },
    }
}

// ---------------------------------------------------------------------------
// DB helpers (private)
// ---------------------------------------------------------------------------

fn get_account_domains(db: &ActionDb, account_id: &str) -> Vec<String> {
    let mut stmt = match db
        .conn_ref()
        .prepare("SELECT domain FROM account_domains WHERE account_id = ?1")
    {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map(rusqlite::params![account_id], |row| {
        row.get::<_, String>(0)
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn get_project_account_id(db: &ActionDb, project_id: &str) -> Option<String> {
    db.conn_ref()
        .query_row(
            "SELECT account_id FROM projects WHERE id = ?1 AND account_id IS NOT NULL",
            rusqlite::params![project_id],
            |row| row.get::<_, String>(0),
        )
        .ok()
}
