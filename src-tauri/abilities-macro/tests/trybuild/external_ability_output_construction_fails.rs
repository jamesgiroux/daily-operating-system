mod abilities {
    pub mod provenance {
        pub struct Provenance;
        pub struct AbilityVersion;
        pub struct Diagnostics;

        pub struct AbilityOutput<T> {
            pub(in crate::abilities::provenance) data: T,
            pub(in crate::abilities::provenance) provenance: Provenance,
            pub(in crate::abilities::provenance) ability_version: AbilityVersion,
            pub(in crate::abilities::provenance) diagnostics: Diagnostics,
        }
    }
}

use abilities::provenance::{AbilityOutput, AbilityVersion, Diagnostics, Provenance};

fn main() {
    let _ = AbilityOutput {
        data: (),
        provenance: Provenance,
        ability_version: AbilityVersion,
        diagnostics: Diagnostics,
    };
}
