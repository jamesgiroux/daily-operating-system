//! Change detection for Clay enrichment.
//!
//! On each enrichment cycle, compare new data from Clay against the stored
//! person record to detect meaningful changes (title change, company change,
//! new social profile). These signals feed into the enrichment_log and can
//! surface in meeting intelligence and daily briefings.

use serde::{Deserialize, Serialize};

/// A detected change signal from enrichment comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrichmentSignal {
    /// Signal category: "title_change", "company_change", "profile_update"
    pub signal_type: String,
    /// Human-readable description of what changed
    pub description: String,
    /// Previous value (None if field was previously empty)
    pub old_value: Option<String>,
    /// New value (None if field was removed — unusual but possible)
    pub new_value: Option<String>,
}

/// Compare new Clay data against stored person data to detect changes.
///
/// Returns a vec of `EnrichmentSignal`s representing meaningful differences.
/// An empty vec means no notable changes were detected.
#[allow(clippy::too_many_arguments)]
pub fn detect_changes(
    stored_title_history: Option<&str>,
    new_title_history: Option<&str>,
    stored_org: Option<&str>,
    new_org: Option<&str>,
    stored_linkedin: Option<&str>,
    new_linkedin: Option<&str>,
    stored_twitter: Option<&str>,
    new_twitter: Option<&str>,
) -> Vec<EnrichmentSignal> {
    let mut signals = Vec::new();

    // 1. Title history: detect new entries in the title_history JSON array.
    //    Both values are JSON strings of [{title, company, startDate, endDate}].
    if let Some(new_json) = new_title_history {
        let new_entries: Vec<serde_json::Value> =
            serde_json::from_str(new_json).unwrap_or_default();
        let stored_entries: Vec<serde_json::Value> = stored_title_history
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        // Compare by length first — new entries at the front indicate a title change.
        if new_entries.len() > stored_entries.len() {
            // The newest entry is typically first in the array.
            if let Some(latest) = new_entries.first() {
                let title = latest
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let company = latest
                    .get("company")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");

                let old_desc = stored_entries.first().map(|prev| {
                    let t = prev
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    let c = prev
                        .get("company")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown");
                    format!("{} at {}", t, c)
                });

                signals.push(EnrichmentSignal {
                    signal_type: "title_change".to_string(),
                    description: format!("New role detected: {} at {}", title, company),
                    old_value: old_desc,
                    new_value: Some(format!("{} at {}", title, company)),
                });
            }
        } else if !new_entries.is_empty() && !stored_entries.is_empty() {
            // Same length — check if the latest entry content changed.
            let new_first = &new_entries[0];
            let stored_first = &stored_entries[0];

            let new_title = new_first.get("title").and_then(|v| v.as_str());
            let stored_title = stored_first.get("title").and_then(|v| v.as_str());
            let new_company = new_first.get("company").and_then(|v| v.as_str());
            let stored_company = stored_first.get("company").and_then(|v| v.as_str());

            if new_title != stored_title || new_company != stored_company {
                let old_desc = format!(
                    "{} at {}",
                    stored_title.unwrap_or("Unknown"),
                    stored_company.unwrap_or("Unknown")
                );
                let new_desc = format!(
                    "{} at {}",
                    new_title.unwrap_or("Unknown"),
                    new_company.unwrap_or("Unknown")
                );

                signals.push(EnrichmentSignal {
                    signal_type: "title_change".to_string(),
                    description: format!(
                        "Title updated: {} -> {}",
                        stored_title.unwrap_or("Unknown"),
                        new_title.unwrap_or("Unknown")
                    ),
                    old_value: Some(old_desc),
                    new_value: Some(new_desc),
                });
            }
        }
    }

    // 2. Company/org change (top-level org field, independent of title history).
    if let Some(new) = new_org {
        match stored_org {
            Some(stored) if stored != new => {
                signals.push(EnrichmentSignal {
                    signal_type: "company_change".to_string(),
                    description: format!("Company changed from {} to {}", stored, new),
                    old_value: Some(stored.to_string()),
                    new_value: Some(new.to_string()),
                });
            }
            None => {
                signals.push(EnrichmentSignal {
                    signal_type: "company_change".to_string(),
                    description: format!("Company identified: {}", new),
                    old_value: None,
                    new_value: Some(new.to_string()),
                });
            }
            _ => {}
        }
    }

    // 3. New LinkedIn profile added where none existed.
    if let Some(new) = new_linkedin {
        if stored_linkedin.is_none() && !new.is_empty() {
            signals.push(EnrichmentSignal {
                signal_type: "profile_update".to_string(),
                description: "LinkedIn profile discovered".to_string(),
                old_value: None,
                new_value: Some(new.to_string()),
            });
        }
    }

    // 4. New Twitter handle added where none existed.
    if let Some(new) = new_twitter {
        if stored_twitter.is_none() && !new.is_empty() {
            signals.push(EnrichmentSignal {
                signal_type: "profile_update".to_string(),
                description: "Twitter handle discovered".to_string(),
                old_value: None,
                new_value: Some(new.to_string()),
            });
        }
    }

    signals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_changes_when_all_same() {
        let signals = detect_changes(
            Some(r#"[{"title":"CTO","company":"Acme"}]"#),
            Some(r#"[{"title":"CTO","company":"Acme"}]"#),
            Some("Acme"),
            Some("Acme"),
            Some("https://linkedin.com/in/alice"),
            Some("https://linkedin.com/in/alice"),
            Some("@alice"),
            Some("@alice"),
        );
        assert!(signals.is_empty());
    }

    #[test]
    fn detects_new_title_entry() {
        let stored = r#"[{"title":"VP Sales","company":"OldCo"}]"#;
        let new = r#"[{"title":"CRO","company":"NewCo"},{"title":"VP Sales","company":"OldCo"}]"#;
        let signals = detect_changes(Some(stored), Some(new), None, None, None, None, None, None);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "title_change");
        assert!(signals[0].description.contains("CRO"));
    }

    #[test]
    fn detects_company_change() {
        let signals =
            detect_changes(None, None, Some("OldCo"), Some("NewCo"), None, None, None, None);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "company_change");
    }

    #[test]
    fn detects_new_linkedin() {
        let signals = detect_changes(
            None,
            None,
            None,
            None,
            None,
            Some("https://linkedin.com/in/bob"),
            None,
            None,
        );
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "profile_update");
        assert!(signals[0].description.contains("LinkedIn"));
    }

    #[test]
    fn detects_new_twitter() {
        let signals = detect_changes(None, None, None, None, None, None, None, Some("@bob"));
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "profile_update");
        assert!(signals[0].description.contains("Twitter"));
    }

    #[test]
    fn detects_company_identified_from_none() {
        let signals =
            detect_changes(None, None, None, Some("Acme Corp"), None, None, None, None);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].signal_type, "company_change");
        assert!(signals[0].description.contains("identified"));
    }
}
