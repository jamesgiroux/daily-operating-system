//! P5 — Title/subject evidence, domain-gated.
//!
//! Whole-word match (≥4 chars) against entity names + keywords.
//! Blocked when P4 domain evidence points to a conflicting account.

use crate::db::ActionDb;
use super::super::{evidence, types::{Candidate, EntityRef, LinkRole, LinkingContext, RuleOutcome}};

pub struct P5TitleEvidence {
    /// Entity_id from the P4 domain-evidence pass (if any). P5 is blocked
    /// when its title match conflicts with this.
    pub p4_entity_id: Option<String>,
}

/// Words that match common entity names but are not meaningful for linking.
const STOPLIST: &[&str] = &[
    "open", "pilot", "plan", "monday", "notion", "mercury", "ramp",
    "handshake", "bridge", "flow", "base", "peak", "note", "space",
    "link", "next", "sync", "ready", "clear", "front", "core", "post",
    "meet", "call", "talk", "chat", "dash", "pulse", "track", "task",
    "work", "team", "loop", "zoom", "slack", "linear",
];

impl super::super::phases::Rule for P5TitleEvidence {
    fn id(&self) -> &'static str { "P5" }

    fn evaluate(
        &self,
        _service_ctx: &crate::services::context::ServiceContext<'_>,
        ctx: &LinkingContext,
        db: &ActionDb,
    ) -> Result<RuleOutcome, String> {
        // P5 deliberately runs for 1:1 internal × internal meetings (AC#5: "Acme
        // renewal plan" internal sync → P5 links to Acme, beats P6). The old
        // name-collision bug (AC#1) was from fuzzy/keyword signals that are now
        // gone. P5 only matches account/project names (≥4 chars, full words), not
        // person first names, so phantom links from shared first names cannot occur.
        let title = match &ctx.title {
            Some(t) if !t.is_empty() => t.to_lowercase(),
            _ => return Ok(RuleOutcome::Skip),
        };

        // Extract whole-word tokens ≥ 4 chars
        let tokens: Vec<&str> = title
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| w.len() >= 4 && !STOPLIST.contains(w))
            .collect();

        if tokens.is_empty() {
            return Ok(RuleOutcome::Skip);
        }

        let entities = match db.get_entities_for_title_match() {
            Ok(e) => e,
            Err(e) => {
                log::warn!("P5 get_entities_for_title_match error: {e}");
                return Ok(RuleOutcome::Skip);
            }
        };

        // Find the best match — entity name appears as whole word in title,
        // OR a keyword appears.
        let mut best: Option<(String, String, String, f64)> = None; // (id, type, name, confidence)

        for (id, entity_type, name, keywords_json) in &entities {
            let name_lower = name.to_lowercase();

            // Whole-word name match: check if the entity name appears as a
            // substring bounded by word separators.
            let name_words: Vec<&str> = name_lower
                .split(|c: char| !c.is_alphabetic())
                .filter(|w| w.len() >= 4)
                .collect();

            // Only require non-stoplist words to appear in the title tokens.
            // This fixes false negatives for entity names that contain stoplist
            // words (e.g. "Open Source Co" → check "source" and "co", not "open").
            let name_words_for_match: Vec<&str> = name_words
                .iter()
                .copied()
                .filter(|w| !STOPLIST.contains(w))
                .collect();

            let name_match = !name_words_for_match.is_empty()
                && name_words_for_match.iter().all(|nw| tokens.contains(nw));

            let keyword_match = keywords_json
                .as_deref()
                .and_then(|k| serde_json::from_str::<Vec<String>>(k).ok())
                .map(|kws| {
                    kws.iter().any(|kw| {
                        let kw_lower = kw.to_lowercase();
                        kw_lower.len() >= 4 && tokens.contains(&kw_lower.as_str())
                    })
                })
                .unwrap_or(false);

            if name_match || keyword_match {
                let conf = if name_match { 0.75 } else { 0.60 };
                if best.as_ref().map(|(_, _, _, c)| conf > *c).unwrap_or(true) {
                    best = Some((id.clone(), entity_type.clone(), name.clone(), conf));
                }
            }
        }

        let (entity_id, entity_type, entity_name, confidence) = match best {
            Some(b) => b,
            None => return Ok(RuleOutcome::Skip),
        };

        // Consistency check: if P4 found a different entity, block this as primary.
        if let Some(p4_id) = &self.p4_entity_id {
            if *p4_id != entity_id {
                // P5 title match conflicts with domain evidence — write as related, not primary.
                return Ok(RuleOutcome::Matched(Candidate {
                    entity: EntityRef { entity_id, entity_type },
                    role: LinkRole::Related,
                    confidence,
                    rule_id: "P5".to_string(),
                    evidence: evidence::title_match_evidence(
                        ctx, &entity_name, p4_id, &entity_name, false, false,
                    ),
                }));
            }
        }

        // External-attendee veto (evidence-hierarchy fix).
        //
        // Title alone cannot elect an account as Primary when external attendees
        // exist and we have no domain or stakeholder evidence connecting them
        // to that account. The title becomes a Related chip at most; the meeting
        // gets no primary and the user sees the picker.
        //
        // Why: an external stakeholder attending a meeting whose title name-drops
        // a *different* account (e.g. an @example.test contact on a call titled
        // "WordPress VIP planning") is strong evidence that the *attendee*
        // relationship matters more than the title match. Without this veto,
        // P5 elected unrelated-account-with-matching-title as Primary whenever
        // P4 found nothing — which is precisely the Bug B production regression.
        //
        // All-internal meetings (no external participants) keep the old behaviour:
        // P5 can elect Primary on title alone, since an internal sync titled
        // "Acme renewal plan" is legitimately about Acme.
        if self.p4_entity_id.is_none() && ctx.has_external_participant() {
            return Ok(RuleOutcome::Matched(Candidate {
                entity: EntityRef { entity_id, entity_type },
                role: LinkRole::Related,
                confidence,
                rule_id: "P5".to_string(),
                evidence: evidence::title_match_evidence(
                    ctx, &entity_name, &entity_name, &entity_name, false, false,
                ),
            }));
        }

        let ev = evidence::title_match_evidence(
            ctx, &entity_name, &entity_id, &entity_name, false, true,
        );
        Ok(RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id, entity_type },
            role: LinkRole::Primary,
            confidence,
            rule_id: "P5".to_string(),
            evidence: ev,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::super::phases::Rule;
    use super::super::super::types::{OwnerRef, OwnerType, Participant, ParticipantRole};
    use crate::db::test_utils::test_db;
    use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
    use chrono::{TimeZone, Utc};

    fn test_ctx<'a>(
        clock: &'a FixedClock,
        rng: &'a SeedableRng,
        ext: &'a ExternalClients,
    ) -> ServiceContext<'a> {
        ServiceContext::test_live(clock, rng, ext)
    }

    fn ctx_with_participants(
        participants: Vec<Participant>,
        title: &str,
    ) -> LinkingContext {
        let attendee_count = participants.len();
        LinkingContext {
            owner: OwnerRef { owner_type: OwnerType::Meeting, owner_id: "m1".to_string() },
            participants,
            title: Some(title.to_string()),
            attendee_count,
            thread_id: None,
            series_id: None,
            graph_version: 0,
            user_domains: vec!["company.com".to_string()],
        }
    }

    fn internal(email: &str) -> Participant {
        Participant {
            email: email.to_string(),
            name: None,
            role: ParticipantRole::Attendee,
            person_id: None,
            domain: email.split('@').nth(1).map(|s| s.to_string()),
        }
    }

    fn external(email: &str) -> Participant {
        internal(email)
    }

    #[test]
    fn p5_external_attendees_no_p4_match_demotes_to_related() {
        let db = test_db();
        // Seed an account whose name appears in the title.
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('acc-wp', 'WordPress VIP', '2026-01-01', 0)",
                [],
            )
            .expect("insert account");

        // External attendee whose domain has nothing to do with WordPress VIP.
        let ctx = ctx_with_participants(
            vec![
                internal("me@company.com"),
                external("jane@example.test"),
            ],
            "WordPress VIP planning",
        );

        let rule = P5TitleEvidence { p4_entity_id: None };
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let service_ctx = test_ctx(&clock, &rng, &ext);
        match rule.evaluate(&service_ctx, &ctx, &db).expect("evaluate") {
            RuleOutcome::Matched(c) => {
                assert_eq!(c.role, LinkRole::Related, "must demote to Related when externals exist and P4 found nothing");
                assert_eq!(c.entity.entity_id, "acc-wp");
            }
            RuleOutcome::Skip => panic!("expected Matched(Related), got Skip"),
        }
    }

    #[test]
    fn p5_all_internal_attendees_title_match_elects_primary_unchanged() {
        let db = test_db();
        db.conn_ref()
            .execute(
                "INSERT INTO accounts (id, name, updated_at, archived) VALUES ('acc-acme', 'Acme Corp', '2026-01-01', 0)",
                [],
            )
            .expect("insert account");

        let ctx = ctx_with_participants(
            vec![
                internal("me@company.com"),
                internal("alice@company.com"),
            ],
            "Acme Corp renewal planning",
        );

        let rule = P5TitleEvidence { p4_entity_id: None };
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 4, 30, 0, 0, 0).unwrap());
        let rng = SeedableRng::new(42);
        let ext = ExternalClients::default();
        let service_ctx = test_ctx(&clock, &rng, &ext);
        match rule.evaluate(&service_ctx, &ctx, &db).expect("evaluate") {
            RuleOutcome::Matched(c) => {
                assert_eq!(c.role, LinkRole::Primary, "all-internal meetings keep Primary on title match");
                assert_eq!(c.entity.entity_id, "acc-acme");
            }
            RuleOutcome::Skip => panic!("expected Matched(Primary), got Skip"),
        }
    }
}
