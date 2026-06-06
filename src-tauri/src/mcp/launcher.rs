//! Guarded DailyOS MCP launcher.
//!
//! This binary intentionally does not import `dailyos_lib` or any DB module.
//! It verifies build provenance and the MCP runtime self-check before replacing
//! itself with `dailyos-mcp`.

#[path = "../mcp_launcher_contract.rs"]
#[allow(dead_code)]
mod mcp_launcher_contract;
#[path = "../mcp_runtime_guard_constants.rs"]
#[allow(dead_code)]
mod mcp_runtime_guard_constants;

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

use mcp_launcher_contract::{
    McpBundleProvenance, McpLauncherManifest, McpRuntimeSourceKind, McpSidecarProvenance,
};
use mcp_runtime_guard_constants::{
    DAILYOS_APP_BUNDLE_IDENTIFIER, MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION,
    MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION, MCP_LAUNCHER_NAME, MCP_NO_ENV_DEFAULT_DB_MODE,
    MCP_RUNTIME_GUARD_EPOCH, MCP_SERVER_NAME,
};
use sha2::{Digest, Sha256};

const SELF_CHECK_TIMEOUT: Duration = Duration::from_secs(3);
#[cfg(all(target_os = "macos", not(test)))]
const APP_BUNDLE_SIGNATURE_TIMEOUT: Duration = Duration::from_secs(5);
const SELF_CHECK_OUTPUT_LIMIT: u64 = 64 * 1024;
const ENV_DB_MODE: &str = "DAILYOS_DB_MODE";

fn main() {
    if let Err(error) = run() {
        eprintln!("dailyos-mcp-launcher refused to start: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = LauncherArgs::parse(std::env::args_os().skip(1))?;
    let manifest = load_and_verify_manifest(&args.manifest_path)?;
    run_self_check(&manifest, SELF_CHECK_TIMEOUT)?;

    if args.check_only {
        println!(
            "{}",
            serde_json::json!({
                "status": "ok",
                "guardEpoch": manifest.guard_epoch,
                "sidecarPath": manifest.sidecar_path,
                "finalServerDbMode": manifest.final_server_db_mode
            })
        );
        return Ok(());
    }

    exec_sidecar(manifest)
}

#[derive(Debug)]
struct LauncherArgs {
    manifest_path: PathBuf,
    check_only: bool,
}

impl LauncherArgs {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = OsString>,
    {
        let mut args = args.into_iter();
        let mut manifest_path = None;
        let mut check_only = false;

        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--manifest" => {
                    if manifest_path.is_some() {
                        return Err("--manifest may only be supplied once".to_string());
                    }
                    let path = args
                        .next()
                        .ok_or_else(|| "--manifest requires a path".to_string())?;
                    manifest_path = Some(PathBuf::from(path));
                }
                "--check" => {
                    if check_only {
                        return Err("--check may only be supplied once".to_string());
                    }
                    check_only = true;
                }
                "--help" | "-h" => {
                    return Err(
                        "usage: dailyos-mcp-launcher --manifest <path> [--check]".to_string()
                    )
                }
                other => return Err(format!("unsupported argument: {other}")),
            }
        }

        Ok(Self {
            manifest_path: manifest_path.ok_or_else(|| "--manifest is required".to_string())?,
            check_only,
        })
    }
}

fn load_and_verify_manifest(path: &Path) -> Result<McpLauncherManifest, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|error| format!("manifest_read_failed: {error}"))?;
    let manifest: McpLauncherManifest =
        serde_json::from_str(&raw).map_err(|error| format!("manifest_json_invalid: {error}"))?;
    verify_manifest(&manifest)?;
    Ok(manifest)
}

fn verify_manifest(manifest: &McpLauncherManifest) -> Result<(), String> {
    if manifest.schema_version != MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION {
        return Err("manifest_schema_unsupported".to_string());
    }
    if manifest.guard_epoch != MCP_RUNTIME_GUARD_EPOCH {
        return Err("manifest_guard_epoch_mismatch".to_string());
    }
    if manifest.final_server_db_mode != "live" && manifest.final_server_db_mode != "replica" {
        return Err("manifest_db_mode_invalid".to_string());
    }
    if manifest.final_server_db_mode != expected_db_mode_for_source_kind(&manifest.source_kind) {
        return Err("manifest_db_mode_source_kind_mismatch".to_string());
    }
    if manifest.generated_at.trim().is_empty() {
        return Err("manifest_generated_at_missing".to_string());
    }
    verify_bundle_provenance(manifest)?;

    verify_launcher_path(manifest)?;
    verify_sidecar_path(manifest)?;
    if manifest.source_kind == McpRuntimeSourceKind::AppBundle {
        verify_app_bundle_runtime(manifest)?;
    }
    let expected_launcher_sha256 = expected_launcher_sha256(manifest)?;
    verify_file_hash(
        &manifest.launcher_path,
        &expected_launcher_sha256,
        "launcher",
    )?;
    verify_file_hash(
        &manifest.sidecar_path,
        &manifest.expected_sidecar_sha256,
        "sidecar",
    )?;
    Ok(())
}

fn expected_db_mode_for_source_kind(source_kind: &McpRuntimeSourceKind) -> &'static str {
    match source_kind {
        McpRuntimeSourceKind::AppBundle => "live",
        McpRuntimeSourceKind::RepoBinaries => MCP_NO_ENV_DEFAULT_DB_MODE,
    }
}

fn expected_launcher_sha256(manifest: &McpLauncherManifest) -> Result<String, String> {
    match manifest.source_kind {
        McpRuntimeSourceKind::RepoBinaries => Ok(manifest.expected_launcher_sha256.clone()),
        McpRuntimeSourceKind::AppBundle => {
            let launcher_source = canonicalize_existing(
                &app_bundle_launcher_source_path(manifest)?,
                "app_bundle_launcher_source",
            )?;
            verified_file_hash(&launcher_source, "app_bundle_launcher_source")
        }
    }
}

fn verify_bundle_provenance(manifest: &McpLauncherManifest) -> Result<(), String> {
    let provenance_path = canonicalize_existing(&manifest.bundle_provenance_path, "provenance")?;
    verify_provenance_path(&manifest.bundle_provenance_path, manifest)?;
    let raw = std::fs::read_to_string(&provenance_path)
        .map_err(|error| format!("provenance_read_failed: {error}"))?;
    let provenance: McpBundleProvenance =
        serde_json::from_str(&raw).map_err(|error| format!("provenance_json_invalid: {error}"))?;

    if provenance.schema_version != MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION {
        return Err("provenance_schema_unsupported".to_string());
    }
    if provenance.guard_epoch != MCP_RUNTIME_GUARD_EPOCH {
        return Err("provenance_guard_epoch_mismatch".to_string());
    }
    if provenance.stub {
        return Err("provenance_is_stub".to_string());
    }
    if provenance.app_build_sha != env!("BUILD_GIT_SHA") {
        return Err("provenance_app_build_sha_mismatch".to_string());
    }
    if manifest.app_build_sha != provenance.app_build_sha {
        return Err("manifest_app_build_sha_mismatch".to_string());
    }
    if provenance.target_triple.trim().is_empty() {
        return Err("provenance_target_triple_missing".to_string());
    }
    if provenance.target_triple != env!("BUILD_TARGET_TRIPLE") {
        return Err("provenance_target_triple_mismatch".to_string());
    }

    let launcher = provenance
        .sidecar(MCP_LAUNCHER_NAME)
        .ok_or_else(|| "provenance_missing_launcher".to_string())?;
    let sidecar = provenance
        .sidecar(MCP_SERVER_NAME)
        .ok_or_else(|| "provenance_missing_mcp".to_string())?;
    verify_sidecar_provenance(launcher)?;
    verify_sidecar_provenance(sidecar)?;

    if manifest.launcher_build_sha != launcher.build_sha {
        return Err("manifest_launcher_build_sha_mismatch".to_string());
    }
    if manifest.sidecar_build_sha != sidecar.build_sha {
        return Err("manifest_sidecar_build_sha_mismatch".to_string());
    }
    if manifest.source_kind == McpRuntimeSourceKind::RepoBinaries {
        if manifest.expected_launcher_sha256 != launcher.sha256 {
            return Err("manifest_launcher_sha256_mismatch".to_string());
        }
        if manifest.expected_sidecar_sha256 != sidecar.sha256 {
            return Err("manifest_sidecar_sha256_mismatch".to_string());
        }
    }
    verify_manifest_sidecar_filename(manifest, sidecar)?;
    Ok(())
}

fn verify_provenance_path(path: &Path, manifest: &McpLauncherManifest) -> Result<(), String> {
    if !cfg!(test) && contains_raw_build_path(path) {
        return Err("provenance_path_raw_build_path".to_string());
    }
    match manifest.source_kind {
        McpRuntimeSourceKind::AppBundle => {
            if !contains_adjacent_components(path, "Contents", "Resources") {
                return Err("provenance_not_in_app_bundle_resources_dir".to_string());
            }
        }
        McpRuntimeSourceKind::RepoBinaries => {
            verify_repo_binaries_provenance_path(path)?;
        }
    }
    Ok(())
}

fn verify_sidecar_provenance(sidecar: &McpSidecarProvenance) -> Result<(), String> {
    if sidecar.stub {
        return Err(format!("{}_provenance_is_stub", sidecar.name));
    }
    if sidecar.build_sha != env!("BUILD_GIT_SHA") {
        return Err(format!("{}_build_sha_mismatch", sidecar.name));
    }
    if sidecar.sha256.len() != 64 || !sidecar.sha256.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("{}_sha256_invalid", sidecar.name));
    }
    Ok(())
}

fn verify_manifest_sidecar_filename(
    manifest: &McpLauncherManifest,
    sidecar: &McpSidecarProvenance,
) -> Result<(), String> {
    let expected_name = match manifest.source_kind {
        McpRuntimeSourceKind::AppBundle => &sidecar.name,
        McpRuntimeSourceKind::RepoBinaries => &sidecar.filename,
    };
    if manifest
        .sidecar_path
        .file_name()
        .and_then(|name| name.to_str())
        != Some(expected_name.as_str())
    {
        return Err("manifest_sidecar_filename_mismatch".to_string());
    }
    Ok(())
}

fn verify_launcher_path(manifest: &McpLauncherManifest) -> Result<(), String> {
    let current =
        std::env::current_exe().map_err(|error| format!("current_exe_failed: {error}"))?;
    let current = canonicalize_existing(&current, "launcher_current")?;
    let expected = canonicalize_existing(&manifest.launcher_path, "launcher_manifest")?;
    if current != expected {
        return Err("launcher_path_mismatch".to_string());
    }
    if !cfg!(test) && contains_raw_build_path(&expected) {
        return Err("launcher_path_raw_build_path".to_string());
    }
    Ok(())
}

fn verify_sidecar_path(manifest: &McpLauncherManifest) -> Result<(), String> {
    let sidecar = canonicalize_existing(&manifest.sidecar_path, "sidecar")?;
    if !cfg!(test) && contains_raw_build_path(&sidecar) {
        return Err("sidecar_path_raw_build_path".to_string());
    }

    match manifest.source_kind {
        McpRuntimeSourceKind::AppBundle => {
            if app_bundle_root_for_macos_runtime_path(&sidecar).is_none() {
                return Err("sidecar_not_in_app_bundle_macos_dir".to_string());
            }
        }
        McpRuntimeSourceKind::RepoBinaries => {
            verify_repo_binaries_sibling_path(manifest, &sidecar, "sidecar")?;
        }
    }
    Ok(())
}

fn verify_repo_binaries_sibling_path(
    manifest: &McpLauncherManifest,
    path: &Path,
    label: &str,
) -> Result<(), String> {
    let launcher = canonicalize_existing(&manifest.launcher_path, "launcher_manifest")?;
    let candidate = canonicalize_existing(path, label)?;
    let launcher_dir = launcher
        .parent()
        .ok_or_else(|| "repo_binaries_launcher_dir_missing".to_string())?;
    let candidate_dir = candidate
        .parent()
        .ok_or_else(|| format!("{label}_dir_missing"))?;
    if candidate_dir != launcher_dir {
        return Err(format!("{label}_not_in_managed_runtime_dir"));
    }
    Ok(())
}

fn verify_repo_binaries_provenance_path(path: &Path) -> Result<(), String> {
    provenance_filename(path)?;
    verify_repo_binaries_provenance_parent(path)
}

#[cfg(test)]
fn verify_repo_binaries_provenance_parent(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(not(test))]
fn verify_repo_binaries_provenance_parent(path: &Path) -> Result<(), String> {
    let parent = canonicalize_existing(
        path.parent()
            .ok_or_else(|| "repo_binaries_provenance_dir_missing".to_string())?,
        "repo_binaries_provenance_dir",
    )?;
    let expected_parent = canonicalize_existing(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("binaries"),
        "repo_binaries_source_binaries_dir",
    )?;
    if parent != expected_parent {
        return Err("repo_binaries_provenance_not_build_binaries_dir".to_string());
    }
    Ok(())
}

fn verify_app_bundle_runtime(manifest: &McpLauncherManifest) -> Result<(), String> {
    let sidecar = canonicalize_existing(&manifest.sidecar_path, "sidecar")?;
    let provenance = canonicalize_existing(&manifest.bundle_provenance_path, "provenance")?;
    let sidecar_root = app_bundle_root_for_macos_runtime_path(&sidecar)
        .ok_or_else(|| "app_bundle_sidecar_root_missing".to_string())?;
    let provenance_root = app_bundle_root_for_resource_path(&provenance)
        .ok_or_else(|| "app_bundle_provenance_root_missing".to_string())?;
    if sidecar_root != provenance_root {
        return Err("app_bundle_root_mismatch".to_string());
    }
    verify_app_bundle_signature(&sidecar_root)
}

fn app_bundle_launcher_source_path(manifest: &McpLauncherManifest) -> Result<PathBuf, String> {
    let sidecar = canonicalize_existing(&manifest.sidecar_path, "sidecar")?;
    let macos_dir = sidecar
        .parent()
        .ok_or_else(|| "app_bundle_macos_dir_missing".to_string())?;
    Ok(macos_dir.join(MCP_LAUNCHER_NAME))
}

fn app_bundle_root_for_macos_runtime_path(path: &Path) -> Option<PathBuf> {
    let macos_dir = path.parent()?;
    if macos_dir.file_name().and_then(|name| name.to_str()) != Some("MacOS") {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return None;
    }
    contents.parent().map(Path::to_path_buf)
}

fn app_bundle_root_for_resource_path(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("Resources") {
            let contents = ancestor.parent()?;
            if contents.file_name().and_then(|name| name.to_str()) == Some("Contents") {
                return contents.parent().map(Path::to_path_buf);
            }
        }
    }
    None
}

#[cfg(all(target_os = "macos", not(test)))]
fn verify_app_bundle_signature(app_bundle: &Path) -> Result<(), String> {
    reject_unsafe_app_bundle_path(app_bundle)?;
    let expected_team_id = expected_apple_team_id()?;
    let status = run_codesign_verify(app_bundle, expected_team_id)?;
    if status.success() {
        verify_app_bundle_signature_identity(app_bundle, expected_team_id)
    } else {
        Err("app_bundle_signature_verify_failed".to_string())
    }
}

#[cfg(all(target_os = "macos", not(test)))]
fn run_codesign_verify(app_bundle: &Path, expected_team_id: &str) -> Result<ExitStatus, String> {
    let trusted_anchor_requirement = app_bundle_trusted_anchor_requirement(expected_team_id)?;
    let mut command = Command::new("/usr/bin/codesign");
    command
        .arg("--verify")
        .arg("--strict")
        .arg("--deep")
        .arg(format!("-R={trusted_anchor_requirement}"))
        .arg(app_bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_own_process_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| "app_bundle_signature_verify_failed".to_string())?;
    wait_process_with_deadline(
        &mut child,
        Instant::now() + APP_BUNDLE_SIGNATURE_TIMEOUT,
        "app_bundle_signature_timeout",
    )
}

#[cfg(all(target_os = "macos", not(test)))]
fn verify_app_bundle_signature_identity(
    app_bundle: &Path,
    expected_team_id: &str,
) -> Result<(), String> {
    let details = codesign_details(app_bundle)?;
    verify_codesign_identity(&details, expected_team_id)
}

#[cfg(all(target_os = "macos", not(test)))]
fn codesign_details(app_bundle: &Path) -> Result<String, String> {
    let mut command = Command::new("/usr/bin/codesign");
    command
        .arg("-dv")
        .arg("--verbose=4")
        .arg(app_bundle)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    configure_own_process_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|_| "app_bundle_signature_details_failed".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "app_bundle_signature_details_failed".to_string())?;
    let status = wait_process_with_deadline(
        &mut child,
        Instant::now() + APP_BUNDLE_SIGNATURE_TIMEOUT,
        "app_bundle_signature_timeout",
    )?;
    if !status.success() {
        return Err("app_bundle_signature_details_failed".to_string());
    }
    let mut details = String::new();
    stderr
        .read_to_string(&mut details)
        .map_err(|_| "app_bundle_signature_details_failed".to_string())?;
    Ok(details)
}

#[cfg(all(target_os = "macos", not(test)))]
fn expected_apple_team_id() -> Result<&'static str, String> {
    let team_id = option_env!("DAILYOS_APPLE_TEAM_ID")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "app_bundle_signature_team_id_missing".to_string())?;
    validate_apple_team_id(team_id)?;
    Ok(team_id)
}

#[cfg(any(all(target_os = "macos", not(test)), test))]
fn app_bundle_trusted_anchor_requirement(expected_team_id: &str) -> Result<String, String> {
    validate_apple_team_id(expected_team_id)?;
    Ok(format!(
        "anchor apple generic and identifier \"{DAILYOS_APP_BUNDLE_IDENTIFIER}\" and certificate leaf[subject.OU] = \"{expected_team_id}\""
    ))
}

#[cfg(any(all(target_os = "macos", not(test)), test))]
fn validate_apple_team_id(team_id: &str) -> Result<(), String> {
    if team_id.len() != 10
        || !team_id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
    {
        return Err("app_bundle_signature_team_id_invalid".to_string());
    }
    Ok(())
}

#[cfg(test)]
fn verify_app_bundle_signature(app_bundle: &Path) -> Result<(), String> {
    reject_unsafe_app_bundle_path(app_bundle)?;
    let details =
        std::fs::read_to_string(app_bundle.join("Contents").join(".dailyos-test-signature"))
            .map_err(|_| "app_bundle_signature_verify_failed".to_string())?;
    verify_codesign_identity(&details, TEST_APPLE_TEAM_ID)
}

#[cfg(test)]
const TEST_APPLE_TEAM_ID: &str = "DAILYOSTEAM";

fn verify_codesign_identity(details: &str, expected_team_id: &str) -> Result<(), String> {
    let identifier = codesign_field_value(
        details,
        "Identifier=",
        "app_bundle_signature_identifier_mismatch",
    )?;
    if identifier != DAILYOS_APP_BUNDLE_IDENTIFIER {
        return Err("app_bundle_signature_identifier_mismatch".to_string());
    }
    let team = codesign_field_value(
        details,
        "TeamIdentifier=",
        "app_bundle_signature_team_id_mismatch",
    )?;
    if team != expected_team_id {
        return Err("app_bundle_signature_team_id_mismatch".to_string());
    }
    Ok(())
}

fn codesign_field_value<'a>(
    details: &'a str,
    prefix: &str,
    missing_error: &str,
) -> Result<&'a str, String> {
    let mut values = details
        .lines()
        .filter_map(|line| line.strip_prefix(prefix).map(str::trim));
    let value = values.next().ok_or_else(|| missing_error.to_string())?;
    if values.next().is_some() {
        return Err("app_bundle_signature_identity_ambiguous".to_string());
    }
    Ok(value)
}

fn reject_unsafe_app_bundle_path(app_bundle: &Path) -> Result<(), String> {
    if path_contains_control_character(app_bundle) {
        return Err("app_bundle_signature_path_unsafe".to_string());
    }
    Ok(())
}

#[cfg(unix)]
fn path_contains_control_character(path: &Path) -> bool {
    path.as_os_str()
        .as_bytes()
        .iter()
        .any(|byte| byte.is_ascii_control())
}

#[cfg(not(unix))]
fn path_contains_control_character(path: &Path) -> bool {
    path.as_os_str()
        .to_string_lossy()
        .chars()
        .any(char::is_control)
}

#[cfg(test)]
fn write_test_signature_marker(app_bundle: &Path, team_id: &str) {
    std::fs::write(
        app_bundle.join("Contents").join(".dailyos-test-signature"),
        format!("Identifier={DAILYOS_APP_BUNDLE_IDENTIFIER}\nTeamIdentifier={team_id}\n"),
    )
    .expect("test signature marker");
}

#[cfg(test)]
fn write_test_wrong_identifier_marker(app_bundle: &Path) {
    std::fs::write(
        app_bundle.join("Contents").join(".dailyos-test-signature"),
        format!("Identifier=com.example.fake\nTeamIdentifier={TEST_APPLE_TEAM_ID}\n"),
    )
    .expect("test signature marker");
}

#[cfg(all(not(target_os = "macos"), not(test)))]
fn verify_app_bundle_signature(_app_bundle: &Path) -> Result<(), String> {
    Err("app_bundle_signature_verification_unavailable".to_string())
}

fn canonicalize_existing(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("{label}_canonicalize_failed: {error}"))
}

fn contains_raw_build_path(path: &Path) -> bool {
    let components: Vec<String> = path
        .components()
        .filter_map(component_name)
        .map(str::to_string)
        .collect();

    components.windows(2).any(|pair| {
        (pair[0] == "target" && (pair[1] == "debug" || pair[1] == "release"))
            || (pair[0] == ".cargo" && pair[1] == "bin")
    })
}

fn contains_adjacent_components(path: &Path, first: &str, second: &str) -> bool {
    let components: Vec<String> = path
        .components()
        .filter_map(component_name)
        .map(str::to_string)
        .collect();
    components
        .windows(2)
        .any(|pair| pair[0] == first && pair[1] == second)
}

fn component_name(component: Component<'_>) -> Option<&str> {
    match component {
        Component::Normal(value) => value.to_str(),
        _ => None,
    }
}

fn provenance_filename(path: &Path) -> Result<&std::ffi::OsStr, String> {
    let file_name = path
        .file_name()
        .ok_or_else(|| "provenance_filename_missing".to_string())?;
    let mut components = Path::new(file_name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(file_name),
        _ => Err("provenance_filename_invalid".to_string()),
    }
}

fn verified_file_hash(path: &Path, label: &str) -> Result<String, String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("{label}_metadata_failed: {error}"))?;
    if !metadata.is_file() {
        return Err(format!("{label}_not_file"));
    }
    if metadata.len() == 0 {
        return Err(format!("{label}_zero_byte"));
    }
    if !is_executable(&metadata) {
        return Err(format!("{label}_not_executable"));
    }

    sha256_file(path).map_err(|error| format!("{label}_hash_failed: {error}"))
}

fn verify_file_hash(path: &Path, expected: &str, label: &str) -> Result<(), String> {
    let actual = verified_file_hash(path, label)?;
    if actual != expected {
        return Err(format!("{label}_hash_mismatch"));
    }
    Ok(())
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn run_self_check(manifest: &McpLauncherManifest, timeout: Duration) -> Result<(), String> {
    let mut command = Command::new(&manifest.sidecar_path);
    command
        .arg("--self-check-json")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = run_bounded(command, timeout, SELF_CHECK_OUTPUT_LIMIT)?;
    if !output.status.success() {
        return Err(format!("self_check_exit_status: {}", output.status));
    }
    if output.stdout_truncated || output.stderr_truncated {
        return Err("self_check_output_too_large".to_string());
    }

    let payload: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("self_check_json_invalid: {error}"))?;
    require_json_str(&payload, "guardEpoch", MCP_RUNTIME_GUARD_EPOCH)?;
    require_json_str(&payload, "buildSha", &manifest.sidecar_build_sha)?;
    require_json_str(&payload, "defaultDbMode", MCP_NO_ENV_DEFAULT_DB_MODE)?;
    require_json_bool(&payload, "runtimeContainsDbModeGuard", true)?;
    require_json_bool(&payload, "dbOpened", false)?;
    Ok(())
}

fn require_json_str(payload: &serde_json::Value, key: &str, expected: &str) -> Result<(), String> {
    match payload.get(key).and_then(serde_json::Value::as_str) {
        Some(actual) if actual == expected => Ok(()),
        _ => Err(format!("self_check_{key}_mismatch")),
    }
}

fn require_json_bool(payload: &serde_json::Value, key: &str, expected: bool) -> Result<(), String> {
    match payload.get(key).and_then(serde_json::Value::as_bool) {
        Some(actual) if actual == expected => Ok(()),
        _ => Err(format!("self_check_{key}_mismatch")),
    }
}

struct BoundedOutput {
    status: ExitStatus,
    stdout: Vec<u8>,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

fn run_bounded(
    mut command: Command,
    timeout: Duration,
    limit: u64,
) -> Result<BoundedOutput, String> {
    configure_own_process_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| format!("self_check_spawn_failed: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "self_check_stdout_unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "self_check_stderr_unavailable".to_string())?;
    let stdout_reader = read_limited(stdout, limit);
    let stderr_reader = read_limited(stderr, limit);

    let deadline = Instant::now() + timeout;
    let status = wait_process_with_deadline(&mut child, deadline, "self_check_timeout")?;
    let (stdout, stdout_truncated) = match recv_reader_until(stdout_reader, "stdout", deadline) {
        Ok(output) => output,
        Err(error) => {
            terminate_child_process_group(&mut child);
            return Err(error);
        }
    };
    let (_stderr, stderr_truncated) = match recv_reader_until(stderr_reader, "stderr", deadline) {
        Ok(output) => output,
        Err(error) => {
            terminate_child_process_group(&mut child);
            return Err(error);
        }
    };

    Ok(BoundedOutput {
        status,
        stdout,
        stdout_truncated,
        stderr_truncated,
    })
}

fn read_limited<R>(reader: R, limit: u64) -> Receiver<io::Result<(Vec<u8>, bool)>>
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut limited = reader.take(limit + 1);
        let mut buffer = Vec::new();
        let result = limited.read_to_end(&mut buffer).map(|_| {
            let truncated = buffer.len() as u64 > limit;
            if truncated {
                buffer.truncate(limit as usize);
            }
            (buffer, truncated)
        });
        if sender.send(result).is_err() {}
    });
    receiver
}

fn wait_process_with_deadline(
    child: &mut Child,
    deadline: Instant,
    timeout_error: &str,
) -> Result<ExitStatus, String> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() >= deadline => {
                terminate_child_process_group(child);
                return Err(timeout_error.to_string());
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => return Err(format!("self_check_wait_failed: {error}")),
        }
    }
}

fn configure_own_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

fn terminate_child_process_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let pid = child.id();
        if pid <= i32::MAX as u32 {
            unsafe {
                let _ = libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
    if let Err(_error) = child.kill() {}
    if let Err(_error) = child.wait() {}
}

fn recv_reader_until(
    reader: Receiver<io::Result<(Vec<u8>, bool)>>,
    label: &str,
    deadline: Instant,
) -> Result<(Vec<u8>, bool), String> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| "self_check_timeout".to_string())?;
    match reader.recv_timeout(remaining) {
        Ok(result) => result.map_err(|error| format!("self_check_{label}_read_failed: {error}")),
        Err(RecvTimeoutError::Timeout) => Err("self_check_timeout".to_string()),
        Err(RecvTimeoutError::Disconnected) => {
            Err(format!("self_check_{label}_reader_disconnected"))
        }
    }
}

fn exec_sidecar(manifest: McpLauncherManifest) -> Result<(), String> {
    let mut command = sidecar_command(&manifest);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        Err(format!("sidecar_exec_failed: {error}"))
    }

    #[cfg(not(unix))]
    {
        let status = command
            .status()
            .map_err(|error| format!("sidecar_spawn_failed: {error}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("sidecar_exit_status: {status}"))
        }
    }
}

fn sidecar_command(manifest: &McpLauncherManifest) -> Command {
    let mut command = Command::new(&manifest.sidecar_path);
    configure_sidecar_env(&mut command, manifest);
    command
}

fn configure_sidecar_env(command: &mut Command, manifest: &McpLauncherManifest) {
    command
        .env_clear()
        .env(ENV_DB_MODE, &manifest.final_server_db_mode);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const SELF_CHECK_TEST_TIMEOUT: Duration = Duration::from_secs(10);

    fn make_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).expect("chmod");
        }
    }

    fn write_script(path: &Path, body: &str) {
        let mut file = File::create(path).expect("create script");
        file.write_all(body.as_bytes()).expect("write script");
        make_executable(path);
    }

    fn current_launcher_path_and_hash() -> (PathBuf, String) {
        let path = std::env::current_exe().expect("current exe");
        let hash = sha256_file(&path).expect("hash current exe");
        (path, hash)
    }

    fn manifest_for(sidecar_path: PathBuf, sidecar_hash: String) -> McpLauncherManifest {
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = sidecar_path.parent().expect("sidecar parent").join(format!(
            "{}.provenance.json",
            sidecar_path
                .file_name()
                .expect("sidecar file name")
                .to_string_lossy()
        ));
        write_test_provenance(
            &provenance_path,
            &sidecar_path,
            &sidecar_hash,
            &launcher_hash,
        );
        McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::RepoBinaries,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "replica".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        }
    }

    fn write_test_provenance(
        path: &Path,
        sidecar_path: &Path,
        sidecar_hash: &str,
        launcher_hash: &str,
    ) {
        let provenance = McpBundleProvenance {
            schema_version: MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            target_triple: env!("BUILD_TARGET_TRIPLE").to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
            stub: false,
            sidecars: vec![
                McpSidecarProvenance {
                    name: MCP_SERVER_NAME.to_string(),
                    filename: sidecar_path
                        .file_name()
                        .expect("sidecar file name")
                        .to_string_lossy()
                        .to_string(),
                    build_sha: env!("BUILD_GIT_SHA").to_string(),
                    sha256: sidecar_hash.to_string(),
                    stub: false,
                },
                McpSidecarProvenance {
                    name: MCP_LAUNCHER_NAME.to_string(),
                    filename: "dailyos-mcp-launcher-test-target".to_string(),
                    build_sha: env!("BUILD_GIT_SHA").to_string(),
                    sha256: launcher_hash.to_string(),
                    stub: false,
                },
            ],
        };
        let content = serde_json::to_string_pretty(&provenance).expect("serialize provenance");
        std::fs::write(path, content).expect("write provenance");
    }

    fn rewrite_test_provenance<F>(path: &Path, mutate: F)
    where
        F: FnOnce(&mut McpBundleProvenance),
    {
        let content = std::fs::read_to_string(path).expect("read provenance");
        let mut provenance: McpBundleProvenance =
            serde_json::from_str(&content).expect("parse provenance");
        mutate(&mut provenance);
        let content = serde_json::to_string_pretty(&provenance).expect("serialize provenance");
        std::fs::write(path, content).expect("write provenance");
    }

    fn repo_sidecar_path(temp: &tempfile::TempDir) -> PathBuf {
        let dir = std::env::current_exe()
            .expect("current exe")
            .parent()
            .expect("current exe parent")
            .to_path_buf();
        let unique = temp
            .path()
            .file_name()
            .expect("temp file name")
            .to_string_lossy();
        dir.join(format!("dailyos-mcp-test-{unique}"))
    }

    #[test]
    fn parser_rejects_duplicate_manifest_args() {
        let error = LauncherArgs::parse([
            OsString::from("--manifest"),
            OsString::from("first.json"),
            OsString::from("--manifest"),
            OsString::from("second.json"),
        ])
        .expect_err("duplicate manifest rejected");

        assert_eq!(error, "--manifest may only be supplied once");
    }

    #[test]
    fn parser_rejects_duplicate_check_args() {
        let error = LauncherArgs::parse([
            OsString::from("--manifest"),
            OsString::from("manifest.json"),
            OsString::from("--check"),
            OsString::from("--check"),
        ])
        .expect_err("duplicate check rejected");

        assert_eq!(error, "--check may only be supplied once");
    }

    #[test]
    fn wrong_sidecar_hash_is_refused_before_running() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("ran");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        );

        let manifest = manifest_for(sidecar, "0".repeat(64));
        let error = verify_manifest(&manifest).expect_err("hash mismatch");

        assert_eq!(error, "sidecar_hash_mismatch");
        assert!(
            !marker.exists(),
            "sidecar must not execute on hash mismatch"
        );
    }

    #[test]
    fn missing_provenance_is_refused_before_running() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("ran");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);
        std::fs::remove_file(&manifest.bundle_provenance_path).expect("remove provenance");

        let error = verify_manifest(&manifest).expect_err("missing provenance");

        assert!(error.starts_with("provenance_canonicalize_failed"));
        assert!(
            !marker.exists(),
            "sidecar must not execute without bundled provenance"
        );
    }

    #[test]
    fn manifest_hash_must_match_bundled_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            "#!/bin/sh\nprintf '%s\\n' '{\"guardEpoch\":\"unused\"}'\n",
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let mut manifest = manifest_for(sidecar, hash);
        manifest.expected_sidecar_sha256 = "0".repeat(64);

        let error = verify_manifest(&manifest).expect_err("manifest/provenance mismatch");

        assert_eq!(error, "manifest_sidecar_sha256_mismatch");
    }

    #[test]
    fn app_bundle_manifest_allows_signed_hashes_to_differ_from_build_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = temp
            .path()
            .join("DailyOS.app")
            .join("Contents")
            .join("MacOS")
            .join(MCP_SERVER_NAME);
        std::fs::create_dir_all(sidecar.parent().expect("sidecar parent"))
            .expect("create sidecar parent");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        std::fs::copy(
            std::env::current_exe().expect("current exe"),
            sidecar
                .parent()
                .expect("sidecar parent")
                .join(MCP_LAUNCHER_NAME),
        )
        .expect("copy launcher sibling");
        let signed_sidecar_hash = sha256_file(&sidecar).expect("signed sidecar hash");
        let (launcher_path, signed_launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = temp
            .path()
            .join("DailyOS.app")
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_signature_marker(&temp.path().join("DailyOS.app"), TEST_APPLE_TEAM_ID);
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: signed_launcher_hash,
            expected_sidecar_sha256: signed_sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        verify_manifest(&manifest).expect("app bundle manifest verifies signed hashes");
    }

    #[test]
    fn app_bundle_manifest_rejects_manifest_blessed_launcher_hash_mismatch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_bundle = temp.path().join("DailyOS.app");
        let macos_dir = app_bundle.join("Contents").join("MacOS");
        let sidecar = macos_dir.join(MCP_SERVER_NAME);
        std::fs::create_dir_all(&macos_dir).expect("create macos dir");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        let signed_launcher_source = macos_dir.join(MCP_LAUNCHER_NAME);
        write_script(&signed_launcher_source, "#!/bin/sh\nexit 0\n");
        let sidecar_hash = sha256_file(&sidecar).expect("sidecar hash");
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = app_bundle
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_signature_marker(&app_bundle, TEST_APPLE_TEAM_ID);
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let error = verify_manifest(&manifest).expect_err("signed launcher source mismatch");

        assert_eq!(error, "launcher_hash_mismatch");
    }

    #[test]
    fn app_bundle_manifest_rejects_wrong_team_signature_before_live_db_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_bundle = temp.path().join("DailyOS.app");
        let sidecar = app_bundle
            .join("Contents")
            .join("MacOS")
            .join(MCP_SERVER_NAME);
        std::fs::create_dir_all(sidecar.parent().expect("sidecar parent"))
            .expect("create sidecar parent");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        let sidecar_hash = sha256_file(&sidecar).expect("sidecar hash");
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = app_bundle
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_signature_marker(&app_bundle, "FAKETEAMID");
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let error = verify_manifest(&manifest).expect_err("wrong team rejected");

        assert_eq!(error, "app_bundle_signature_team_id_mismatch");
    }

    #[test]
    fn app_bundle_manifest_rejects_wrong_identifier_signature_before_live_db_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_bundle = temp.path().join("DailyOS.app");
        let sidecar = app_bundle
            .join("Contents")
            .join("MacOS")
            .join(MCP_SERVER_NAME);
        std::fs::create_dir_all(sidecar.parent().expect("sidecar parent"))
            .expect("create sidecar parent");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        let sidecar_hash = sha256_file(&sidecar).expect("sidecar hash");
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = app_bundle
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_wrong_identifier_marker(&app_bundle);
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let error = verify_manifest(&manifest).expect_err("wrong identifier rejected");

        assert_eq!(error, "app_bundle_signature_identifier_mismatch");
    }

    #[test]
    fn app_bundle_manifest_rejects_newline_injected_signature_path_before_live_db_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let app_bundle = temp.path().join(format!(
            "Fake\nIdentifier={DAILYOS_APP_BUNDLE_IDENTIFIER}\nTeamIdentifier={TEST_APPLE_TEAM_ID}\nX.app"
        ));
        let sidecar = app_bundle
            .join("Contents")
            .join("MacOS")
            .join(MCP_SERVER_NAME);
        std::fs::create_dir_all(sidecar.parent().expect("sidecar parent"))
            .expect("create sidecar parent");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        let sidecar_hash = sha256_file(&sidecar).expect("sidecar hash");
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = app_bundle
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_signature_marker(&app_bundle, TEST_APPLE_TEAM_ID);
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let error = verify_manifest(&manifest).expect_err("newline-injected path rejected");

        assert_eq!(error, "app_bundle_signature_path_unsafe");
    }

    #[test]
    fn codesign_identity_rejects_duplicate_identifier_fields() {
        let details = format!(
            "Executable=/tmp/Fake\nIdentifier={DAILYOS_APP_BUNDLE_IDENTIFIER}\nTeamIdentifier={TEST_APPLE_TEAM_ID}\nIdentifier=com.example.fake\n"
        );

        let error = verify_codesign_identity(&details, TEST_APPLE_TEAM_ID)
            .expect_err("ambiguous identity rejected");

        assert_eq!(error, "app_bundle_signature_identity_ambiguous");
    }

    #[test]
    fn app_bundle_trusted_anchor_requirement_requires_apple_anchor_identifier_and_team() {
        let requirement = app_bundle_trusted_anchor_requirement("TEAMID1234")
            .expect("trusted anchor requirement");

        assert_eq!(
            requirement,
            format!(
                "anchor apple generic and identifier \"{DAILYOS_APP_BUNDLE_IDENTIFIER}\" and certificate leaf[subject.OU] = \"TEAMID1234\""
            )
        );
    }

    #[test]
    fn app_bundle_trusted_anchor_requirement_rejects_invalid_team_id() {
        let error = app_bundle_trusted_anchor_requirement("TEAM\"ID")
            .expect_err("invalid team id rejected");

        assert_eq!(error, "app_bundle_signature_team_id_invalid");
    }

    #[test]
    fn app_bundle_manifest_rejects_forged_unsigned_bundle_before_live_db_mode() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = temp
            .path()
            .join("Fake.app")
            .join("Contents")
            .join("MacOS")
            .join(MCP_SERVER_NAME);
        std::fs::create_dir_all(sidecar.parent().expect("sidecar parent"))
            .expect("create sidecar parent");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        let sidecar_hash = sha256_file(&sidecar).expect("sidecar hash");
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = temp
            .path()
            .join("Fake.app")
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let error = verify_manifest(&manifest).expect_err("unsigned forged app rejected");

        assert_eq!(error, "app_bundle_signature_verify_failed");
    }

    #[test]
    fn app_bundle_manifest_rejects_sidecar_outside_provenance_bundle() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = temp
            .path()
            .join("Fake.app")
            .join("Contents")
            .join("MacOS")
            .join(MCP_SERVER_NAME);
        std::fs::create_dir_all(sidecar.parent().expect("sidecar parent"))
            .expect("create sidecar parent");
        write_script(&sidecar, "#!/bin/sh\nexit 0\n");
        let sidecar_hash = sha256_file(&sidecar).expect("sidecar hash");
        let (launcher_path, launcher_hash) = current_launcher_path_and_hash();
        let provenance_path = temp
            .path()
            .join("DailyOS.app")
            .join("Contents")
            .join("Resources")
            .join("binaries")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
        std::fs::create_dir_all(provenance_path.parent().expect("provenance parent"))
            .expect("create provenance parent");
        write_test_signature_marker(&temp.path().join("Fake.app"), TEST_APPLE_TEAM_ID);
        write_test_provenance(&provenance_path, &sidecar, &"1".repeat(64), &"2".repeat(64));
        let manifest = McpLauncherManifest {
            schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            launcher_build_sha: env!("BUILD_GIT_SHA").to_string(),
            sidecar_build_sha: env!("BUILD_GIT_SHA").to_string(),
            source_kind: McpRuntimeSourceKind::AppBundle,
            bundle_provenance_path: provenance_path,
            launcher_path,
            sidecar_path: sidecar,
            expected_launcher_sha256: launcher_hash,
            expected_sidecar_sha256: sidecar_hash,
            final_server_db_mode: "live".to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
        };

        let error = verify_manifest(&manifest).expect_err("cross-bundle manifest rejected");

        assert_eq!(error, "app_bundle_root_mismatch");
    }

    #[test]
    fn repo_binaries_manifest_rejects_live_db_mode_before_running() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("ran");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let mut manifest = manifest_for(sidecar, hash);
        manifest.final_server_db_mode = "live".to_string();

        let error = verify_manifest(&manifest).expect_err("repo live DB mode rejected");

        assert_eq!(error, "manifest_db_mode_source_kind_mismatch");
        assert!(
            !marker.exists(),
            "sidecar must not run for DB-mode/source-kind mismatch"
        );
    }

    #[test]
    fn manifest_refusal_matrix_blocks_before_running() {
        fn expect_error<F, G>(mutate_manifest: F, mutate_provenance: G, expected_prefix: &str)
        where
            F: FnOnce(&mut McpLauncherManifest, &Path),
            G: FnOnce(&McpLauncherManifest),
        {
            let temp = tempfile::tempdir().expect("tempdir");
            let marker = temp.path().join("ran");
            let sidecar = repo_sidecar_path(&temp);
            write_script(
                &sidecar,
                &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
            );
            let hash = sha256_file(&sidecar).expect("hash sidecar");
            let mut manifest = manifest_for(sidecar, hash);
            mutate_provenance(&manifest);
            let sidecar_path = manifest.sidecar_path.clone();
            mutate_manifest(&mut manifest, &sidecar_path);

            let error = verify_manifest(&manifest).expect_err(expected_prefix);

            assert!(
                error.starts_with(expected_prefix),
                "expected prefix {expected_prefix}, got {error}"
            );
            assert!(
                !marker.exists(),
                "sidecar must not run for {expected_prefix}"
            );
        }

        expect_error(
            |manifest, _| manifest.schema_version = 0,
            |_| {},
            "manifest_schema_unsupported",
        );
        expect_error(
            |manifest, _| manifest.guard_epoch = "stale".to_string(),
            |_| {},
            "manifest_guard_epoch_mismatch",
        );
        expect_error(
            |manifest, _| manifest.final_server_db_mode = "production".to_string(),
            |_| {},
            "manifest_db_mode_invalid",
        );
        expect_error(
            |manifest, _| manifest.launcher_path = PathBuf::from("/no/such/launcher"),
            |_| {},
            "launcher_manifest_canonicalize_failed",
        );
        expect_error(
            |manifest, sidecar| manifest.launcher_path = sidecar.to_path_buf(),
            |_| {},
            "launcher_path_mismatch",
        );
        expect_error(
            |_, sidecar| std::fs::remove_file(sidecar).expect("remove sidecar"),
            |_| {},
            "sidecar_canonicalize_failed",
        );
        expect_error(
            |manifest, sidecar| {
                std::fs::write(sidecar, b"").expect("zero sidecar");
                make_executable(sidecar);
                manifest.expected_sidecar_sha256 = sha256_file(sidecar).expect("hash sidecar");
                rewrite_test_provenance(&manifest.bundle_provenance_path, |provenance| {
                    let sidecar = provenance
                        .sidecars
                        .iter_mut()
                        .find(|entry| entry.name == MCP_SERVER_NAME)
                        .expect("sidecar provenance");
                    sidecar.sha256 = manifest.expected_sidecar_sha256.clone();
                });
            },
            |_| {},
            "sidecar_zero_byte",
        );
        expect_error(
            |_, _| {},
            |manifest| {
                rewrite_test_provenance(&manifest.bundle_provenance_path, |provenance| {
                    provenance.stub = true;
                });
            },
            "provenance_is_stub",
        );
        expect_error(
            |_, _| {},
            |manifest| {
                rewrite_test_provenance(&manifest.bundle_provenance_path, |provenance| {
                    provenance.target_triple = "wrong-target".to_string();
                });
            },
            "provenance_target_triple_mismatch",
        );
        expect_error(
            |_, _| {},
            |manifest| {
                rewrite_test_provenance(&manifest.bundle_provenance_path, |provenance| {
                    let sidecar = provenance
                        .sidecars
                        .iter_mut()
                        .find(|entry| entry.name == MCP_SERVER_NAME)
                        .expect("sidecar provenance");
                    sidecar.build_sha = "wrong".to_string();
                });
            },
            "dailyos-mcp_build_sha_mismatch",
        );
        expect_error(
            |manifest, _| manifest.sidecar_build_sha = "wrong".to_string(),
            |_| {},
            "manifest_sidecar_build_sha_mismatch",
        );
    }

    #[test]
    fn wrong_source_kind_path_is_refused_before_running() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("ran");
        let sidecar = temp.path().join("outside").join("dailyos-mcp-test");
        std::fs::create_dir_all(sidecar.parent().expect("parent")).expect("create parent");
        write_script(
            &sidecar,
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);

        let error = verify_manifest(&manifest).expect_err("wrong source path");

        assert_eq!(error, "sidecar_not_in_managed_runtime_dir");
        assert!(
            !marker.exists(),
            "sidecar must not run for wrong source path"
        );
    }

    #[test]
    fn self_check_runs_without_db_mode_env() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("db-mode");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            &format!(
                "#!/bin/sh\nprintf '%s' \"${{DAILYOS_DB_MODE-unset}}\" > '{}'\nprintf '%s\\n' '{{\"guardEpoch\":\"{}\",\"buildSha\":\"{}\",\"defaultDbMode\":\"replica\",\"runtimeContainsDbModeGuard\":true,\"dbOpened\":false}}'\n",
                marker.display(),
                MCP_RUNTIME_GUARD_EPOCH,
                env!("BUILD_GIT_SHA")
            ),
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);

        verify_manifest(&manifest).expect("manifest verifies");
        run_self_check(&manifest, SELF_CHECK_TEST_TIMEOUT).expect("self-check succeeds");

        assert_eq!(
            std::fs::read_to_string(marker).expect("read marker"),
            "unset"
        );
    }

    #[test]
    fn self_check_does_not_inherit_injected_env() {
        let temp = tempfile::tempdir().expect("tempdir");
        let marker = temp.path().join("injected-env");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            &format!(
                "#!/bin/sh\nprintf '%s' \"${{DAILYOS_ATTACKER_ENV-unset}}\" > '{}'\nprintf '%s\\n' '{{\"guardEpoch\":\"{}\",\"buildSha\":\"{}\",\"defaultDbMode\":\"replica\",\"runtimeContainsDbModeGuard\":true,\"dbOpened\":false}}'\n",
                marker.display(),
                MCP_RUNTIME_GUARD_EPOCH,
                env!("BUILD_GIT_SHA")
            ),
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);

        std::env::set_var("DAILYOS_ATTACKER_ENV", "present");
        let result = run_self_check(&manifest, SELF_CHECK_TEST_TIMEOUT);
        std::env::remove_var("DAILYOS_ATTACKER_ENV");

        result.expect("self-check succeeds");
        assert_eq!(
            std::fs::read_to_string(marker).expect("read marker"),
            "unset"
        );
    }

    #[test]
    fn hanging_self_check_times_out() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = repo_sidecar_path(&temp);
        write_script(&sidecar, "#!/bin/sh\nsleep 5\n");
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);

        verify_manifest(&manifest).expect("manifest verifies");
        let error =
            run_self_check(&manifest, Duration::from_millis(100)).expect_err("timeout expected");

        assert_eq!(error, "self_check_timeout");
    }

    #[test]
    fn leaked_self_check_stdio_stays_under_timeout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            &format!(
                "#!/bin/sh\n(sleep 5) &\nprintf '%s\\n' '{{\"guardEpoch\":\"{}\",\"buildSha\":\"{}\",\"defaultDbMode\":\"replica\",\"runtimeContainsDbModeGuard\":true,\"dbOpened\":false}}'\n",
                MCP_RUNTIME_GUARD_EPOCH,
                env!("BUILD_GIT_SHA")
            ),
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);

        verify_manifest(&manifest).expect("manifest verifies");
        let error =
            run_self_check(&manifest, Duration::from_millis(100)).expect_err("timeout expected");

        assert_eq!(error, "self_check_timeout");
    }

    #[test]
    fn sidecar_command_owns_db_mode_and_clears_inherited_env() {
        let temp = tempfile::tempdir().expect("tempdir");
        let sidecar = repo_sidecar_path(&temp);
        write_script(
            &sidecar,
            "#!/bin/sh\nprintf 'legacy=%s injected=%s db=%s\\n' \"${DAILYOS_MCP_LEGACY_V1-unset}\" \"${DAILYOS_ATTACKER_ENV-unset}\" \"${DAILYOS_DB_MODE-unset}\"\n",
        );
        let hash = sha256_file(&sidecar).expect("hash sidecar");
        let manifest = manifest_for(sidecar, hash);
        let mut command = Command::new(&manifest.sidecar_path);
        command.env("DAILYOS_MCP_LEGACY_V1", "1");
        command.env("DAILYOS_ATTACKER_ENV", "present");
        configure_sidecar_env(&mut command, &manifest);
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output =
            run_bounded(command, SELF_CHECK_TEST_TIMEOUT, SELF_CHECK_OUTPUT_LIMIT).expect("run");

        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).expect("utf8"),
            "legacy=unset injected=unset db=replica\n"
        );
    }

    #[test]
    fn self_check_refusal_matrix() {
        fn expect_self_check_error(body: String, expected: &str) {
            let temp = tempfile::tempdir().expect("tempdir");
            let sidecar = repo_sidecar_path(&temp);
            write_script(&sidecar, &body);
            let hash = sha256_file(&sidecar).expect("hash sidecar");
            let manifest = manifest_for(sidecar, hash);

            verify_manifest(&manifest).expect("manifest verifies");
            let error = run_self_check(&manifest, SELF_CHECK_TEST_TIMEOUT).expect_err(expected);

            assert!(
                error.starts_with(expected),
                "expected prefix {expected}, got {error}"
            );
        }

        let valid = format!(
            "{{\"guardEpoch\":\"{}\",\"buildSha\":\"{}\",\"defaultDbMode\":\"replica\",\"runtimeContainsDbModeGuard\":true,\"dbOpened\":false}}",
            MCP_RUNTIME_GUARD_EPOCH,
            env!("BUILD_GIT_SHA")
        );
        expect_self_check_error("#!/bin/sh\nexit 42\n".to_string(), "self_check_exit_status");
        expect_self_check_error(
            "#!/bin/sh\nprintf '%s\\n' 'not-json'\n".to_string(),
            "self_check_json_invalid",
        );
        expect_self_check_error(
            format!("#!/bin/sh\nprintf '%s\\n' '{}'\n", "{}"),
            "self_check_guardEpoch_mismatch",
        );
        expect_self_check_error(
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                valid.replace(MCP_RUNTIME_GUARD_EPOCH, "wrong")
            ),
            "self_check_guardEpoch_mismatch",
        );
        expect_self_check_error(
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                valid.replace(env!("BUILD_GIT_SHA"), "wrong")
            ),
            "self_check_buildSha_mismatch",
        );
        expect_self_check_error(
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                valid.replace(
                    "\"defaultDbMode\":\"replica\"",
                    "\"defaultDbMode\":\"live\""
                )
            ),
            "self_check_defaultDbMode_mismatch",
        );
        expect_self_check_error(
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                valid.replace(
                    "\"runtimeContainsDbModeGuard\":true",
                    "\"runtimeContainsDbModeGuard\":false"
                )
            ),
            "self_check_runtimeContainsDbModeGuard_mismatch",
        );
        expect_self_check_error(
            format!(
                "#!/bin/sh\nprintf '%s\\n' '{}'\n",
                valid.replace("\"dbOpened\":false", "\"dbOpened\":true")
            ),
            "self_check_dbOpened_mismatch",
        );
        expect_self_check_error(
            "#!/bin/sh\nawk 'BEGIN { for (i = 0; i < 70000; i++) printf \"x\" }'\n".to_string(),
            "self_check_output_too_large",
        );
        expect_self_check_error(
            "#!/bin/sh\nawk 'BEGIN { for (i = 0; i < 70000; i++) printf \"x\" }' >&2\n".to_string(),
            "self_check_output_too_large",
        );
    }
}
