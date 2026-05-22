//! AC-461.5b — per-subject expected-fixture matrix.
//!
//! Replaces "where supported" wording from cycle-0 with an explicit
//! Account ✕ Project ✕ Person × fixture-class table. Absent expected
//! fixture = harness fail (this is the matrix-completeness assertion).
//!
//! Columns: fixture class. Rows: subject kind. `true` = fixture file
//! must exist under `fixtures/{subject}_{class}.json`.

use crate::harness::{fixtures_dir, list_fixture_files};

/// Per-subject expected-fixture matrix per AC-461.5b. Anchored
/// alphabetically by fixture class for review ergonomics.
pub const FIXTURE_MATRIX: &[FixtureClassExpectation] = &[
    FixtureClassExpectation {
        class: "metadata_proposal",
        account: true,
        project: true,
        person: false,
    },
    FixtureClassExpectation {
        class: "upcoming_touchpoint",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "recent_touchpoint",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "thread_summary",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "glean_citation",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "wrong_subject",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "ambiguous_association",
        account: false,
        project: false,
        person: true,
    },
    FixtureClassExpectation {
        class: "project_account_overlap",
        account: true,
        project: true,
        person: false,
    },
    FixtureClassExpectation {
        class: "parent_child",
        account: true,
        project: false,
        person: false,
    },
    // AC-461.5 additional fixture classes that apply broadly across all subjects:
    FixtureClassExpectation {
        class: "stale_fact",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "corrected_superseded",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "low_trust",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "open_loop",
        account: true,
        project: true,
        person: true,
    },
    FixtureClassExpectation {
        class: "confidential_user_only_claim",
        account: true,
        project: true,
        person: true,
    },
    // AC-461.6b distinguished failure mode fixture (Account-only by spec):
    FixtureClassExpectation {
        class: "claim_retracted_mid_render",
        account: true,
        project: false,
        person: false,
    },
];

#[derive(Debug, Clone, Copy)]
pub struct FixtureClassExpectation {
    pub class: &'static str,
    pub account: bool,
    pub project: bool,
    pub person: bool,
}

impl FixtureClassExpectation {
    /// Expected fixture filename for `(subject, class)` if the matrix expects
    /// it. Returns `None` if the cell is "-" (not applicable).
    pub fn filename_for(&self, subject: SubjectKind) -> Option<String> {
        let required = match subject {
            SubjectKind::Account => self.account,
            SubjectKind::Project => self.project,
            SubjectKind::Person => self.person,
        };
        if required {
            Some(format!("{}_{}.json", subject.prefix(), self.class))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectKind {
    Account,
    Project,
    Person,
}

impl SubjectKind {
    pub fn prefix(self) -> &'static str {
        match self {
            SubjectKind::Account => "account",
            SubjectKind::Project => "project",
            SubjectKind::Person => "person",
        }
    }

    pub fn all() -> [SubjectKind; 3] {
        [
            SubjectKind::Account,
            SubjectKind::Project,
            SubjectKind::Person,
        ]
    }
}

/// All fixture filenames the matrix requires to exist on disk.
pub fn required_fixture_filenames() -> Vec<String> {
    let mut out = Vec::new();
    for expectation in FIXTURE_MATRIX {
        for subject in SubjectKind::all() {
            if let Some(name) = expectation.filename_for(subject) {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn fixture_matrix_complete_ac_461_5b() {
    let required = required_fixture_filenames();
    let present: std::collections::BTreeSet<String> = list_fixture_files().into_iter().collect();

    let missing: Vec<&String> = required
        .iter()
        .filter(|name| !present.contains(*name))
        .collect();

    assert!(
        missing.is_empty(),
        "AC-461.5b matrix-completeness failure — required fixtures missing under {}: {:?}",
        fixtures_dir().display(),
        missing
    );
}

#[test]
fn fixture_matrix_no_orphan_unexpected_fixtures() {
    // Every fixture on disk must be one of the matrix-required filenames OR a
    // red-first proof fixture (prefixed `__bad_`). This prevents fixture-set
    // drift where a fixture is added but the matrix isn't updated.
    let required: std::collections::BTreeSet<String> =
        required_fixture_filenames().into_iter().collect();
    let allowed_prefixes = ["__bad_", "__good_"];

    let orphans: Vec<String> = list_fixture_files()
        .into_iter()
        .filter(|name| {
            !required.contains(name)
                && !allowed_prefixes
                    .iter()
                    .any(|prefix| name.starts_with(prefix))
        })
        .collect();

    assert!(
        orphans.is_empty(),
        "fixtures present on disk but absent from matrix (update FIXTURE_MATRIX or rename `__bad_`/`__good_*`): {:?}",
        orphans
    );
}
