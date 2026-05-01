#[path = "../src/scoring.rs"]
mod scoring;

use scoring::{path_is_allowlisted_mutator, MutationVisitor};

fn path(path: &str) -> syn::Path {
    syn::parse_str(path).expect("valid path")
}

fn scan(block: syn::Block) -> MutationVisitor {
    let mut visitor = MutationVisitor::new();
    visitor.scan_fn_body(&block);
    visitor
}

#[test]
fn allowlist_matches_direct_service_path() {
    assert!(path_is_allowlisted_mutator(&path(
        "services::accounts::update_account_field"
    )));
}

#[test]
fn allowlist_matches_crate_prefixed_service_path() {
    assert!(path_is_allowlisted_mutator(&path(
        "crate::services::accounts::update_account_field"
    )));
}

#[test]
fn allowlist_matches_nested_entity_linking_path() {
    assert!(path_is_allowlisted_mutator(&path(
        "services::entity_linking::cascade::backfill_account_domain_from_person"
    )));
}

#[test]
fn allowlist_rejects_non_mutator_path() {
    assert!(!path_is_allowlisted_mutator(&path(
        "services::nonexistent::function"
    )));
}

#[test]
fn visitor_detects_direct_call() {
    let visitor = scan(syn::parse_quote!({
        services::accounts::update_account_field(db, state, account_id, field, value);
    }));

    assert_eq!(
        visitor.detected,
        ["services::accounts::update_account_field"]
    );
}

#[test]
fn visitor_detects_crate_prefixed_call() {
    let visitor = scan(syn::parse_quote!({
        crate::services::accounts::update_account_field(db, state, account_id, field, value);
    }));

    assert_eq!(
        visitor.detected,
        ["services::accounts::update_account_field"]
    );
}

#[test]
fn visitor_detects_import_alias_call() {
    let visitor = scan(syn::parse_quote!({
        use services::accounts::update_account_field as foo;
        foo(db, state, account_id, field, value);
    }));

    assert_eq!(
        visitor.aliases.get("foo").map(String::as_str),
        Some("services::accounts::update_account_field")
    );
    assert_eq!(
        visitor.detected,
        ["services::accounts::update_account_field"]
    );
}

#[test]
fn visitor_detects_module_alias_call() {
    let visitor = scan(syn::parse_quote!({
        use crate::services::accounts;
        accounts::update_account_field(db, state, account_id, field, value);
    }));

    assert_eq!(
        visitor.module_aliases.get("accounts").map(String::as_str),
        Some("services::accounts")
    );
    assert_eq!(
        visitor.detected,
        ["services::accounts::update_account_field"]
    );
}

#[test]
fn visitor_detects_crate_module_alias_call() {
    let visitor = scan(syn::parse_quote!({
        use crate::services as svc;
        svc::accounts::update_account_field(db, state, account_id, field, value);
    }));

    assert_eq!(
        visitor.module_aliases.get("svc").map(String::as_str),
        Some("services")
    );
    assert_eq!(
        visitor.detected,
        ["services::accounts::update_account_field"]
    );
}

#[test]
fn visitor_detects_alias_even_when_use_follows_call() {
    let visitor = scan(syn::parse_quote!({
        foo(db, state, account_id, field, value);
        use services::accounts::update_account_field as foo;
    }));

    assert_eq!(
        visitor.detected,
        ["services::accounts::update_account_field"]
    );
}

#[test]
fn visitor_ignores_nonexistent_service_call() {
    let visitor = scan(syn::parse_quote!({
        services::nonexistent::function(db);
    }));

    assert!(visitor.detected.is_empty());
}

#[test]
fn visitor_ignores_method_call_with_mutator_like_name() {
    let visitor = scan(syn::parse_quote!({
        db.update_account(account_id, value);
    }));

    assert!(visitor.detected.is_empty());
}
