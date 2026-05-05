#![allow(dead_code)]

use crate::harness::loader::FixtureLoadError;
use crate::harness::types::EvalFixture;
use dailyos_lib::intelligence::provider::{Completion, FingerprintMetadata, ReplayProvider};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::services::external_replay::JsonExternalReplayFixture;
use rusqlite::Connection;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const HARNESS_AUTH_SCOPE_ID: &str = "harness-default-tenant";

/// Output of running a single fixture through its ability.
pub struct RunResult {
    /// The ability's response payload (JSON).
    pub actual_output: Value,
    /// The ability's W3-B Provenance envelope, JSON-serialized.
    pub actual_provenance: Value,
    /// Post-action DB state captured for expected_state.json comparison.
    /// None if the fixture has no expected_state.json (CP-E optional).
    pub actual_state: Option<Value>,
    /// Diagnostics from the bridge (errors, warnings, timings).
    pub diagnostics: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("fixture load failed: {0}")]
    FixtureLoad(#[from] FixtureLoadError),
    #[error("state.sql failed to apply: {0}")]
    StateSqlFailed(String),
    #[error("ability invocation failed: {0}")]
    InvocationFailed(String),
    #[error("required dep not yet wired: {0}")]
    NotYetWired(String),
}

pub struct RunnerDeps {
    /// W3-A registry — provides invoke_by_name_json or the eval bridge.
    pub registry: Arc<dailyos_lib::abilities::registry::AbilityRegistry>,
}

pub(crate) struct PreparedFixtureRun {
    pub conn: Connection,
    pub clock: FixedClock,
    pub rng: SeedableRng,
    pub external_clients: ExternalClients,
    pub provider: ReplayProvider,
}

impl PreparedFixtureRun {
    pub fn service_context(&self) -> ServiceContext<'_> {
        ServiceContext::new_evaluate(&self.clock, &self.rng, &self.external_clients)
            .with_actor("eval_fixture")
    }
}

/// Run a single fixture: build evaluate context from fixture clock/seed/replay,
/// load state.sql into in-memory SQLite, invoke ability, capture output.
pub fn run_fixture(deps: &RunnerDeps, fixture: &EvalFixture) -> Result<RunResult, RunError> {
    let _prepared = prepare_fixture_for_run(fixture)?;

    // W4-C's EvalAbilityBridge is public, but the runner still needs the chunk-3
    // glue that binds fixture ability names, replay providers, and state capture
    // into the bridge result. Chunk 2 intentionally stops after proving the
    // hermetic run context and fixture DB setup.
    let _registered_abilities = deps
        .registry
        .iter_for(dailyos_lib::abilities::registry::Actor::System)
        .count();

    Err(RunError::NotYetWired(
        "ability invocation pending W4-C bridge integration in chunk 3".to_string(),
    ))
}

pub(crate) fn prepare_fixture_for_run(
    fixture: &EvalFixture,
) -> Result<PreparedFixtureRun, RunError> {
    let conn = Connection::open_in_memory()
        .map_err(|error| RunError::StateSqlFailed(error.to_string()))?;
    apply_all_migrations(&conn)?;
    conn.execute_batch(&fixture.state_sql)
        .map_err(|error| RunError::StateSqlFailed(error.to_string()))?;

    let clock = FixedClock::new(fixture.clock);
    let rng = SeedableRng::new(fixture.seed);
    let external_replay = JsonExternalReplayFixture::from_json_value(
        &fixture.external_replay,
        &fixture.fixture_dir.display().to_string(),
    )
    .map_err(|error| RunError::StateSqlFailed(format!("external_replay: {error}")))?;
    let external_clients =
        ExternalClients::from_replay(Arc::new(external_replay), HARNESS_AUTH_SCOPE_ID.to_string());
    let provider = replay_provider_from_fixture(&fixture.provider_replay)?;

    Ok(PreparedFixtureRun {
        conn,
        clock,
        rng,
        external_clients,
        provider,
    })
}

fn replay_provider_from_fixture(value: &Value) -> Result<ReplayProvider, RunError> {
    let fixtures = value
        .get("fixtures")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            RunError::NotYetWired(
                "provider_replay.json must contain a fixtures array".to_string(),
            )
        })?;

    let mut completions = HashMap::with_capacity(fixtures.len());
    for fixture in fixtures {
        let prompt_replay_hash = fixture
            .get("prompt_replay_hash")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RunError::NotYetWired(
                    "provider_replay fixture is not keyed by prompt_replay_hash".to_string(),
                )
            })?;
        let completion_text = completion_text(fixture)?;

        completions.insert(
            prompt_replay_hash.to_string(),
            Completion {
                text: completion_text.to_string(),
                fingerprint_metadata: FingerprintMetadata::default(),
            },
        );
    }

    Ok(ReplayProvider::new(completions))
}

fn completion_text(fixture: &Value) -> Result<&str, RunError> {
    match fixture.get("completion") {
        Some(Value::String(text)) => Ok(text.as_str()),
        Some(Value::Object(completion)) => completion
            .get("text")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                RunError::NotYetWired(
                    "provider_replay completion object must contain text".to_string(),
                )
            }),
        _ => Err(RunError::NotYetWired(
            "provider_replay fixture must contain completion text".to_string(),
        )),
    }
}

fn apply_all_migrations(conn: &Connection) -> Result<(), RunError> {
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|error| RunError::StateSqlFailed(error.to_string()))?;

    for path in migration_paths()? {
        let sql = fs::read_to_string(&path).map_err(|error| {
            RunError::StateSqlFailed(format!(
                "failed to read migration {}: {error}",
                path.display()
            ))
        })?;
        apply_migration_sql(conn, &path, &sql)?;
    }

    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|error| RunError::StateSqlFailed(error.to_string()))
}

fn migration_paths() -> Result<Vec<PathBuf>, RunError> {
    let migrations_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/migrations");
    let mut paths = fs::read_dir(&migrations_dir)
        .map_err(|error| {
            RunError::StateSqlFailed(format!(
                "failed to read migrations dir {}: {error}",
                migrations_dir.display()
            ))
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|error| RunError::StateSqlFailed(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    paths.retain(|path| path.extension().and_then(|ext| ext.to_str()) == Some("sql"));
    paths.sort_by_key(|path| migration_sort_key(path));
    Ok(paths)
}

fn migration_sort_key(path: &Path) -> (u32, String) {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let version = file_name
        .split_once('_')
        .map_or(file_name, |(version, _)| version)
        .parse::<u32>()
        .unwrap_or(u32::MAX);
    (version, file_name.to_string())
}

fn apply_migration_sql(conn: &Connection, path: &Path, sql: &str) -> Result<(), RunError> {
    match conn.execute_batch(sql) {
        Ok(()) => Ok(()),
        Err(error) if is_tolerated_schema_conflict(sql, &error.to_string()) => Ok(()),
        Err(error) => Err(RunError::StateSqlFailed(format!(
            "migration {} failed: {error}",
            path.display()
        ))),
    }
}

fn is_tolerated_schema_conflict(sql: &str, error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    if error.contains("duplicate column name") {
        return true;
    }

    is_single_alter_migration(sql) && error.contains("no such column")
}

fn is_single_alter_migration(sql: &str) -> bool {
    sql.split(';')
        .map(|statement| {
            statement
                .lines()
                .filter(|line| !line.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .map(|statement| statement.trim().to_ascii_uppercase())
        .filter(|statement| !statement.is_empty())
        .all(|statement| statement.starts_with("ALTER"))
}
