//! Raw migration/backfill API. NOT for runtime callers.
//! Production code importing this module fails the pre-commit grep hook.

pub fn raw_rebuild_account_domains(_db: &crate::db::ActionDb) -> Result<(), String> {
    // TODO(Lane-A): implement trusted-source rebuild. See DOS-258 "account_domains trust rebuild".
    unimplemented!("Lane A completion task")
}
