#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use abilities_runtime::intelligence::provider::{
    Completion, FingerprintMetadata, IntelligenceProvider, ModelName, ModelTier, PromptInput,
    ProviderError, ProviderKind,
};
use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use dailyos_lib::abilities::registry::{AbilityRegistry, ScopeSet, SurfaceClientId, SurfaceScope};
use dailyos_lib::abilities::{AbilityContext, Actor, NOOP_ABILITY_TRACER};
use dailyos_lib::db::claims::{ClaimSensitivity, TemporalScope};
use dailyos_lib::db::ActionDb;
use dailyos_lib::migration_test_api::run_migrations;
use dailyos_lib::services::claims::{
    commit_claim, ClaimProposal, CommittedClaim, DeterministicInsertProposal,
};
use dailyos_lib::services::compositions;
use dailyos_lib::services::context::{
    ClaimDismissalSurface, CommittedComposition, CompositionCommitError, CompositionCommitFuture,
    CompositionCommitHandle, CompositionCommitRequest, EntityContextClaimReadFuture,
    EntityContextClaimReadHandle, ExternalClients, FixedClock, SeedableRng, ServiceContext,
};
use dailyos_lib::services::surface_pairing::ValidatedSurfaceSession;
use rusqlite::{params, Connection};
use serde_json::{json, Value};

pub const ACCOUNT_OVERVIEW_ABILITY: &str = "dailyos/account-overview";
pub const ACCOUNT_ID: &str = "acct-dos568-fixture";
pub const COMPOSITION_ID: &str = "dailyos/account-overview:account:acct-dos568-fixture";
const FIXTURE_NOW: &str = "2026-05-15T12:00:00Z";

pub type SharedConn = Arc<Mutex<Connection>>;

pub fn fresh_full_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    run_migrations(&conn).expect("apply production migrations");
    conn
}

pub fn shared(conn: Connection) -> SharedConn {
    Arc::new(Mutex::new(conn))
}

pub fn clone_connection(source: &Connection) -> Connection {
    let mut cloned = Connection::open_in_memory().expect("open cloned in-memory db");
    let backup =
        rusqlite::backup::Backup::new(source, &mut cloned).expect("initialize clone backup");
    backup.step(-1).expect("copy full db state");
    drop(backup);
    cloned
}

pub fn seed_account(conn: &Connection) {
    conn.execute(
        "INSERT INTO accounts (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params![ACCOUNT_ID, "Example Account", FIXTURE_NOW],
    )
    .expect("seed account");
}

pub fn seed_claim(
    conn: &Connection,
    id: &str,
    claim_type: &str,
    field_path: &str,
    text: &str,
) -> String {
    let clock = fixture_clock();
    let rng = SeedableRng::new(568);
    let external = ExternalClients::default();
    let ctx = ServiceContext::new_live(&clock, &rng, &external).with_actor("agent:dos568_fixture");
    let proposal = ClaimProposal {
        id: None,
        expected_claim_version: None,
        subject_ref: json!({ "kind": "account", "id": ACCOUNT_ID }).to_string(),
        claim_type: claim_type.to_string(),
        field_path: Some(field_path.to_string()),
        topic_key: None,
        text: text.to_string(),
        actor: "agent:dos568_fixture".to_string(),
        data_source: "test".to_string(),
        source_ref: Some(format!("source-{id}")),
        source_asof: Some(FIXTURE_NOW.to_string()),
        observed_at: FIXTURE_NOW.to_string(),
        provenance_json: "{}".to_string(),
        metadata_json: None,
        thread_id: None,
        temporal_scope: Some(TemporalScope::State),
        sensitivity: Some(ClaimSensitivity::Internal),
        supersedes: None,
        tombstone: None,
    };
    let committed = commit_claim(
        &ctx,
        ActionDb::from_conn(conn),
        DeterministicInsertProposal::new(id.to_string(), proposal),
    )
    .expect("commit fixture claim through service");

    match committed {
        CommittedClaim::Inserted { claim } => claim.id,
        other => panic!("expected inserted fixture claim, got {other:?}"),
    }
}

pub fn seed_base_account_state(conn: &Connection) {
    seed_account(conn);
    seed_claim(
        conn,
        "claim-dos568-risk",
        "entity_risk",
        "/risk/current",
        "Implementation risk requires attention",
    );
    seed_claim(
        conn,
        "claim-dos568-win",
        "entity_win",
        "/wins/latest",
        "Adoption momentum is improving",
    );
}

pub async fn invoke_account_overview_json(
    db: SharedConn,
    actor: Actor,
    expected_composition_version: u64,
) -> Value {
    let registry = AbilityRegistry::global_checked().expect("global registry builds");
    let clock = fixture_clock();
    let rng = SeedableRng::new(568);
    let external = ExternalClients::default();
    let reader = Arc::new(SqliteEntityContextClaimReader { db: db.clone() });
    let committer = Arc::new(SqliteCompositionCommitter {
        db,
        now: fixture_now(),
    });
    let services = ServiceContext::new_live(&clock, &rng, &external)
        .with_actor("surface_client")
        .with_ability_id(ACCOUNT_OVERVIEW_ABILITY)
        .with_entity_context_claim_reader(reader)
        .with_composition_commit_handle(committer);
    let provider = StaticProvider;
    let ctx = AbilityContext::new(
        &services,
        &provider,
        &NOOP_ABILITY_TRACER,
        actor,
        None,
        ClaimDismissalSurface::LogStructured,
    );

    registry
        .invoke_by_name_json(
            &ctx,
            ACCOUNT_OVERVIEW_ABILITY,
            json!({
                "schema_version": 1,
                "account_id": ACCOUNT_ID,
                "expected_composition_version": expected_composition_version,
                "composition_id": COMPOSITION_ID,
            }),
        )
        .await
        .expect("account overview invocation succeeds")
}

pub fn surface_actor_with_account_scope() -> Actor {
    let _ = AbilityRegistry::global_checked().expect("global registry seeds scope allowlist");
    Actor::SurfaceClient {
        instance: SurfaceClientId::new("sc_dos568_fixture"),
        scopes: ScopeSet::new([SurfaceScope::new("read.account_overview")]).expect("scope set"),
    }
}

pub fn surface_session_with_account_scope() -> ValidatedSurfaceSession {
    let actor = surface_actor_with_account_scope();
    ValidatedSurfaceSession {
        surface_client_id: "sc_dos568_fixture".to_string(),
        session_id: "sess_dos568_fixture".to_string(),
        actor,
        wp_user_id: Some(42),
        wp_user_hash: Some("wp_user_hash_dos568".to_string()),
        wp_site_id: "wp_site_dos568".to_string(),
        wp_site_id_hash: "wp_site_hash_dos568".to_string(),
        site_binding_digest: "site_binding_digest_dos568".to_string(),
        site_nonce: "site_nonce_dos568".to_string(),
        scope_digest: "scope_digest_dos568".to_string(),
        granted_scopes: vec!["read.account_overview".to_string()],
    }
}

pub fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    write_canonical_json(value, &mut out);
    out
}

pub fn composition_version(output: &Value) -> u64 {
    output
        .pointer("/data/metadata/composition_version")
        .and_then(Value::as_u64)
        .expect("composition version is present")
}

pub fn composition_event_count(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM version_events WHERE composition_id = ?1 AND event_kind = 'composition.updated'",
        params![COMPOSITION_ID],
        |row| row.get(0),
    )
    .expect("count composition events")
}

pub fn claim_event_count(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COUNT(*) FROM version_events WHERE claim_id IS NOT NULL AND event_kind LIKE 'claim.%'",
        [],
        |row| row.get(0),
    )
    .expect("count claim events")
}

fn write_canonical_json(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.extend_from_slice(b"null"),
        Value::Bool(true) => out.extend_from_slice(b"true"),
        Value::Bool(false) => out.extend_from_slice(b"false"),
        Value::Number(number) => out.extend_from_slice(number.to_string().as_bytes()),
        Value::String(string) => serde_json::to_writer(out, string).expect("write string"),
        Value::Array(values) => {
            out.push(b'[');
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_canonical_json(item, out);
            }
            out.push(b']');
        }
        Value::Object(map) => {
            out.push(b'{');
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            for (index, (key, item)) in entries.into_iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                serde_json::to_writer(&mut *out, key).expect("write key");
                out.push(b':');
                write_canonical_json(item, out);
            }
            out.push(b'}');
        }
    }
}

struct SqliteEntityContextClaimReader {
    db: SharedConn,
}

impl EntityContextClaimReadHandle for SqliteEntityContextClaimReader {
    fn read_entity_context_claims<'a>(
        &'a self,
        entity_type: String,
        entity_id: String,
        surface: ClaimDismissalSurface,
        depth: usize,
    ) -> EntityContextClaimReadFuture<'a> {
        let result = {
            let conn = self.db.lock().expect("db lock");
            dailyos_lib::services::claims::load_entity_context_claims_active_for_surface(
                ActionDb::from_conn(&conn),
                &entity_type,
                &entity_id,
                depth,
                surface.as_str(),
            )
            .map_err(|error| error.to_string())
        };
        Box::pin(std::future::ready(result))
    }
}

struct SqliteCompositionCommitter {
    db: SharedConn,
    now: DateTime<Utc>,
}

impl CompositionCommitHandle for SqliteCompositionCommitter {
    fn commit_composition<'a>(
        &'a self,
        request: CompositionCommitRequest,
    ) -> CompositionCommitFuture<'a> {
        let result = {
            let conn = self.db.lock().expect("db lock");
            let clock = FixedClock::new(self.now);
            let rng = SeedableRng::new(568);
            let external = ExternalClients::default();
            let mut ctx = ServiceContext::new_live(&clock, &rng, &external)
                .with_actor(request.actor.as_str());
            if let Some(ability_id) = request.ability_id.as_deref() {
                ctx = ctx.with_ability_id(ability_id);
            }
            let proposal = compositions::CompositionProposal {
                composition_id: request.proposal.composition_id,
                expected_composition_version: request.proposal.expected_composition_version,
                composition: request.proposal.composition,
            };
            compositions::commit_composition(&ctx, ActionDb::from_conn(&conn), proposal)
                .map(|committed| CommittedComposition {
                    composition_id: committed.composition_id,
                    composition_version: committed.composition_version,
                    composition: committed.composition,
                })
                .map_err(map_composition_error)
        };
        Box::pin(std::future::ready(result))
    }
}

fn map_composition_error(error: compositions::CompositionError) -> CompositionCommitError {
    match error {
        compositions::CompositionError::EmptyCompositionId => {
            CompositionCommitError::EmptyCompositionId
        }
        compositions::CompositionError::StaleVersion {
            composition_id,
            expected,
            current,
        } => CompositionCommitError::StaleVersion {
            composition_id,
            expected,
            current,
        },
        compositions::CompositionError::InflatedVersion {
            composition_id,
            expected,
            current,
        } => CompositionCommitError::InflatedVersion {
            composition_id,
            expected,
            current,
        },
        compositions::CompositionError::Overflow { composition_id } => {
            CompositionCommitError::Overflow { composition_id }
        }
        compositions::CompositionError::Mode(message) => CompositionCommitError::Mode(message),
        compositions::CompositionError::Transaction(message) => {
            CompositionCommitError::Transaction(message)
        }
    }
}

struct StaticProvider;

#[async_trait]
impl IntelligenceProvider for StaticProvider {
    async fn complete(
        &self,
        _prompt: PromptInput,
        _tier: ModelTier,
    ) -> Result<Completion, ProviderError> {
        Ok(Completion {
            text: String::new(),
            fingerprint_metadata: FingerprintMetadata {
                provider: ProviderKind::Other("dos568_fixture"),
                model: ModelName::new("unused"),
                temperature: 0.0,
                top_p: None,
                seed: None,
                tokens_input: None,
                tokens_output: None,
                provider_completion_id: None,
            },
        })
    }

    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Other("dos568_fixture")
    }

    fn current_model(&self, _tier: ModelTier) -> ModelName {
        ModelName::new("unused")
    }
}

fn fixture_now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(FIXTURE_NOW)
        .expect("fixture timestamp parses")
        .with_timezone(&Utc)
}

fn fixture_clock() -> FixedClock {
    FixedClock::new(Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap())
}
