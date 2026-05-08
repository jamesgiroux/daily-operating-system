#[path = "../src/scoring.rs"]
mod scoring;

use scoring::{path_is_allowlisted_mutator, BoundaryVisitor, MutationVisitor};

fn path(path: &str) -> syn::Path {
    syn::parse_str(path).expect("valid path")
}

fn scan(block: syn::Block) -> MutationVisitor {
    let mut visitor = MutationVisitor::new();
    visitor.scan_fn_body(&block);
    visitor
}

fn scan_boundary(block: syn::Block) -> BoundaryVisitor {
    let mut visitor = BoundaryVisitor::new();
    visitor.scan_fn_body(&block);
    visitor
}

fn scan_boundary_with_module(
    item_fn: syn::ItemFn,
    module_items: Vec<syn::Item>,
) -> BoundaryVisitor {
    let mut visitor = BoundaryVisitor::new();
    visitor.scan_fn_body_with_module_items(&item_fn, &module_items);
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

#[test]
fn boundary_visitor_detects_crate_db_import() {
    let visitor = scan_boundary(syn::parse_quote!({
        use crate::db::ActionDb;
        let _ = std::any::type_name::<ActionDb>();
    }));

    assert_eq!(visitor.detected, ["crate::db"]);
}

#[test]
fn boundary_visitor_detects_std_fs_write() {
    let visitor = scan_boundary(syn::parse_quote!({
        std::fs::write(path, bytes)?;
    }));

    assert_eq!(visitor.detected, ["std::fs::write"]);
}

#[test]
fn boundary_visitor_detects_file_create_alias() {
    let visitor = scan_boundary(syn::parse_quote!({
        use std::fs::File;
        let _file = File::create(path)?;
    }));

    assert_eq!(visitor.detected, ["File::create", "std::fs::File::create"]);
}

#[test]
fn boundary_visitor_detects_tokio_fs_import() {
    let visitor = scan_boundary(syn::parse_quote!({
        use tokio::fs;
        fs::write(path, bytes).await?;
    }));

    assert_eq!(visitor.detected, ["tokio::fs"]);
}

#[test]
fn boundary_visitor_detects_open_options_write_handle() {
    let visitor = scan_boundary(syn::parse_quote!({
        use std::fs::OpenOptions;
        let _file = OpenOptions::new().create(true).write(true).open(path)?;
    }));

    assert_eq!(visitor.detected, ["std::fs::OpenOptions::open(write)"]);
}

#[test]
fn boundary_visitor_detects_same_module_helper_indirection() {
    let ability_fn: syn::ItemFn = syn::parse_quote! {
        async fn fixture_ability() {
            write_behind_helper();
        }
    };
    let module_items: Vec<syn::Item> = vec![
        syn::parse_quote! {
            fn write_behind_helper() {
                std::fs::write("target/ability-runtime-boundary-proof", b"forbidden").unwrap();
            }
        },
        syn::parse_quote! {
            async fn fixture_ability() {
                write_behind_helper();
            }
        },
    ];

    let visitor = scan_boundary_with_module(ability_fn, module_items);

    assert_eq!(visitor.detected, ["std::fs::write"]);
}
