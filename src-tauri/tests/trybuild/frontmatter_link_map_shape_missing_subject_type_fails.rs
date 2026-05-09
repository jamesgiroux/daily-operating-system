mod abilities {
    pub mod claims {
        #[derive(Clone, Copy)]
        pub enum CanonicalSubjectType {
            Meeting,
        }
    }
}

mod services {
    pub mod claims {
        pub mod link_map {
            use crate::abilities::claims::CanonicalSubjectType;

            pub enum EdgeDirection {
                Forward,
                Incoming,
            }

            pub struct LinkRule {
                pub field: &'static str,
                pub edge_type: &'static str,
                pub direction: EdgeDirection,
                pub fanout: bool,
                pub subject_type: CanonicalSubjectType,
            }
        }
    }
}

include!(concat!(
    env!("DAILYOS_SRC_TAURI"),
    "/src/services/claims/link_map_macro.rs"
));

use services::claims::link_map::EdgeDirection;

frontmatter_link_map! {
    pub const TRYBUILD_FRONTMATTER_LINK_MAP = [
        {
            field: "account",
            edge_type: "trybuild_mentions_account",
            direction: EdgeDirection::Forward,
            fanout: false,
        },
    ];
}

fn main() {
    let _ = abilities::claims::CanonicalSubjectType::Meeting;
}
