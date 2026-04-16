use super::*;
use rusqlite::OptionalExtension;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageBreakdownCount {
    pub label: String,
    pub count: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageTrendPoint {
    pub date: String,
    pub call_count: u32,
    pub estimated_prompt_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiUsageDiagnostics {
    pub today: AiUsageTrendPoint,
    pub operation_counts: Vec<AiUsageBreakdownCount>,
    pub model_counts: Vec<AiUsageBreakdownCount>,
    pub budget_limit: u32,
    pub budget_remaining: u32,
    pub estimated_daily_token_budget: u32,
    pub estimated_token_budget_remaining: u32,
    pub background_pause: crate::pty::BackgroundAiPauseStatus,
    pub trend: Vec<AiUsageTrendPoint>,
}

#[tauri::command]
pub async fn get_processing_history(
    limit: Option<i32>,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::DbProcessingLog>, String> {
    let lim = limit.unwrap_or(50);
    state
        .db_read(move |db| db.get_processing_log(lim).map_err(|e| e.to_string()))
        .await
}

// =============================================================================
// Onboarding: Demo Data
// =============================================================================

/// Install demo data for first-run experience (I56).
///
/// Seeds curated accounts, actions, meetings, and people marked `is_demo = 1`.
/// Writes fixture files if a workspace path is configured.
#[tauri::command]
pub async fn install_demo_data(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .and_then(|c| {
            if c.workspace_path.is_empty() {
                None
            } else {
                Some(c.workspace_path.clone())
            }
        });

    let ws = workspace_path.clone();
    state
        .db_write(move |db| crate::demo::install_demo(db, ws.as_deref().map(std::path::Path::new)))
        .await?;

    Ok("Demo data installed".into())
}

/// Clear all demo data and reset demo mode (I56).
#[tauri::command]
pub async fn clear_demo_data(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .and_then(|c| {
            if c.workspace_path.is_empty() {
                None
            } else {
                Some(c.workspace_path.clone())
            }
        });

    let ws = workspace_path.clone();
    state
        .db_write(move |db| crate::demo::clear_demo(db, ws.as_deref().map(std::path::Path::new)))
        .await?;

    Ok("Demo data cleared".into())
}

/// Get app-level state (demo mode, tour, wizard progress).
#[tauri::command]
pub async fn get_app_state(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::demo::AppStateRow, String> {
    state.db_read(crate::demo::get_app_state).await
}

/// Mark the post-wizard tour as completed.
#[tauri::command]
pub async fn set_tour_completed(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    state.db_write(crate::demo::set_tour_completed).await?;
    Ok("Tour completed".into())
}

/// Mark the wizard as completed with current timestamp.
#[tauri::command]
pub async fn set_wizard_completed(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    state.db_write(crate::demo::set_wizard_completed).await?;
    Ok("Wizard completed".into())
}

/// Set wizard last step for mid-wizard resume.
#[tauri::command]
pub async fn set_wizard_step(
    step: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    state
        .db_write(move |db| crate::demo::set_wizard_step(db, &step))
        .await?;
    Ok("Wizard step saved".into())
}

// =============================================================================
// Onboarding: Populate Workspace (I57)
// =============================================================================

/// Create account/project folders and save user domain during onboarding.
///
/// For each account: creates `Accounts/{name}/` and upserts a minimal DbAccount
/// record (bridge pattern fires `ensure_entity_for_account` automatically).
/// For each project: creates `Projects/{name}/` (filesystem only, no SQLite — I50).
/// DB errors are non-fatal; folder creation is the primary value.
#[tauri::command]
pub async fn populate_workspace(
    accounts: Vec<String>,
    projects: Vec<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    // 1. Get workspace path
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    let workspace = std::path::Path::new(&workspace_path);
    let now = chrono::Utc::now().to_rfc3339();

    // 3. Process accounts: filesystem first, collect valid names
    let mut valid_account_names: Vec<String> = Vec::new();
    for name in &accounts {
        let name = match crate::util::validate_entity_name(name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Skipping invalid account name '{}': {}", name, e);
                continue;
            }
        };

        // Create folder + directory template (ADR-0059, idempotent)
        let account_dir = workspace.join("Accounts").join(name);
        if let Err(e) = std::fs::create_dir_all(&account_dir) {
            log::warn!("Failed to create account dir '{}': {}", name, e);
            continue;
        }
        if let Err(e) = crate::util::bootstrap_entity_directory(&account_dir, name, "account") {
            log::warn!("Failed to bootstrap account template '{}': {}", name, e);
        }
        valid_account_names.push(name.to_string());
    }
    let account_count = valid_account_names.len();

    // 4. Process projects: filesystem first, collect valid entries
    let mut valid_projects: Vec<crate::db::DbProject> = Vec::new();
    for name in &projects {
        let name = match crate::util::validate_entity_name(name) {
            Ok(n) => n,
            Err(e) => {
                log::warn!("Skipping invalid project name '{}': {}", name, e);
                continue;
            }
        };

        // Create folder + directory template (ADR-0059, idempotent)
        let project_dir = workspace.join("Projects").join(name);
        if let Err(e) = std::fs::create_dir_all(&project_dir) {
            log::warn!("Failed to create project dir '{}': {}", name, e);
        }
        if let Err(e) = crate::util::bootstrap_entity_directory(&project_dir, name, "project") {
            log::warn!("Failed to bootstrap project template '{}': {}", name, e);
        }

        let slug = crate::util::slugify(name);
        valid_projects.push(crate::db::DbProject {
            id: slug,
            name: name.to_string(),
            status: "active".to_string(),
            milestone: None,
            owner: None,
            target_date: None,
            tracker_path: Some(format!("Projects/{}", name)),
            parent_id: None,
            updated_at: now.clone(),
            archived: false,
            keywords: None,
            keywords_extracted_at: None,
            metadata: None,
            description: None,
            milestones: None,
            notes: None,
        });
    }
    let project_count = valid_projects.len();

    // Batch DB operations
    let engine = std::sync::Arc::clone(&state.signals.engine);
    let wp = workspace_path.clone();
    let _ = state
        .db_write(move |db| {
            let workspace = std::path::Path::new(&wp);
            // Upsert accounts
            for name in &valid_account_names {
                let slug = crate::util::slugify(name);
                let existing = db.get_account(&slug).ok().flatten();
                let db_account = crate::db::DbAccount {
                    id: slug,
                    name: name.to_string(),
                    lifecycle: existing.as_ref().and_then(|e| e.lifecycle.clone()),
                    arr: existing.as_ref().and_then(|e| e.arr),
                    health: existing.as_ref().and_then(|e| e.health.clone()),
                    contract_start: existing.as_ref().and_then(|e| e.contract_start.clone()),
                    contract_end: existing.as_ref().and_then(|e| e.contract_end.clone()),
                    nps: existing.as_ref().and_then(|e| e.nps),
                    tracker_path: Some(format!("Accounts/{}", name)),
                    parent_id: existing.as_ref().and_then(|e| e.parent_id.clone()),
                    account_type: existing
                        .as_ref()
                        .map(|e| e.account_type.clone())
                        .unwrap_or(crate::db::AccountType::Customer),
                    updated_at: now.clone(),
                    archived: existing.as_ref().map(|e| e.archived).unwrap_or(false),
                    keywords: existing.as_ref().and_then(|e| e.keywords.clone()),
                    keywords_extracted_at: existing
                        .as_ref()
                        .and_then(|e| e.keywords_extracted_at.clone()),
                    metadata: existing.as_ref().and_then(|e| e.metadata.clone()),
                    commercial_stage: existing.as_ref().and_then(|e| e.commercial_stage.clone()),
                    // I644 fact columns
                    arr_range_low: existing.as_ref().and_then(|e| e.arr_range_low),
                    arr_range_high: existing.as_ref().and_then(|e| e.arr_range_high),
                    renewal_likelihood: existing.as_ref().and_then(|e| e.renewal_likelihood),
                    renewal_likelihood_source: existing
                        .as_ref()
                        .and_then(|e| e.renewal_likelihood_source.clone()),
                    renewal_likelihood_updated_at: existing
                        .as_ref()
                        .and_then(|e| e.renewal_likelihood_updated_at.clone()),
                    renewal_model: existing.as_ref().and_then(|e| e.renewal_model.clone()),
                    renewal_pricing_method: existing
                        .as_ref()
                        .and_then(|e| e.renewal_pricing_method.clone()),
                    support_tier: existing.as_ref().and_then(|e| e.support_tier.clone()),
                    support_tier_source: existing
                        .as_ref()
                        .and_then(|e| e.support_tier_source.clone()),
                    support_tier_updated_at: existing
                        .as_ref()
                        .and_then(|e| e.support_tier_updated_at.clone()),
                    active_subscription_count: existing
                        .as_ref()
                        .and_then(|e| e.active_subscription_count),
                    growth_potential_score: existing
                        .as_ref()
                        .and_then(|e| e.growth_potential_score),
                    growth_potential_score_source: existing
                        .as_ref()
                        .and_then(|e| e.growth_potential_score_source.clone()),
                    icp_fit_score: existing.as_ref().and_then(|e| e.icp_fit_score),
                    icp_fit_score_source: existing
                        .as_ref()
                        .and_then(|e| e.icp_fit_score_source.clone()),
                    primary_product: existing.as_ref().and_then(|e| e.primary_product.clone()),
                    customer_status: existing.as_ref().and_then(|e| e.customer_status.clone()),
                    customer_status_source: existing
                        .as_ref()
                        .and_then(|e| e.customer_status_source.clone()),
                    customer_status_updated_at: existing
                        .as_ref()
                        .and_then(|e| e.customer_status_updated_at.clone()),
                    // I644 dashboard.json fields
                    company_overview: existing.as_ref().and_then(|e| e.company_overview.clone()),
                    strategic_programs: existing
                        .as_ref()
                        .and_then(|e| e.strategic_programs.clone()),
                    notes: existing.as_ref().and_then(|e| e.notes.clone()),
                    user_health_sentiment: existing.as_ref().and_then(|e| e.user_health_sentiment.clone()),
                    sentiment_set_at: existing.as_ref().and_then(|e| e.sentiment_set_at.clone()),
                };
                if let Err(e) = crate::services::mutations::upsert_account(db, &engine, &db_account)
                {
                    log::warn!("Failed to upsert account '{}': {}", name, e);
                }
            }
            // Upsert projects + write dashboard files
            for db_project in &valid_projects {
                if let Err(e) = crate::services::mutations::upsert_project(db, &engine, db_project)
                {
                    log::warn!("Failed to upsert project '{}': {}", db_project.name, e);
                }
                let json = crate::projects::default_project_json(db_project);
                let _ = crate::projects::write_project_json(workspace, db_project, Some(&json), db);
                let _ =
                    crate::projects::write_project_markdown(workspace, db_project, Some(&json), db);
            }
            Ok(())
        })
        .await;

    Ok(format!(
        "Created {} accounts, {} projects",
        account_count, project_count
    ))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingPrimingCard {
    pub id: String,
    pub title: String,
    pub start_time: Option<String>,
    pub day_label: String,
    pub suggested_entity_id: Option<String>,
    pub suggested_entity_name: Option<String>,
    pub suggested_action: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingPrimingContext {
    pub google_connected: bool,
    pub cards: Vec<OnboardingPrimingCard>,
    pub prompt: String,
}

#[tauri::command]
pub async fn get_onboarding_priming_context(
    state: State<'_, Arc<AppState>>,
) -> Result<OnboardingPrimingContext, String> {
    let config = state
        .config
        .read()
        .clone()
        .ok_or("Config not loaded")?;
    let user_domains = config.resolved_user_domains();

    let access_token = match crate::google_api::get_valid_access_token().await {
        Ok(token) => token,
        Err(_) => {
            return Ok(OnboardingPrimingContext {
                google_connected: false,
                cards: Vec::new(),
                prompt: "Connect Google Calendar to preview your first full briefing. You can still generate a first run now."
                    .to_string(),
            })
        }
    };

    let today = chrono::Local::now().date_naive();
    let tomorrow = today + chrono::Duration::days(1);
    let raw_events = crate::google_api::calendar::fetch_events(&access_token, today, tomorrow)
        .await
        .map_err(|e| format!("Calendar fetch failed: {}", e))?;

    let (hints, internal_root) = state
        .db_read(|db| {
            Ok((
                crate::helpers::build_entity_hints(db),
                db.get_internal_root_account().ok().flatten(),
            ))
        })
        .await?;

    // Pre-classify all meetings and collect account hints for batch DB lookup
    let mut classified: Vec<(
        crate::google_api::classify::ClassifiedMeeting,
        crate::types::CalendarEvent,
        String,
        Option<String>,
    )> = Vec::new();
    for raw in raw_events.iter().filter(|e| !e.is_all_day).take(8) {
        let cm = crate::google_api::classify::classify_meeting_multi(raw, &user_domains, &hints);
        let event = cm.to_calendar_event();
        let start = event.start.with_timezone(&chrono::Local);
        let day_label = if start.date_naive() == today {
            "Today".to_string()
        } else if start.date_naive() == tomorrow {
            "Tomorrow".to_string()
        } else {
            start.format("%a").to_string()
        };
        let account_hint = cm.account().map(|s| s.to_string());
        classified.push((cm, event, day_label, account_hint));
    }

    // Batch-resolve account hints in a single DB read
    let account_hints: Vec<Option<String>> =
        classified.iter().map(|(_, _, _, h)| h.clone()).collect();
    let resolved_accounts = state
        .db_read(move |db| {
            let mut results = Vec::new();
            for hint in &account_hints {
                if let Some(ref name) = hint {
                    if let Ok(Some(account)) = db.get_account_by_name(name) {
                        results.push(Some((account.id.clone(), account.name.clone())));
                    } else {
                        results.push(None);
                    }
                } else {
                    results.push(None);
                }
            }
            Ok(results)
        })
        .await?;

    let mut cards = Vec::new();
    for (i, (_cm, event, day_label, _account_hint)) in classified.into_iter().enumerate() {
        let mut suggested_entity_id = None;
        let mut suggested_entity_name = None;

        if let Some(Some((ref id, ref name))) = resolved_accounts.get(i) {
            suggested_entity_id = Some(id.clone());
            suggested_entity_name = Some(name.clone());
        } else if matches!(
            event.meeting_type,
            crate::types::MeetingType::Internal
                | crate::types::MeetingType::TeamSync
                | crate::types::MeetingType::OneOnOne
        ) {
            if let Some(ref root) = internal_root {
                suggested_entity_id = Some(root.id.clone());
                suggested_entity_name = Some(root.name.clone());
            }
        }

        let suggested_action = match event.meeting_type {
            crate::types::MeetingType::Customer
            | crate::types::MeetingType::Qbr
            | crate::types::MeetingType::Partnership => {
                "Open context and prep follow-up questions".to_string()
            }
            crate::types::MeetingType::Internal
            | crate::types::MeetingType::TeamSync
            | crate::types::MeetingType::OneOnOne => {
                "Capture decisions and owners in Inbox".to_string()
            }
            _ => "Review context before kickoff".to_string(),
        };

        cards.push(OnboardingPrimingCard {
            id: event.id,
            title: event.title,
            start_time: Some(event.start.with_timezone(&chrono::Local).to_rfc3339()),
            day_label,
            suggested_entity_id,
            suggested_entity_name,
            suggested_action,
        });
    }

    Ok(OnboardingPrimingContext {
        google_connected: true,
        cards,
        prompt:
            "Prime your first briefing by reviewing high-priority meetings and running a full 'today' workflow preview."
                .to_string(),
    })
}

// =============================================================================
// Onboarding: Claude Code Status (I79)
// =============================================================================

/// Check whether Claude Code CLI is installed and authenticated.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeStatus {
    pub installed: bool,
    pub authenticated: bool,
    pub node_installed: bool,
}

#[derive(Debug, Clone)]
struct ClaudeStatusCacheEntry {
    status: ClaudeStatus,
    checked_at: std::time::Instant,
}

static CLAUDE_STATUS_CACHE: OnceLock<Mutex<Option<ClaudeStatusCacheEntry>>> = OnceLock::new();

fn claude_status_cache() -> &'static Mutex<Option<ClaudeStatusCacheEntry>> {
    CLAUDE_STATUS_CACHE.get_or_init(|| Mutex::new(None))
}

/// Return in-memory command latency rollups for diagnostics/devtools.
#[tauri::command]
pub fn get_latency_rollups() -> crate::latency::LatencyRollupsPayload {
    crate::latency::get_rollups()
}

#[tauri::command]
pub async fn get_ai_usage_diagnostics(
    state: State<'_, Arc<AppState>>,
) -> Result<AiUsageDiagnostics, String> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let budget_limit = state.hygiene.budget.daily_limit;
    let budget_remaining = budget_limit.saturating_sub(state.hygiene.budget.used_today());
    let background_pause = crate::pty::current_background_ai_pause_status();

    state
        .db_read(move |db| {
            let ledger: crate::pty::AiUsageLedger = db
                .conn_ref()
                .query_row(
                    "SELECT value_json FROM app_state_kv WHERE key = ?1",
                    rusqlite::params![crate::pty::AI_USAGE_DAILY_KEY],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(|e| e.to_string())?
                .and_then(|json| serde_json::from_str(&json).ok())
                .unwrap_or_default();

            let mut trend = ledger
                .days
                .iter()
                .map(
                    |(date, day): (&String, &crate::pty::AiUsageDay)| AiUsageTrendPoint {
                        date: date.clone(),
                        call_count: day.call_count,
                        estimated_prompt_tokens: day.estimated_prompt_tokens,
                        estimated_output_tokens: day.estimated_output_tokens,
                        estimated_total_tokens: day.estimated_prompt_tokens
                            + day.estimated_output_tokens,
                        total_duration_ms: day.total_duration_ms,
                    },
                )
                .collect::<Vec<_>>();
            trend.sort_by(|a, b| a.date.cmp(&b.date));
            let trend = if trend.len() > 7 {
                trend[trend.len() - 7..].to_vec()
            } else {
                trend
            };

            let today_usage = ledger.days.get(&today).cloned().unwrap_or_default();
            let mut operation_counts = today_usage
                .operation_counts
                .iter()
                .map(|(label, count): (&String, &u32)| AiUsageBreakdownCount {
                    label: label.clone(),
                    count: *count,
                })
                .collect::<Vec<_>>();
            if operation_counts.is_empty() {
                operation_counts = today_usage
                    .call_sites
                    .iter()
                    .map(|(label, count): (&String, &u32)| AiUsageBreakdownCount {
                        label: label.clone(),
                        count: *count,
                    })
                    .collect::<Vec<_>>();
            }
            operation_counts
                .sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));

            let mut model_counts = today_usage
                .model_counts
                .iter()
                .map(|(label, count): (&String, &u32)| AiUsageBreakdownCount {
                    label: label.clone(),
                    count: *count,
                })
                .collect::<Vec<_>>();
            model_counts.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.label.cmp(&b.label)));

            let estimated_budget_remaining = crate::pty::ESTIMATED_DAILY_TOKEN_BUDGET
                .saturating_sub(
                    today_usage.estimated_prompt_tokens + today_usage.estimated_output_tokens,
                );

            Ok(AiUsageDiagnostics {
                today: AiUsageTrendPoint {
                    date: today,
                    call_count: today_usage.call_count,
                    estimated_prompt_tokens: today_usage.estimated_prompt_tokens,
                    estimated_output_tokens: today_usage.estimated_output_tokens,
                    estimated_total_tokens: today_usage.estimated_prompt_tokens
                        + today_usage.estimated_output_tokens,
                    total_duration_ms: today_usage.total_duration_ms,
                },
                operation_counts,
                model_counts,
                budget_limit,
                budget_remaining,
                estimated_daily_token_budget: crate::pty::ESTIMATED_DAILY_TOKEN_BUDGET,
                estimated_token_budget_remaining: estimated_budget_remaining,
                background_pause,
                trend,
            })
        })
        .await
}

/// Cache Claude status checks to avoid shelling out on every focus event.
///
/// The subprocess spawn (`claude --print hello`) runs on a blocking thread
/// via `spawn_blocking` so it never ties up a Tauri IPC thread.
#[tauri::command]
pub async fn check_claude_status() -> ClaudeStatus {
    let started = std::time::Instant::now();

    // Dev override: return mocked status without spawning subprocess
    if cfg!(debug_assertions) {
        let ov = DEV_CLAUDE_OVERRIDE.load(Ordering::Relaxed);
        if ov != 0 {
            log_command_latency("check_claude_status", started, READ_CMD_LATENCY_BUDGET_MS);
            return match ov {
                1 => ClaudeStatus {
                    installed: true,
                    authenticated: true,
                    node_installed: true,
                },
                2 => ClaudeStatus {
                    installed: false,
                    authenticated: false,
                    node_installed: false,
                },
                3 => ClaudeStatus {
                    installed: true,
                    authenticated: false,
                    node_installed: true,
                },
                _ => ClaudeStatus {
                    installed: false,
                    authenticated: false,
                    node_installed: false,
                },
            };
        }
    }

    let cache = claude_status_cache();
    let ttl = std::time::Duration::from_secs(CLAUDE_STATUS_CACHE_TTL_SECS);

    // Fast path: return cached result without blocking
    {
        let guard = cache.lock();
        if let Some(entry) = guard.as_ref() {
            if entry.checked_at.elapsed() < ttl {
                log_command_latency("check_claude_status", started, READ_CMD_LATENCY_BUDGET_MS);
                return entry.status.clone();
            }
        }
    }

    // Slow path: spawn subprocess on a blocking thread so IPC stays responsive
    let status = tokio::task::spawn_blocking(|| {
        let installed = crate::pty::PtyManager::is_claude_available();
        let authenticated = if installed {
            crate::pty::PtyManager::is_claude_authenticated().unwrap_or(false)
        } else {
            false
        };
        let node_installed = crate::util::resolve_node_binary().is_some();
        ClaudeStatus {
            installed,
            authenticated,
            node_installed,
        }
    })
    .await
    .unwrap_or(ClaudeStatus {
        installed: false,
        authenticated: false,
        node_installed: false,
    });

    *cache.lock() = Some(ClaudeStatusCacheEntry {
        status: status.clone(),
        checked_at: std::time::Instant::now(),
    });

    log_command_latency("check_claude_status", started, READ_CMD_LATENCY_BUDGET_MS);
    status
}

/// Open the Claude sign-in page in the user's default browser.
///
/// Claude Code stores credentials in the macOS Keychain after OAuth completes
/// on the website. After the user signs in, clicking "Check again" will pick
/// up the new keychain entry.
///
/// Also clears the status cache so the next `check_claude_status` call
/// performs a fresh probe.
#[tauri::command]
pub fn launch_claude_login() -> Result<(), String> {
    // Clear cached status so the next check returns a fresh result.
    *claude_status_cache().lock() = None;

    open::that("https://claude.ai/login").map_err(|e| e.to_string())
}

/// Clear the Claude status TTL cache so the next `check_claude_status` call
/// performs a fresh probe. Called by the "Re-check" button in onboarding so
/// installing Node/Claude while the app is running is detected immediately.
#[tauri::command]
pub fn clear_claude_status_cache() {
    *claude_status_cache().lock() = None;
}

// =============================================================================
// Onboarding: Claude CLI Installer (DOS-57) + Node.js Auto-Installer (DOS-65)
// =============================================================================

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallProgress {
    step: String,
    status: String,
    message: String,
}

static INSTALL_IN_PROGRESS: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

// Pinned Node.js LTS version + checksum for integrity verification (DOS-65)
const NODE_VERSION: &str = "22.15.0";
const NODE_PKG_SHA256: &str =
    "0bc096a279cd7cbc57bf3a6c6570f3ca03b3ab684d1878a1425919e9b6b76317";

/// Download and install Node.js via the official macOS .pkg installer.
///
/// Uses `osascript` to invoke the macOS installer with admin privileges,
/// which shows the standard system auth dialog. Emits progress events on
/// `install-claude-progress`.
fn install_nodejs_blocking(app: &tauri::AppHandle) -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let pkg_name = format!("node-v{}.pkg", NODE_VERSION);
    let download_url = format!("https://nodejs.org/dist/v{}/{}", NODE_VERSION, pkg_name);

    log::info!("DOS-65: downloading Node.js {} from {}", NODE_VERSION, download_url);

    // --- Download .pkg to temp file ---
    let _ = app.emit(
        "install-claude-progress",
        InstallProgress {
            step: "downloading_node".to_string(),
            status: "running".to_string(),
            message: "Downloading Node.js...".to_string(),
        },
    );

    let response = reqwest::blocking::get(&download_url).map_err(|e| {
        let msg = format!("Failed to download Node.js — check your internet connection: {}", e);
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "error".to_string(),
                status: "error".to_string(),
                message: msg.clone(),
            },
        );
        msg
    })?;

    if !response.status().is_success() {
        let msg = format!(
            "Failed to download Node.js — server returned {}",
            response.status()
        );
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "error".to_string(),
                status: "error".to_string(),
                message: msg.clone(),
            },
        );
        return Err(msg);
    }

    let bytes = response.bytes().map_err(|e| {
        let msg = format!("Failed to download Node.js — connection interrupted: {}", e);
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "error".to_string(),
                status: "error".to_string(),
                message: msg.clone(),
            },
        );
        msg
    })?;

    // --- Verify SHA-256 checksum ---
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = format!("{:x}", hasher.finalize());
    log::info!("DOS-65: download SHA-256 = {}", digest);

    if digest != NODE_PKG_SHA256 {
        let msg = "Node.js download integrity check failed — please try again".to_string();
        log::error!(
            "DOS-65: checksum mismatch: expected {} got {}",
            NODE_PKG_SHA256,
            digest
        );
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "error".to_string(),
                status: "error".to_string(),
                message: msg.clone(),
            },
        );
        return Err(msg);
    }

    // --- Write to temp file ---
    let tmp_dir = tempfile::tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
    let pkg_path = tmp_dir.path().join(&pkg_name);
    std::fs::write(&pkg_path, &bytes)
        .map_err(|e| format!("Failed to write Node.js installer: {}", e))?;

    // --- Run macOS installer with admin privileges via osascript ---
    let _ = app.emit(
        "install-claude-progress",
        InstallProgress {
            step: "installing_node".to_string(),
            status: "running".to_string(),
            message: "Installing Node.js (admin password required)...".to_string(),
        },
    );

    let script = format!(
        r#"do shell script "installer -pkg '{}' -target /" with administrator privileges"#,
        pkg_path.display()
    );
    log::info!("DOS-65: running macOS installer via osascript");

    let output = std::process::Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| format!("Failed to launch installer: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = if stderr.contains("User canceled")
            || stderr.contains("user canceled")
            || stderr.contains("-128")
        {
            "Installation cancelled — admin privileges required to install Node.js".to_string()
        } else {
            format!("Node.js installation failed: {}", stderr.trim())
        };
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "error".to_string(),
                status: "error".to_string(),
                message: msg.clone(),
            },
        );
        return Err(msg);
    }

    // --- Clear cached binary lookups so resolve_node_binary() re-probes ---
    crate::util::clear_node_binary_cache();

    // --- Verify Node is now available ---
    if crate::util::resolve_node_binary().is_none() {
        let msg = "Node.js installer completed but node binary not found on PATH".to_string();
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "error".to_string(),
                status: "error".to_string(),
                message: msg.clone(),
            },
        );
        return Err(msg);
    }

    log::info!("DOS-65: Node.js {} installed successfully", NODE_VERSION);
    Ok(())
}

/// Install Claude Code CLI, auto-installing Node.js first if needed.
///
/// When Node.js is missing, downloads and installs the official macOS .pkg
/// (with SHA-256 verification and admin auth dialog), then proceeds to
/// install Claude Code via npm. Emits `install-claude-progress` events
/// for frontend progress UI.
#[tauri::command]
pub async fn install_claude_cli(app: tauri::AppHandle) -> Result<(), String> {
    use std::sync::atomic::Ordering;

    // Single-flight guard — prevents concurrent installs
    if INSTALL_IN_PROGRESS
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("Installation already in progress".to_string());
    }

    // NOTE: If `spawn_blocking` panics, the guard won't reset. Acceptable for
    // a user-initiated one-shot action — restart clears it.
    let result = tokio::task::spawn_blocking(move || {
        // Step 1: Resolve npm binary — install Node.js if missing
        let npm_path = match crate::util::resolve_npm_binary() {
            Some(path) => path,
            None => {
                // Node.js not found — auto-install it (DOS-65)
                install_nodejs_blocking(&app)?;

                // Re-resolve npm after Node install
                match crate::util::resolve_npm_binary() {
                    Some(path) => path,
                    None => {
                        let msg =
                            "Node.js installed but npm not found — please restart and try again"
                                .to_string();
                        let _ = app.emit(
                            "install-claude-progress",
                            InstallProgress {
                                step: "error".to_string(),
                                status: "error".to_string(),
                                message: msg.clone(),
                            },
                        );
                        return Err(msg);
                    }
                }
            }
        };

        // Step 2: Install Claude Code CLI
        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "installing_claude".to_string(),
                status: "running".to_string(),
                message: "Installing Claude Code CLI...".to_string(),
            },
        );

        let output = std::process::Command::new(&npm_path)
            .args(["install", "-g", "@anthropic-ai/claude-code"])
            .output()
            .map_err(|e| format!("Failed to run npm: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("npm install failed: {}", stderr.trim());
            let _ = app.emit(
                "install-claude-progress",
                InstallProgress {
                    step: "error".to_string(),
                    status: "error".to_string(),
                    message: msg.clone(),
                },
            );
            return Err(msg);
        }

        // Step 3: Clear status cache so next check picks up the new install
        *claude_status_cache().lock() = None;

        let _ = app.emit(
            "install-claude-progress",
            InstallProgress {
                step: "complete".to_string(),
                status: "done".to_string(),
                message: "Claude Code CLI installed successfully!".to_string(),
            },
        );

        Ok(())
    })
    .await
    .map_err(|e| format!("Install task failed: {}", e))?;

    INSTALL_IN_PROGRESS.store(false, Ordering::SeqCst);

    result
}

// =============================================================================
// Onboarding: Inbox Training Sample (I78)
// =============================================================================

/// Copy a bundled sample meeting notes file into _inbox/ for onboarding training.
///
/// Returns the filename of the installed sample.
#[tauri::command]
pub fn install_inbox_sample(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    let workspace_path = state
        .config
        .read()
        .as_ref()
        .map(|c| c.workspace_path.clone())
        .ok_or("No workspace configured")?;

    let workspace = std::path::Path::new(&workspace_path);
    let inbox_dir = workspace.join("_inbox");

    // Ensure _inbox/ exists
    if !inbox_dir.exists() {
        std::fs::create_dir_all(&inbox_dir)
            .map_err(|e| format!("Failed to create _inbox: {}", e))?;
    }

    let filename = "sample-meeting-notes.md";
    let content = include_str!("../../resources/sample-meeting-notes.md");
    let dest = inbox_dir.join(filename);

    std::fs::write(&dest, content).map_err(|e| format!("Failed to write sample file: {}", e))?;

    Ok(filename.to_string())
}

/// Get frequent same-domain correspondents from Gmail sent mail.
#[tauri::command]
pub async fn get_frequent_correspondents(
    user_email: String,
    _state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::google_api::gmail::FrequentCorrespondent>, String> {
    let token =
        crate::google_api::load_token().map_err(|e| format!("Google not connected: {}", e))?;

    crate::google_api::gmail::fetch_frequent_correspondents(&token.token, &user_email, 10)
        .await
        .map_err(|e| format!("Failed to fetch correspondents: {}", e))
}

// =============================================================================
// Dev Tools
// =============================================================================

/// Apply a dev scenario (reset, full, no_connectors, pipeline).
///
/// Returns an error in release builds. In debug builds, delegates to
/// `devtools::apply_scenario` which orchestrates the scenario switch.
/// Async because it must reinitialize the DB connection pool after
/// entering dev mode (the sync DB swaps immediately, but the async
/// `DbService` readers/writers need to be reopened at the dev path).
#[tauri::command]
pub async fn dev_apply_scenario(
    scenario: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    let result = crate::devtools::apply_scenario(&scenario, &state)?;

    // Reinitialize the async DB connection pool at the dev path
    if let Err(e) = state.reinit_db_service().await {
        log::warn!(
            "Failed to reinit db_service after dev_apply_scenario: {}",
            e
        );
    }

    Ok(result)
}

/// Get current dev state for the dev tools panel.
///
/// Returns an error in release builds. In debug builds, returns counts
/// and status for config, database, today data, and Google auth.
#[tauri::command]
pub fn dev_get_state(state: State<'_, Arc<AppState>>) -> Result<crate::devtools::DevState, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::get_dev_state(&state)
}

/// Daily briefing — mechanical delivery only (no AI).
///
/// Requires `simulate_briefing` scenario first. Delivers schedule, actions,
/// preps, emails, manifest from the seeded today-directive.json.
#[tauri::command]
pub fn dev_run_today_mechanical(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_today_mechanical(&state)
}

/// Daily briefing — full pipeline with AI enrichment.
///
/// Requires `simulate_briefing` scenario + Claude Code CLI installed.
/// Mechanical delivery + enrich_emails, enrich_preps, enrich_briefing.
#[tauri::command]
pub fn dev_run_today_full(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::run_today_full(&state)
}

/// Restore from dev mode to live mode (I298).
///
/// Deactivates dev DB isolation, reopens the live database, reinitializes the
/// async DB connection pool, and restores the original workspace path.
#[tauri::command]
pub async fn dev_restore_live(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    let result = crate::devtools::restore_live(&state)?;

    // Reinitialize the async DB connection pool at the live path
    if let Err(e) = state.reinit_db_service().await {
        log::warn!("Failed to reinit db_service after dev_restore_live: {}", e);
    }

    Ok(result)
}

/// Purge all known mock/dev data from the current database (I298).
///
/// Removes exact mock IDs seeded by devtools scenarios. Safe for the live DB.
#[tauri::command]
pub fn dev_purge_mock_data(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::purge_mock_data(&state)
}

/// Delete stale dev artifact files from disk (I298).
///
/// Removes dailyos-dev.db and optionally ~/Documents/DailyOS-dev/.
#[tauri::command]
pub fn dev_clean_artifacts(include_workspace: bool) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    crate::devtools::clean_dev_artifacts(include_workspace)
}

/// Set dev auth overrides for Claude and Google status checks.
///
/// 0 = real check (no override), 1 = authenticated/ready,
/// 2 = not installed/not configured, 3 = installed-not-authed / token expired.
#[tauri::command]
pub fn dev_set_auth_override(claude_mode: u8, google_mode: u8) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    DEV_CLAUDE_OVERRIDE.store(claude_mode, Ordering::Relaxed);
    DEV_GOOGLE_OVERRIDE.store(google_mode, Ordering::Relaxed);
    Ok(format!(
        "Auth overrides set — Claude: {}, Google: {}",
        claude_mode, google_mode
    ))
}

/// Apply a named onboarding scenario: reset wizard state + set auth overrides.
///
/// Scenarios: fresh, auth_ready, no_claude, claude_unauthed, no_google, google_expired, nothing_works.
#[tauri::command]
pub async fn dev_onboarding_scenario(
    scenario: String,
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    if !cfg!(debug_assertions) {
        return Err("Dev tools not available in release builds".into());
    }
    let result = crate::devtools::onboarding_scenario(&scenario, &state)?;

    // Reinitialize the async DB connection pool at the dev path
    if let Err(e) = state.reinit_db_service().await {
        log::warn!(
            "Failed to reinit db_service after dev_onboarding_scenario: {}",
            e
        );
    }

    Ok(result)
}

/// Build MeetingOutcomeData from a TranscriptResult + state lookups.
pub fn build_outcome_data(
    meeting_id: &str,
    result: &crate::types::TranscriptResult,
    _state: &AppState,
) -> crate::types::MeetingOutcomeData {
    // Try to get actions from DB for richer data
    let actions = crate::db::ActionDb::open()
        .ok()
        .and_then(|db| db.get_actions_for_meeting(meeting_id).ok())
        .unwrap_or_default();

    crate::types::MeetingOutcomeData {
        meeting_id: meeting_id.to_string(),
        summary: result.summary.clone(),
        wins: result.wins.clone(),
        risks: result.risks.clone(),
        decisions: result.decisions.clone(),
        actions,
        transcript_path: result.destination.clone(),
        processed_at: Some(chrono::Utc::now().to_rfc3339()),
    }
}

/// Compute executive intelligence signals (I42).
#[tauri::command]
pub async fn get_executive_intelligence(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::intelligence::ExecutiveIntelligence, String> {
    let started = std::time::Instant::now();
    let result = crate::services::entities::get_executive_intelligence(&state);
    log_command_latency(
        "get_executive_intelligence",
        started,
        READ_CMD_LATENCY_BUDGET_MS,
    );
    result.await
}

// =============================================================================
// I427: Global Search
// =============================================================================

#[tauri::command]
pub async fn search_global(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::db::search::GlobalSearchResult>, String> {
    use crate::db::search::SearchDb;
    state
        .db_read(move |db| {
            db.conn_ref()
                .search_global(&query, 20)
                .map_err(|e| e.to_string())
        })
        .await
}

#[tauri::command]
pub async fn rebuild_search_index(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    use crate::db::search::SearchDb;
    state
        .db_write(move |db| {
            db.conn_ref()
                .rebuild_search_index()
                .map_err(|e| e.to_string())
        })
        .await
}

// =============================================================================
// I428: Connectivity / Sync Freshness
// =============================================================================

#[tauri::command]
pub async fn get_sync_freshness(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::connectivity::SyncFreshness>, String> {
    state
        .db_read(|db| crate::connectivity::get_sync_freshness(db.conn_ref()))
        .await
}

// =============================================================================
// I429: Data Export
// =============================================================================

#[tauri::command]
pub async fn export_all_data(
    dest_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<crate::export::ExportReport, String> {
    let path = std::path::PathBuf::from(&dest_path);
    state
        .db_read(move |db| crate::export::export_data_zip(db, &path))
        .await
}

// =============================================================================
// I430: Privacy Controls
// =============================================================================

#[tauri::command]
pub async fn get_data_summary(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::privacy::DataSummary, String> {
    state.db_read(crate::privacy::get_data_summary).await
}

#[tauri::command]
pub async fn clear_intelligence(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::privacy::ClearReport, String> {
    state.db_write(crate::privacy::clear_intelligence).await
}

#[tauri::command]
pub async fn delete_all_data(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    // I609: Get DB path from static method, close db_service before deleting.
    let db_path = crate::db::ActionDb::db_path_public()
        .map(|p| p.to_string_lossy().to_string())
        .ok();

    // Close async DB service
    {
        let mut db_svc = state.db_service.write().await;
        *db_svc = None;
    }

    // Delete database file
    if let Some(path) = db_path {
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path).map_err(|e| format!("Failed to delete database: {e}"))?;
        }
        // Also delete WAL and SHM files
        let wal = format!("{path}-wal");
        let shm = format!("{path}-shm");
        let _ = std::fs::remove_file(&wal);
        let _ = std::fs::remove_file(&shm);
    }

    // Clear workspace directory
    if let Some(home) = dirs::home_dir() {
        let workspace = home.join(".dailyos").join("_today");
        if workspace.exists() {
            let _ = std::fs::remove_dir_all(&workspace);
        }
    }

    Ok(())
}

// =============================================================================
// Feature Flags (I537)
// =============================================================================

/// Returns current feature flags. Role presets are gated off for GA.
#[tauri::command]
pub async fn get_feature_flags() -> Result<crate::types::FeatureFlags, String> {
    Ok(crate::types::FeatureFlags::default())
}

// =============================================================================
// I614: DB Growth Monitoring
// =============================================================================

/// Return DB file size and row counts for key tables (I614).
#[tauri::command]
pub async fn get_db_growth_report(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::db::data_lifecycle::DbGrowthReport, String> {
    state
        .db_read(|db| Ok(crate::db::data_lifecycle::db_growth_report(db)))
        .await
}

// =============================================================================
// I645: Feedback & Suppression Diagnostics
// =============================================================================

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackDiagnostics {
    pub event_count: i64,
    pub suppression_count: i64,
    pub last_feedback: Option<String>,
}

/// Return feedback event count, active suppression count, and last feedback timestamp.
#[tauri::command]
pub async fn get_feedback_diagnostics(
    state: State<'_, Arc<AppState>>,
) -> Result<FeedbackDiagnostics, String> {
    state
        .db_read(move |db| {
            let event_count: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM entity_feedback_events",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let suppression_count: i64 = db
                .conn_ref()
                .query_row(
                    "SELECT COUNT(*) FROM suppression_tombstones \
                     WHERE expires_at IS NULL OR expires_at > datetime('now')",
                    [],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            let last_feedback: Option<String> = db
                .conn_ref()
                .query_row(
                    "SELECT created_at FROM entity_feedback_events \
                     ORDER BY created_at DESC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .ok();
            Ok(FeedbackDiagnostics {
                event_count,
                suppression_count,
                last_feedback,
            })
        })
        .await
}

// =============================================================================
// Health Scoring (I633)
// =============================================================================

/// Bulk recompute health scores for all accounts after formula fixes.
#[tauri::command]
pub async fn bulk_recompute_health(state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    state
        .db_write(crate::services::intelligence::bulk_recompute_health)
        .await
}

// =============================================================================
// People Commands (I51)
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    #[test]
    fn install_single_flight_guard_works() {
        // Verify the atomic flag can be set and cleared
        assert!(!INSTALL_IN_PROGRESS.load(Ordering::SeqCst));
        INSTALL_IN_PROGRESS.store(true, Ordering::SeqCst);
        assert!(INSTALL_IN_PROGRESS.load(Ordering::SeqCst));
        // Reset for other tests
        INSTALL_IN_PROGRESS.store(false, Ordering::SeqCst);
    }

    #[test]
    fn install_progress_serializes_to_camel_case() {
        let progress = InstallProgress {
            step: "installing".to_string(),
            status: "running".to_string(),
            message: "Installing...".to_string(),
        };
        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("\"step\""));
        assert!(json.contains("\"status\""));
        assert!(json.contains("\"message\""));
    }
}
