// Settings service — extracted from commands.rs (I454)
// Business logic for configuration and settings mutations.

use crate::state::AppState;
use crate::types::{Config, WorkflowId};

/// Set entity mode (account, project, or both) with workspace dir creation.
pub fn set_entity_mode(mode: &str, state: &AppState) -> Result<Config, String> {
    crate::types::validate_entity_mode(mode)?;

    let mode = mode.to_string();
    let config = crate::state::create_or_update_config(state, |config| {
        config.entity_mode = mode.clone();
        config.profile = crate::types::profile_for_entity_mode(&mode);
    })?;

    if !config.workspace_path.is_empty() {
        let workspace = std::path::Path::new(&config.workspace_path);
        if workspace.exists() {
            if mode == "account" || mode == "both" {
                let accounts_dir = workspace.join("Accounts");
                if !accounts_dir.exists() {
                    let _ = std::fs::create_dir_all(&accounts_dir);
                }
            }
            if mode == "project" || mode == "both" {
                let projects_dir = workspace.join("Projects");
                if !projects_dir.exists() {
                    let _ = std::fs::create_dir_all(&projects_dir);
                }
            }
        }
    }

    Ok(config)
}

/// Set workspace path, scaffold directory structure, and sync entities.
pub fn set_workspace_path(path: &str, state: &AppState) -> Result<Config, String> {
    let workspace = std::path::Path::new(path);

    if !workspace.is_absolute() {
        return Err("Workspace path must be absolute".to_string());
    }

    let entity_mode = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.entity_mode.clone()))
        .unwrap_or_else(|| "account".to_string());

    crate::state::initialize_workspace(workspace, &entity_mode)?;

    let path = path.to_string();
    let config = crate::state::create_or_update_config(state, |config| {
        config.workspace_path = path.clone();
    })?;

    if let Ok(db_guard) = state.db.lock() {
        if let Some(db) = db_guard.as_ref() {
            let _ = crate::people::sync_people_from_workspace(
                workspace,
                db,
                &config.resolved_user_domains(),
            );
            let _ = crate::accounts::sync_accounts_from_workspace(workspace, db);
            let _ = crate::projects::sync_projects_from_workspace(workspace, db);
        }
    }

    Ok(config)
}

/// Set AI model for a tier (synthesis, extraction, mechanical).
pub fn set_ai_model(tier: &str, model: &str, state: &AppState) -> Result<Config, String> {
    let valid_tiers = ["synthesis", "extraction", "mechanical"];
    if !valid_tiers.contains(&tier) {
        return Err(format!(
            "Invalid tier '{}'. Must be one of: {}",
            tier,
            valid_tiers.join(", ")
        ));
    }

    let valid_models = ["opus", "sonnet", "haiku"];
    if !valid_models.contains(&model) {
        return Err(format!(
            "Invalid model '{}'. Must be one of: {}",
            model,
            valid_models.join(", ")
        ));
    }

    let model = model.to_string();
    crate::state::create_or_update_config(state, |config| {
        match tier {
            "synthesis" => config.ai_models.synthesis = model.clone(),
            "extraction" => config.ai_models.extraction = model.clone(),
            "mechanical" => config.ai_models.mechanical = model.clone(),
            _ => {}
        }
    })
}

/// Set hygiene configuration (I271).
pub fn set_hygiene_config(
    scan_interval_hours: Option<u32>,
    ai_budget: Option<u32>,
    pre_meeting_hours: Option<u32>,
    state: &AppState,
) -> Result<Config, String> {
    if let Some(v) = scan_interval_hours {
        if ![1, 2, 4, 8].contains(&v) {
            return Err(format!(
                "Invalid scan interval: {}. Must be 1, 2, 4, or 8.",
                v
            ));
        }
    }
    if let Some(v) = ai_budget {
        if ![5, 10, 20, 50].contains(&v) {
            return Err(format!(
                "Invalid AI budget: {}. Must be 5, 10, 20, or 50.",
                v
            ));
        }
    }
    if let Some(v) = pre_meeting_hours {
        if ![2, 4, 12, 24].contains(&v) {
            return Err(format!(
                "Invalid pre-meeting window: {}. Must be 2, 4, 12, or 24.",
                v
            ));
        }
    }

    crate::state::create_or_update_config(state, |config| {
        if let Some(v) = scan_interval_hours {
            config.hygiene_scan_interval_hours = v;
        }
        if let Some(v) = ai_budget {
            config.hygiene_ai_budget = v;
        }
        if let Some(v) = pre_meeting_hours {
            config.hygiene_pre_meeting_hours = v;
        }
    })
}

/// Set schedule for a workflow.
pub fn set_schedule(
    workflow: &str,
    hour: u32,
    minute: u32,
    timezone: &str,
    state: &AppState,
) -> Result<Config, String> {
    if hour > 23 {
        return Err("Hour must be 0-23".to_string());
    }
    if minute > 59 {
        return Err("Minute must be 0-59".to_string());
    }

    timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| format!("Invalid timezone: {}", timezone))?;

    let workflow_id: WorkflowId = workflow.parse()?;
    let timezone = timezone.to_string();

    crate::state::create_or_update_config(state, |config| {
        let cron = match workflow_id {
            WorkflowId::Today => format!("{} {} * * 1-5", minute, hour),
            WorkflowId::Archive => format!("{} {} * * *", minute, hour),
            WorkflowId::InboxBatch => format!("{} {} * * 1-5", minute, hour),
            WorkflowId::Week => format!("{} {} * * 1", minute, hour),
        };

        let entry = match workflow_id {
            WorkflowId::Today => &mut config.schedules.today,
            WorkflowId::Archive => &mut config.schedules.archive,
            WorkflowId::InboxBatch => &mut config.schedules.inbox_batch,
            WorkflowId::Week => &mut config.schedules.week,
        };

        entry.cron = cron;
        entry.timezone = timezone.clone();
    })
}

/// Save user profile fields with internal org entity sync.
pub fn set_user_profile(
    name: Option<String>,
    company: Option<String>,
    title: Option<String>,
    focus: Option<String>,
    domain: Option<String>,
    domains: Option<Vec<String>>,
    state: &AppState,
) -> Result<String, String> {
    crate::state::create_or_update_config(state, |config| {
        fn clean(val: Option<String>) -> Option<String> {
            val.and_then(|s| {
                let trimmed = s.trim().to_string();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed)
                }
            })
        }

        config.user_name = clean(name);
        config.user_company = clean(company);
        config.user_title = clean(title);
        config.user_focus = clean(focus);

        if let Some(d) = domains {
            let cleaned: Vec<String> = d
                .into_iter()
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if cleaned.is_empty() {
                config.user_domains = None;
                config.user_domain = None;
            } else {
                config.user_domain = Some(cleaned[0].clone());
                config.user_domains = Some(cleaned);
            }
        } else if let Some(d) = domain {
            let trimmed = d.trim().to_lowercase();
            config.user_domain = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            };
        }
    })?;

    // Sync company name to internal root account entity if it changed
    let current_company = state
        .config
        .read()
        .ok()
        .and_then(|g| g.as_ref().and_then(|c| c.user_company.clone()));
    if let Some(ref company_name) = current_company {
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                if let Ok(Some(root)) = db.get_internal_root_account() {
                    if root.name != *company_name {
                        let _ = db.update_account_field(&root.id, "name", company_name);
                    }
                }
            }
        }
    }

    Ok("ok".to_string())
}

/// Set multiple user domains with reclassification of people and meetings.
pub fn set_user_domains(domains: &str, state: &AppState) -> Result<Config, String> {
    let parsed: Vec<String> = domains
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let config = crate::state::create_or_update_config(state, |config| {
        config.user_domain = parsed.first().cloned();
        config.user_domains = if parsed.is_empty() {
            None
        } else {
            Some(parsed.clone())
        };
    })?;

    if !parsed.is_empty() {
        if let Ok(db_guard) = state.db.lock() {
            if let Some(db) = db_guard.as_ref() {
                match db.reclassify_people_for_domains(&parsed) {
                    Ok(n) if n > 0 => {
                        log::info!("Reclassified {} people after domain change", n);
                        match db.reclassify_meeting_types_from_attendees() {
                            Ok(m) if m > 0 => {
                                log::info!("Reclassified {} meetings after domain change", m);
                            }
                            Ok(_) => {}
                            Err(e) => log::warn!("Meeting reclassification failed: {}", e),
                        }
                    }
                    Ok(_) => {}
                    Err(e) => log::warn!("People reclassification failed: {}", e),
                }
            }
        }
    }

    Ok(config)
}
