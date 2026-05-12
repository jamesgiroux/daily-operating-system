use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};
use dailyos_lib::context_provider::ContextMode;
use dailyos_lib::db::ActionDb;
use dailyos_lib::db_service::DbService;
use dailyos_lib::intelligence::io::IntelligenceJson;
use dailyos_lib::intelligence::write_fence::{fenced_write_intelligence_json, FenceCycle};
use dailyos_lib::services::context::{ExternalClients, FixedClock, SeedableRng, ServiceContext};
use dailyos_lib::state::AppState;
use std::hint::black_box;

fn suite_p_service_context_mutation_gate(c: &mut Criterion) {
    let clock = FixedClock::new(
        chrono::DateTime::parse_from_rfc3339("2026-05-11T00:00:00Z")
            .expect("fixed timestamp")
            .with_timezone(&chrono::Utc),
    );
    let rng = SeedableRng::new(42);
    let external = ExternalClients::default();
    let live_ctx = ServiceContext::new_live(&clock, &rng, &external);
    let evaluate_ctx = ServiceContext::new_evaluate_default(&clock, &rng);
    let mut iteration = 0usize;

    c.bench_function("mutation_gate_mode_check", |b| {
        b.iter(|| {
            iteration = iteration.wrapping_add(1);
            let mut allowed_count = 0usize;
            for offset in 0..64 {
                let ctx = if black_box((iteration + offset) & 1) == 0 {
                    &live_ctx
                } else {
                    &evaluate_ctx
                };
                allowed_count += usize::from(black_box(ctx).check_mutation_allowed().is_ok());
            }
            black_box(allowed_count);
        });
    });
}

fn suite_p_fenced_write_intelligence_json(c: &mut Criterion) {
    let temp_dir = tempfile::tempdir().expect("suite-p temp dir");
    let db = isolated_db(temp_dir.path().join("suite-p-write-fence.db"));
    let entity_dir = temp_dir
        .path()
        .join("workspace")
        .join("accounts")
        .join("acct-test-1");
    let intel = IntelligenceJson {
        entity_id: "acct-test-1".to_string(),
        entity_type: "account".to_string(),
        enriched_at: "2026-05-11T00:00:00Z".to_string(),
        executive_assessment: Some("Synthetic account intelligence benchmark.".to_string()),
        ..IntelligenceJson::default()
    };

    c.bench_function("fenced_write_intelligence_json", |b| {
        b.iter(|| {
            let cycle = FenceCycle::capture(&db).expect("capture schema epoch");
            fenced_write_intelligence_json(
                black_box(&cycle),
                black_box(&db),
                black_box(&entity_dir),
                black_box(&intel),
            )
            .expect("fenced write");
        });
    });
}

fn suite_p_context_provider_swap_snapshot(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let temp_dir = tempfile::tempdir().expect("suite-p app state dir");
    let db_service = runtime
        .block_on(DbService::open_at_unencrypted_for_tests(
            temp_dir.path().join("suite-p-provider.db"),
        ))
        .expect("test db service");
    let state = AppState::test_with_db_service(Arc::clone(&db_service));
    let mode = ContextMode::Local;

    c.bench_function("context_mode_atomic_local_snapshot", |b| {
        b.iter(|| {
            state.set_context_mode_atomic(black_box(&mode));
            let snapshot = state.context_snapshot();
            black_box(snapshot.provider_name());
        });
    });
}

fn isolated_db(path: std::path::PathBuf) -> ActionDb {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create suite-p db parent");
    }
    let conn = rusqlite::Connection::open(path).expect("open suite-p db");
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;",
    )
    .expect("configure suite-p db");
    dailyos_lib::migration_test_api::run_migrations(&conn).expect("run migrations");
    ActionDb::from_connection_for_tests(conn)
}

criterion_group! {
    name = suite_p_baseline;
    config = Criterion::default().sample_size(20);
    targets =
        suite_p_service_context_mutation_gate,
        suite_p_fenced_write_intelligence_json,
        suite_p_context_provider_swap_snapshot
}
criterion_main!(suite_p_baseline);
