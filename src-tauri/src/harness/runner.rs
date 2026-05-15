#![allow(dead_code)]

use crate::abilities::feedback::ClaimVerificationState;
use crate::abilities::Actor;
use crate::abilities::NoopAbilityTracer;
use crate::bridges::eval::{EvalAbilityBridge, EvalAbilityDeps, EvalFixtureServices};
use crate::db::claims::{
    ClaimSensitivity, ClaimState, IntelligenceClaim, SurfacingState, TemporalScope,
};
use crate::db::ActionDb;
use crate::harness::classifier::{
    baseline_fingerprint_for_fixture, current_fingerprint_for_run, RegressionClassifier,
};
use crate::harness::loader::{load_fixture, FixtureLoadError};
use crate::harness::report::{FixtureRunSummary, HarnessReport};
use crate::harness::scoring::{
    CategoryScorer, MaintenanceScorer, PublishScorer, ReadScorer, ScoreResult, TransformScorer,
};
use crate::harness::types::{AbilityCategory, EvalFixture, FixtureRef};
use crate::intelligence::provider::{
    Completion, FingerprintMetadata, ModelName, ProviderKind, ReplayProvider,
};
#[cfg(feature = "harness-hermetic")]
use crate::services::context::validate_harness_hermetic_db_path;
use crate::services::context::{
    ClaimDismissalSurface, EntityContextClaimReadFuture, EntityContextClaimReadHandle,
    EntityContextReadFuture, EntityContextReadHandle, PrepareMeetingContextReadFuture,
    PrepareMeetingContextReadHandle, PrepareMeetingContextSnapshot,
};
use crate::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use crate::services::external_replay::JsonExternalReplayFixture;
use crate::types::EntityContextEntry;
use base64::Engine;
use rusqlite::types::ValueRef;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const HARNESS_AUTH_SCOPE_ID: &str = "harness-default-tenant";
const HARNESS_IN_MEMORY_DB_PATH: &str = ":memory:";
#[cfg(feature = "harness-hermetic")]
const HARNESS_DB_PATH_ENV: &str = "DAILYOS_HARNESS_DB_PATH";

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
    #[error("JSON serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("provider replay fixture invalid: {0}")]
    ProviderReplayInvalid(String),
    #[error("required dep not yet wired: {0}")]
    NotYetWired(String),
    #[error("harness report write failed: {0}")]
    ReportWrite(#[source] io::Error),
    #[cfg(feature = "harness-hermetic")]
    #[error("harness hermetic invariant failed: {0}")]
    HermeticInvariant(String),
}

pub struct RunnerDeps {
    /// W3-A registry — provides invoke_by_name_json or the eval bridge.
    pub registry: Arc<crate::abilities::registry::AbilityRegistry>,
}

pub fn run_harness_suite(
    deps: &RunnerDeps,
    fixture_refs: &[FixtureRef],
    output_path: &Path,
) -> Result<HarnessReport, RunError> {
    let mut report = HarnessReport::new();

    for fixture_ref in fixture_refs {
        let fixture = load_fixture(&fixture_ref.fixture_dir)?;
        let category = category_for_fixture(deps, &fixture)?;
        let started = Instant::now();
        let run_result = run_fixture(deps, &fixture)?;
        let runtime_ms = started.elapsed().as_millis() as u64;
        let score = score_fixture(category, &fixture, &run_result);
        let baseline = baseline_fingerprint_for_fixture(&fixture);
        let current = current_fingerprint_for_run(&fixture, &run_result);
        let regression = RegressionClassifier.classify(&baseline, &current, &score.diffs);

        report.add_fixture_summary(FixtureRunSummary {
            fixture_dir: fixture.fixture_dir.display().to_string(),
            bundle: fixture.metadata.bundle,
            scenario_id: fixture.metadata.scenario_id.clone(),
            category,
            passed: score.passed,
            continuous_score: score.continuous_score,
            regression,
            diff_count: score.diffs.len(),
            runtime_ms,
        });
    }

    report.finalize();
    report
        .bind_to_current_tree()
        .map_err(RunError::ReportWrite)?;
    report
        .write_json(output_path)
        .map_err(RunError::ReportWrite)?;

    Ok(report)
}

pub struct PreparedFixtureRun {
    pub conn: Connection,
    pub clock: FixedClock,
    pub rng: SeedableRng,
    pub external_clients: ExternalClients,
    pub provider: ReplayProvider,
    pub entity_context_rows: Vec<EntityContextEntry>,
    pub entity_context_claims: Vec<IntelligenceClaim>,
    pub prepare_meeting_context: Option<PrepareMeetingContextSnapshot>,
}

impl PreparedFixtureRun {
    pub fn service_context(&self) -> ServiceContext<'_> {
        let services = ServiceContext::new_evaluate(&self.clock, &self.rng, &self.external_clients)
            .with_actor("eval_fixture")
            .with_entity_context_reader(Arc::new(FixtureEntityContextReader {
                rows: self.entity_context_rows.clone(),
            }))
            .with_entity_context_claim_reader(Arc::new(FixtureEntityContextClaimReader {
                claims: self.entity_context_claims.clone(),
            }));
        if let Some(snapshot) = self.prepare_meeting_context.as_ref().cloned() {
            services.with_prepare_meeting_context_reader(Arc::new(
                FixturePrepareMeetingContextReader { snapshot },
            ))
        } else {
            services
        }
    }
}

fn score_fixture(
    category: AbilityCategory,
    fixture: &EvalFixture,
    run_result: &RunResult,
) -> ScoreResult {
    match category {
        AbilityCategory::Read => ReadScorer.score(&fixture.expected, run_result),
        AbilityCategory::Transform => {
            TransformScorer { threshold: 0.8 }.score(&fixture.expected, run_result)
        }
        AbilityCategory::Maintenance => MaintenanceScorer.score(&fixture.expected, run_result),
        AbilityCategory::Publish => PublishScorer.score(&fixture.expected, run_result),
    }
}

fn category_for_fixture(
    deps: &RunnerDeps,
    fixture: &EvalFixture,
) -> Result<AbilityCategory, RunError> {
    let ability_name = fixture
        .inputs_json
        .get("ability_name")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunError::InvocationFailed(
                "inputs.json ability_name must be a non-empty string".to_string(),
            )
        })?;

    deps.registry
        .iter_for(Actor::System)
        .find(|descriptor| descriptor.name == ability_name)
        .map(|descriptor| harness_category_from_registry(descriptor.category))
        .ok_or_else(|| RunError::InvocationFailed("AbilityUnavailable".to_string()))
}

fn harness_category_from_registry(category: crate::abilities::AbilityCategory) -> AbilityCategory {
    match category {
        crate::abilities::AbilityCategory::Read => AbilityCategory::Read,
        crate::abilities::AbilityCategory::Transform => AbilityCategory::Transform,
        crate::abilities::AbilityCategory::Maintenance => AbilityCategory::Maintenance,
        crate::abilities::AbilityCategory::Publish => AbilityCategory::Publish,
    }
}

struct PreparedFixtureServices<'run> {
    clock: &'run FixedClock,
    rng: &'run SeedableRng,
    external_clients: &'run ExternalClients,
    entity_context_rows: &'run [EntityContextEntry],
    entity_context_claims: &'run [IntelligenceClaim],
    prepare_meeting_context: &'run Option<PrepareMeetingContextSnapshot>,
}

impl EvalFixtureServices for PreparedFixtureServices<'_> {
    fn service_context(&self) -> ServiceContext<'_> {
        let services = ServiceContext::new_evaluate(self.clock, self.rng, self.external_clients)
            .with_actor("eval_fixture")
            .with_entity_context_reader(Arc::new(FixtureEntityContextReader {
                rows: self.entity_context_rows.to_vec(),
            }))
            .with_entity_context_claim_reader(Arc::new(FixtureEntityContextClaimReader {
                claims: self.entity_context_claims.to_vec(),
            }));
        if let Some(snapshot) = self.prepare_meeting_context.clone() {
            services.with_prepare_meeting_context_reader(Arc::new(
                FixturePrepareMeetingContextReader { snapshot },
            ))
        } else {
            services
        }
    }
}

#[derive(Clone)]
struct FixtureEntityContextReader {
    rows: Vec<EntityContextEntry>,
}

impl EntityContextReadHandle for FixtureEntityContextReader {
    fn read_entity_context_entries<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
    ) -> EntityContextReadFuture<'a> {
        Box::pin(async move {
            Ok(self
                .rows
                .iter()
                .filter(|row| row.entity_type == entity_type && row.entity_id == entity_id)
                .cloned()
                .collect())
        })
    }
}

#[derive(Clone)]
struct FixtureEntityContextClaimReader {
    claims: Vec<IntelligenceClaim>,
}

impl EntityContextClaimReadHandle for FixtureEntityContextClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        _surface: ClaimDismissalSurface,
        _depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        Box::pin(async move {
            let mut claims = self
                .claims
                .iter()
                .filter(|claim| {
                    claim.claim_state == ClaimState::Active
                        && claim.surfacing_state == SurfacingState::Active
                        && claim_subject_matches(claim, &entity_type, &entity_id)
                })
                .cloned()
                .collect::<Vec<_>>();
            claims.sort_by(|left, right| right.created_at.cmp(&left.created_at));
            Ok(claims)
        })
    }
}

#[derive(Clone)]
struct FixturePrepareMeetingContextReader {
    snapshot: PrepareMeetingContextSnapshot,
}

impl PrepareMeetingContextReadHandle for FixturePrepareMeetingContextReader {
    fn read_prepare_meeting_context<'a>(
        &'a self,
        meeting_id: String,
    ) -> PrepareMeetingContextReadFuture<'a> {
        Box::pin(async move {
            if self.snapshot.meeting.id == meeting_id {
                Ok(self.snapshot.clone())
            } else {
                Err(format!("fixture meeting `{meeting_id}` not seeded"))
            }
        })
    }
}

/// Run a single fixture: build evaluate context from fixture clock/seed/replay,
/// load state.sql into in-memory SQLite, invoke ability, capture output.
pub fn run_fixture(deps: &RunnerDeps, fixture: &EvalFixture) -> Result<RunResult, RunError> {
    let prepared = prepare_fixture_for_run(fixture)?;
    let invocation = parse_invocation_envelope(&fixture.inputs_json)?;
    let tracer = NoopAbilityTracer;
    let fixture_services = PreparedFixtureServices {
        clock: &prepared.clock,
        rng: &prepared.rng,
        external_clients: &prepared.external_clients,
        entity_context_rows: &prepared.entity_context_rows,
        entity_context_claims: &prepared.entity_context_claims,
        prepare_meeting_context: &prepared.prepare_meeting_context,
    };
    let eval_bridge = EvalAbilityBridge::new(
        deps.registry.as_ref(),
        EvalAbilityDeps {
            fixture_services: &fixture_services,
            provider: &prepared.provider,
            tracer: &tracer,
        },
    );
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| RunError::InvocationFailed(format!("tokio runtime: {error}")))?;
    let response = runtime
        .block_on(eval_bridge.invoke_json(
            &invocation.ability_name,
            invocation.input_json,
            invocation.dry_run,
        ))
        .map_err(|error| RunError::InvocationFailed(format!("{error:?}")))?;
    let actual_state = capture_post_action_state(&prepared.conn)?;

    Ok(RunResult {
        actual_output: response.data,
        actual_provenance: serde_json::to_value(&response.rendered_provenance)?,
        actual_state: Some(actual_state),
        diagnostics: diagnostics_to_strings(&response.diagnostics),
    })
}

pub fn prepare_fixture_for_run(fixture: &EvalFixture) -> Result<PreparedFixtureRun, RunError> {
    let conn = open_harness_connection()?;
    conn.execute_batch(&fixture.state_sql)
        .map_err(|error| RunError::StateSqlFailed(error.to_string()))?;
    let entity_context_rows = capture_entity_context_rows(&conn)?;
    let entity_context_claims = capture_intelligence_claim_rows(&conn)?;
    let prepare_meeting_context = capture_prepare_meeting_context(&conn, fixture)?;

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
        entity_context_rows,
        entity_context_claims,
        prepare_meeting_context,
    })
}

#[cfg(not(feature = "harness-hermetic"))]
fn open_harness_connection() -> Result<Connection, RunError> {
    Connection::open_in_memory().map_err(|error| RunError::StateSqlFailed(error.to_string()))
}

#[cfg(feature = "harness-hermetic")]
fn open_harness_connection() -> Result<Connection, RunError> {
    let db_path = std::env::var(HARNESS_DB_PATH_ENV)
        .unwrap_or_else(|_| HARNESS_IN_MEMORY_DB_PATH.to_string());
    validate_harness_hermetic_db_path(&db_path)
        .map_err(|error| RunError::HermeticInvariant(error.to_string()))?;

    if db_path == HARNESS_IN_MEMORY_DB_PATH {
        Connection::open_in_memory().map_err(|error| RunError::StateSqlFailed(error.to_string()))
    } else {
        Connection::open(&db_path).map_err(|error| RunError::StateSqlFailed(error.to_string()))
    }
}

fn replay_provider_from_fixture(value: &Value) -> Result<ReplayProvider, RunError> {
    let fixtures = value
        .get("fixtures")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            RunError::ProviderReplayInvalid(
                "provider_replay.json must contain a fixtures array".to_string(),
            )
        })?;

    let mut completions = HashMap::with_capacity(fixtures.len());
    for fixture in fixtures {
        if fixture.get("prompt_replay_hash").is_some()
            && fixture.get("canonical_prompt_hash").is_none()
        {
            return Err(RunError::ProviderReplayInvalid(
                "provider_replay fixture uses legacy prompt_replay_hash without canonical_prompt_hash"
                    .to_string(),
            ));
        }
        let replay_key = fixture
            .get("canonical_prompt_hash")
            .and_then(Value::as_str)
            .or_else(|| fixture.get("request_key_hex").and_then(Value::as_str))
            .ok_or_else(|| {
                RunError::ProviderReplayInvalid(
                    "provider_replay fixture is not keyed by canonical_prompt_hash or request_key_hex"
                        .to_string(),
                )
            })?;
        let completion_text = completion_text(fixture)?;

        completions.insert(
            replay_key.to_string(),
            Completion {
                text: completion_text,
                fingerprint_metadata: FingerprintMetadata::default(),
            },
        );
    }

    let metadata = replay_fingerprint_metadata(value)?;
    Ok(ReplayProvider::new(completions)
        .with_provider_kind(metadata.provider)
        .with_model_for_tier(crate::pty::ModelTier::Synthesis, metadata.model)
        .with_sampling(metadata.temperature, metadata.top_p, metadata.seed))
}

fn completion_text(fixture: &Value) -> Result<String, RunError> {
    match fixture.get("completion") {
        Some(Value::String(text)) => Ok(text.clone()),
        Some(Value::Object(completion)) => completion
            .get("text")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                RunError::ProviderReplayInvalid(
                    "provider_replay completion object must contain text".to_string(),
                )
            }),
        _ if fixture.get("response").is_some() => response_body_text(fixture),
        _ => Err(RunError::ProviderReplayInvalid(
            "provider_replay fixture must contain completion text".to_string(),
        )),
    }
}

fn response_body_text(fixture: &Value) -> Result<String, RunError> {
    let body_base64 = fixture
        .get("response")
        .and_then(|response| response.get("body_base64"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunError::ProviderReplayInvalid(
                "provider_replay response fixture must contain body_base64".to_string(),
            )
        })?;
    let body = base64::engine::general_purpose::STANDARD
        .decode(body_base64)
        .map_err(|error| {
            RunError::ProviderReplayInvalid(format!("provider_replay body_base64: {error}"))
        })?;
    String::from_utf8(body).map_err(|error| {
        RunError::ProviderReplayInvalid(format!("provider_replay body UTF-8: {error}"))
    })
}

fn replay_fingerprint_metadata(value: &Value) -> Result<FingerprintMetadata, RunError> {
    let default = FingerprintMetadata::default();
    let provider = value
        .get("provider")
        .and_then(Value::as_str)
        .map(provider_kind_from_str)
        .unwrap_or_else(|| default.provider.clone());
    let model = value
        .get("model")
        .and_then(Value::as_str)
        .map(ModelName::new)
        .unwrap_or_else(|| default.model.clone());
    let temperature = optional_f32_field(value, "temperature")?.unwrap_or(default.temperature);
    let top_p = optional_f32_field(value, "top_p")?.or(default.top_p);
    let seed = value.get("seed").and_then(Value::as_u64).or(default.seed);

    Ok(FingerprintMetadata {
        provider,
        model,
        temperature,
        top_p,
        seed,
        tokens_input: None,
        tokens_output: None,
        provider_completion_id: None,
    })
}

fn provider_kind_from_str(provider: &str) -> ProviderKind {
    match provider {
        "claude_code" => ProviderKind::ClaudeCode,
        "ollama" => ProviderKind::Ollama,
        "openai" => ProviderKind::OpenAI,
        "replay" => ProviderKind::Other("replay"),
        "glean" => ProviderKind::Other("glean"),
        _ => ProviderKind::Other("fixture"),
    }
}

fn optional_f32_field(value: &Value, field: &str) -> Result<Option<f32>, RunError> {
    value
        .get(field)
        .map(|value| {
            value.as_f64().map(|number| number as f32).ok_or_else(|| {
                RunError::ProviderReplayInvalid(format!(
                    "provider_replay {field} must be a finite number"
                ))
            })
        })
        .transpose()
}

struct InvocationEnvelope {
    ability_name: String,
    input_json: Value,
    dry_run: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawInvocationEnvelope {
    ability_name: String,
    input_json: Value,
    actor: String,
    mode: String,
    dry_run: bool,
}

fn parse_invocation_envelope(value: &Value) -> Result<InvocationEnvelope, RunError> {
    let raw: RawInvocationEnvelope = serde_json::from_value(value.clone()).map_err(|error| {
        RunError::InvocationFailed(format!("invalid inputs.json invocation envelope: {error}"))
    })?;
    if raw.ability_name.trim().is_empty() {
        return Err(RunError::InvocationFailed(
            "inputs.json ability_name must be non-empty".to_string(),
        ));
    }
    if !matches!(raw.actor.as_str(), "user" | "agent" | "system") {
        return Err(RunError::InvocationFailed(format!(
            "inputs.json actor must be user, agent, or system; got `{}`",
            raw.actor
        )));
    }
    if raw.mode != "evaluate" {
        return Err(RunError::InvocationFailed(format!(
            "inputs.json mode must be evaluate; got `{}`",
            raw.mode
        )));
    }

    Ok(InvocationEnvelope {
        ability_name: raw.ability_name,
        input_json: raw.input_json,
        dry_run: raw.dry_run,
    })
}

fn capture_post_action_state(conn: &Connection) -> Result<Value, RunError> {
    let mut post_action_state = Map::new();
    post_action_state.insert(
        "intelligence_claims".to_string(),
        Value::Array(capture_intelligence_claims(conn)?),
    );
    post_action_state.insert(
        "preserved_claims".to_string(),
        Value::Array(capture_preserved_claims(conn)?),
    );

    let claim_corroborations = capture_table_rows(
        conn,
        "claim_corroborations",
        &[
            "id",
            "claim_id",
            "data_source",
            "source_asof",
            "source_mechanism",
            "strength",
            "reinforcement_count",
            "last_reinforced_at",
            "created_at",
        ],
        &[],
        "id",
    )?;
    if !claim_corroborations.is_empty() {
        post_action_state.insert(
            "claim_corroborations".to_string(),
            Value::Array(claim_corroborations),
        );
    }

    let claim_feedback = capture_table_rows(
        conn,
        "claim_feedback",
        &[
            "id",
            "claim_id",
            "feedback_type",
            "actor",
            "actor_id",
            "payload_json",
            "submitted_at",
            "applied_at",
        ],
        &["payload_json"],
        "id",
    )?;
    if !claim_feedback.is_empty() {
        post_action_state.insert("claim_feedback".to_string(), Value::Array(claim_feedback));
    }

    Ok(serde_json::json!({ "post_action_state": post_action_state }))
}

fn capture_entity_context_rows(conn: &Connection) -> Result<Vec<EntityContextEntry>, RunError> {
    if !table_exists(conn, "entity_context_entries")? {
        return Ok(Vec::new());
    }

    let mut statement = conn
        .prepare(
            "SELECT id, entity_type, entity_id, title, content, created_at, updated_at
             FROM entity_context_entries
             ORDER BY created_at DESC",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok(EntityContextEntry {
                id: row.get("id")?,
                entity_type: row.get("entity_type")?,
                entity_id: row.get("entity_id")?,
                title: row.get::<_, String>("title")?.into(),
                content: row.get::<_, String>("content")?.into(),
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })
        .map_err(sql_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn capture_intelligence_claim_rows(conn: &Connection) -> Result<Vec<IntelligenceClaim>, RunError> {
    if !table_exists(conn, "intelligence_claims")? {
        return Ok(Vec::new());
    }

    let mut statement = conn
        .prepare(
            "SELECT id, subject_ref, claim_type, field_path, topic_key, text, dedup_key,
                    item_hash, actor, data_source, source_ref, source_asof, observed_at,
                    created_at, provenance_json, metadata_json, claim_state, surfacing_state,
                    demotion_reason, reactivated_at, retraction_reason, expires_at,
                    superseded_by, trust_score, trust_computed_at, trust_version, thread_id,
                    temporal_scope, sensitivity, verification_state, verification_reason,
                    needs_user_decision_at
             FROM intelligence_claims
             ORDER BY created_at DESC, id",
        )
        .map_err(sql_error)?;
    let mut rows = statement.query([]).map_err(sql_error)?;
    let mut claims = Vec::new();

    while let Some(row) = rows.next().map_err(sql_error)? {
        let claim_state = parse_claim_state(&row.get::<_, String>(16).map_err(sql_error)?)?;
        let surfacing_state = parse_surfacing_state(&row.get::<_, String>(17).map_err(sql_error)?)?;
        let temporal_scope = parse_temporal_scope(&row.get::<_, String>(27).map_err(sql_error)?)?;
        let sensitivity = parse_claim_sensitivity(&row.get::<_, String>(28).map_err(sql_error)?)?;
        let verification_state =
            parse_verification_state(&row.get::<_, String>(29).map_err(sql_error)?)?;

        claims.push(IntelligenceClaim {
            id: row.get(0).map_err(sql_error)?,
            claim_version: 1,
            subject_ref: row.get(1).map_err(sql_error)?,
            claim_type: row.get(2).map_err(sql_error)?,
            field_path: row.get(3).map_err(sql_error)?,
            topic_key: row.get(4).map_err(sql_error)?,
            text: row.get(5).map_err(sql_error)?,
            dedup_key: row.get(6).map_err(sql_error)?,
            item_hash: row.get(7).map_err(sql_error)?,
            actor: row.get(8).map_err(sql_error)?,
            data_source: row.get(9).map_err(sql_error)?,
            source_ref: row.get(10).map_err(sql_error)?,
            source_asof: row.get(11).map_err(sql_error)?,
            observed_at: row.get(12).map_err(sql_error)?,
            created_at: row.get(13).map_err(sql_error)?,
            provenance_json: row.get(14).map_err(sql_error)?,
            metadata_json: row.get(15).map_err(sql_error)?,
            claim_state,
            surfacing_state,
            demotion_reason: row.get(18).map_err(sql_error)?,
            reactivated_at: row.get(19).map_err(sql_error)?,
            retraction_reason: row.get(20).map_err(sql_error)?,
            expires_at: row.get(21).map_err(sql_error)?,
            superseded_by: row.get(22).map_err(sql_error)?,
            trust_score: row.get(23).map_err(sql_error)?,
            trust_computed_at: row.get(24).map_err(sql_error)?,
            trust_version: row.get(25).map_err(sql_error)?,
            thread_id: row.get(26).map_err(sql_error)?,
            temporal_scope,
            sensitivity,
            verification_state,
            verification_reason: row.get(30).map_err(sql_error)?,
            needs_user_decision_at: row.get(31).map_err(sql_error)?,
        });
    }

    Ok(claims)
}

fn capture_prepare_meeting_context(
    conn: &Connection,
    fixture: &EvalFixture,
) -> Result<Option<PrepareMeetingContextSnapshot>, RunError> {
    if fixture
        .inputs_json
        .get("ability_name")
        .and_then(Value::as_str)
        != Some("prepare_meeting")
    {
        return Ok(None);
    }

    let meeting_id = fixture
        .inputs_json
        .get("input_json")
        .and_then(|input| input.get("meeting_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            RunError::InvocationFailed(
                "prepare_meeting fixture input_json.meeting_id must be a string".to_string(),
            )
        })?;
    let db = ActionDb::from_conn(conn);
    crate::services::meetings::load_prepare_meeting_context_snapshot(db, meeting_id)
        .map(Some)
        .map_err(RunError::StateSqlFailed)
}

fn capture_intelligence_claims(conn: &Connection) -> Result<Vec<Value>, RunError> {
    if !table_exists(conn, "intelligence_claims")? {
        return Ok(Vec::new());
    }

    let mut statement = conn
        .prepare(
            "SELECT id, subject_ref, claim_type, field_path, topic_key, text, data_source, \
                    source_ref, source_asof, observed_at, claim_state, surfacing_state, \
                    verification_state, verification_reason, trust_score, trust_computed_at, \
                    trust_version, temporal_scope, sensitivity, metadata_json, \
                    (SELECT COUNT(*) FROM claim_corroborations WHERE claim_id = intelligence_claims.id) \
             FROM intelligence_claims \
             ORDER BY id",
        )
        .map_err(sql_error)?;
    let rows = statement
        .query_map([], |row| {
            let trust_score = row.get::<_, Option<f64>>(14)?;
            let mut claim = Map::new();
            insert_text(&mut claim, "claim_id", row.get(0)?);
            insert_json_text(&mut claim, "subject_ref", row.get(1)?);
            insert_text(&mut claim, "claim_type", row.get(2)?);
            insert_optional_text(&mut claim, "field_path", row.get(3)?);
            insert_optional_text(&mut claim, "topic_key", row.get(4)?);
            insert_text(&mut claim, "text", row.get(5)?);
            insert_text(&mut claim, "data_source", row.get(6)?);
            insert_optional_json_text(&mut claim, "source_ref", row.get(7)?);
            insert_optional_text(&mut claim, "source_asof", row.get(8)?);
            insert_text(&mut claim, "observed_at", row.get(9)?);
            insert_text(&mut claim, "claim_state", row.get(10)?);
            insert_text(&mut claim, "surfacing_state", row.get(11)?);
            insert_text(&mut claim, "verification_state", row.get(12)?);
            insert_optional_text(&mut claim, "verification_reason", row.get(13)?);
            insert_optional_f64(&mut claim, "trust_score", trust_score);
            insert_optional_text(&mut claim, "trust_computed_at", row.get(15)?);
            insert_optional_i64(&mut claim, "trust_version", row.get(16)?);
            insert_text(&mut claim, "temporal_scope", row.get(17)?);
            insert_text(&mut claim, "sensitivity", row.get(18)?);
            insert_optional_json_text(&mut claim, "metadata", row.get(19)?);
            insert_optional_text(
                &mut claim,
                "trust_band",
                trust_score.map(trust_band_for_score),
            );
            insert_optional_i64(&mut claim, "corroboration_count", row.get(20)?);
            Ok(Value::Object(claim))
        })
        .map_err(sql_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn capture_preserved_claims(conn: &Connection) -> Result<Vec<Value>, RunError> {
    if !table_exists(conn, "intelligence_claims")? {
        return Ok(Vec::new());
    }

    let mut statement = conn
        .prepare("SELECT id, text FROM intelligence_claims ORDER BY id")
        .map_err(sql_error)?;
    let rows = statement
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let text: String = row.get(1)?;
            Ok(Value::String(format!("{id}: {text}")))
        })
        .map_err(sql_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn capture_table_rows(
    conn: &Connection,
    table: &str,
    columns: &[&str],
    json_columns: &[&str],
    order_by: &str,
) -> Result<Vec<Value>, RunError> {
    if !table_exists(conn, table)? {
        return Ok(Vec::new());
    }

    let sql = format!(
        "SELECT {} FROM {} ORDER BY {}",
        columns.join(", "),
        table,
        order_by
    );
    let mut statement = conn.prepare(&sql).map_err(sql_error)?;
    let rows = statement
        .query_map([], |row| {
            let mut object = Map::new();
            for (index, column) in columns.iter().enumerate() {
                let mut value = sqlite_value_to_json(row.get_ref(index)?);
                if json_columns.contains(column) {
                    value = parse_json_value(value);
                }
                object.insert((*column).to_string(), value);
            }
            Ok(Value::Object(object))
        })
        .map_err(sql_error)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(sql_error)
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, RunError> {
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(sql_error)?;
    Ok(count > 0)
}

fn claim_subject_matches(claim: &IntelligenceClaim, entity_type: &str, entity_id: &str) -> bool {
    serde_json::from_str::<Value>(&claim.subject_ref)
        .ok()
        .and_then(|subject| {
            Some((
                subject.get("kind")?.as_str()?.to_ascii_lowercase(),
                subject.get("id")?.as_str()?.to_string(),
            ))
        })
        .is_some_and(|(kind, id)| kind == entity_type && id == entity_id)
}

fn parse_claim_state(value: &str) -> Result<ClaimState, RunError> {
    match value {
        "active" => Ok(ClaimState::Active),
        "dormant" => Ok(ClaimState::Dormant),
        "tombstoned" => Ok(ClaimState::Tombstoned),
        "withdrawn" => Ok(ClaimState::Withdrawn),
        other => Err(RunError::StateSqlFailed(format!(
            "invalid claim_state `{other}`"
        ))),
    }
}

fn parse_surfacing_state(value: &str) -> Result<SurfacingState, RunError> {
    match value {
        "active" => Ok(SurfacingState::Active),
        "dormant" => Ok(SurfacingState::Dormant),
        other => Err(RunError::StateSqlFailed(format!(
            "invalid surfacing_state `{other}`"
        ))),
    }
}

fn parse_temporal_scope(value: &str) -> Result<TemporalScope, RunError> {
    match value {
        "state" => Ok(TemporalScope::State),
        "point_in_time" => Ok(TemporalScope::PointInTime),
        "trend" => Ok(TemporalScope::Trend),
        "closed" => Ok(TemporalScope::Closed),
        other => Err(RunError::StateSqlFailed(format!(
            "invalid temporal_scope `{other}`"
        ))),
    }
}

fn parse_claim_sensitivity(value: &str) -> Result<ClaimSensitivity, RunError> {
    match value {
        "public" => Ok(ClaimSensitivity::Public),
        "internal" => Ok(ClaimSensitivity::Internal),
        "confidential" => Ok(ClaimSensitivity::Confidential),
        "user_only" => Ok(ClaimSensitivity::UserOnly),
        other => Err(RunError::StateSqlFailed(format!(
            "invalid sensitivity `{other}`"
        ))),
    }
}

fn parse_verification_state(value: &str) -> Result<ClaimVerificationState, RunError> {
    match value {
        "active" => Ok(ClaimVerificationState::Active),
        "contested" => Ok(ClaimVerificationState::Contested),
        "needs_user_decision" => Ok(ClaimVerificationState::NeedsUserDecision),
        other => Err(RunError::StateSqlFailed(format!(
            "invalid verification_state `{other}`"
        ))),
    }
}

fn insert_text(object: &mut Map<String, Value>, key: &str, value: String) {
    object.insert(key.to_string(), Value::String(value));
}

fn insert_optional_text(object: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        object.insert(key.to_string(), Value::String(value));
    }
}

fn insert_json_text(object: &mut Map<String, Value>, key: &str, value: String) {
    object.insert(key.to_string(), parse_json_string(value));
}

fn insert_optional_json_text(object: &mut Map<String, Value>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        object.insert(key.to_string(), parse_json_string(value));
    }
}

fn insert_optional_f64(object: &mut Map<String, Value>, key: &str, value: Option<f64>) {
    if let Some(value) = value.and_then(Number::from_f64) {
        object.insert(key.to_string(), Value::Number(value));
    }
}

fn insert_optional_i64(object: &mut Map<String, Value>, key: &str, value: Option<i64>) {
    if let Some(value) = value {
        object.insert(key.to_string(), Value::Number(Number::from(value)));
    }
}

fn parse_json_string(value: String) -> Value {
    serde_json::from_str(&value).unwrap_or(Value::String(value))
}

fn parse_json_value(value: Value) -> Value {
    match value {
        Value::String(text) => parse_json_string(text),
        value => value,
    }
}

fn sqlite_value_to_json(value: ValueRef<'_>) -> Value {
    match value {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(value) => Value::Number(Number::from(value)),
        ValueRef::Real(value) => Number::from_f64(value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        ValueRef::Text(value) => Value::String(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Value::String(hex::encode(value)),
    }
}

fn trust_band_for_score(score: f64) -> String {
    if score >= 0.75 {
        "likely_current".to_string()
    } else if score >= 0.5 {
        "mixed".to_string()
    } else {
        "needs_verification".to_string()
    }
}

fn diagnostics_to_strings(diagnostics: &Value) -> Vec<String> {
    if let Some(warnings) = diagnostics.get("warnings").and_then(Value::as_array) {
        return warnings.iter().map(diagnostic_value_to_string).collect();
    }
    if let Some(items) = diagnostics.as_array() {
        return items.iter().map(diagnostic_value_to_string).collect();
    }
    if diagnostics == &serde_json::json!({ "warnings": [] }) {
        return Vec::new();
    }
    vec![diagnostic_value_to_string(diagnostics)]
}

fn diagnostic_value_to_string(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{value:?}"))
}

fn sql_error(error: rusqlite::Error) -> RunError {
    RunError::StateSqlFailed(error.to_string())
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
