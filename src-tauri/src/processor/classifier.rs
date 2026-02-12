//! Pattern-based file classification for inbox processing.
//!
//! Classifies files by filename patterns and optional frontmatter.
//! No AI needed — just regex matching against known conventions.

use std::path::Path;

/// Classification result for an inbox file.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    /// Meeting notes — route to archive
    MeetingNotes { account: Option<String> },
    /// Account dashboard/update — route to Accounts/<name>/
    AccountUpdate { account: String },
    /// Action items — extract to SQLite, archive original
    ActionItems { account: Option<String> },
    /// Context for an upcoming meeting
    MeetingContext { meeting_name: Option<String> },
    /// Unknown type — needs AI enrichment
    Unknown,
}

impl Classification {
    /// Human-readable label for this classification.
    pub fn label(&self) -> &'static str {
        match self {
            Self::MeetingNotes { .. } => "meeting_notes",
            Self::AccountUpdate { .. } => "account_update",
            Self::ActionItems { .. } => "action_items",
            Self::MeetingContext { .. } => "meeting_context",
            Self::Unknown => "unknown",
        }
    }
}

/// Classify a file based on its filename and content.
///
/// Filename patterns take priority. Content-based classification
/// is used as a fallback for files without recognizable names.
pub fn classify_file(path: &Path, content: &str) -> Classification {
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Strip known extensions for pattern matching (supports non-md files too)
    let mut stem = &filename[..];
    for ext in super::extract::KNOWN_EXTENSIONS {
        if let Some(stripped) = stem.strip_suffix(ext) {
            stem = stripped;
            break;
        }
    }

    // Pattern: *-meeting-notes* or *-meeting-recap*
    if stem.contains("meeting-notes") || stem.contains("meeting-recap") {
        let account = extract_account_from_filename(stem, &["meeting-notes", "meeting-recap"]);
        return Classification::MeetingNotes { account };
    }

    // Pattern: *-account-dashboard* or *-account-update*
    if stem.contains("account-dashboard") || stem.contains("account-update") {
        if let Some(account) =
            extract_account_from_filename(stem, &["account-dashboard", "account-update"])
        {
            return Classification::AccountUpdate { account };
        }
    }

    // Pattern: *-action* or *-actions* or *-todo* or *-tasks*
    if stem.contains("-action") || stem.contains("-todo") || stem.contains("-tasks") {
        let account = extract_account_from_filename(stem, &["-action", "-todo", "-tasks"]);
        return Classification::ActionItems { account };
    }

    // Pattern: *-context-for-* or *-prep-for-*
    if stem.contains("context-for-") || stem.contains("prep-for-") {
        let meeting_name = stem
            .split("context-for-")
            .nth(1)
            .or_else(|| stem.split("prep-for-").nth(1))
            .map(|s| s.replace('-', " "));
        return Classification::MeetingContext { meeting_name };
    }

    // Content-based fallback: check for action item markers
    if has_action_markers(content) {
        return Classification::ActionItems { account: None };
    }

    Classification::Unknown
}

/// Extract an account name from the part of the filename before a known suffix.
///
/// e.g., "acme-corp-meeting-notes" with suffix "meeting-notes" → Some("acme-corp")
fn extract_account_from_filename(stem: &str, suffixes: &[&str]) -> Option<String> {
    for suffix in suffixes {
        if !stem.contains(suffix) {
            continue;
        }
        if let Some(before) = stem.split(suffix).next() {
            let trimmed = before.trim_end_matches('-');
            if !trimmed.is_empty() {
                return Some(trimmed.replace('-', " "));
            }
        }
    }
    None
}

/// Check if content contains action item markers (checkboxes, "Action:" headers).
fn has_action_markers(content: &str) -> bool {
    let lines = content.lines().take(50); // Only check first 50 lines
    let mut action_count = 0;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ]")
            || trimmed.starts_with("- [x]")
            || trimmed.starts_with("* [ ]")
            || trimmed.starts_with("## Action")
            || trimmed.starts_with("### Action")
        {
            action_count += 1;
        }
    }
    action_count >= 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_meeting_notes() {
        let path = PathBuf::from("acme-corp-meeting-notes.md");
        let result = classify_file(&path, "");
        match result {
            Classification::MeetingNotes { account } => {
                assert_eq!(account, Some("acme corp".to_string()));
            }
            other => panic!("Expected MeetingNotes, got {:?}", other),
        }
    }

    #[test]
    fn test_account_update() {
        let path = PathBuf::from("acme-corp-account-dashboard.md");
        let result = classify_file(&path, "");
        match result {
            Classification::AccountUpdate { account } => {
                assert_eq!(account, "acme corp");
            }
            other => panic!("Expected AccountUpdate, got {:?}", other),
        }
    }

    #[test]
    fn test_action_items() {
        let path = PathBuf::from("acme-action-items.md");
        let result = classify_file(&path, "");
        assert!(matches!(result, Classification::ActionItems { .. }));
    }

    #[test]
    fn test_meeting_context() {
        let path = PathBuf::from("context-for-qbr-review.md");
        let result = classify_file(&path, "");
        match result {
            Classification::MeetingContext { meeting_name } => {
                assert_eq!(meeting_name, Some("qbr review".to_string()));
            }
            other => panic!("Expected MeetingContext, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown() {
        let path = PathBuf::from("random-notes.md");
        let result = classify_file(&path, "Just some text");
        assert!(matches!(result, Classification::Unknown));
    }

    #[test]
    fn test_content_based_actions() {
        let path = PathBuf::from("notes.md");
        let content =
            "# Meeting\n\n- [ ] Follow up with team\n- [ ] Send proposal\n- [ ] Review docs\n";
        let result = classify_file(&path, content);
        assert!(matches!(result, Classification::ActionItems { .. }));
    }

    // Non-md extension tests (ADR-0050: universal file extraction)

    #[test]
    fn test_classifier_strips_pdf_extension() {
        let path = PathBuf::from("acme-corp-meeting-notes.pdf");
        let result = classify_file(&path, "");
        match result {
            Classification::MeetingNotes { account } => {
                assert_eq!(account, Some("acme corp".to_string()));
            }
            other => panic!("Expected MeetingNotes for .pdf, got {:?}", other),
        }
    }

    #[test]
    fn test_classifier_strips_docx_extension() {
        let path = PathBuf::from("acme-corp-account-update.docx");
        let result = classify_file(&path, "");
        match result {
            Classification::AccountUpdate { account } => {
                assert_eq!(account, "acme corp");
            }
            other => panic!("Expected AccountUpdate for .docx, got {:?}", other),
        }
    }

    #[test]
    fn test_classifier_strips_xlsx_extension() {
        let path = PathBuf::from("acme-action-items.xlsx");
        let result = classify_file(&path, "");
        assert!(matches!(result, Classification::ActionItems { .. }));
    }

    #[test]
    fn test_classifier_strips_html_extension() {
        let path = PathBuf::from("context-for-weekly-sync.html");
        let result = classify_file(&path, "");
        match result {
            Classification::MeetingContext { meeting_name } => {
                assert_eq!(meeting_name, Some("weekly sync".to_string()));
            }
            other => panic!("Expected MeetingContext for .html, got {:?}", other),
        }
    }
}
