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
    // CS extension hooks -> Phase 4 (log as "skipped" when profile == "customer-success")
    if ctx.profile == "customer-success" {
        results.push(HookResult {
            hook_name: "cs_extension",
            success: true,
            message: Some("CS hooks deferred to Phase 4".to_string()),
        });
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
