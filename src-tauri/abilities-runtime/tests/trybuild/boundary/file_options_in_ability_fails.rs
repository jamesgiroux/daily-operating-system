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

#[ability(
    name = "file_options_boundary",
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
async fn file_options_boundary(
    ctx: &AbilityContext<'_>,
    input: Input,
) -> AbilityResult<Output> {
    use std::fs::File;

    let _ = (ctx.mode(), input);
    let mut opts = File::options();
    opts.write(true);
    let _file = opts
        .open("target/ability-runtime-boundary-proof")
        .unwrap();
    unimplemented!()
}

fn main() {}
