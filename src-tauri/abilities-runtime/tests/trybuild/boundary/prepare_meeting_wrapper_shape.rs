#![allow(dead_code, unused_imports)]

use dailyos_abilities_macro::ability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod abilities {
    pub use abilities_runtime::abilities::*;
}

mod observability {
    pub use abilities_runtime::observability::*;
}

mod services {
    pub use abilities_runtime::services::*;
}

#[path = "prepare_meeting_wrapper_shape/synthesis.rs"]
mod synthesis;

use abilities::registry::{AbilityContext, AbilityResult};

#[derive(Deserialize, JsonSchema)]
pub struct Input;

#[derive(Serialize, JsonSchema)]
pub struct Output;

#[ability(
    name = "prepare_meeting_wrapper_shape",
    category = Transform,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [Agent],
    allowed_modes = [Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn prepare_meeting_wrapper_shape(
    ctx: &AbilityContext<'_>,
    input: Input,
) -> AbilityResult<Output> {
    synthesis::build_meeting_brief(ctx, input).await
}

fn main() {}
