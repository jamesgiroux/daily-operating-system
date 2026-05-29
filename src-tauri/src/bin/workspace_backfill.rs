use clap::{ArgAction, Parser};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, action = ArgAction::SetTrue)]
    live: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    apply: bool,
    #[arg(long)]
    workspace_root: Option<std::path::PathBuf>,
    #[arg(long)]
    resume_run_id: Option<String>,
    #[arg(long, default_value = "json", value_parser = ["json", "human"])]
    format: String,
}

fn main() {
    dailyos_lib::db::resolve_and_set_db_mode_from_process();

    let args = Args::parse();
    let resolved_mode = dailyos_lib::db::db_mode();
    eprintln!("Resolved DbMode: {resolved_mode:?}");
    if resolved_mode != dailyos_lib::db::DbMode::Live || !explicit_live_db_mode_requested(args.live)
    {
        eprintln!(
            "refusing to run maintenance against an implicit or non-Live DB mode; \
             pass --live or set DAILYOS_DB_MODE=live"
        );
        std::process::exit(2);
    }
    match run(args) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(3);
        }
    }
}

fn explicit_live_db_mode_requested(live_arg: bool) -> bool {
    live_arg || std::env::var("DAILYOS_DB_MODE").is_ok_and(|value| value.trim() == "live")
}

fn run(args: Args) -> Result<i32, String> {
    let mode = if args.apply {
        dailyos_lib::services::workspace_backfill::BackfillMode::Apply
    } else {
        dailyos_lib::services::workspace_backfill::BackfillMode::DryRun
    };
    if !args.apply && args.resume_run_id.is_some() {
        return Err("resume_requires_apply".to_string());
    }
    let options = dailyos_lib::services::workspace_backfill::WorkspaceBackfillOptions {
        workspace_root: args.workspace_root,
        mode,
        resume_run_id: args.resume_run_id,
        max_file_bytes:
            dailyos_lib::services::workspace_ingestion::pipeline::DEFAULT_MAX_FILE_BYTES,
    };
    let summary =
        dailyos_lib::services::workspace_backfill::run_workspace_backfill_from_local_db(options)?;

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&summary)
                .map_err(|_| "summary_serialization_failed".to_string())?;
            println!("{json}");
        }
        "human" => print_human(&summary),
        _ => return Ok(2),
    }

    Ok(if summary.failed_count > 0 { 1 } else { 0 })
}

fn print_human(summary: &dailyos_lib::services::workspace_backfill::BackfillSummary) {
    println!("workspace backfill");
    println!("mode: {}", summary.mode);
    if let Some(run_id) = &summary.run_id {
        println!("run_id: {run_id}");
    }
    println!("status: {}", summary.status);
    println!("scanned_count: {}", summary.scanned_count);
    println!("eligible_count: {}", summary.eligible_count);
    println!("applied_count: {}", summary.applied_count);
    println!("skipped_count: {}", summary.skipped_count);
    println!("failed_count: {}", summary.failed_count);
    for (reason, count) in &summary.reason_counts {
        println!("reason.{reason}: {count}");
    }
    for (class, count) in &summary.source_class_counts {
        println!("source_class.{class}: {count}");
    }
    for (class, count) in &summary.divergence_counts {
        println!("divergence.{class}: {count}");
    }
    for group in &summary.duplicate_groups {
        println!(
            "duplicate_group.{}: {}",
            group.duplicate_group_handle, group.source_count
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn default_mode_is_dry_run() {
        let args = Args::parse_from(["workspace_backfill"]);

        assert!(!args.apply);
        assert!(args.resume_run_id.is_none());
    }

    #[test]
    fn apply_must_be_explicit() {
        let args = Args::parse_from(["workspace_backfill", "--apply"]);

        assert!(args.apply);
    }

    #[test]
    fn resume_requires_apply() {
        let args = Args::parse_from(["workspace_backfill", "--resume-run-id", "run-1"]);

        let err = run(args).expect_err("resume without apply should fail before opening db");
        assert_eq!(err, "resume_requires_apply");
    }
}
