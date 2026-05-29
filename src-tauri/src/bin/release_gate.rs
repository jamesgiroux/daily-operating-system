use std::process::ExitCode;

fn main() -> ExitCode {
    dailyos_lib::db::resolve_and_set_db_mode_from_process();

    match dailyos_lib::release_gate::run_from_args(std::env::args_os()) {
        Ok(outcome) => {
            println!("{}", outcome.summary_markdown.trim_end());
            ExitCode::from(outcome.exit_code)
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
