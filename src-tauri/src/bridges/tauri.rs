use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use parking_lot::Mutex;

use crate::abilities::AbilityRegistry;
use crate::abilities::NOOP_ABILITY_TRACER;
use crate::bridges::types::{invoke_registry_json, provider_from_context_snapshot, surface_error};
use crate::bridges::{
    AbilityResponseJson, BridgeActor, BridgeSurface, BridgeSurfaceError, ConfirmationToken,
    InvocationContext, UserAttestationRequest,
};
use crate::services::context::Clock;
use crate::services::context::ExecutionMode;
use crate::state::AppState;

const CONFIRMATION_RATE_LIMIT_MAX_REQUESTS: u32 = 3;
const CONFIRMATION_RATE_LIMIT_WINDOW_SECONDS: i64 = 5 * 60;

#[derive(Debug, Clone)]
struct RateLimitWindow {
    started_at: DateTime<Utc>,
    count: u32,
}

pub trait UserAttestationHost: Send + Sync {
    fn request_user_attestation<'a>(
        &'a self,
        request: UserAttestationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), BridgeSurfaceError>> + Send + 'a>>;
}

#[derive(Default)]
struct PendingUserAttestationHost;

impl UserAttestationHost for PendingUserAttestationHost {
    fn request_user_attestation<'a>(
        &'a self,
        _request: UserAttestationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), BridgeSurfaceError>> + Send + 'a>> {
        Box::pin(std::future::pending())
    }
}

impl UserAttestationHost for AppState {
    fn request_user_attestation<'a>(
        &'a self,
        request: UserAttestationRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), BridgeSurfaceError>> + Send + 'a>> {
        Box::pin(async move {
            self.request_confirmation_attestation(request).await;
            Ok(())
        })
    }
}

pub struct TauriAbilityBridge<'registry> {
    registry: &'registry AbilityRegistry,
    rate_limits: Arc<Mutex<HashMap<(BridgeActor, String), RateLimitWindow>>>,
    attestation_host: Arc<dyn UserAttestationHost>,
}

impl<'registry> TauriAbilityBridge<'registry> {
    pub fn new(registry: &'registry AbilityRegistry) -> Self {
        Self::new_with_attestation_host(
            registry,
            Arc::new(PendingUserAttestationHost),
        )
    }

    pub fn new_with_attestation_host(
        registry: &'registry AbilityRegistry,
        attestation_host: Arc<dyn UserAttestationHost>,
    ) -> Self {
        Self {
            registry,
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
            attestation_host,
        }
    }

    pub async fn invoke(
        &self,
        state: &AppState,
        ability_name: &str,
        input_json: serde_json::Value,
        dry_run: bool,
        confirmation: Option<&ConfirmationToken>,
    ) -> Result<AbilityResponseJson, BridgeSurfaceError> {
        if state.lock_state.lock().is_locked {
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }

        let snapshot = state.context_snapshot();
        let provider = provider_from_context_snapshot(&snapshot);
        let services = state.live_service_context().with_actor("user");
        let invocation = InvocationContext {
            actor: BridgeActor::User,
            mode: ExecutionMode::Live,
            surface: BridgeSurface::TauriApp,
            dry_run,
            confirmation,
        };

        invoke_registry_json(
            self.registry,
            &services,
            provider,
            &NOOP_ABILITY_TRACER,
            invocation,
            ability_name,
            input_json,
        )
        .await
        .map_err(surface_error)
    }

    pub async fn issue_confirmation_token(
        &self,
        actor: BridgeActor,
        ability: String,
        args_hash: [u8; 32],
        user_attestation: UserAttestationRequest,
    ) -> Result<ConfirmationToken, BridgeSurfaceError> {
        if actor != user_attestation.actor
            || ability != user_attestation.ability
            || args_hash != user_attestation.args_hash
        {
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }

        self.consume_rate_limit(actor, &ability, user_attestation.requested_at)?;

        tokio::time::timeout(
            Duration::from_secs(user_attestation.ttl_seconds as u64),
            self.attestation_host
                .request_user_attestation(user_attestation.clone()),
        )
        .await
        .map_err(|_| BridgeSurfaceError::AbilityUnavailable)??;

        Ok(ConfirmationToken {
            actor,
            ability,
            args_hash,
            issued_at: user_attestation.requested_at,
            ttl_seconds: user_attestation.ttl_seconds,
            token: uuid::Uuid::new_v4().to_string(),
        })
    }

    pub fn user_attestation_request(
        &self,
        actor: BridgeActor,
        ability: String,
        args_hash: [u8; 32],
        ttl_seconds: u32,
    ) -> UserAttestationRequest {
        UserAttestationRequest {
            actor,
            ability,
            args_hash,
            requested_at: self.now(),
            ttl_seconds,
        }
    }

    fn now(&self) -> DateTime<Utc> {
        crate::services::context::SystemClock.now()
    }

    fn consume_rate_limit(
        &self,
        actor: BridgeActor,
        ability: &str,
        now: DateTime<Utc>,
    ) -> Result<(), BridgeSurfaceError> {
        let mut rate_limits = self.rate_limits.lock();
        let key = (actor, ability.to_string());
        let window = rate_limits.entry(key).or_insert_with(|| RateLimitWindow {
            started_at: now,
            count: 0,
        });

        if now.signed_duration_since(window.started_at).num_seconds()
            >= CONFIRMATION_RATE_LIMIT_WINDOW_SECONDS
        {
            window.started_at = now;
            window.count = 0;
        }

        if window.count >= CONFIRMATION_RATE_LIMIT_MAX_REQUESTS {
            return Err(BridgeSurfaceError::AbilityUnavailable);
        }

        window.count += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;

    use chrono::TimeZone;
    use async_trait::async_trait;
    use serde_json::json;
    use tokio::sync::Notify;

    use super::*;
    use crate::abilities::registry::{AbilityPolicy, SignalPolicy};
    use crate::abilities::{
        AbilityCategory, AbilityContext, AbilityDescriptor, AbilityError, AbilityErrorKind, Actor,
    };
    use crate::bridges::confirmation_args_hash;
    use crate::bridges::types::{BridgeRejectReason, PRE_DISPATCH_RESOLUTION_ORDER};
    use crate::intelligence::provider::{
        Completion, IntelligenceProvider, ModelName, ModelTier, PromptInput, ProviderError,
        ProviderKind,
    };

    const USER_ACTORS: &[Actor] = &[Actor::User];
    const ADMIN_ACTORS: &[Actor] = &[Actor::Admin];
    const USER_SYSTEM_ACTORS: &[Actor] = &[Actor::User, Actor::System];
    const LIVE_MODES: &[ExecutionMode] = &[ExecutionMode::Live];
    const EVALUATE_MODES: &[ExecutionMode] = &[ExecutionMode::Evaluate];

    static PRE_ADMISSION_STARTED: Notify = Notify::const_new();
    static PRE_ADMISSION_RELEASE: Notify = Notify::const_new();
    static PROVIDER_SNAPSHOT_STARTED: Notify = Notify::const_new();
    static PROVIDER_SNAPSHOT_RELEASE: Notify = Notify::const_new();

    type ErasedFuture<'a> =
        Pin<Box<dyn Future<Output = Result<serde_json::Value, AbilityError>> + Send + 'a>>;

    fn success_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Ok(envelope_json(
                ctx,
                json!({
                    "input": input,
                    "actor": format!("{:?}", ctx.actor),
                    "mode": ctx.mode().as_str(),
                }),
            ))
        })
    }

    fn pre_admission_wait_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        _input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            PRE_ADMISSION_STARTED.notify_one();
            PRE_ADMISSION_RELEASE.notified().await;
            Ok(envelope_json(
                ctx,
                json!({
                    "completed_after_lock": true,
                    "mode": ctx.mode().as_str(),
                }),
            ))
        })
    }

    fn provider_snapshot_erased<'a>(
        ctx: &'a AbilityContext<'a>,
        _input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            let before = ctx.provider.current_model(ModelTier::Synthesis).as_str().to_string();
            PROVIDER_SNAPSHOT_STARTED.notify_one();
            PROVIDER_SNAPSHOT_RELEASE.notified().await;
            let after = ctx.provider.current_model(ModelTier::Synthesis).as_str().to_string();
            Ok(envelope_json(
                ctx,
                json!({
                    "provider_before": before,
                    "provider_after": after,
                }),
            ))
        })
    }

    fn panic_if_dispatched_erased<'a>(
        _ctx: &'a AbilityContext<'a>,
        _input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            panic!("schema-invalid bridge input reached ability dispatch")
        })
    }

    fn leaking_error_erased<'a>(
        _ctx: &'a AbilityContext<'a>,
        input: serde_json::Value,
    ) -> ErasedFuture<'a> {
        Box::pin(async move {
            Err(AbilityError {
                kind: AbilityErrorKind::HardError("provider prompt leaked".to_string()),
                message: format!("raw prompt was {}", input["prompt"]),
            })
        })
    }

    fn envelope_json(ctx: &AbilityContext<'_>, data: serde_json::Value) -> serde_json::Value {
        json!({
            "data": data,
            "ability_version": { "major": 1, "minor": 0 },
            "diagnostics": { "warnings": [] },
            "provenance": {
                "invocation_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
                "ability_name": "fixture",
                "ability_version": { "major": 1, "minor": 0 },
                "ability_schema_version": 1,
                "actor": format!("{:?}", ctx.actor),
                "mode": ctx.mode().as_str(),
                "warnings": []
            }
        })
    }

    fn descriptor(
        name: &'static str,
        category: AbilityCategory,
        actors: &'static [Actor],
        modes: &'static [ExecutionMode],
        invoke_erased: for<'a> fn(&'a AbilityContext<'a>, serde_json::Value) -> ErasedFuture<'a>,
    ) -> AbilityDescriptor {
        AbilityDescriptor {
            name,
            version: "1.0.0",
            schema_version: 1,
            category,
            policy: AbilityPolicy {
                allowed_actors: actors,
                allowed_modes: modes,
                requires_confirmation: false,
                may_publish: false,
            },
            composes: &[],
            mutates: &[],
            experimental: false,
            registered_at: None,
            signal_policy: SignalPolicy {
                emits_on_output_change: &[],
                coalesce: false,
            },
            invoke_erased,
            input_schema: closed_object_schema,
            output_schema: closed_object_schema,
        }
    }

    fn registry(descriptors: Vec<AbilityDescriptor>) -> AbilityRegistry {
        AbilityRegistry::from_descriptors_checked(descriptors).unwrap()
    }

    fn closed_object_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "prompt": { "type": "string" },
                "subject": { "type": "string" },
                "value": {}
            }
        })
    }

    fn open_object_schema() -> serde_json::Value {
        json!({ "type": "object" })
    }

    fn strict_subject_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["subject"],
            "properties": {
                "subject": { "type": "string" }
            }
        })
    }

    fn actor_override_schema() -> serde_json::Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "actor": { "type": "string" }
            }
        })
    }

    fn with_input_schema(
        mut descriptor: AbilityDescriptor,
        input_schema: fn() -> serde_json::Value,
    ) -> AbilityDescriptor {
        descriptor.input_schema = input_schema;
        descriptor
    }

    fn issued_at() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 4, 12, 0, 0).unwrap()
    }

    fn confirmation_token(
        actor: BridgeActor,
        ability: &str,
        input: &serde_json::Value,
        issued_at: DateTime<Utc>,
        ttl_seconds: u32,
    ) -> ConfirmationToken {
        ConfirmationToken {
            actor,
            ability: ability.to_string(),
            args_hash: confirmation_args_hash(input),
            issued_at,
            ttl_seconds,
            token: "fixture-token".to_string(),
        }
    }

    fn user_attestation_request(
        actor: BridgeActor,
        ability: &str,
        input: &serde_json::Value,
    ) -> UserAttestationRequest {
        UserAttestationRequest {
            actor,
            ability: ability.to_string(),
            args_hash: confirmation_args_hash(input),
            requested_at: issued_at(),
            ttl_seconds: 300,
        }
    }

    #[derive(Default)]
    struct ApprovingAttestationHost {
        requests: parking_lot::Mutex<Vec<UserAttestationRequest>>,
    }

    struct FixtureProvider {
        model: &'static str,
    }

    #[async_trait]
    impl IntelligenceProvider for FixtureProvider {
        async fn complete(
            &self,
            _prompt: PromptInput,
            _tier: ModelTier,
        ) -> Result<Completion, ProviderError> {
            Ok(Completion::default())
        }

        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Other("tauri-fixture")
        }

        fn current_model(&self, _tier: ModelTier) -> ModelName {
            ModelName::new(self.model)
        }
    }

    impl UserAttestationHost for ApprovingAttestationHost {
        fn request_user_attestation<'a>(
            &'a self,
            request: UserAttestationRequest,
        ) -> Pin<Box<dyn Future<Output = Result<(), BridgeSurfaceError>> + Send + 'a>> {
            self.requests.lock().push(request);
            Box::pin(async { Ok(()) })
        }
    }

    async fn error_bytes_for(registry: AbilityRegistry, ability_name: &'static str) -> Vec<u8> {
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);
        let err = bridge
            .invoke(&state, ability_name, json!({}), false, None)
            .await
            .unwrap_err();
        serde_json::to_vec(&err).unwrap()
    }

    #[test]
    fn confirmation_token_scoped_to_actor_ability_args_hash_ttl() {
        let input = json!({ "subject": "dailyos" });
        let token = confirmation_token(
            BridgeActor::Agent,
            "agent_write",
            &input,
            issued_at(),
            300,
        );

        assert_eq!(token.actor, BridgeActor::Agent);
        assert_eq!(token.ability, "agent_write");
        assert_eq!(token.args_hash, confirmation_args_hash(&input));
        assert_eq!(token.issued_at, issued_at());
        assert_eq!(token.ttl_seconds, 300);
        assert_eq!(token.token, "fixture-token");
    }

    #[test]
    fn confirmation_token_matches_returns_true_for_matching_triple_and_unexpired() {
        let input = json!({ "subject": "dailyos" });
        let token = confirmation_token(
            BridgeActor::Agent,
            "agent_write",
            &input,
            issued_at(),
            300,
        );
        let args_hash = confirmation_args_hash(&input);

        assert!(token.matches(&BridgeActor::Agent, "agent_write", &args_hash));
        assert!(!token.is_expired(issued_at() + chrono::Duration::seconds(299)));
    }

    #[test]
    fn confirmation_token_matches_returns_false_for_wrong_args_hash() {
        let token = confirmation_token(
            BridgeActor::Agent,
            "agent_write",
            &json!({ "subject": "dailyos" }),
            issued_at(),
            300,
        );
        let wrong_args_hash = confirmation_args_hash(&json!({ "subject": "other" }));

        assert!(!token.matches(&BridgeActor::Agent, "agent_write", &wrong_args_hash));
    }

    #[test]
    fn confirmation_token_is_expired_after_ttl() {
        let token = confirmation_token(
            BridgeActor::Agent,
            "agent_write",
            &json!({}),
            issued_at(),
            300,
        );

        assert!(token.is_expired(issued_at() + chrono::Duration::seconds(300)));
    }

    #[tokio::test]
    async fn tauri_bridge_issue_confirmation_token_blocks_on_user_attestation() {
        let registry = registry(vec![]);
        let state = Arc::new(AppState::new());
        let host: Arc<dyn UserAttestationHost> = state.clone();
        let bridge = TauriAbilityBridge::new_with_attestation_host(&registry, host);
        let request =
            user_attestation_request(BridgeActor::Agent, "agent_write", &json!({ "x": 1 }));

        let issue = bridge.issue_confirmation_token(
            BridgeActor::Agent,
            "agent_write".to_string(),
            request.args_hash,
            request.clone(),
        );
        tokio::pin!(issue);

        let result = tokio::time::timeout(Duration::from_millis(25), &mut issue).await;

        assert!(result.is_err());
        assert_eq!(state.pending_confirmation_attestation_requests(), vec![request]);
    }

    #[tokio::test]
    async fn tauri_bridge_issue_confirmation_token_rate_limited_per_actor_ability() {
        let registry = registry(vec![]);
        let host = Arc::new(ApprovingAttestationHost::default());
        let bridge = TauriAbilityBridge::new_with_attestation_host(&registry, host.clone());
        let request =
            user_attestation_request(BridgeActor::Agent, "agent_write", &json!({ "x": 1 }));

        for _ in 0..3 {
            let token = bridge
                .issue_confirmation_token(
                    BridgeActor::Agent,
                    "agent_write".to_string(),
                    request.args_hash,
                    request.clone(),
                )
                .await
                .unwrap();
            assert!(token.matches(&BridgeActor::Agent, "agent_write", &request.args_hash));
        }

        let err = bridge
            .issue_confirmation_token(
                BridgeActor::Agent,
                "agent_write".to_string(),
                request.args_hash,
                request,
            )
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
        assert_eq!(host.requests.lock().len(), 3);
    }

    #[tokio::test]
    async fn tauri_bridge_rejects_locked_app() {
        let registry = registry(vec![]);
        let state = AppState::new();
        state.lock_state.lock().is_locked = true;

        let err = TauriAbilityBridge::new(&registry)
            .invoke(&state, "missing", json!({}), false, None)
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }

    #[tokio::test]
    async fn invoke_ability_rejects_unknown_ability_without_enumeration() {
        let registry = registry(vec![descriptor(
            "visible_read",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            success_erased,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let err = bridge
            .invoke(&state, "not_registered", json!({}), false, None)
            .await
            .unwrap_err();
        let serialized = serde_json::to_vec(&err).unwrap();

        assert_eq!(serialized, br#""ability_unavailable""#);
        assert!(!String::from_utf8_lossy(&serialized).contains("visible_read"));
    }

    #[tokio::test]
    async fn tauri_bridge_documents_lock_as_pre_admission_only() {
        let registry = registry(vec![descriptor(
            "pre_admission",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            pre_admission_wait_erased,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let invoke = bridge.invoke(&state, "pre_admission", json!({}), false, None);
        let lock_after_admission = async {
            PRE_ADMISSION_STARTED.notified().await;
            state.lock_state.lock().is_locked = true;
            PRE_ADMISSION_RELEASE.notify_one();
        };

        let (response, _) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(invoke, lock_after_admission)
        })
        .await
        .unwrap();

        let response = response.unwrap();
        assert_eq!(response.data["completed_after_lock"], true);
        assert!(state.lock_state.lock().is_locked);
    }

    #[tokio::test]
    async fn bridge_provider_snapshot_is_consistent_per_invocation() {
        let registry = registry(vec![descriptor(
            "provider_snapshot",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            provider_snapshot_erased,
        )]);
        let state = AppState::new();
        let first_provider = Arc::new(FixtureProvider { model: "first-model" });
        let second_provider = Arc::new(FixtureProvider { model: "second-model" });
        state.swap_intelligence_provider(Some(first_provider));
        let bridge = TauriAbilityBridge::new(&registry);

        let invoke = bridge.invoke(&state, "provider_snapshot", json!({}), false, None);
        let swap_after_snapshot = async {
            PROVIDER_SNAPSHOT_STARTED.notified().await;
            state.swap_intelligence_provider(Some(second_provider));
            PROVIDER_SNAPSHOT_RELEASE.notify_one();
        };

        let (response, _) = tokio::time::timeout(Duration::from_secs(2), async {
            tokio::join!(invoke, swap_after_snapshot)
        })
        .await
        .unwrap();

        let response = response.unwrap();
        assert_eq!(response.data["provider_before"], "first-model");
        assert_eq!(response.data["provider_after"], "first-model");
    }

    #[tokio::test]
    async fn invoke_ability_schema_invalid_input_fails_before_dispatch() {
        let registry = registry(vec![with_input_schema(
            descriptor(
                "strict_subject",
                AbilityCategory::Read,
                USER_ACTORS,
                LIVE_MODES,
                panic_if_dispatched_erased,
            ),
            strict_subject_schema,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let err = bridge
            .invoke(
                &state,
                "strict_subject",
                json!({ "subject": 217 }),
                false,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }

    #[tokio::test]
    async fn invoke_ability_rejects_actor_override_in_input_json() {
        let registry = registry(vec![with_input_schema(
            descriptor(
                "actor_echo",
                AbilityCategory::Read,
                USER_ACTORS,
                LIVE_MODES,
                success_erased,
            ),
            actor_override_schema,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let err = bridge
            .invoke(
                &state,
                "actor_echo",
                json!({ "actor": "System" }),
                false,
                None,
            )
            .await
            .unwrap_err();

        assert_eq!(err, BridgeSurfaceError::AbilityUnavailable);
    }

    #[tokio::test]
    async fn invoke_ability_returns_ability_response_json_with_tauri_provenance() {
        let registry = registry(vec![descriptor(
            "tauri_response",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            success_erased,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let response = bridge
            .invoke(&state, "tauri_response", json!({}), false, None)
            .await
            .unwrap();

        assert_eq!(response.ability_name, "tauri_response");
        assert_eq!(response.ability_version, "1.0.0");
        assert_eq!(response.schema_version, 1);
        assert_eq!(response.data["actor"], "User");
        assert_eq!(response.data["mode"], "live");
        assert_eq!(response.rendered_provenance.surface, BridgeSurface::TauriApp);
        assert_eq!(response.rendered_provenance.value["actor"], "User");
    }

    #[tokio::test]
    async fn bridge_provenance_tests_compare_rendered_surface_output_only() {
        let registry = registry(vec![descriptor(
            "rendered_surface_only",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            success_erased,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);

        let response = bridge
            .invoke(&state, "rendered_surface_only", json!({}), false, None)
            .await
            .unwrap();

        let rendered = response.rendered_provenance;
        assert_eq!(rendered.surface, BridgeSurface::TauriApp);
        assert_eq!(rendered.value["mode"], "live");
        assert_eq!(rendered.value["ability_name"], "fixture");
    }

    #[tokio::test]
    async fn bridge_errors_do_not_include_input_or_prompt_content() {
        let registry = registry(vec![descriptor(
            "leaking_error",
            AbilityCategory::Read,
            USER_ACTORS,
            LIVE_MODES,
            leaking_error_erased,
        )]);
        let state = AppState::new();
        let bridge = TauriAbilityBridge::new(&registry);
        let secret = "PROMPT_SECRET_DOS_217";

        let err = bridge
            .invoke(
                &state,
                "leaking_error",
                json!({ "prompt": secret }),
                false,
                None,
            )
            .await
            .unwrap_err();
        let serialized = String::from_utf8(serde_json::to_vec(&err).unwrap()).unwrap();

        assert_eq!(serialized, "\"ability_unavailable\"");
        assert!(!serialized.contains(secret));
        assert!(!serialized.contains("provider prompt leaked"));
    }

    #[tokio::test]
    async fn bridge_pre_dispatch_resolution_order_is_fixed_and_timing_independent() {
        assert_eq!(
            PRE_DISPATCH_RESOLUTION_ORDER,
            [
                BridgeRejectReason::UnknownAbility,
                BridgeRejectReason::ActorPolicy,
                BridgeRejectReason::ModePolicy,
                BridgeRejectReason::MaintenanceGate,
                BridgeRejectReason::ExperimentalGate,
            ]
        );

        let cases = [
            (
                registry(vec![descriptor(
                    "visible",
                    AbilityCategory::Read,
                    USER_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "unknown",
            ),
            (
                registry(vec![descriptor(
                    "admin_only",
                    AbilityCategory::Read,
                    ADMIN_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "admin_only",
            ),
            (
                registry(vec![descriptor(
                    "evaluate_only",
                    AbilityCategory::Read,
                    USER_ACTORS,
                    EVALUATE_MODES,
                    success_erased,
                )]),
                "evaluate_only",
            ),
            (
                registry(vec![descriptor(
                    "maintenance",
                    AbilityCategory::Maintenance,
                    USER_SYSTEM_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "maintenance",
            ),
            (
                registry(vec![descriptor(
                    "visible",
                    AbilityCategory::Read,
                    USER_ACTORS,
                    LIVE_MODES,
                    success_erased,
                )]),
                "experimental_hidden",
            ),
        ];

        let mut serialized_errors = Vec::new();
        for (registry, ability_name) in cases {
            serialized_errors.push(error_bytes_for(registry, ability_name).await);
        }

        for serialized in &serialized_errors[1..] {
            assert_eq!(serialized, &serialized_errors[0]);
        }
    }

    #[tokio::test]
    async fn bridge_unknown_ability_unauthorized_actor_maintenance_experimental_mode_all_yield_byte_equal_error(
    ) {
        let unknown = error_bytes_for(registry(vec![]), "unknown").await;
        let unauthorized_actor = error_bytes_for(
            registry(vec![descriptor(
                "admin_only",
                AbilityCategory::Read,
                ADMIN_ACTORS,
                LIVE_MODES,
                success_erased,
            )]),
            "admin_only",
        )
        .await;
        let maintenance = error_bytes_for(
            registry(vec![descriptor(
                "maintenance",
                AbilityCategory::Maintenance,
                USER_SYSTEM_ACTORS,
                LIVE_MODES,
                success_erased,
            )]),
            "maintenance",
        )
        .await;
        let experimental = error_bytes_for(registry(vec![]), "experimental_hidden").await;
        let mode_hidden = error_bytes_for(
            registry(vec![descriptor(
                "evaluate_only",
                AbilityCategory::Read,
                USER_ACTORS,
                EVALUATE_MODES,
                success_erased,
            )]),
            "evaluate_only",
        )
        .await;

        assert_eq!(unauthorized_actor, unknown);
        assert_eq!(maintenance, unknown);
        assert_eq!(experimental, unknown);
        assert_eq!(mode_hidden, unknown);
    }

    #[tokio::test]
    async fn invoke_ability_rejects_descriptor_with_open_schema_at_runtime_via_byte_equal_unavailable(
    ) {
        let unknown = error_bytes_for(registry(vec![]), "unknown").await;
        let open_schema = error_bytes_for(
            AbilityRegistry::from_descriptors_unchecked_for_runtime_validation_tests(vec![
                with_input_schema(
                    descriptor(
                        "open_runtime_schema",
                        AbilityCategory::Read,
                        USER_ACTORS,
                        LIVE_MODES,
                        success_erased,
                    ),
                    open_object_schema,
                ),
            ]),
            "open_runtime_schema",
        )
        .await;

        assert_eq!(open_schema, unknown);
        assert_eq!(open_schema, br#""ability_unavailable""#);
    }
}
