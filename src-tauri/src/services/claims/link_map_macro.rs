#[macro_export]
macro_rules! frontmatter_link_map {
    (
        $(#[$meta:meta])*
        $vis:vis const $name:ident = [
            $(
                {
                    field: $field:literal,
                    edge_type: $edge_type:literal,
                    direction: $direction:path,
                    fanout: $fanout:literal,
                    subject_type: $subject_type:path $(,)?
                }
            ),* $(,)?
        ];
    ) => {
        $(#[$meta])*
        $vis const $name: &[$crate::services::claims::link_map::LinkRule] = &[
            $(
                $crate::services::claims::link_map::LinkRule {
                    field: $field,
                    edge_type: $edge_type,
                    direction: $direction,
                    fanout: $fanout,
                    subject_type: $subject_type,
                },
            )*
        ];
    };
}
