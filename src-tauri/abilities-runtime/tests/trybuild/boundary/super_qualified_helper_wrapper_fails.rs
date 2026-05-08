#![allow(dead_code, unused_imports)]

mod abilities {
    pub mod registry {
        pub use abilities_runtime::abilities::registry::*;
    }

    pub mod provenance {
        pub use abilities_runtime::abilities::provenance::*;
    }
}

mod boundary {
    pub mod helper {
        pub fn write_behind_helper() {
            std::fs::write("target/ability-runtime-boundary-proof", b"forbidden").unwrap();
        }
    }

    pub mod wrapper {
        use crate::abilities::registry::{AbilityContext, AbilityResult};
        use dailyos_abilities_macro::ability;
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};

        #[derive(Deserialize, JsonSchema)]
        pub struct Input;

        #[derive(Serialize, JsonSchema)]
        pub struct Output;

        #[ability(
            name = "super_qualified_helper_wrapper_boundary",
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
        pub async fn super_qualified_helper_wrapper_boundary(
            ctx: &AbilityContext<'_>,
            input: Input,
        ) -> AbilityResult<Output> {
            let _ = (ctx.mode(), input);
            super::helper::write_behind_helper();
            unimplemented!()
        }
    }
}

fn main() {}
