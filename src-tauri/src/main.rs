// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(all(
    feature = "bench-harness",
    not(debug_assertions),
    not(dailyos_suite_p_bench_build)
))]
compile_error!(
    "the bench-harness feature exposes benchmark-only helpers and must not be enabled for the release app binary"
);

fn main() {
    if let Some(code) = dailyos_lib::doctor::run_from_args(std::env::args()) {
        std::process::exit(code);
    }
    dailyos_lib::run()
}
