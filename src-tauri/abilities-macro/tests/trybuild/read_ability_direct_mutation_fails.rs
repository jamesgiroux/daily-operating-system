#![allow(dead_code, unused_imports)]

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

    pub mod accounts {
        pub fn update_account_field() {}
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
        pub struct CompositionId(pub String);

        impl CompositionId {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }
        }

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

        pub struct AbilityPolicy {
            pub allowed_actors: Vec<Actor>,
            pub allowed_modes: Vec<ExecutionMode>,
            pub requires_confirmation: bool,
            pub may_publish: bool,
        }

        pub struct ComposesEntry {
            pub id: CompositionId,
            pub ability: String,
            pub optional: bool,
        }

        pub struct SignalPolicy {
            pub emits_on_output_change: Vec<String>,
            pub coalesce: bool,
        }

        pub struct AbilityDescriptor {
            pub name: &'static str,
            pub version: &'static str,
            pub schema_version: u32,
            pub category: AbilityCategory,
            pub policy: AbilityPolicy,
            pub composes: Vec<ComposesEntry>,
            pub mutates: Vec<&'static str>,
            pub experimental: bool,
            pub registered_at: Option<&'static str>,
            pub signal_policy: SignalPolicy,
            pub invoke_erased:
                fn(&AbilityContext<'_>, serde_json::Value) -> Result<serde_json::Value, AbilityError>,
            pub input_schema: fn() -> serde_json::Value,
            pub output_schema: fn() -> serde_json::Value,
        }

        inventory::collect!(AbilityDescriptor);

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

#[ability(
    name = "read_direct_mutation",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn read_direct_mutation(
    _ctx: &AbilityContext<'_>,
    _input: FixtureInput,
) -> AbilityResult<FixtureOutput> {
    services::accounts::update_account_field();
    Ok(AbilityOutput {
        data: FixtureOutput,
    })
}

fn main() {}
