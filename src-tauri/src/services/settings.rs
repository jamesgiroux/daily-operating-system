// Settings service — extracted from commands.rs
// Business logic for configuration and settings mutations.

use crate::state::AppState;
use crate::services::context::ServiceContext;
use crate::types::{Config, WorkflowId};

fn validate_ai_model_choice(tier: &str, model: &str) -> Result<(), String> {
    let valid_tiers = ["synthesis", "extraction", "background", "mechanical"];
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

    Ok(())
}

/// Set entity mode (account, project, or both) with workspace dir creation.
pub fn set_entity_mode(
    ctx: &ServiceContext<'_>,
    mode: &str,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
pub async fn set_workspace_path(
    ctx: &ServiceContext<'_>,
    path: &str,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let workspace = std::path::Path::new(path);

    if !workspace.is_absolute() {
        return Err("Workspace path must be absolute".to_string());
    }

    let entity_mode = state
        .config
        .read()
        .as_ref()
        .map(|c| c.entity_mode.clone())
        .unwrap_or_else(|| "account".to_string());

    crate::state::initialize_workspace(workspace, &entity_mode)?;

    let path = path.to_string();
    let config = crate::state::create_or_update_config(state, |config| {
        config.workspace_path = path.clone();
    })?;

    let workspace_path = config.workspace_path.clone();
    let user_domains = config.resolved_user_domains();
    let _ = state
        .db_write(move |db| {
            let workspace = std::path::Path::new(&workspace_path);
            let _ = crate::people::sync_people_from_workspace(workspace, db, &user_domains);
            let _ = crate::accounts::sync_accounts_from_workspace(workspace, db);
            let _ = crate::projects::sync_projects_from_workspace(workspace, db);
            Ok(())
        })
        .await;

    Ok(config)
}

/// Set AI model for a tier (synthesis, extraction, background, mechanical).
pub fn set_ai_model(
    ctx: &ServiceContext<'_>,
    tier: &str,
    model: &str,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    validate_ai_model_choice(tier, model)?;

    let model = model.to_string();
    crate::state::create_or_update_config(state, |config| match tier {
        "synthesis" => config.ai_models.synthesis = model.clone(),
        "extraction" => config.ai_models.extraction = model.clone(),
        "background" => config.ai_models.background = model.clone(),
        "mechanical" => config.ai_models.mechanical = model.clone(),
        _ => {}
    })
}

/// Reset AI model routing to the recommended default bundle.
pub fn reset_ai_models_to_recommended(
    ctx: &ServiceContext<'_>,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    crate::state::create_or_update_config(state, |config| {
        config.ai_models = crate::types::AiModelConfig::default();
        config.ai_model_routing_version = crate::types::AI_MODEL_ROUTING_VERSION;
    })
}

/// Set Google calendar and email poll intervals in minutes.
pub fn set_google_poll_settings(
    ctx: &ServiceContext<'_>,
    calendar_poll_interval_minutes: Option<u32>,
    email_poll_interval_minutes: Option<u32>,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if let Some(v) = calendar_poll_interval_minutes {
        if !(1..=60).contains(&v) {
            return Err(format!(
                "Invalid calendar poll interval: {}. Must be 1-60 minutes.",
                v
            ));
        }
    }
    if let Some(v) = email_poll_interval_minutes {
        if !(1..=60).contains(&v) {
            return Err(format!(
                "Invalid email poll interval: {}. Must be 1-60 minutes.",
                v
            ));
        }
    }

    crate::state::create_or_update_config(state, |config| {
        if let Some(v) = calendar_poll_interval_minutes {
            config.google.calendar_poll_interval_minutes = v;
        }
        if let Some(v) = email_poll_interval_minutes {
            config.google.email_poll_interval_minutes = v;
        }
    })
}

/// Set hygiene configuration.
///
/// `ai_budget` is accepted but ignored — the hygiene call-count budget has been
/// replaced by the single daily AI token budget. Callers should use
/// `set_daily_ai_budget` instead.
pub fn set_hygiene_config(
    ctx: &ServiceContext<'_>,
    scan_interval_hours: Option<u32>,
    _ai_budget: Option<u32>,
    pre_meeting_hours: Option<u32>,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if let Some(v) = scan_interval_hours {
        if ![1, 2, 4, 8].contains(&v) {
            return Err(format!(
                "Invalid scan interval: {}. Must be 1, 2, 4, or 8.",
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
        if let Some(v) = pre_meeting_hours {
            config.hygiene_pre_meeting_hours = v;
        }
    })
}

/// Validate a daily AI budget value.
///
/// Exported for unit-test use only; production callers use `set_daily_ai_budget`.
#[cfg(test)]
pub(super) fn set_daily_ai_budget_validate(budget: u32) -> bool {
    const VALID_TIERS: &[u32] = &[50_000, 100_000, 250_000];
    VALID_TIERS.contains(&budget)
}

/// Set the daily AI token budget.
///
/// Valid tiers: 50_000, 100_000, 250_000.
/// Persists to config and syncs to KV store for the preflight gate.
pub fn set_daily_ai_budget(
    ctx: &ServiceContext<'_>,
    budget: u32,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    const VALID_TIERS: &[u32] = &[50_000, 100_000, 250_000];
    if !VALID_TIERS.contains(&budget) {
        return Err(format!(
            "Invalid daily AI budget: {}. Must be one of: 50000, 100000, 250000.",
            budget
        ));
    }

    let config = crate::state::create_or_update_config(state, |config| {
        config.daily_ai_token_budget = budget;
    })?;

    // Sync to KV store so the preflight gate (sync path) sees the update immediately.
    if let Ok(db) = crate::db::ActionDb::open() {
        crate::pty::sync_budget_config_to_kv(&db, budget);
    }

    Ok(config)
}

/// Update notification preferences.
pub fn set_notification_config(
    ctx: &ServiceContext<'_>,
    config_update: crate::types::NotificationConfig,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Validate quiet hours
    if let Some(start) = config_update.quiet_hours_start {
        if start > 23 {
            return Err("Quiet hours start must be 0-23".to_string());
        }
    }
    if let Some(end) = config_update.quiet_hours_end {
        if end > 23 {
            return Err("Quiet hours end must be 0-23".to_string());
        }
    }
    // Both must be set or both must be None
    if config_update.quiet_hours_start.is_some() != config_update.quiet_hours_end.is_some() {
        return Err("Both quiet hours start and end must be set, or neither".to_string());
    }

    crate::state::create_or_update_config(state, |config| {
        config.notifications = config_update.clone();
    })
}

/// Set UI text scale percentage.
pub fn set_text_scale(
    ctx: &ServiceContext<'_>,
    percent: u32,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    if !(80..=150).contains(&percent) {
        return Err(format!(
            "Invalid text scale: {}%. Must be between 80% and 150%.",
            percent
        ));
    }

    crate::state::create_or_update_config(state, |config| {
        config.text_scale_percent = percent;
    })
}

/// Set schedule for a workflow.
pub fn set_schedule(
    ctx: &ServiceContext<'_>,
    workflow: &str,
    hour: u32,
    minute: u32,
    timezone: &str,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
// ServiceContext adds 1 arg; request-object refactor deferred.
#[allow(clippy::too_many_arguments)]
pub async fn set_user_profile(
    ctx: &ServiceContext<'_>,
    name: Option<String>,
    company: Option<String>,
    title: Option<String>,
    focus: Option<String>,
    domain: Option<String>,
    domains: Option<Vec<String>>,
    state: &AppState,
) -> Result<String, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    // Clean helper shared by identity fields and domain writes
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

    // Clean identity fields (written to user_entity only, NOT config)
    let uname = clean(name);
    let ucompany = clean(company);
    let utitle = clean(title);
    let ufocus = clean(focus);

    // Domain/domains stay in Config
    if domain.is_some() || domains.is_some() {
        crate::state::create_or_update_config(state, |config| {
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
    }

    // Write identity fields to user_entity table only (AC2: no dual storage)
    state.db_write(move |db| {
        // Upsert: create row if missing, then update
        db.conn_ref()
            .execute(
                "INSERT INTO user_entity (id) VALUES (1) ON CONFLICT(id) DO NOTHING",
                [],
            )
            .map_err(|e| e.to_string())?;
        db.conn_ref()
            .execute(
                "UPDATE user_entity SET name = ?1, company = ?2, title = ?3, focus = ?4, updated_at = CURRENT_TIMESTAMP WHERE id = 1",
                rusqlite::params![uname, ucompany, utitle, ufocus],
            )
            .map_err(|e| e.to_string())?;

        // Sync company name to internal root account entity if it changed
        if let Some(ref company_name) = ucompany {
            if let Ok(Some(root)) = db.get_internal_root_account() {
                if root.name != *company_name {
                    let _ = db.update_account_field(&root.id, "name", company_name);
                }
            }
        }
        Ok(())
    }).await?;

    Ok("ok".to_string())
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::validate_ai_model_choice;

    #[test]
    fn validates_background_ai_model_choice() {
        assert!(validate_ai_model_choice("background", "haiku").is_ok());
    }

    #[test]
    fn rejects_invalid_ai_model_choice() {
        assert!(validate_ai_model_choice("background", "unknown").is_err());
        assert!(validate_ai_model_choice("invalid", "haiku").is_err());
    }

    // Daily AI budget settings validation
    #[test]
    fn daily_ai_budget_accepts_valid_tiers() {
        // These are the only valid tiers
        for budget in [50_000u32, 100_000, 250_000] {
            assert!(
                super::set_daily_ai_budget_validate(budget),
                "Expected {} to be valid",
                budget
            );
        }
    }

    #[test]
    fn daily_ai_budget_rejects_invalid_values() {
        for budget in [0u32, 10, 1000, 50_001, 99_999, 500_000] {
            assert!(
                !super::set_daily_ai_budget_validate(budget),
                "Expected {} to be invalid",
                budget
            );
        }
    }

    #[test]
    fn migration_existing_config_with_old_hygiene_budget_deserializes() {
        // Simulate a config JSON from before has hygiene_ai_budget=10
        // but no daily_ai_token_budget. Should deserialize with the serde default.
        let json = r#"{
            "workspacePath": "/tmp/test",
            "hygieneAiBudget": 10
        }"#;
        let config: crate::types::Config = serde_json::from_str(json).unwrap();
        // Old field is preserved (serde default 0 if missing, or actual value if present)
        assert_eq!(config.hygiene_ai_budget, 10);
        // New field should use the serde default (50_000)
        assert_eq!(config.daily_ai_token_budget, crate::pty::DEFAULT_DAILY_AI_TOKEN_BUDGET);
    }
}

/// Set multiple user domains with reclassification of people and meetings.
pub async fn set_user_domains(
    ctx: &ServiceContext<'_>,
    domains: &str,
    state: &AppState,
) -> Result<Config, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
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
        let _ = state
            .db_write(move |db| {
                // Always run people reclassification; log count for observability.
                match db.reclassify_people_for_domains(&parsed) {
                    Ok(n) => {
                        if n > 0 {
                            log::info!("Reclassified {} people after domain change", n);
                        }
                    }
                    Err(e) => log::warn!("People reclassification failed: {}", e),
                }

                // Always run meeting reclassification after a domain
                // save, regardless of whether any *people* rows changed. When
                // people are already correctly classified but meetings were
                // previously mis-typed (e.g., a past all-internal meeting
                // stuck on "customer" due to a title-slug match), we still
                // need to sweep the stale meeting rows.
                match db.reclassify_meeting_types_from_attendees() {
                    Ok(m) => {
                        if m > 0 {
                            log::info!("Reclassified {} meetings after domain change", m);
                        }
                    }
                    Err(e) => log::warn!("Meeting reclassification failed: {}", e),
                }
                Ok(())
            })
            .await;
    }

    Ok(config)
}
