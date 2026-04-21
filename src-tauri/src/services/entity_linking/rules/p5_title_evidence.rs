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

    fn evaluate(&self, ctx: &LinkingContext, db: &ActionDb) -> RuleOutcome {
        // P5 deliberately runs for 1:1 internal × internal meetings (AC#5: "Acme
        // renewal plan" internal sync → P5 links to Acme, beats P6). The old
        // name-collision bug (AC#1) was from fuzzy/keyword signals that are now
        // gone. P5 only matches account/project names (≥4 chars, full words), not
        // person first names, so phantom links from shared first names cannot occur.
        let title = match &ctx.title {
            Some(t) if !t.is_empty() => t.to_lowercase(),
            _ => return RuleOutcome::Skip,
        };

        // Extract whole-word tokens ≥ 4 chars
        let tokens: Vec<&str> = title
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| w.len() >= 4 && !STOPLIST.contains(w))
            .collect();

        if tokens.is_empty() {
            return RuleOutcome::Skip;
        }

        let entities = match db.get_entities_for_title_match() {
            Ok(e) => e,
            Err(e) => {
                log::warn!("P5 get_entities_for_title_match error: {e}");
                return RuleOutcome::Skip;
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
            None => return RuleOutcome::Skip,
        };

        // Consistency check: if P4 found a different entity, block this as primary.
        if let Some(p4_id) = &self.p4_entity_id {
            if *p4_id != entity_id {
                // P5 title match conflicts with domain evidence — write as related, not primary.
                return RuleOutcome::Matched(Candidate {
                    entity: EntityRef { entity_id, entity_type },
                    role: LinkRole::Related,
                    confidence,
                    rule_id: "P5".to_string(),
                    evidence: evidence::title_match_evidence(
                        ctx, &entity_name, p4_id, &entity_name, false, false,
                    ),
                });
            }
        }

        let ev = evidence::title_match_evidence(
            ctx, &entity_name, &entity_id, &entity_name, false, true,
        );
        RuleOutcome::Matched(Candidate {
            entity: EntityRef { entity_id, entity_type },
            role: LinkRole::Primary,
            confidence,
            rule_id: "P5".to_string(),
            evidence: ev,
        })
    }
}
