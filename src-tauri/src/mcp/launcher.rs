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
use std::thread;
use std::time::{Duration, Instant};

use mcp_launcher_contract::{
    McpBundleProvenance, McpLauncherManifest, McpRuntimeSourceKind, McpSidecarProvenance,
};
use mcp_runtime_guard_constants::{
    MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION, MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION, MCP_LAUNCHER_NAME,
    MCP_NO_ENV_DEFAULT_DB_MODE, MCP_RUNTIME_GUARD_EPOCH, MCP_SERVER_NAME,
};
use sha2::{Digest, Sha256};

const SELF_CHECK_TIMEOUT: Duration = Duration::from_secs(3);
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
                    let path = args
                        .next()
                        .ok_or_else(|| "--manifest requires a path".to_string())?;
                    manifest_path = Some(PathBuf::from(path));
                }
                "--check" => check_only = true,
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
    if manifest.generated_at.trim().is_empty() {
        return Err("manifest_generated_at_missing".to_string());
    }
    verify_bundle_provenance(manifest)?;

    verify_launcher_path(manifest)?;
    verify_sidecar_path(manifest)?;
    verify_file_hash(
        &manifest.launcher_path,
        &manifest.expected_launcher_sha256,
        "launcher",
    )?;
    verify_file_hash(
        &manifest.sidecar_path,
        &manifest.expected_sidecar_sha256,
        "sidecar",
    )?;
    Ok(())
}

fn verify_bundle_provenance(manifest: &McpLauncherManifest) -> Result<(), String> {
    let provenance_path = canonicalize_existing(&manifest.bundle_provenance_path, "provenance")?;
    verify_provenance_path(&provenance_path, manifest)?;
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
    if manifest.expected_launcher_sha256 != launcher.sha256 {
        return Err("manifest_launcher_sha256_mismatch".to_string());
    }
    if manifest.expected_sidecar_sha256 != sidecar.sha256 {
        return Err("manifest_sidecar_sha256_mismatch".to_string());
    }
    verify_manifest_sidecar_filename(manifest, sidecar)?;
    Ok(())
}

fn verify_provenance_path(path: &Path, manifest: &McpLauncherManifest) -> Result<(), String> {
    if contains_raw_build_path(path) {
        return Err("provenance_path_raw_build_path".to_string());
    }
    match manifest.source_kind {
        McpRuntimeSourceKind::AppBundle => {
            if !contains_adjacent_components(path, "Contents", "Resources") {
                return Err("provenance_not_in_app_bundle_resources_dir".to_string());
            }
        }
        McpRuntimeSourceKind::RepoBinaries => {
            if !ends_with_components(path, &["src-tauri", "binaries"]) {
                return Err("provenance_not_in_repo_binaries_dir".to_string());
            }
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
    if contains_raw_build_path(&sidecar) {
        return Err("sidecar_path_raw_build_path".to_string());
    }

    match manifest.source_kind {
        McpRuntimeSourceKind::AppBundle => {
            if !contains_adjacent_components(&sidecar, "Contents", "MacOS") {
                return Err("sidecar_not_in_app_bundle_macos_dir".to_string());
            }
        }
        McpRuntimeSourceKind::RepoBinaries => {
            if !ends_with_components(&sidecar, &["src-tauri", "binaries"]) {
                return Err("sidecar_not_in_repo_binaries_dir".to_string());
            }
        }
    }
    Ok(())
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

fn ends_with_components(path: &Path, suffix: &[&str]) -> bool {
    let parent = match path.parent() {
        Some(parent) => parent,
        None => return false,
    };
    let components: Vec<String> = parent
        .components()
        .filter_map(component_name)
        .map(str::to_string)
        .collect();
    components.len() >= suffix.len()
        && components
            .iter()
            .rev()
            .zip(suffix.iter().rev())
            .all(|(actual, expected)| actual == expected)
}

fn component_name(component: Component<'_>) -> Option<&str> {
    match component {
        Component::Normal(value) => value.to_str(),
        _ => None,
    }
}

fn verify_file_hash(path: &Path, expected: &str, label: &str) -> Result<(), String> {
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

    let actual = sha256_file(path).map_err(|error| format!("{label}_hash_failed: {error}"))?;
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
        .env_remove(ENV_DB_MODE)
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

    let status = wait_with_timeout(&mut child, timeout);
    let (stdout, stdout_truncated) = join_reader(stdout_reader, "stdout")?;
    let (_stderr, stderr_truncated) = join_reader(stderr_reader, "stderr")?;
    let status = status?;

    Ok(BoundedOutput {
        status,
        stdout,
        stdout_truncated,
        stderr_truncated,
    })
}

fn read_limited<R>(reader: R, limit: u64) -> thread::JoinHandle<io::Result<(Vec<u8>, bool)>>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut limited = reader.take(limit + 1);
        let mut buffer = Vec::new();
        limited.read_to_end(&mut buffer)?;
        let truncated = buffer.len() as u64 > limit;
        if truncated {
            buffer.truncate(limit as usize);
        }
        Ok((buffer, truncated))
    })
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<ExitStatus, String> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() >= deadline => {
                if let Err(_error) = child.kill() {}
                if let Err(_error) = child.wait() {}
                return Err("self_check_timeout".to_string());
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(error) => return Err(format!("self_check_wait_failed: {error}")),
        }
    }
}

fn join_reader(
    reader: thread::JoinHandle<io::Result<(Vec<u8>, bool)>>,
    label: &str,
) -> Result<(Vec<u8>, bool), String> {
    reader
        .join()
        .map_err(|_| format!("self_check_{label}_reader_panicked"))?
        .map_err(|error| format!("self_check_{label}_read_failed: {error}"))
}

fn exec_sidecar(manifest: McpLauncherManifest) -> Result<(), String> {
    let mut command = Command::new(&manifest.sidecar_path);
    command.env(ENV_DB_MODE, &manifest.final_server_db_mode);

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

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
        let provenance_path = sidecar_path
            .parent()
            .expect("sidecar parent")
            .join("dailyos-mcp-bundle-test-target.provenance.json");
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
            target_triple: "test-target".to_string(),
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

    fn repo_sidecar_path(temp: &tempfile::TempDir) -> PathBuf {
        let dir = temp.path().join("src-tauri").join("binaries");
        std::fs::create_dir_all(&dir).expect("create binaries dir");
        dir.join("dailyos-mcp-test")
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
        run_self_check(&manifest, Duration::from_secs(1)).expect("self-check succeeds");

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
}
