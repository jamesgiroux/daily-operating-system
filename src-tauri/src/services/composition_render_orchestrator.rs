//! Composition render orchestrator.
//!
//! Single entry-point that bridges first-party and external render surfaces
//! to the abilities-runtime composition projector. Surface clients call
//! `/v1/surface/project-composition`; the Tauri app uses the command surface.
//!
//! 1. Looks up the in-memory cache keyed by
//!    `(composition_id, current_db_composition_version, surface_kind,
//!    fallback_policy_version, scopes_canonical_id)`.
//! 2. On hit: returns the cached `ProjectedComposition` + a refreshed
//!    `cache_hint_token`.
//! 3. On miss: invokes the W4-A0 producer ability
//!    (`dailyos/account-overview`) → gets `AbilityOutput<Composition>` →
//!    runs W4-D `project_composition_for_surface(composition, ctx)` →
//!    caches the result → returns
//!    `ProjectedComposition` + a new `cache_hint_token`.
//!
//! The substrate owns the cache and the scope-identity authority per
//! packet §6.2 V3 / §6.12. PHP receives only an opaque `cache_hint_token`
//! that it echoes back; it never derives or interprets the token.
//!
//! Cache scope-identity key: SHA256 of the sorted scope strings of the
//! authenticated `Actor::SurfaceClient`. Scope-change means key-change and a
//! natural miss-and-recompute. First-party Tauri invocations use the fixed
//! user actor scope. Cache misses are singleflighted by actor scope,
//! composition id, surface kind, and fallback-policy version; the request's
//! composition version is intentionally not part of the miss guard.
//! `local_first_party` identity and still split by render policy.

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use abilities_runtime::abilities::composition::Composition;
use abilities_runtime::abilities::{
    project_composition_for_surface as project_for_surface_fn, AuditIntent,
    FallbackProjectionContext, ProjectedComposition, SurfaceKind,
};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use dashmap::DashMap;
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use crate::abilities::registry::ScopeSet;
use crate::abilities::Actor;
use crate::bridges::{AbilityResponseJson, BridgeSurfaceError, RenderedProvenance};
use crate::state::AppState;

/// Cache TTL per packet §6.2 V3 (matches W4-E nonce lifetime).
pub const CACHE_TTL: Duration = Duration::from_secs(60);

/// Fallback projection policy version — bumps trigger cache invalidation
/// because the policy version contributes to the projected output. Pinned
/// here so the route handler doesn't have to know how to construct the
/// `FallbackProjectionContext`.
pub const FALLBACK_POLICY_VERSION: u32 = 1;

#[derive(Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    composition_id: String,
    composition_version: i64,
    surface_kind: SurfaceKind,
    fallback_policy_version: u32,
    scopes_canonical_id: String,
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct MissKey {
    composition_id: String,
    surface_kind: SurfaceKind,
    fallback_policy_version: u32,
    scopes_canonical_id: String,
}

struct CacheEntry {
    projection: ProjectedComposition,
    rendered_provenance: Option<RenderedProvenance>,
    cache_hint_token: String,
    cached_at: Instant,
}

/// W4-A render orchestrator. Lives in `AppState` as a singleton.
pub struct CompositionRenderOrchestrator {
    cache: DashMap<CacheKey, CacheEntry>,
    miss_locks: DashMap<MissKey, Arc<AsyncMutex<()>>>,
    rng: SystemRandom,
}

impl Default for CompositionRenderOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositionRenderOrchestrator {
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            miss_locks: DashMap::new(),
            rng: SystemRandom::new(),
        }
    }

    fn make_cache_key(
        actor: &Actor,
        composition_id: &str,
        composition_version: i64,
        surface_kind: SurfaceKind,
        fallback_policy_version: u32,
    ) -> Option<CacheKey> {
        // First-party loopback runs as Actor::User and needs the cache to
        // function the same as Actor::SurfaceClient. Without an Actor::User
        // branch the local composition route would re-run the producer on
        // every render. Use a fixed "local_first_party" canonical id since
        // first-party invocations share the full scope set.
        let scopes_canonical = match actor {
            Actor::SurfaceClient { scopes, .. } => scopes_canonical_id(scopes),
            Actor::User => "local_first_party".to_string(),
            _ => return None,
        };
        Some(CacheKey {
            composition_id: composition_id.to_string(),
            composition_version,
            surface_kind,
            fallback_policy_version,
            scopes_canonical_id: scopes_canonical,
        })
    }

    fn make_miss_key(
        actor: &Actor,
        composition_id: &str,
        surface_kind: SurfaceKind,
        fallback_policy_version: u32,
    ) -> Option<MissKey> {
        let scopes_canonical = match actor {
            Actor::SurfaceClient { scopes, .. } => scopes_canonical_id(scopes),
            Actor::User => "local_first_party".to_string(),
            _ => return None,
        };
        Some(MissKey {
            composition_id: composition_id.to_string(),
            surface_kind,
            fallback_policy_version,
            scopes_canonical_id: scopes_canonical,
        })
    }

    /// Cache lookup. Returns the cached projection + a refreshed
    /// `cache_hint_token` on hit; `None` on miss or expired entry.
    pub fn cache_lookup(
        self: &Arc<Self>,
        actor: &Actor,
        composition_id: &str,
        composition_version: i64,
        surface_kind: SurfaceKind,
        fallback_policy_version: u32,
    ) -> Option<CachedProjection> {
        let key = Self::make_cache_key(
            actor,
            composition_id,
            composition_version,
            surface_kind,
            fallback_policy_version,
        )?;
        let entry = self.cache.get(&key)?;
        if entry.cached_at.elapsed() >= CACHE_TTL {
            return None;
        }
        Some(CachedProjection {
            projection: entry.projection.clone(),
            rendered_provenance: entry.rendered_provenance.clone(),
            cache_hint_token: entry.cache_hint_token.clone(),
        })
    }

    /// Store an entry. The opaque `cache_hint_token` is freshly generated
    /// per insert so a captured token does not survive an entry-replace.
    pub fn cache_store(
        self: &Arc<Self>,
        actor: &Actor,
        composition_id: &str,
        composition_version: i64,
        surface_kind: SurfaceKind,
        fallback_policy_version: u32,
        payload: CacheStorePayload,
    ) -> Option<String> {
        let key = Self::make_cache_key(
            actor,
            composition_id,
            composition_version,
            surface_kind,
            fallback_policy_version,
        )?;
        let mut token_bytes = [0u8; 16];
        self.rng.fill(&mut token_bytes).ok()?;
        let cache_hint_token = URL_SAFE_NO_PAD.encode(token_bytes);
        let entry = CacheEntry {
            projection: payload.projection,
            rendered_provenance: payload.rendered_provenance,
            cache_hint_token: cache_hint_token.clone(),
            cached_at: Instant::now(),
        };
        self.cache.insert(key, entry);
        Some(cache_hint_token)
    }

    /// Serialize cache miss recomposition for one render-policy identity. The
    /// caller must re-check the cache after acquiring this guard.
    pub async fn cache_miss_guard(
        self: &Arc<Self>,
        actor: &Actor,
        composition_id: &str,
        surface_kind: SurfaceKind,
        fallback_policy_version: u32,
    ) -> Option<OwnedMutexGuard<()>> {
        let key =
            Self::make_miss_key(actor, composition_id, surface_kind, fallback_policy_version)?;
        let lock = self
            .miss_locks
            .entry(key)
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone();
        Some(lock.lock_owned().await)
    }

    /// Test helper: cache size.
    #[doc(hidden)]
    pub fn __test_cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Test helper: drop all entries (e.g. after scope rotation in a test).
    #[doc(hidden)]
    pub fn __test_clear(&self) {
        self.cache.clear();
        self.miss_locks.clear();
    }
}

/// Cache hit payload.
#[derive(Debug, Clone)]
pub struct CachedProjection {
    pub projection: ProjectedComposition,
    pub rendered_provenance: Option<RenderedProvenance>,
    pub cache_hint_token: String,
}

/// Cache store payload.
#[derive(Debug, Clone)]
pub struct CacheStorePayload {
    pub projection: ProjectedComposition,
    pub rendered_provenance: Option<RenderedProvenance>,
}

#[derive(Debug, Clone)]
pub struct ProducerProjectionInput {
    pub ability_name: &'static str,
    pub subject: ProducerSubject,
    pub composition_id: String,
    pub schema_version: u32,
    pub expected_composition_version: u64,
}

impl ProducerProjectionInput {
    fn with_expected_composition_version(&self, expected_composition_version: u64) -> Self {
        Self {
            expected_composition_version,
            ..self.clone()
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        let mut value = serde_json::json!({
            "composition_id": self.composition_id,
            "schema_version": self.schema_version,
            "expected_composition_version": self.expected_composition_version,
        });
        let object = value.as_object_mut().expect("producer input json object");
        match &self.subject {
            ProducerSubject::Entity {
                entity_type,
                entity_id,
            } => {
                object.insert(
                    "entity_type".to_string(),
                    serde_json::Value::from(entity_type.as_str()),
                );
                object.insert(
                    "entity_id".to_string(),
                    serde_json::Value::from(entity_id.as_str()),
                );
                object.insert(
                    entity_type.input_key().to_string(),
                    serde_json::Value::from(entity_id.as_str()),
                );
            }
            ProducerSubject::Action { action_id } => {
                object.insert(
                    "action_id".to_string(),
                    serde_json::Value::from(action_id.as_str()),
                );
                object.insert(
                    "subject_ref".to_string(),
                    serde_json::json!({ "action": action_id.as_str() }),
                );
            }
            ProducerSubject::Briefing {
                workspace_scope,
                date,
            } => {
                object.insert(
                    "workspace_scope".to_string(),
                    serde_json::Value::from(workspace_scope.as_str()),
                );
                object.insert("date".to_string(), serde_json::Value::from(date.as_str()));
            }
            ProducerSubject::Meeting { meeting_token } => {
                object.insert(
                    "meeting_token".to_string(),
                    serde_json::Value::from(meeting_token.as_str()),
                );
            }
        }
        value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerEntityType {
    Account,
    Project,
    Person,
}

impl ProducerEntityType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Project => "project",
            Self::Person => "person",
        }
    }

    const fn input_key(self) -> &'static str {
        match self {
            Self::Account => "account_id",
            Self::Project => "project_id",
            Self::Person => "person_id",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerSubject {
    Entity {
        entity_type: ProducerEntityType,
        entity_id: String,
    },
    Action {
        action_id: String,
    },
    Briefing {
        workspace_scope: String,
        date: String,
    },
    Meeting {
        meeting_token: String,
    },
}

#[derive(Debug, Clone)]
pub struct ProjectedCompositionRender {
    pub projection: ProjectedComposition,
    pub cache_hint_token: String,
    pub served_from_cache: bool,
    pub rendered_provenance: Option<RenderedProvenance>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectCompositionRenderOptions {
    pub force_refresh: bool,
}

/// Shared projection pipeline for Tauri, local loopback, and signed surface
/// renders. Authorization and entry-point-specific audits stay at the caller;
/// cache identity, miss serialization, producer retry, projection, and cache
/// insert live here so all surfaces share the same stale-version semantics.
pub async fn project_composition_for_surface<F, Fut>(
    state: &AppState,
    actor: Actor,
    surface_kind: SurfaceKind,
    composition_id: &str,
    invoke_producer: F,
) -> Result<ProjectedCompositionRender, BridgeSurfaceError>
where
    F: FnMut(ProducerProjectionInput) -> Fut,
    Fut: Future<Output = Result<AbilityResponseJson, BridgeSurfaceError>>,
{
    project_composition_for_surface_with_options(
        state,
        actor,
        surface_kind,
        composition_id,
        ProjectCompositionRenderOptions::default(),
        invoke_producer,
    )
    .await
}

/// Variant of [`project_composition_for_surface`] that lets first-party
/// mutation flows force a recomposition after service-owned writes. External
/// surfaces keep the default cache behavior.
pub async fn project_composition_for_surface_with_options<F, Fut>(
    state: &AppState,
    actor: Actor,
    surface_kind: SurfaceKind,
    composition_id: &str,
    options: ProjectCompositionRenderOptions,
    mut invoke_producer: F,
) -> Result<ProjectedCompositionRender, BridgeSurfaceError>
where
    F: FnMut(ProducerProjectionInput) -> Fut,
    Fut: Future<Output = Result<AbilityResponseJson, BridgeSurfaceError>>,
{
    let Some(producer_input_template) = parse_producer_projection_input(composition_id, 0) else {
        return Err(BridgeSurfaceError::Validation(
            "project_composition_invalid_id".to_string(),
        ));
    };

    let orchestrator = state.composition_render_orchestrator.clone();
    let current_db_version_for_lookup = current_composition_version(state, composition_id).await;
    let current_db_version = i64::try_from(current_db_version_for_lookup).unwrap_or(i64::MAX);

    if !options.force_refresh {
        if let Some(cached) = orchestrator.cache_lookup(
            &actor,
            composition_id,
            current_db_version,
            surface_kind,
            FALLBACK_POLICY_VERSION,
        ) {
            return Ok(ProjectedCompositionRender {
                projection: cached.projection,
                cache_hint_token: cached.cache_hint_token,
                served_from_cache: true,
                rendered_provenance: cached.rendered_provenance,
            });
        }
    }

    let _miss_guard = orchestrator
        .cache_miss_guard(
            &actor,
            composition_id,
            surface_kind,
            FALLBACK_POLICY_VERSION,
        )
        .await
        .ok_or(BridgeSurfaceError::AbilityUnavailable)?;

    let guarded_db_version_for_lookup = current_composition_version(state, composition_id).await;
    let guarded_db_version = i64::try_from(guarded_db_version_for_lookup).unwrap_or(i64::MAX);
    if !options.force_refresh {
        if let Some(cached) = orchestrator.cache_lookup(
            &actor,
            composition_id,
            guarded_db_version,
            surface_kind,
            FALLBACK_POLICY_VERSION,
        ) {
            return Ok(ProjectedCompositionRender {
                projection: cached.projection,
                cache_hint_token: cached.cache_hint_token,
                served_from_cache: true,
                rendered_provenance: cached.rendered_provenance,
            });
        }
    }

    let mut expected_composition_version = guarded_db_version_for_lookup;
    let mut response = None;
    for attempt in 0..2 {
        let producer_input =
            producer_input_template.with_expected_composition_version(expected_composition_version);
        match invoke_producer(producer_input).await {
            Ok(invocation_response) => {
                response = Some(invocation_response);
                break;
            }
            Err(BridgeSurfaceError::StaleComposition { current, .. })
                if attempt == 0 && expected_composition_version != current =>
            {
                expected_composition_version = current;
            }
            Err(error) => return Err(error),
        }
    }
    let Some(response) = response else {
        return Err(BridgeSurfaceError::AbilityUnavailable);
    };

    let rendered_provenance = response.rendered_provenance.clone();
    let (projection, _audits) = project_from_ability_data(
        &response.data,
        actor.clone(),
        surface_kind,
        FALLBACK_POLICY_VERSION,
    )
    .map_err(|err| {
        log::warn!("project_composition_for_surface: project_from_ability_data failed: {err:?}");
        BridgeSurfaceError::ProducerUnavailable
    })?;

    let projection_cache_version = projection
        .composition_version
        .unwrap_or(guarded_db_version_for_lookup);
    let projection_cache_version = i64::try_from(projection_cache_version).unwrap_or(i64::MAX);
    let cache_hint_token = orchestrator
        .cache_store(
            &actor,
            composition_id,
            projection_cache_version,
            surface_kind,
            FALLBACK_POLICY_VERSION,
            CacheStorePayload {
                projection: projection.clone(),
                rendered_provenance: Some(rendered_provenance.clone()),
            },
        )
        .unwrap_or_default();

    Ok(ProjectedCompositionRender {
        projection,
        cache_hint_token,
        served_from_cache: false,
        rendered_provenance: Some(rendered_provenance),
    })
}

async fn current_composition_version(state: &AppState, composition_id: &str) -> u64 {
    let composition_id_for_lookup = composition_id.to_string();
    state
        .db_read(move |db| {
            let clock = crate::services::context::SystemClock;
            let rng = crate::services::context::SystemRng;
            let external = crate::services::context::ExternalClients::default();
            let ctx = crate::services::context::ServiceContext::new_live(&clock, &rng, &external);
            crate::services::compositions::current_composition_version_for_composition_id(
                &ctx,
                db,
                &composition_id_for_lookup,
            )
            .map_err(|e| e.to_string())
        })
        .await
        .ok()
        .unwrap_or(0)
}

fn scopes_canonical_id(scopes: &ScopeSet) -> String {
    let sorted: Vec<&str> = scopes.iter().map(|s| s.as_str()).collect();
    let mut hasher = Sha256::new();
    hasher.update(sorted.join("\n").as_bytes());
    hex::encode(hasher.finalize())
}

/// Deserialize a `Composition` JSON value (the `data` field returned by the
/// abilities bridge) and project it for the requested render surface.
///
/// Returns `(projection, audits)` matching W4-D's contract. The caller is
/// responsible for draining `audits` through `emit_surface_audit` and
/// inserting the result into the cache via [`CompositionRenderOrchestrator::cache_store`].
pub fn project_from_ability_data(
    data: &serde_json::Value,
    actor: Actor,
    surface_kind: SurfaceKind,
    fallback_policy_version: u32,
) -> Result<(ProjectedComposition, Vec<AuditIntent>), OrchestratorError> {
    let composition: Composition = serde_json::from_value(data.clone())
        .map_err(|e| OrchestratorError::CompositionDeserialize(e.to_string()))?;
    let ctx = FallbackProjectionContext::new(actor, surface_kind, fallback_policy_version);
    project_for_surface_fn(&composition, &ctx)
        .map_err(|e| OrchestratorError::ProjectionFailed(format!("{e:?}")))
}

pub async fn hydrate_producer_projection_input(
    state: &AppState,
    producer_input: &ProducerProjectionInput,
) -> Result<serde_json::Value, BridgeSurfaceError> {
    let mut input = producer_input.to_json();
    let object = input
        .as_object_mut()
        .ok_or_else(|| BridgeSurfaceError::Validation("project_composition_input_shape".into()))?;

    match &producer_input.subject {
        ProducerSubject::Briefing { .. } => {
            let config = state.config.read().clone().ok_or_else(|| {
                BridgeSurfaceError::Validation("project_composition_workspace_unavailable".into())
            })?;
            object.insert(
                "workspace_id".to_string(),
                serde_json::Value::from(config.workspace_path),
            );
        }
        ProducerSubject::Meeting { meeting_token } => {
            let token = meeting_token.clone();
            let meeting_id = state
                .db_read(move |db| {
                    crate::services::meetings::resolve_meeting_composition_token(db, &token)
                })
                .await
                .map_err(|error| {
                    BridgeSurfaceError::Validation(format!(
                        "project_composition_meeting_token_read_failed: {error}"
                    ))
                })?
                .ok_or_else(|| {
                    BridgeSurfaceError::Validation(
                        "project_composition_unknown_meeting_token".to_string(),
                    )
                })?;
            object.insert(
                "meeting_id".to_string(),
                serde_json::Value::from(meeting_id),
            );
        }
        ProducerSubject::Entity { .. } | ProducerSubject::Action { .. } => {}
    }

    Ok(input)
}

/// Map a request composition_id back to its producer ability name. Keeps
/// this bounded to first-party detail/briefing composition ids so the
/// orchestrator never invokes a foreign producer.
pub fn resolve_producer_ability_name(composition_id: &str) -> Option<&'static str> {
    parse_producer_projection_input(composition_id, 0).map(|input| input.ability_name)
}

/// Extract the account_id encoded in an `account-overview` composition_id.
/// Pattern: `dailyos/account-overview:account:{account_id}`.
pub fn extract_account_id_from_composition_id(composition_id: &str) -> Option<&str> {
    let parts = split_composition_id(composition_id)?;
    (parts.ability == "dailyos/account-overview" && parts.subject_key == "account")
        .then_some(parts.subject_id)
}

pub fn parse_producer_projection_input(
    composition_id: &str,
    expected_composition_version: u64,
) -> Option<ProducerProjectionInput> {
    let parts = split_composition_id(composition_id)?;
    let (ability_name, subject) = match (parts.ability, parts.subject_key) {
        ("dailyos/account-overview", "account") => (
            "dailyos/account-overview",
            ProducerSubject::Entity {
                entity_type: ProducerEntityType::Account,
                entity_id: parts.subject_id.to_string(),
            },
        ),
        ("dailyos/project-overview", "project") => (
            "dailyos/project-overview",
            ProducerSubject::Entity {
                entity_type: ProducerEntityType::Project,
                entity_id: parts.subject_id.to_string(),
            },
        ),
        ("dailyos/person-overview", "person") => (
            "dailyos/person-overview",
            ProducerSubject::Entity {
                entity_type: ProducerEntityType::Person,
                entity_id: parts.subject_id.to_string(),
            },
        ),
        ("dailyos/action-detail", "action") => (
            "dailyos/action-detail",
            ProducerSubject::Action {
                action_id: parts.subject_id.to_string(),
            },
        ),
        ("dailyos/daily-briefing", "briefing") => {
            let (workspace_scope, date) = parse_daily_briefing_subject(parts.subject_id)?;
            (
                "dailyos/daily-briefing",
                ProducerSubject::Briefing {
                    workspace_scope,
                    date,
                },
            )
        }
        ("dailyos/meeting-detail", "meeting") => {
            if !crate::services::meetings::valid_meeting_composition_token(parts.subject_id) {
                return None;
            }
            (
                "dailyos/meeting-detail",
                ProducerSubject::Meeting {
                    meeting_token: parts.subject_id.to_string(),
                },
            )
        }
        _ => return None,
    };
    Some(ProducerProjectionInput {
        ability_name,
        subject,
        composition_id: composition_id.to_string(),
        schema_version: 1,
        expected_composition_version,
    })
}

fn parse_daily_briefing_subject(subject_id: &str) -> Option<(String, String)> {
    let (workspace_scope, date) = subject_id.split_once('~')?;
    if workspace_scope != "local" {
        return None;
    }
    if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return None;
    }
    Some((workspace_scope.to_string(), date.to_string()))
}

#[derive(Debug, Clone, Copy)]
struct ParsedCompositionId<'a> {
    ability: &'a str,
    subject_key: &'a str,
    subject_id: &'a str,
}

fn split_composition_id(composition_id: &str) -> Option<ParsedCompositionId<'_>> {
    if composition_id.chars().any(char::is_control) {
        return None;
    }
    let mut parts = composition_id.split(':');
    let ability = parts.next()?;
    let subject_key = parts.next()?;
    let subject_id = parts.next()?;
    if parts.next().is_some()
        || ability.trim().is_empty()
        || subject_key.trim().is_empty()
        || subject_id.trim().is_empty()
    {
        return None;
    }
    Some(ParsedCompositionId {
        ability,
        subject_key,
        subject_id,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("composition deserialize failed: {0}")]
    CompositionDeserialize(String),
    #[error("projection failed: {0}")]
    ProjectionFailed(String),
    #[error("unknown producer for composition: {0}")]
    UnknownProducer(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abilities::registry::{ScopeSet, SurfaceClientId, SurfaceScope};

    fn surface_actor() -> Actor {
        let scopes = ScopeSet::new([SurfaceScope::new("read.account_overview")]).unwrap();
        Actor::SurfaceClient {
            instance: SurfaceClientId::new("sc-test"),
            scopes,
        }
    }

    fn minimal_projection(
        composition_id: &str,
        fallback_policy_version: u32,
    ) -> ProjectedComposition {
        serde_json::from_value(serde_json::json!({
            "composition_id": composition_id,
            "composition_version": 1,
            "fallback_policy_version": fallback_policy_version,
            "blocks": [],
            "diagnostics": [],
            "unknown_block_count": 0,
            "unknown_block_cap": 4,
            "dropped_unknown_block_count": 0
        }))
        .expect("projection deserialize")
    }

    #[test]
    fn cache_round_trip() {
        let composition_id = "dailyos/account-overview:account:acct-1";
        let projection = minimal_projection(composition_id, FALLBACK_POLICY_VERSION);

        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor = surface_actor();
        let token = orchestrator
            .cache_store(
                &actor,
                composition_id,
                1,
                SurfaceKind::SurfaceClient,
                FALLBACK_POLICY_VERSION,
                CacheStorePayload {
                    projection,
                    rendered_provenance: None,
                },
            )
            .expect("cache_store with SurfaceClient");
        assert!(!token.is_empty(), "non-empty cache_hint_token");

        let hit = orchestrator
            .cache_lookup(
                &actor,
                composition_id,
                1,
                SurfaceKind::SurfaceClient,
                FALLBACK_POLICY_VERSION,
            )
            .expect("cache hit");
        assert_eq!(hit.cache_hint_token, token);
    }

    #[test]
    fn cache_miss_on_scope_change() {
        let composition_id = "dailyos/account-overview:account:acct-2";
        let projection = minimal_projection(composition_id, FALLBACK_POLICY_VERSION);
        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor_a = surface_actor();
        let _ = orchestrator.cache_store(
            &actor_a,
            composition_id,
            1,
            SurfaceKind::SurfaceClient,
            FALLBACK_POLICY_VERSION,
            CacheStorePayload {
                projection,
                rendered_provenance: None,
            },
        );

        // Different scope set → different canonical id → miss.
        let scopes_b = ScopeSet::new([
            SurfaceScope::new("read.account_overview"),
            SurfaceScope::new("read.composition"),
        ])
        .expect("scopes");
        let actor_b = Actor::SurfaceClient {
            instance: SurfaceClientId::new("sc-test"),
            scopes: scopes_b,
        };
        let hit = orchestrator.cache_lookup(
            &actor_b,
            composition_id,
            1,
            SurfaceKind::SurfaceClient,
            FALLBACK_POLICY_VERSION,
        );
        assert!(hit.is_none(), "scope change must produce miss");
    }

    #[test]
    fn cache_miss_on_surface_kind_change() {
        let composition_id = "dailyos/account-overview:account:acct-surface";
        let projection = minimal_projection(composition_id, FALLBACK_POLICY_VERSION);
        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor = surface_actor();
        let _ = orchestrator.cache_store(
            &actor,
            composition_id,
            1,
            SurfaceKind::SurfaceClient,
            FALLBACK_POLICY_VERSION,
            CacheStorePayload {
                projection,
                rendered_provenance: None,
            },
        );

        let hit = orchestrator.cache_lookup(
            &actor,
            composition_id,
            1,
            SurfaceKind::TauriApp,
            FALLBACK_POLICY_VERSION,
        );
        assert!(hit.is_none(), "surface kind change must produce miss");
    }

    #[test]
    fn cache_miss_on_fallback_policy_version_change() {
        let composition_id = "dailyos/account-overview:account:acct-policy";
        let projection = minimal_projection(composition_id, FALLBACK_POLICY_VERSION);
        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor = surface_actor();
        let _ = orchestrator.cache_store(
            &actor,
            composition_id,
            1,
            SurfaceKind::SurfaceClient,
            FALLBACK_POLICY_VERSION,
            CacheStorePayload {
                projection,
                rendered_provenance: None,
            },
        );

        let hit = orchestrator.cache_lookup(
            &actor,
            composition_id,
            1,
            SurfaceKind::SurfaceClient,
            FALLBACK_POLICY_VERSION + 1,
        );
        assert!(
            hit.is_none(),
            "fallback policy version change must produce miss"
        );
    }

    #[tokio::test]
    async fn cache_miss_guard_serializes_same_render_identity() {
        let composition_id = "dailyos/account-overview:account:acct-singleflight";
        let orchestrator = Arc::new(CompositionRenderOrchestrator::new());
        let actor = surface_actor();
        let first = orchestrator
            .cache_miss_guard(
                &actor,
                composition_id,
                SurfaceKind::SurfaceClient,
                FALLBACK_POLICY_VERSION,
            )
            .await
            .expect("first guard");

        let same_identity = tokio::time::timeout(
            Duration::from_millis(10),
            orchestrator.cache_miss_guard(
                &actor,
                composition_id,
                SurfaceKind::SurfaceClient,
                FALLBACK_POLICY_VERSION,
            ),
        )
        .await;
        assert!(same_identity.is_err(), "same identity must wait");

        let different_surface = tokio::time::timeout(
            Duration::from_millis(10),
            orchestrator.cache_miss_guard(
                &actor,
                composition_id,
                SurfaceKind::TauriApp,
                FALLBACK_POLICY_VERSION,
            ),
        )
        .await;
        assert!(
            different_surface.is_ok(),
            "different render policy must not wait"
        );

        drop(first);
        let same_identity_after_drop = tokio::time::timeout(
            Duration::from_millis(10),
            orchestrator.cache_miss_guard(
                &actor,
                composition_id,
                SurfaceKind::SurfaceClient,
                FALLBACK_POLICY_VERSION,
            ),
        )
        .await;
        assert!(
            same_identity_after_drop.is_ok(),
            "same identity proceeds after guard drops"
        );
    }

    #[test]
    fn unknown_producer_rejected() {
        let valid_ids = [
            (
                "dailyos/account-overview:account:acct-1",
                "dailyos/account-overview",
            ),
            (
                "dailyos/project-overview:project:project-1",
                "dailyos/project-overview",
            ),
            (
                "dailyos/person-overview:person:person-1",
                "dailyos/person-overview",
            ),
            (
                "dailyos/action-detail:action:action-1",
                "dailyos/action-detail",
            ),
            (
                "dailyos/daily-briefing:briefing:local~2026-06-02",
                "dailyos/daily-briefing",
            ),
            (
                "dailyos/meeting-detail:meeting:mtg_0123456789abcdef",
                "dailyos/meeting-detail",
            ),
        ];
        for (composition_id, ability_name) in valid_ids {
            assert_eq!(
                resolve_producer_ability_name(composition_id),
                Some(ability_name)
            );
        }

        assert!(resolve_producer_ability_name("foreign/ability").is_none());
        assert!(
            resolve_producer_ability_name("dailyos/project-overview:person:person-1").is_none()
        );
        assert!(
            resolve_producer_ability_name("dailyos/account-overview:account:acct:extra").is_none()
        );
    }

    #[test]
    fn account_id_extracts() {
        assert_eq!(
            extract_account_id_from_composition_id("dailyos/account-overview:account:acct-42"),
            Some("acct-42")
        );
        assert!(extract_account_id_from_composition_id("dailyos/other:foo").is_none());
        assert!(extract_account_id_from_composition_id(
            "dailyos/account-overview:account:acct-42:extra"
        )
        .is_none());
    }

    #[test]
    fn producer_projection_input_emits_subject_specific_json() {
        let project =
            parse_producer_projection_input("dailyos/project-overview:project:project-42", 9)
                .expect("project input");
        assert_eq!(project.ability_name, "dailyos/project-overview");
        assert_eq!(
            project.subject,
            ProducerSubject::Entity {
                entity_type: ProducerEntityType::Project,
                entity_id: "project-42".to_string(),
            }
        );
        assert_eq!(
            project.to_json(),
            serde_json::json!({
                "composition_id": "dailyos/project-overview:project:project-42",
                "schema_version": 1,
                "expected_composition_version": 9,
                "entity_type": "project",
                "entity_id": "project-42",
                "project_id": "project-42",
            })
        );

        let action = parse_producer_projection_input("dailyos/action-detail:action:action-42", 12)
            .expect("action input");
        assert_eq!(action.ability_name, "dailyos/action-detail");
        assert_eq!(
            action.subject,
            ProducerSubject::Action {
                action_id: "action-42".to_string(),
            }
        );
        assert_eq!(
            action.to_json(),
            serde_json::json!({
                "composition_id": "dailyos/action-detail:action:action-42",
                "schema_version": 1,
                "expected_composition_version": 12,
                "action_id": "action-42",
                "subject_ref": { "action": "action-42" },
            })
        );

        let briefing =
            parse_producer_projection_input("dailyos/daily-briefing:briefing:local~2026-06-02", 3)
                .expect("briefing input");
        assert_eq!(briefing.ability_name, "dailyos/daily-briefing");
        assert_eq!(
            briefing.subject,
            ProducerSubject::Briefing {
                workspace_scope: "local".to_string(),
                date: "2026-06-02".to_string(),
            }
        );
        assert_eq!(
            briefing.to_json(),
            serde_json::json!({
                "composition_id": "dailyos/daily-briefing:briefing:local~2026-06-02",
                "schema_version": 1,
                "expected_composition_version": 3,
                "workspace_scope": "local",
                "date": "2026-06-02",
            })
        );

        let meeting = parse_producer_projection_input(
            "dailyos/meeting-detail:meeting:mtg_0123456789abcdef",
            4,
        )
        .expect("meeting input");
        assert_eq!(meeting.ability_name, "dailyos/meeting-detail");
        assert_eq!(
            meeting.subject,
            ProducerSubject::Meeting {
                meeting_token: "mtg_0123456789abcdef".to_string(),
            }
        );
        assert_eq!(
            meeting.to_json(),
            serde_json::json!({
                "composition_id": "dailyos/meeting-detail:meeting:mtg_0123456789abcdef",
                "schema_version": 1,
                "expected_composition_version": 4,
                "meeting_token": "mtg_0123456789abcdef",
            })
        );
    }

    #[test]
    fn producer_projection_input_fails_closed_for_malformed_ids() {
        let malformed = [
            "",
            "dailyos/account-overview:account:",
            "dailyos/account-overview::acct-1",
            "dailyos/account-overview:account:acct-1:extra",
            "dailyos/account-overview:project:project-1",
            "dailyos/project-overview:account:acct-1",
            "dailyos/person-overview:action:action-1",
            "dailyos/action-detail:person:person-1",
            "dailyos/action-detail:action:action-1\n",
            "dailyos/daily-briefing:briefing:remote~2026-06-02",
            "dailyos/daily-briefing:briefing:local~2026-99-02",
            "dailyos/daily-briefing:briefing:local",
            "dailyos/meeting-detail:meeting:",
            "dailyos/meeting-detail:meeting:mtg_0123456789abcde",
            "dailyos/meeting-detail:meeting:mtg_0123456789abcdef0123456789abcdef0",
            "dailyos/meeting-detail:meeting:mtg_0123456789abcdeg",
            "dailyos/meeting-detail:meeting:mtg_0123456789ABCDEF",
            "dailyos/meeting-detail:meeting:calendar_evt_123",
            "dailyos/meeting-detail:meeting:event_at_provider",
            "dailyos/meeting-detail:meeting:provider@example.com",
            "dailyos/meeting-detail:meeting:../meeting",
            "dailyos/meeting-detail:meeting:path/to/meeting",
            "dailyos/meeting-detail:meeting:meeting token",
            "dailyos/unknown:account:acct-1",
        ];
        for composition_id in malformed {
            assert!(
                parse_producer_projection_input(composition_id, 0).is_none(),
                "{composition_id:?} must be rejected"
            );
        }
    }

    #[tokio::test]
    async fn hydrate_producer_projection_input_adds_briefing_workspace_id() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let db_service = crate::db_service::DbService::open_at_unencrypted(
            tempdir.path().join("briefing-hydration.db"),
        )
        .await
        .expect("test db service opens");
        let state = AppState::test_with_db_service(db_service);
        let config: crate::types::Config = serde_json::from_value(serde_json::json!({
            "workspacePath": "/tmp/dailyos-briefing-workspace"
        }))
        .expect("test config");
        *state.config.write() = Some(config);
        let producer_input =
            parse_producer_projection_input("dailyos/daily-briefing:briefing:local~2026-06-02", 3)
                .expect("briefing input");

        let hydrated = hydrate_producer_projection_input(&state, &producer_input)
            .await
            .expect("briefing input hydrates");

        assert_eq!(
            hydrated,
            serde_json::json!({
                "composition_id": "dailyos/daily-briefing:briefing:local~2026-06-02",
                "schema_version": 1,
                "expected_composition_version": 3,
                "workspace_scope": "local",
                "workspace_id": "/tmp/dailyos-briefing-workspace",
                "date": "2026-06-02",
            })
        );
    }
}
