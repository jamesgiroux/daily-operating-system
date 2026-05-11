#![allow(dead_code, unused_imports)]
//! W1-B compile-error gate positive test (ADR-0102 §7.6, DOS-546 W1-B
//! AC §449 + §1254):
//!
//! An ability whose `allowed_actors` includes `SurfaceClient` AND declares
//! a non-empty `required_scopes = [...]` set must compile cleanly. This
//! is the legitimate W1-D account-overview shape: the SurfaceClient may
//! invoke the ability, gated by the named scope grant.
//!
//! See `surface_client_without_scopes_fails.rs` for the matching negative
//! case (SurfaceClient + empty scopes + no opt-out → compile error).

use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod services {
    pub mod context {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ExecutionMode {
            Live,
            Simulate,
            Evaluate,
        }

        impl ExecutionMode {
            pub fn as_str(self) -> &'static str {
                match self {
                    Self::Live => "live",
                    Self::Simulate => "simulate",
                    Self::Evaluate => "evaluate",
                }
            }
        }
    }
}

mod observability {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Outcome {
        Ok,
        Err { kind: String },
    }

    pub struct InvocationRecord {
        pub invocation_id: uuid::Uuid,
        pub ability_name: String,
        pub ability_category: String,
        pub actor: String,
        pub mode: String,
        pub started_at: chrono::DateTime<chrono::Utc>,
        pub ended_at: chrono::DateTime<chrono::Utc>,
        pub outcome: Outcome,
        pub duration_ms: u64,
    }

    pub struct EvaluateModeSubscriber;

    impl EvaluateModeSubscriber {
        pub fn record(&self, _record: InvocationRecord) {}
    }
}

mod abilities {
    pub mod provenance {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct CompositionId(pub &'static str);

        impl CompositionId {
            pub const fn from_static(value: &'static str) -> Self {
                Self(value)
            }
        }

        #[derive(serde::Serialize, schemars::JsonSchema)]
        pub struct AbilityOutput<T> {
            pub data: T,
        }
    }

    pub mod registry {
        use std::marker::PhantomData;

        use crate::abilities::provenance::{AbilityOutput, CompositionId};
        use crate::services::context::ExecutionMode;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum AbilityCategory {
            Read,
            Transform,
            Publish,
            Maintenance,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Actor {
            Agent,
            User,
            Admin,
            System,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum ActorKind {
            Agent,
            User,
            Admin,
            System,
            SurfaceClient,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum McpExposure {
            None,
            MetadataOnly,
            Invocable,
        }

        pub struct AbilityPolicy {
            pub allowed_actors: &'static [ActorKind],
            pub allowed_modes: &'static [ExecutionMode],
            pub requires_confirmation: bool,
            pub may_publish: bool,
            pub required_scopes: &'static [&'static str],
            pub mcp_exposure: McpExposure,
            pub client_side_executable: bool,
        }

        pub struct ComposesEntry {
            pub id: CompositionId,
            pub ability: &'static str,
            pub optional: bool,
        }

        pub struct SignalPolicy {
            pub emits_on_output_change: &'static [&'static str],
            pub coalesce: bool,
        }

        pub struct AbilityDescriptor {
            pub name: &'static str,
            pub version: &'static str,
            pub schema_version: u32,
            pub category: AbilityCategory,
            pub policy: AbilityPolicy,
            pub composes: &'static [ComposesEntry],
            pub mutates: &'static [&'static str],
            pub experimental: bool,
            pub registered_at: Option<&'static str>,
            pub signal_policy: SignalPolicy,
            pub invoke_erased: for<'a> fn(
                &'a AbilityContext<'a>,
                serde_json::Value,
            ) -> std::pin::Pin<
                Box<
                    dyn std::future::Future<Output = Result<serde_json::Value, AbilityError>>
                        + Send
                        + 'a,
                >,
            >,
            pub input_schema: fn() -> serde_json::Value,
            pub output_schema: fn() -> serde_json::Value,
        }

        inventory::collect!(AbilityDescriptor);

        pub fn close_schema_objects(_schema: &mut serde_json::Value) {}

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum AbilityErrorKind {
            Validation,
            Capability,
            OptionalComposedReadFailed {
                composition_id: CompositionId,
                reason: String,
            },
            HardError(String),
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct AbilityError {
            pub kind: AbilityErrorKind,
            pub message: String,
        }

        pub type AbilityResult<T> = Result<AbilityOutput<T>, AbilityError>;

        pub struct AbilityContext<'a> {
            pub actor: Actor,
            mode: ExecutionMode,
            _marker: PhantomData<&'a ()>,
        }

        impl<'a> AbilityContext<'a> {
            pub fn mode(&self) -> ExecutionMode {
                self.mode
            }
        }
    }
}

use abilities::provenance::AbilityOutput;
use abilities::registry::{AbilityContext, AbilityResult};

#[derive(Deserialize, JsonSchema)]
struct FixtureInput;

#[derive(Serialize, JsonSchema)]
struct FixtureOutput;

// MUST compile cleanly: `SurfaceClient` is in `allowed_actors`,
// `required_scopes` is non-empty (the legitimate W1-D account-overview
// shape). The W1-B macro gate must NOT fire here.
#[ability(
    name = "surface_client_with_scopes",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User, SurfaceClient],
    allowed_modes = [Live],
    requires_confirmation = false,
    may_publish = false,
    required_scopes = ["read.account_overview"],
    mcp_exposure = None,
    client_side_executable = true,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn surface_client_with_scopes(
    _ctx: &AbilityContext<'_>,
    _input: FixtureInput,
) -> AbilityResult<FixtureOutput> {
    Ok(AbilityOutput {
        data: FixtureOutput,
    })
}

fn main() {}
