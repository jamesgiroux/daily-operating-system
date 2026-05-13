use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    emit_suite_p_bench_cfg();
    emit_build_git_sha();
    validate_operations_contract();
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

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"));
    watch_git_head(&manifest_dir);

    let sha = env_sha("DAILYOS_BUILD_SHA")
        .or_else(|| env_sha("GITHUB_SHA"))
        .or_else(|| git_sha(&manifest_dir))
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_GIT_SHA={sha}");
}

fn env_sha(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

fn watch_git_head(manifest_dir: &Path) {
    let Some(git_dir) = git_dir(manifest_dir) else {
        return;
    };
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

fn git_dir(manifest_dir: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .args([
            "-C",
            &manifest_dir.display().to_string(),
            "rev-parse",
            "--git-dir",
        ])
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
