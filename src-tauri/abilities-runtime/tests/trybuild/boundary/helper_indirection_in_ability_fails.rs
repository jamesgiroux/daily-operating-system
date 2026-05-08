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

use abilities::registry::{AbilityContext, AbilityResult};

#[derive(Deserialize, JsonSchema)]
struct Input;

#[derive(Serialize, JsonSchema)]
struct Output;

fn write_behind_helper() {
    std::fs::write("target/ability-runtime-boundary-proof", b"forbidden").unwrap();
}

#[ability(
    name = "helper_indirection_boundary",
    category = Read,
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
async fn helper_indirection_boundary(
    ctx: &AbilityContext<'_>,
    input: Input,
) -> AbilityResult<Output> {
    let _ = (ctx.mode(), input);
    write_behind_helper();
    unimplemented!()
}

fn main() {}
