use std::process::ExitCode;

use dailyos_lib::services::entity_linking::repair::{
    apply_repair, build_report, RepairOptions,
};

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let apply = args.iter().any(|arg| arg == "--apply");
    let opts = match parse_options(&args) {
        Ok(opts) => opts,
        Err(e) => {
            eprintln!("{e}");
            print_usage();
            return ExitCode::from(2);
        }
    };

    let db = match dailyos_lib::db::ActionDb::open() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("failed to open database: {e}");
            return ExitCode::from(2);
        }
    };

    let result = if apply {
        apply_repair(&db, &opts)
    } else {
        build_report(&db, &opts)
    };

    let report = match result {
        Ok(report) => report,
        Err(e) => {
            eprintln!("repair failed: {e}");
            return ExitCode::from(2);
        }
    };

    println!("{}", report.to_operator_summary());
    if !apply && report.has_changes() {
        println!("mode=dry-run");
        println!("next=run with --apply to write the repair ledger and quarantine these rows");
        return ExitCode::from(1);
    }

    ExitCode::from(0)
}

fn parse_options(args: &[String]) -> Result<RepairOptions, String> {
    let mut opts = RepairOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--apply" => {
                i += 1;
            }
            "--dry-run" => {
                i += 1;
            }
            "--min-batch" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--min-batch requires a value".to_string());
                };
                opts.min_batch_size = value
                    .parse()
                    .map_err(|_| "--min-batch must be an integer".to_string())?;
                i += 1;
            }
            "--min-coattendees" => {
                i += 1;
                let Some(value) = args.get(i) else {
                    return Err("--min-coattendees requires a value".to_string());
                };
                opts.min_coattendees = value
                    .parse()
                    .map_err(|_| "--min-coattendees must be an integer".to_string())?;
                i += 1;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    opts.validate()?;
    Ok(opts)
}

fn print_usage() {
    eprintln!(
        "usage: repair_entity_linking [--dry-run] [--apply] [--min-batch N] [--min-coattendees N]"
    );
}
