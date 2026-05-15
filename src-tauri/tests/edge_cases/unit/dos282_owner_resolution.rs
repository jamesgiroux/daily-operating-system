use dailyos_lib::abilities::extractors::commitment::OwnerRef;
use dailyos_lib::abilities::read::resolve_owner::resolve_owner;
use rusqlite::params;

use crate::support::{action_db, owner_resolution_db};

#[test]
fn owner_resolution_covers_exact_alias_fuzzy_ambiguous_and_unassigned() {
    let conn = owner_resolution_db();
    conn.execute(
        "INSERT INTO people (id, email, name, updated_at) VALUES (?1, ?2, ?3, '2026-05-15')",
        params!["person-alex", "alex@example.com", "Alex Example"],
    )
    .expect("insert person");
    conn.execute(
        "INSERT INTO people (id, email, name, updated_at) VALUES (?1, ?2, ?3, '2026-05-15')",
        params!["person-alex-alt", "alex.alt@example.com", "Alex Example"],
    )
    .expect("insert duplicate name");
    conn.execute(
        "INSERT INTO people (id, email, name, updated_at) VALUES (?1, ?2, ?3, '2026-05-15')",
        params!["person-jamie", "jamie@example.com", "Jamie Example"],
    )
    .expect("insert fuzzy person");
    for person_id in ["person-alex", "person-alex-alt", "person-jamie"] {
        conn.execute(
            "INSERT INTO account_stakeholders (account_id, person_id, role) VALUES ('account-example', ?1, 'member')",
            params![person_id],
        )
        .expect("insert stakeholder");
    }

    let db = action_db(&conn);
    assert_eq!(
        resolve_owner(db, "account-example", "commitment-1", Some("alex@example.com"))
            .expect("exact email")
            .owner_entity_id
            .as_deref(),
        Some("person-alex")
    );
    assert_eq!(
        resolve_owner(db, "account-example", "commitment-2", Some("Jamie Exampel"))
            .expect("fuzzy name")
            .owner_entity_id
            .as_deref(),
        Some("person-jamie")
    );
    assert!(matches!(
        resolve_owner(db, "account-example", "commitment-3", Some("Alex Example"))
            .expect("ambiguous exact name")
            .owner_ref,
        OwnerRef::Ambiguous { .. }
    ));
    assert_eq!(
        resolve_owner(db, "account-example", "commitment-4", None)
            .expect("unassigned")
            .owner_ref,
        OwnerRef::Unassigned
    );
}
