use abilities_runtime::abilities::workspace_graph::contracts::{
    WorkspaceGraphInput, WorkspaceGraphPrivacyProfile, WorkspaceGraphReadRequest,
    WorkspaceGraphResponse,
};
use clap::{ArgAction, Parser};

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "json", value_parser = ["json", "human"])]
    format: String,
    #[arg(long, default_value_t = true, action = ArgAction::Set)]
    fail_on_gaps: bool,
}

fn main() {
    let args = Args::parse();
    match run(args) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(3);
        }
    }
}

fn run(args: Args) -> Result<i32, String> {
    let db =
        dailyos_lib::db::ActionDb::open(std::sync::Arc::new(dailyos_lib::db::LocalKeychain::new()))
            .map_err(|error| format!("read_unavailable: {error}"))?;
    let diagnostic_key =
        dailyos_lib::services::workspace_ingestion::graph::local_install_diagnostic_key()
            .map_err(|error| format!("read_unavailable: {error}"))?;
    let response = dailyos_lib::services::workspace_ingestion::graph::read_workspace_graph(
        db.conn_ref(),
        WorkspaceGraphReadRequest {
            input: WorkspaceGraphInput {
                schema_version: 1,
                entity_filter: None,
                category_filter: None,
                cursor: None,
                if_none_match: None,
                include_entity_names: false,
                page_size: 200,
            },
            privacy_profile: WorkspaceGraphPrivacyProfile::FirstParty,
        },
        &diagnostic_key,
    )
    .map_err(|error| format!("read_unavailable: {error}"))?;

    let audit = match response {
        WorkspaceGraphResponse::Projection(projection) => projection.audit,
        WorkspaceGraphResponse::NotModified(_) => {
            return Err(
                "read_unavailable: audit query unexpectedly returned not_modified".to_string(),
            )
        }
    };
    let gap_count: u32 = audit.gap_counts.values().sum();

    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&audit)
                .map_err(|error| format!("audit_failed: {error}"))?;
            println!("{json}");
        }
        "human" => {
            let mut reason_counts = std::collections::BTreeMap::<&str, u32>::new();
            for gap in &audit.gaps {
                *reason_counts.entry(gap.reason.as_str()).or_default() += 1;
            }
            println!("workspace graph audit");
            println!("graph_version: {}", audit.graph_version);
            println!("gap_count: {gap_count}");
            for (category, count) in &audit.gap_counts {
                println!("{category}: {count}");
            }
            for (reason, count) in reason_counts {
                println!("reason.{reason}: {count}");
            }
        }
        _ => return Ok(2),
    }

    Ok(audit_exit_code(gap_count, args.fail_on_gaps))
}

fn audit_exit_code(gap_count: u32, fail_on_gaps: bool) -> i32 {
    if gap_count > 0 && fail_on_gaps {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fail_on_gaps_false() {
        let args = Args::parse_from(["workspace_graph_audit", "--fail-on-gaps=false"]);

        assert!(!args.fail_on_gaps);
    }

    #[test]
    fn exit_code_matches_gap_policy() {
        assert_eq!(audit_exit_code(0, true), 0);
        assert_eq!(audit_exit_code(0, false), 0);
        assert_eq!(audit_exit_code(2, false), 0);
        assert_eq!(audit_exit_code(2, true), 1);
    }
}
