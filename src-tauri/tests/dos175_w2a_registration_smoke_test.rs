//! W2-A Phase-A registration smoke test per DOS-175 AC-6.
//!
//! Asserts the in-process wiring works: registration helper accepts the
//! embedded catalog, the handler registers under the canonical scoped name,
//! `seal()` succeeds, and the list of unregistered slots matches expectations
//! (9 remaining after account_status registers).
//!
//! The full end-to-end stdio JSON-RPC subprocess test is deferred to L4 manual
//! smoke (AC-9) where the real demo target lives.

use std::sync::Arc;

use dailyos_lib::db::ActionDb;
use dailyos_lib::services::mcp_v2::contracts::ScopedName;
use dailyos_lib::services::mcp_v2::gateway::Gateway;
use dailyos_lib::services::mcp_v2::handlers::registration::register_v147_handlers;
use dailyos_lib::services::mcp_v2::taxonomy::{TaxonomyCatalog, YamlTaxonomyCatalog};
use parking_lot::Mutex as ParkingMutex;
use rusqlite::Connection;

/// Per DOS-175 AC-3 + AC-5 + AC-6: the registration helper exposes
/// `dailyos.read.account_status` on the gateway against the embedded
/// catalog, `seal()` succeeds, and `validate_catalog_against_handlers`
/// returns the 9 unregistered slots as a `Vec` (not an `Err`) per
/// W1-B AC-7 split.
#[test]
fn account_status_handler_registers_against_embedded_catalog() {
    // Build a tokio runtime to source a Handle for the handler.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");

    // Load the embedded catalog (same path mcp_v2/main.rs::run_serve uses).
    let catalog = YamlTaxonomyCatalog::load_embedded().expect("load embedded catalog");
    let catalog: Arc<dyn TaxonomyCatalog> = Arc::new(catalog);

    // In-memory ActionDb for the workspace readers. Real run_serve uses
    // ActionDb::open_readonly against the encrypted DB; smoke test only
    // exercises registration, not invocation, so in-memory is sufficient.
    let conn = Connection::open_in_memory().expect("open in-memory connection");
    let action_db = ActionDb::from_connection_for_tests(conn);
    let action_db = Arc::new(ParkingMutex::new(action_db));

    // Build the gateway and register the W2-A handler.
    let mut gateway = Gateway::new();
    gateway.set_taxonomy(catalog.clone());
    register_v147_handlers(&mut gateway, &catalog, action_db, runtime.handle().clone())
        .expect("register_v147_handlers succeeds");

    // AC-3 + AC-5: the handler is registered under the canonical scoped name.
    let registered: Vec<_> = gateway.registered_tools().cloned().collect();
    assert_eq!(registered.len(), 1, "expected exactly one registered tool");
    assert_eq!(
        registered[0],
        ScopedName::new("dailyos.read.account_status"),
        "expected dailyos.read.account_status registered"
    );

    // AC-6: seal succeeds. The pending list is the 9 catalog entries
    // without registered handlers per W1-B AC-7 split (Vec, not Err).
    let pending = gateway.seal().expect("seal succeeds with 1 handler");
    assert_eq!(
        pending.len(),
        9,
        "expected 9 unregistered catalog slots, got {pending:?}"
    );

    drop(gateway);
}
