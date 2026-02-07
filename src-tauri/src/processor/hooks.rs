//! Post-enrichment hooks for the inbox processing pipeline.
//!
//! After a file is classified and routed (or AI-enriched), these hooks run
//! mechanical, deterministic follow-up steps: syncing actions to SQLite,
//! updating markdown checkboxes for completed actions, etc.
//!
//! Each hook is error-isolated: one failure doesn't block others.

use std::path::{Path, PathBuf};

use crate::db::{ActionDb, DbAction};

/// Context passed to every post-enrichment hook.
pub struct EnrichmentContext {
    pub workspace: PathBuf,
    pub filename: String,
    pub classification: String,
    pub account: Option<String>,
    pub summary: String,
    pub actions: Vec<DbAction>,
    pub destination_path: Option<String>,
    pub profile: String,
    pub wins: Vec<String>,
    pub risks: Vec<String>,
}

/// Result from a single hook.
pub struct HookResult {
    pub hook_name: &'static str,
    pub success: bool,
    pub message: Option<String>,
}

/// Run all post-enrichment hooks. Error-isolated: one failure doesn't block others.
pub fn run_post_enrichment_hooks(ctx: &EnrichmentContext, db: &ActionDb) -> Vec<HookResult> {
    let mut results = Vec::new();
    results.push(sync_actions_to_sqlite(ctx, db));
    results.push(sync_completion_to_markdown(ctx, db));
    // CS account intelligence: write wins/risks as captures, touch last-contact
    if ctx.profile == "customer-success" {
        results.push(cs_account_intelligence(ctx, db));
    }
    results
}

/// Sync extracted actions to the SQLite database.
///
/// Reads `ctx.actions` and upserts each one, setting source_type = "enrichment"
/// and source_label = ctx.filename. Skips already-completed actions.
fn sync_actions_to_sqlite(ctx: &EnrichmentContext, db: &ActionDb) -> HookResult {
    let mut synced = 0;
    let mut errors = 0;

    for action in &ctx.actions {
        let mut db_action = action.clone();
        db_action.source_type = Some("enrichment".to_string());
        db_action.source_label = Some(ctx.filename.clone());

        match db.upsert_action_if_not_completed(&db_action) {
            Ok(()) => synced += 1,
            Err(e) => {
                log::warn!(
                    "Hook sync_actions_to_sqlite: failed to upsert '{}': {}",
                    action.title,
                    e
                );
                errors += 1;
            }
        }
    }

    HookResult {
        hook_name: "sync_actions_to_sqlite",
        success: errors == 0,
        message: Some(format!("Synced {} actions ({} errors)", synced, errors)),
    }
}

/// Sync completed actions back to their source markdown files.
///
/// Queries recently-completed actions that have a `source_label` pointing to a
/// markdown file in the workspace. For each, scans the source file for
/// `- [ ] {title}` and replaces with `- [x] {title}`.
fn sync_completion_to_markdown(ctx: &EnrichmentContext, db: &ActionDb) -> HookResult {
    let recently_completed = match db.get_recently_completed(24) {
        Ok(actions) => actions,
        Err(e) => {
            return HookResult {
                hook_name: "sync_completion_to_markdown",
                success: false,
                message: Some(format!("Failed to query completed actions: {}", e)),
            };
        }
    };

    let mut files_updated = 0;
    let mut actions_toggled = 0;

    for action in &recently_completed {
        let source_label = match &action.source_label {
            Some(label) if label.ends_with(".md") => label,
            _ => continue,
        };

        // Try to find the source file in the workspace
        let file_path = find_source_file(&ctx.workspace, source_label);
        let file_path = match file_path {
            Some(p) => p,
            None => continue,
        };

        // Read, update checkboxes, write back
        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Prefer source_id (raw text before metadata stripping) for matching.
        // Falls back to title for actions created before metadata parsing existed.
        let match_text = action.source_id.as_deref().unwrap_or(&action.title);
        let needle_unchecked = format!("- [ ] {}", match_text);
        if !content.contains(&needle_unchecked) {
            continue;
        }

        let updated = content.replace(
            &needle_unchecked,
            &format!("- [x] {}", match_text),
        );

        if updated != content {
            if let Err(e) = std::fs::write(&file_path, &updated) {
                log::warn!(
                    "Hook sync_completion_to_markdown: failed to write '{}': {}",
                    file_path.display(),
                    e
                );
            } else {
                files_updated += 1;
                actions_toggled += 1;
                log::info!(
                    "Toggled checkbox for '{}' in '{}'",
                    action.title,
                    file_path.display()
                );
            }
        }
    }

    HookResult {
        hook_name: "sync_completion_to_markdown",
        success: true,
        message: Some(format!(
            "Updated {} files, toggled {} checkboxes",
            files_updated, actions_toggled
        )),
    }
}

/// Write extracted wins/risks as captures and touch account last-contact.
///
/// Only runs when an account is associated with the file. Uses a synthetic
/// `meeting_id` of `inbox-{filename}` since inbox files don't have a real
/// meeting context.
fn cs_account_intelligence(ctx: &EnrichmentContext, db: &ActionDb) -> HookResult {
    let account = match &ctx.account {
        Some(a) => a,
        None => {
            return HookResult {
                hook_name: "cs_account_intelligence",
                success: true,
                message: Some("Skipped: no account associated".to_string()),
            };
        }
    };

    let synthetic_meeting_id = format!("inbox-{}", ctx.filename);
    let mut captures_written = 0;
    let mut errors = 0;

    for win in &ctx.wins {
        match db.insert_capture(
            &synthetic_meeting_id,
            &ctx.filename,
            Some(account),
            "win",
            win,
        ) {
            Ok(()) => captures_written += 1,
            Err(e) => {
                log::warn!("cs_account_intelligence: failed to write win: {}", e);
                errors += 1;
            }
        }
    }

    for risk in &ctx.risks {
        match db.insert_capture(
            &synthetic_meeting_id,
            &ctx.filename,
            Some(account),
            "risk",
            risk,
        ) {
            Ok(()) => captures_written += 1,
            Err(e) => {
                log::warn!("cs_account_intelligence: failed to write risk: {}", e);
                errors += 1;
            }
        }
    }

    // Touch last-contact on the account
    let touched = match db.touch_account_last_contact(account) {
        Ok(matched) => matched,
        Err(e) => {
            log::warn!("cs_account_intelligence: failed to touch account: {}", e);
            false
        }
    };

    HookResult {
        hook_name: "cs_account_intelligence",
        success: errors == 0,
        message: Some(format!(
            "Wrote {} captures ({} errors), account touched: {}",
            captures_written, errors, touched
        )),
    }
}

/// Search common workspace locations for a source file by label.
fn find_source_file(workspace: &Path, filename: &str) -> Option<PathBuf> {
    // Direct path in common locations
    let candidates = [
        workspace.join("_today").join(filename),
        workspace.join("_inbox").join(filename),
        workspace.join(filename),
    ];

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    // Search in _today/ subdirectories
    if let Ok(entries) = std::fs::read_dir(workspace.join("_today")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let candidate = path.join(filename);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{ActionDb, DbAccount};

    fn test_db() -> ActionDb {
        let dir = tempfile::tempdir().expect("Failed to create temp dir");
        let path = dir.path().join("test_hooks.db");
        std::mem::forget(dir);
        ActionDb::open_at(path).expect("Failed to open test database")
    }

    fn base_context(account: Option<String>, profile: &str) -> EnrichmentContext {
        EnrichmentContext {
            workspace: PathBuf::from("/tmp/test-workspace"),
            filename: "acme-update.md".to_string(),
            classification: "account_update".to_string(),
            account,
            summary: "Test summary".to_string(),
            actions: Vec::new(),
            destination_path: None,
            profile: profile.to_string(),
            wins: Vec::new(),
            risks: Vec::new(),
        }
    }

    #[test]
    fn test_cs_hook_writes_captures() {
        let db = test_db();

        // Create the account so touch works
        let account = DbAccount {
            id: "acme".to_string(),
            name: "Acme".to_string(),
            ring: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        db.upsert_account(&account).expect("upsert account");

        let mut ctx = base_context(Some("Acme".to_string()), "customer-success");
        ctx.wins = vec!["Expanded to 3 teams".to_string()];
        ctx.risks = vec!["Budget freeze".to_string(), "Champion leaving".to_string()];

        let result = cs_account_intelligence(&ctx, &db);

        assert!(result.success);
        assert_eq!(result.hook_name, "cs_account_intelligence");

        // Verify captures were written
        let captures = db
            .get_captures_for_account("Acme", 30)
            .expect("query captures");
        assert_eq!(captures.len(), 3);

        let wins: Vec<_> = captures
            .iter()
            .filter(|c| c.capture_type == "win")
            .collect();
        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].content, "Expanded to 3 teams");
        assert_eq!(wins[0].meeting_id, "inbox-acme-update.md");

        let risks: Vec<_> = captures
            .iter()
            .filter(|c| c.capture_type == "risk")
            .collect();
        assert_eq!(risks.len(), 2);
    }

    #[test]
    fn test_cs_hook_touches_account() {
        let db = test_db();

        let account = DbAccount {
            id: "acme".to_string(),
            name: "Acme".to_string(),
            ring: None,
            arr: None,
            health: None,
            contract_start: None,
            contract_end: None,
            csm: None,
            champion: None,
            tracker_path: None,
            updated_at: "2020-01-01T00:00:00Z".to_string(),
        };
        db.upsert_account(&account).expect("upsert");

        let ctx = base_context(Some("Acme".to_string()), "customer-success");
        cs_account_intelligence(&ctx, &db);

        let acct = db.get_account("acme").expect("get").unwrap();
        assert_ne!(acct.updated_at, "2020-01-01T00:00:00Z");
    }

    #[test]
    fn test_cs_hook_skips_when_no_account() {
        let db = test_db();

        let ctx = base_context(None, "customer-success");
        let result = cs_account_intelligence(&ctx, &db);

        assert!(result.success);
        assert!(result
            .message
            .as_ref()
            .unwrap()
            .contains("no account"));
    }

    #[test]
    fn test_cs_hook_not_run_for_general_profile() {
        let db = test_db();

        let mut ctx = base_context(Some("Acme".to_string()), "general");
        ctx.wins = vec!["Should not be written".to_string()];

        let results = run_post_enrichment_hooks(&ctx, &db);

        // CS hook should not appear in results for general profile
        let cs_hooks: Vec<_> = results
            .iter()
            .filter(|r| r.hook_name == "cs_account_intelligence")
            .collect();
        assert!(cs_hooks.is_empty());
    }
}
