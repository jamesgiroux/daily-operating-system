#![allow(dead_code, unused_imports)]

mod abilities {
    pub mod registry {
        pub use abilities_runtime::abilities::registry::*;
    }

    pub mod provenance {
        pub use abilities_runtime::abilities::provenance::*;
    }

    pub mod prepare_meeting {
        use crate::abilities::registry::{AbilityContext, AbilityResult};
        use dailyos_abilities_macro::ability;
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        pub mod synthesis {
            use crate::abilities::registry::{AbilityContext, AbilityResult};

            use super::{Input, Output};

            pub async fn build_meeting_brief(
                ctx: &AbilityContext<'_>,
                input: Input,
            ) -> AbilityResult<Output> {
                let _ = (ctx.mode(), input);
                std::fs::write("target/ability-runtime-boundary-proof", b"forbidden").unwrap();
                unimplemented!()
            }
        }

        #[derive(Deserialize, JsonSchema)]
        pub struct Input;

        #[derive(Serialize, JsonSchema)]
        pub struct Output;

        #[ability(
            name = "crate_qualified_helper_wrapper_boundary",
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
        pub async fn crate_qualified_helper_wrapper_boundary(
            ctx: &AbilityContext<'_>,
            input: Input,
        ) -> AbilityResult<Output> {
            crate::abilities::prepare_meeting::synthesis::build_meeting_brief(ctx, input).await
        }
    }
}

fn main() {}
