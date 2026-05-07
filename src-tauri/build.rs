use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    emit_build_git_sha();
    tauri_build::build()
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
