use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[path = "src/observability/aggregate_metric/lint.rs"]
mod aggregate_metric_lint;

const MCP_GUARD_SOURCE_PATHS: &[&str] = &[
    "build.rs",
    "Cargo.lock",
    "Cargo.toml",
    "scripts/build-mcp.sh",
    "src/db/core.rs",
    "src/mcp/main.rs",
    "src/mcp/launcher.rs",
    "src/mcp_launcher_contract.rs",
    "src/mcp_runtime_guard_constants.rs",
    "src/services/integrations.rs",
];

fn main() {
    emit_suite_p_bench_cfg();
    emit_build_git_sha();
    emit_build_target_triple();
    emit_apple_team_id();
    validate_operations_contract();
    validate_aggregate_metric_contract();
    tauri_build::build()
}

fn emit_suite_p_bench_cfg() {
    println!("cargo:rerun-if-env-changed=DAILYOS_SUITE_P_BENCH_BUILD");
    println!("cargo:rustc-check-cfg=cfg(dailyos_suite_p_bench_build)");
    if env::var_os("DAILYOS_SUITE_P_BENCH_BUILD").is_some() {
        println!("cargo:rustc-cfg=dailyos_suite_p_bench_build");
    }
}

fn emit_build_git_sha() {
    println!("cargo:rerun-if-env-changed=DAILYOS_BUILD_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_RELEASE_GATE");

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    watch_git_head(&manifest_dir);
    watch_mcp_guard_sources(&manifest_dir);

    let dailyos_build_sha = env_sha("DAILYOS_BUILD_SHA");
    let github_sha = env_sha("GITHUB_SHA");
    let git_rev_parse_head = git_sha(&manifest_dir);
    let release_gate_enabled = std::env::var("CARGO_FEATURE_RELEASE_GATE").is_ok();

    let sha = match (dailyos_build_sha, github_sha, git_rev_parse_head) {
        (Some(value), _, _) => value,
        (None, Some(value), _) => value,
        (None, None, Some(value)) => build_git_id(&manifest_dir, &value),
        (None, None, None) if release_gate_enabled => {
            panic!(
                "BUILD_GIT_SHA cannot be determined. Set DAILYOS_BUILD_SHA, GITHUB_SHA, or run inside a git checkout. For source-only local builds, set DAILYOS_BUILD_SHA=dev-unknown."
            );
        }
        (None, None, None) => "unknown".to_string(),
    };
    println!("cargo:rustc-env=BUILD_GIT_SHA={sha}");
}

fn emit_build_target_triple() {
    println!("cargo:rerun-if-env-changed=TARGET");
    let target = env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_TARGET_TRIPLE={target}");
}

fn emit_apple_team_id() {
    println!("cargo:rerun-if-env-changed=DAILYOS_APPLE_TEAM_ID");
    println!("cargo:rerun-if-env-changed=APPLE_SIGNING_IDENTITY");
    if let Some(team_id) = env::var("DAILYOS_APPLE_TEAM_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            env::var("APPLE_SIGNING_IDENTITY")
                .ok()
                .and_then(|identity| parse_apple_team_id(&identity))
        })
    {
        println!("cargo:rustc-env=DAILYOS_APPLE_TEAM_ID={team_id}");
    }
}

fn parse_apple_team_id(identity: &str) -> Option<String> {
    let open = identity.rfind('(')?;
    let close = identity[open + 1..].find(')')? + open + 1;
    let team_id = identity[open + 1..close].trim();
    if team_id.is_empty() {
        None
    } else {
        Some(team_id.to_string())
    }
}

fn env_sha(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn watch_mcp_guard_sources(manifest_dir: &Path) {
    for relative_path in MCP_GUARD_SOURCE_PATHS {
        println!(
            "cargo:rerun-if-changed={}",
            manifest_dir.join(relative_path).display()
        );
    }
}

fn git_sha(manifest_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &manifest_dir.display().to_string(),
            "rev-parse",
            "HEAD",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn build_git_id(manifest_dir: &Path, head: &str) -> String {
    match dirty_mcp_guard_digest(manifest_dir) {
        Some(digest) => format!("{head}+dirty.{}", &digest[..12.min(digest.len())]),
        None => head.to_string(),
    }
}

fn dirty_mcp_guard_digest(manifest_dir: &Path) -> Option<String> {
    let diff_output = Command::new("git")
        .arg("-C")
        .arg(manifest_dir)
        .arg("diff")
        .arg("--binary")
        .arg("HEAD")
        .arg("--")
        .args(MCP_GUARD_SOURCE_PATHS)
        .output()
        .ok()?;
    if !diff_output.status.success() {
        return None;
    }

    let mut digest_input = diff_output.stdout;
    append_untracked_mcp_guard_sources(manifest_dir, &mut digest_input);
    if digest_input.is_empty() {
        return None;
    }

    let mut hash = Command::new("git")
        .arg("-C")
        .arg(manifest_dir)
        .arg("hash-object")
        .arg("--stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    hash.stdin.take()?.write_all(&digest_input).ok()?;
    let output = hash.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn append_untracked_mcp_guard_sources(manifest_dir: &Path, digest_input: &mut Vec<u8>) {
    let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(manifest_dir)
        .arg("ls-files")
        .arg("--others")
        .arg("--exclude-standard")
        .arg("-z")
        .arg("--")
        .args(MCP_GUARD_SOURCE_PATHS)
        .output()
    else {
        return;
    };
    if !output.status.success() || output.stdout.is_empty() {
        return;
    }

    let mut paths: Vec<String> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).to_string())
        .collect();
    paths.sort();

    for relative_path in paths {
        let full_path = manifest_dir.join(&relative_path);
        if !full_path.is_file() {
            continue;
        }
        let Ok(contents) = fs::read(&full_path) else {
            continue;
        };
        digest_input.extend_from_slice(b"\n-- DAILYOS UNTRACKED MCP GUARD SOURCE --\n");
        digest_input.extend_from_slice(relative_path.as_bytes());
        digest_input.push(b'\n');
        digest_input.extend_from_slice(&contents);
        digest_input.push(b'\n');
    }
}

fn watch_git_head(manifest_dir: &Path) {
    let Some(git_dir) = git_dir(manifest_dir) else {
        return;
    };
    let Some(git_common_dir) = git_common_dir(manifest_dir) else {
        watch_standard_git_head(&git_dir);
        return;
    };

    if git_dir != git_common_dir {
        watch_linked_worktree_git_head(manifest_dir, &git_dir, &git_common_dir);
        return;
    }

    watch_standard_git_head(&git_dir);
}

fn watch_standard_git_head(git_dir: &Path) {
    let head_path = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head_path.display());

    let Ok(head) = fs::read_to_string(&head_path) else {
        return;
    };
    let Some(reference) = head
        .trim()
        .strip_prefix("ref:")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join(reference).display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        git_dir.join("packed-refs").display()
    );
}

fn watch_linked_worktree_git_head(manifest_dir: &Path, git_dir: &Path, git_common_dir: &Path) {
    // Manual repro for linked-worktree SHA watching:
    //   git init /tmp/dailyos-sha-watch && cd /tmp/dailyos-sha-watch
    //   # add the DailyOS sources, then create an initial commit
    //   git add . && git commit -m "initial"
    //   git worktree add /tmp/dailyos-sha-watch-linked
    //   cd /tmp/dailyos-sha-watch-linked
    //   cargo build --features release-gate -p dailyos
    //   git commit --allow-empty -m "second"
    //   cargo build --features release-gate -p dailyos
    // The second build must rerun build.rs so BUILD_GIT_SHA tracks HEAD.
    println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
    println!(
        "cargo:rerun-if-changed={}",
        git_common_dir.join("packed-refs").display()
    );

    let Some(reference) = symbolic_head_reference(manifest_dir)
        .filter(|reference| reference.starts_with("refs/heads/"))
    else {
        return;
    };
    println!(
        "cargo:rerun-if-changed={}",
        git_common_dir.join(reference).display()
    );
}

fn git_dir(manifest_dir: &Path) -> Option<PathBuf> {
    git_rev_parse_path(manifest_dir, "--git-dir")
}

fn git_common_dir(manifest_dir: &Path) -> Option<PathBuf> {
    git_rev_parse_path(manifest_dir, "--git-common-dir")
}

fn git_rev_parse_path(manifest_dir: &Path, flag: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["-C", &manifest_dir.display().to_string(), "rev-parse", flag])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    Some(if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    })
}

fn symbolic_head_reference(manifest_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .args([
            "-C",
            &manifest_dir.display().to_string(),
            "symbolic-ref",
            "HEAD",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validate_operations_contract() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    let operations_mod = manifest_dir.join("src/operations/mod.rs");
    let lib_rs = manifest_dir.join("src/lib.rs");

    println!("cargo:rerun-if-changed={}", operations_mod.display());
    println!("cargo:rerun-if-changed={}", lib_rs.display());

    let Ok(source) = fs::read_to_string(&operations_mod) else {
        return;
    };
    let lib_source = fs::read_to_string(&lib_rs).unwrap_or_default();

    let blocks = operation_def_blocks(&source);
    if blocks.is_empty() {
        panic!("operations contract must declare at least one operation_def! entry");
    }

    let schema_base = operations_mod
        .parent()
        .expect("src/operations/mod.rs has a parent");
    let mut names = Vec::new();
    for block in blocks {
        if !block.contains("remote:") {
            panic!("operation_def! entries must declare the explicit `remote` field");
        }

        let name = field_string_literal(&block, "name")
            .unwrap_or_else(|| panic!("operation_def! entry is missing string `name`"));
        if !is_kebab_case(&name) {
            panic!("operation `{name}` must use kebab-case");
        }
        names.push(name);

        let category = field_ident(&block, "category")
            .unwrap_or_else(|| panic!("operation_def! entry is missing `category`"));
        let executor = field_path(&block, "executor")
            .unwrap_or_else(|| panic!("operation_def! entry is missing `executor`"));
        let executor_name = executor.rsplit("::").next().unwrap_or(&executor);
        let expected_prefix = format!("{}_", category.to_ascii_lowercase());
        if !executor_name.starts_with(&expected_prefix) {
            panic!(
                "operation category `{category}` must use an executor whose name starts with `{expected_prefix}`"
            );
        }

        for field in ["input_schema", "output_schema"] {
            let schema = include_str_path(&block, field).unwrap_or_else(|| {
                panic!(
                    "operation `{}` is missing include_str! for `{field}`",
                    names.last().unwrap()
                )
            });
            let schema_path = schema_base.join(&schema);
            println!("cargo:rerun-if-changed={}", schema_path.display());
            if !schema_path.is_file() {
                panic!(
                    "operation `{}` references missing schema file `{}`",
                    names.last().unwrap(),
                    schema_path.display()
                );
            }
        }
    }

    if !operation_command_is_generic_only(&source) {
        panic!("operations module must expose exactly one Tauri command: invoke_operation");
    }
    if !lib_source.contains("operations::invoke_operation") {
        panic!("Tauri generate_handler! must expose operations::invoke_operation");
    }

    for name in names {
        let snake = name.replace('-', "_");
        for disallowed in [format!("commands::{snake}"), format!("operations::{snake}")] {
            if generate_handler_contains(&lib_source, &disallowed) && snake != "invoke_operation" {
                panic!(
                    "operation `{name}` must be exposed through operations::invoke_operation, not `{disallowed}`"
                );
            }
        }
    }
}

fn validate_aggregate_metric_contract() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    aggregate_metric_lint::validate_aggregate_metric_contract(&manifest_dir);
}

fn operation_def_blocks(source: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut offset = 0;
    let marker = "operation_def!";

    while let Some(relative_start) = source[offset..].find(marker) {
        let start = offset + relative_start;
        let Some(open_relative) = source[start..].find('{') else {
            break;
        };
        let open = start + open_relative;
        let mut depth = 0usize;
        let mut end = None;
        for (relative_index, ch) in source[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = Some(open + relative_index + ch.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(block_end) = end else {
            break;
        };
        blocks.push(source[open + 1..block_end - 1].to_string());
        offset = block_end;
    }

    blocks
}

fn field_string_literal(block: &str, field: &str) -> Option<String> {
    let value = field_value(block, field)?;
    let value = value.trim();
    let rest = value.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn field_ident(block: &str, field: &str) -> Option<String> {
    let value = field_value(block, field)?;
    Some(
        value
            .trim()
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect(),
    )
    .filter(|value: &String| !value.is_empty())
}

fn field_path(block: &str, field: &str) -> Option<String> {
    let value = field_value(block, field)?;
    Some(
        value
            .trim()
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == ':')
            .collect(),
    )
    .filter(|value: &String| !value.is_empty())
}

fn include_str_path(block: &str, field: &str) -> Option<String> {
    let value = field_value(block, field)?;
    let include = value.find("include_str!")?;
    let rest = &value[include..];
    let first_quote = rest.find('"')?;
    let after_quote = &rest[first_quote + 1..];
    let second_quote = after_quote.find('"')?;
    Some(after_quote[..second_quote].to_string())
}

fn field_value<'a>(block: &'a str, field: &str) -> Option<&'a str> {
    let marker = format!("{field}:");
    let start = block.find(&marker)? + marker.len();
    let rest = &block[start..];
    let end = rest.find('\n').unwrap_or(rest.len());
    Some(&rest[..end])
}

fn is_kebab_case(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
        && value.contains('-')
}

fn operation_command_is_generic_only(source: &str) -> bool {
    source.matches("#[tauri::command]").count() == 1
        && source.contains("pub async fn invoke_operation")
}

fn generate_handler_contains(source: &str, handler: &str) -> bool {
    source
        .lines()
        .map(str::trim)
        .any(|line| line == format!("{handler},") || line == handler)
}
