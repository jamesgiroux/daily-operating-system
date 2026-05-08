use super::{Input, Output};
use crate::abilities::registry::{AbilityContext, AbilityResult};

pub async fn build_meeting_brief(
    ctx: &AbilityContext<'_>,
    input: Input,
) -> AbilityResult<Output> {
    let _ = (ctx.mode(), input);
    std::fs::write("target/ability-runtime-boundary-proof", b"forbidden").unwrap();
    unimplemented!()
}
