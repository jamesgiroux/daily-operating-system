#![allow(dead_code, unused_imports)]

use dailyos_abilities_macro::ability;

mod db {
    pub struct ActionDb;
}

use db::ActionDb;

struct FixtureInput;
struct FixtureOutput;
type AbilityResult<T> = Result<T, ()>;

#[ability(
    name = "raw_actiondb_boundary",
    category = Read,
    version = "0.1.0",
    schema_version = 1,
    allowed_actors = [User],
    allowed_modes = [Evaluate],
    requires_confirmation = false,
    may_publish = false,
    composes = [],
    experimental = false,
    signal_policy = { emits_on_output_change = [], coalesce = false }
)]
async fn raw_actiondb_boundary(
    _db: &ActionDb,
    _input: FixtureInput,
) -> AbilityResult<FixtureOutput> {
    Ok(FixtureOutput)
}

fn main() {}
