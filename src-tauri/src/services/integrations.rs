// Integrations service - business logic for Claude Desktop MCP configuration.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use crate::mcp_launcher_contract::{
    McpBundleProvenance, McpLauncherManifest, McpRuntimeSourceKind, McpSidecarProvenance,
};
use crate::mcp_runtime_guard_constants::{
    MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION, MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION, MCP_LAUNCHER_NAME,
    MCP_RUNTIME_GUARD_EPOCH, MCP_SERVER_NAME,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

const CLAUDE_CONFIG_RELATIVE_PATH: &[&str] = &[
    "Library",
    "Application Support",
    "Claude",
    "claude_desktop_config.json",
];
const DAILYOS_MCP_DIR: &str = ".dailyos/mcp";
const MANIFEST_FILENAME: &str = "dailyos-mcp-manifest.json";
const MCP_CONFIG_KEY: &str = "dailyos";

/// Result of Claude Desktop MCP configuration.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeDesktopConfigResult {
    pub success: bool,
    pub message: String,
    pub config_path: Option<String>,
    pub binary_path: Option<String>,
}

#[derive(Debug, Clone)]
struct IntegrationPaths {
    home: PathBuf,
    current_exe: PathBuf,
    current_dir: PathBuf,
    run_launcher_check: bool,
}

impl IntegrationPaths {
    fn from_environment() -> Result<Self, String> {
        Ok(Self {
            home: dirs::home_dir().ok_or_else(|| "Could not find home directory".to_string())?,
            current_exe: std::env::current_exe()
                .map_err(|error| format!("Could not resolve current executable: {error}"))?,
            current_dir: std::env::current_dir()
                .map_err(|error| format!("Could not resolve current directory: {error}"))?,
            run_launcher_check: true,
        })
    }

    fn claude_config_path(&self) -> PathBuf {
        CLAUDE_CONFIG_RELATIVE_PATH
            .iter()
            .fold(self.home.clone(), |path, part| path.join(part))
    }

    fn app_mcp_dir(&self) -> PathBuf {
        self.home.join(DAILYOS_MCP_DIR)
    }

    fn managed_launcher_path(&self) -> PathBuf {
        self.app_mcp_dir().join(MCP_LAUNCHER_NAME)
    }

    fn managed_manifest_path(&self) -> PathBuf {
        self.app_mcp_dir().join(MANIFEST_FILENAME)
    }
}

#[derive(Debug, Clone)]
struct ResolvedMcpRuntime {
    source_kind: McpRuntimeSourceKind,
    provenance_path: PathBuf,
    launcher_source_path: PathBuf,
    sidecar_path: PathBuf,
    launcher: McpSidecarProvenance,
    sidecar: McpSidecarProvenance,
}

/// Check whether DailyOS is safely registered in Claude Desktop's MCP config.
pub fn get_claude_desktop_status() -> ClaudeDesktopConfigResult {
    match IntegrationPaths::from_environment() {
        Ok(paths) => get_claude_desktop_status_with_paths(&paths),
        Err(message) => ClaudeDesktopConfigResult {
            success: false,
            message,
            config_path: None,
            binary_path: None,
        },
    }
}

fn get_claude_desktop_status_with_paths(paths: &IntegrationPaths) -> ClaudeDesktopConfigResult {
    let config_path = paths.claude_config_path();
    if !config_path.exists() {
        return result(false, "Not configured", None, None);
    }

    let config: Value = match read_json_file(&config_path) {
        Ok(config) => config,
        Err(message) => return result(false, &message, Some(config_path.clone()), None),
    };

    let server = match config
        .get("mcpServers")
        .and_then(|servers| servers.get(MCP_CONFIG_KEY))
    {
        Some(server) => server,
        None => return result(false, "Not configured", Some(config_path), None),
    };

    let command = server
        .get("command")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let command = match command {
        Some(command) => command,
        None => {
            return result(
                false,
                "Unsafe Claude Desktop config: missing DailyOS MCP command",
                Some(config_path),
                None,
            )
        }
    };

    if is_legacy_raw_command(&command) {
        return result(
            false,
            "Unsafe legacy Claude Desktop config: reconfigure DailyOS",
            Some(config_path),
            Some(command),
        );
    }

    let expected_launcher = paths.managed_launcher_path();
    if !paths_equal(&command, &expected_launcher) {
        return result(
            false,
            "Unsafe Claude Desktop config: command is not the DailyOS managed launcher",
            Some(config_path),
            Some(command),
        );
    }

    let manifest_path = match manifest_arg(server) {
        Some(path) => path,
        None => {
            return result(
                false,
                "Unsafe Claude Desktop config: missing launcher manifest argument",
                Some(config_path),
                Some(command),
            )
        }
    };

    if !paths_equal(&manifest_path, &paths.managed_manifest_path()) {
        return result(
            false,
            "Unsafe Claude Desktop config: manifest is not DailyOS managed",
            Some(config_path),
            Some(command),
        );
    }

    match validate_existing_launcher(paths, &command, &manifest_path) {
        Ok(()) => result(
            true,
            "Connected",
            Some(config_path),
            Some(expected_launcher),
        ),
        Err(message) => result(false, &message, Some(config_path), Some(command)),
    }
}

/// Configure Claude Desktop to use the guarded DailyOS MCP launcher.
pub fn configure_claude_desktop(
    ctx: &crate::services::context::ServiceContext<'_>,
) -> Result<ClaudeDesktopConfigResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let paths = IntegrationPaths::from_environment()?;
    configure_claude_desktop_with_paths(&paths)
}

fn configure_claude_desktop_with_paths(
    paths: &IntegrationPaths,
) -> Result<ClaudeDesktopConfigResult, String> {
    let runtime = resolve_mcp_runtime(paths)?;
    std::fs::create_dir_all(paths.app_mcp_dir())
        .map_err(|error| format!("Failed to create MCP runtime directory: {error}"))?;

    let managed_launcher = paths.managed_launcher_path();
    std::fs::copy(&runtime.launcher_source_path, &managed_launcher)
        .map_err(|error| format!("Failed to install MCP launcher: {error}"))?;
    ensure_executable(&managed_launcher)
        .map_err(|error| format!("Failed to make MCP launcher executable: {error}"))?;

    let manifest = build_manifest(&runtime, &managed_launcher);
    let manifest_path = paths.managed_manifest_path();
    write_json_file(&manifest_path, &manifest)
        .map_err(|error| format!("Failed to write MCP launcher manifest: {error}"))?;

    validate_existing_launcher(paths, &managed_launcher, &manifest_path)?;

    let config_path = paths.claude_config_path();
    let mut config = read_json_file(&config_path).unwrap_or_else(|_| serde_json::json!({}));
    if !config.get("mcpServers").is_some_and(Value::is_object) {
        config["mcpServers"] = serde_json::json!({});
    }
    config["mcpServers"][MCP_CONFIG_KEY] = serde_json::json!({
        "command": managed_launcher.to_string_lossy(),
        "args": ["--manifest", manifest_path.to_string_lossy()],
        "env": {}
    });

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create config directory: {error}"))?;
    }
    write_json_file(&config_path, &config)
        .map_err(|error| format!("Failed to write Claude Desktop config: {error}"))?;

    Ok(result(
        true,
        "Claude Desktop configured. Restart Claude Desktop to connect.",
        Some(config_path),
        Some(managed_launcher),
    ))
}

fn result(
    success: bool,
    message: &str,
    config_path: Option<PathBuf>,
    binary_path: Option<PathBuf>,
) -> ClaudeDesktopConfigResult {
    ClaudeDesktopConfigResult {
        success,
        message: message.to_string(),
        config_path: config_path.map(|path| path.to_string_lossy().to_string()),
        binary_path: binary_path.map(|path| path.to_string_lossy().to_string()),
    }
}

fn resolve_mcp_runtime(paths: &IntegrationPaths) -> Result<ResolvedMcpRuntime, String> {
    let mut errors = Vec::new();
    for candidate in runtime_candidates(paths) {
        match verify_runtime_candidate(candidate) {
            Ok(runtime) => return Ok(runtime),
            Err(error) => errors.push(error),
        }
    }

    Err(if errors.is_empty() {
        "The guarded DailyOS MCP runtime is missing. Run pnpm build:mcp or reinstall DailyOS."
            .to_string()
    } else {
        format!(
            "The guarded DailyOS MCP runtime is not usable: {}",
            errors.join("; ")
        )
    })
}

#[derive(Debug)]
struct RuntimeCandidate {
    source_kind: McpRuntimeSourceKind,
    binary_dir: PathBuf,
    provenance_path: PathBuf,
    packaged_filenames: bool,
}

fn runtime_candidates(paths: &IntegrationPaths) -> Vec<RuntimeCandidate> {
    let mut candidates = Vec::new();

    if let Some(exe_dir) = paths.current_exe.parent() {
        if let Some(resource_dir) = resource_dir_for_exe(&paths.current_exe) {
            for provenance_dir in packaged_provenance_dirs(&resource_dir) {
                candidates.extend(provenance_files(&provenance_dir).into_iter().map(
                    |provenance_path| RuntimeCandidate {
                        source_kind: McpRuntimeSourceKind::AppBundle,
                        binary_dir: exe_dir.to_path_buf(),
                        provenance_path,
                        packaged_filenames: true,
                    },
                ));
            }
        }
    }

    for binary_dir in dev_binary_dirs(&paths.current_dir) {
        candidates.extend(
            provenance_files(&binary_dir)
                .into_iter()
                .map(|provenance_path| RuntimeCandidate {
                    source_kind: McpRuntimeSourceKind::RepoBinaries,
                    binary_dir: binary_dir.clone(),
                    provenance_path,
                    packaged_filenames: false,
                }),
        );
    }

    candidates
}

fn packaged_provenance_dirs(resource_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for dir in [resource_dir.to_path_buf(), resource_dir.join("binaries")] {
        if !dirs.iter().any(|existing| existing == &dir) {
            dirs.push(dir);
        }
    }
    dirs
}

fn resource_dir_for_exe(exe: &Path) -> Option<PathBuf> {
    let macos_dir = exe.parent()?;
    if macos_dir.file_name().and_then(|name| name.to_str()) == Some("MacOS")
        && macos_dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("Contents")
    {
        return macos_dir
            .parent()
            .map(|contents| contents.join("Resources"));
    }
    None
}

fn dev_binary_dirs(current_dir: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for ancestor in current_dir.ancestors().take(6) {
        let src_tauri_binaries = ancestor.join("src-tauri").join("binaries");
        if src_tauri_binaries.is_dir() && !dirs.iter().any(|dir| dir == &src_tauri_binaries) {
            dirs.push(src_tauri_binaries);
        }
        let direct_binaries = ancestor.join("binaries");
        if ancestor.file_name().and_then(|name| name.to_str()) == Some("src-tauri")
            && direct_binaries.is_dir()
            && !dirs.iter().any(|dir| dir == &direct_binaries)
        {
            dirs.push(direct_binaries);
        }
    }
    dirs
}

fn provenance_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".provenance.json"))
        })
        .collect()
}

fn verify_runtime_candidate(candidate: RuntimeCandidate) -> Result<ResolvedMcpRuntime, String> {
    let provenance: McpBundleProvenance = read_json_file(&candidate.provenance_path)?;
    verify_bundle_provenance(&provenance)?;

    let launcher = provenance
        .sidecar(MCP_LAUNCHER_NAME)
        .cloned()
        .ok_or_else(|| "provenance_missing_launcher".to_string())?;
    let sidecar = provenance
        .sidecar(MCP_SERVER_NAME)
        .cloned()
        .ok_or_else(|| "provenance_missing_mcp".to_string())?;

    verify_sidecar_provenance(&launcher)?;
    verify_sidecar_provenance(&sidecar)?;

    let launcher_source_path = candidate
        .binary_dir
        .join(runtime_filename(&launcher, candidate.packaged_filenames));
    let sidecar_path = candidate
        .binary_dir
        .join(runtime_filename(&sidecar, candidate.packaged_filenames));

    verify_runtime_file(&launcher_source_path, &launcher.sha256, MCP_LAUNCHER_NAME)?;
    verify_runtime_file(&sidecar_path, &sidecar.sha256, MCP_SERVER_NAME)?;

    Ok(ResolvedMcpRuntime {
        source_kind: candidate.source_kind,
        provenance_path: candidate.provenance_path,
        launcher_source_path,
        sidecar_path,
        launcher,
        sidecar,
    })
}

fn runtime_filename(sidecar: &McpSidecarProvenance, packaged: bool) -> String {
    if packaged {
        sidecar.name.clone()
    } else {
        sidecar.filename.clone()
    }
}

fn verify_bundle_provenance(provenance: &McpBundleProvenance) -> Result<(), String> {
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
    if provenance.target_triple.trim().is_empty() {
        return Err("provenance_target_triple_missing".to_string());
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

fn verify_runtime_file(path: &Path, expected_sha256: &str, label: &str) -> Result<(), String> {
    let metadata =
        std::fs::metadata(path).map_err(|error| format!("{label}_metadata_failed: {error}"))?;
    if !metadata.is_file() {
        return Err(format!("{label}_not_file"));
    }
    if metadata.len() == 0 {
        return Err(format!("{label}_zero_byte"));
    }
    if contains_raw_build_path(path) {
        return Err(format!("{label}_raw_build_path"));
    }
    ensure_executable(path).map_err(|error| format!("{label}_not_executable: {error}"))?;
    let actual = sha256_file(path).map_err(|error| format!("{label}_hash_failed: {error}"))?;
    if actual != expected_sha256 {
        return Err(format!("{label}_hash_mismatch"));
    }
    Ok(())
}

fn build_manifest(runtime: &ResolvedMcpRuntime, managed_launcher: &Path) -> McpLauncherManifest {
    let final_server_db_mode = match runtime.source_kind {
        McpRuntimeSourceKind::AppBundle => "live",
        McpRuntimeSourceKind::RepoBinaries => "replica",
    };

    McpLauncherManifest {
        schema_version: MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION,
        guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
        app_build_sha: env!("BUILD_GIT_SHA").to_string(),
        launcher_build_sha: runtime.launcher.build_sha.clone(),
        sidecar_build_sha: runtime.sidecar.build_sha.clone(),
        source_kind: runtime.source_kind.clone(),
        bundle_provenance_path: runtime.provenance_path.clone(),
        launcher_path: managed_launcher.to_path_buf(),
        sidecar_path: runtime.sidecar_path.clone(),
        expected_launcher_sha256: runtime.launcher.sha256.clone(),
        expected_sidecar_sha256: runtime.sidecar.sha256.clone(),
        final_server_db_mode: final_server_db_mode.to_string(),
        generated_at: generated_at(),
    }
}

fn generated_at() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn validate_existing_launcher(
    paths: &IntegrationPaths,
    launcher_path: &Path,
    manifest_path: &Path,
) -> Result<(), String> {
    if contains_raw_build_path(launcher_path) || contains_raw_build_path(manifest_path) {
        return Err("Unsafe Claude Desktop config: raw build path".to_string());
    }
    let manifest: McpLauncherManifest = read_json_file(manifest_path)?;
    if manifest.schema_version != MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION {
        return Err("Unsafe Claude Desktop config: unsupported launcher manifest".to_string());
    }
    if manifest.guard_epoch != MCP_RUNTIME_GUARD_EPOCH {
        return Err("Unsafe Claude Desktop config: stale launcher manifest".to_string());
    }
    if !paths_equal(&manifest.launcher_path, launcher_path) {
        return Err("Unsafe Claude Desktop config: launcher manifest path mismatch".to_string());
    }
    verify_manifest_provenance(&manifest)?;
    verify_runtime_file(
        launcher_path,
        &manifest.expected_launcher_sha256,
        MCP_LAUNCHER_NAME,
    )?;
    verify_runtime_file(
        &manifest.sidecar_path,
        &manifest.expected_sidecar_sha256,
        MCP_SERVER_NAME,
    )?;

    if paths.run_launcher_check {
        run_launcher_check(launcher_path, manifest_path)?;
    }

    Ok(())
}

fn verify_manifest_provenance(manifest: &McpLauncherManifest) -> Result<(), String> {
    if contains_raw_build_path(&manifest.bundle_provenance_path) {
        return Err("Unsafe Claude Desktop config: raw provenance path".to_string());
    }
    let provenance: McpBundleProvenance = read_json_file(&manifest.bundle_provenance_path)?;
    verify_bundle_provenance(&provenance)?;
    if manifest.app_build_sha != provenance.app_build_sha {
        return Err("Unsafe Claude Desktop config: app build mismatch".to_string());
    }

    let launcher = provenance
        .sidecar(MCP_LAUNCHER_NAME)
        .ok_or_else(|| "Unsafe Claude Desktop config: provenance missing launcher".to_string())?;
    let sidecar = provenance.sidecar(MCP_SERVER_NAME).ok_or_else(|| {
        "Unsafe Claude Desktop config: provenance missing MCP sidecar".to_string()
    })?;
    verify_sidecar_provenance(launcher)?;
    verify_sidecar_provenance(sidecar)?;

    if manifest.launcher_build_sha != launcher.build_sha
        || manifest.expected_launcher_sha256 != launcher.sha256
    {
        return Err("Unsafe Claude Desktop config: launcher provenance mismatch".to_string());
    }
    if manifest.sidecar_build_sha != sidecar.build_sha
        || manifest.expected_sidecar_sha256 != sidecar.sha256
    {
        return Err("Unsafe Claude Desktop config: sidecar provenance mismatch".to_string());
    }
    Ok(())
}

fn run_launcher_check(launcher_path: &Path, manifest_path: &Path) -> Result<(), String> {
    let output = Command::new(launcher_path)
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--check")
        .env_remove("DAILYOS_DB_MODE")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("launcher_check_spawn_failed: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("launcher_check_failed: {}", output.status))
    }
}

fn manifest_arg(server: &Value) -> Option<PathBuf> {
    let args = server.get("args")?.as_array()?;
    let mut iter = args.iter().filter_map(Value::as_str);
    while let Some(arg) = iter.next() {
        if arg == "--manifest" {
            return iter.next().map(PathBuf::from);
        }
    }
    None
}

fn is_legacy_raw_command(path: &Path) -> bool {
    contains_raw_build_path(path)
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

fn component_name(component: Component<'_>) -> Option<&str> {
    match component {
        Component::Normal(value) => value.to_str(),
        _ => None,
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }
    match (left.canonicalize(), right.canonicalize()) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let content =
        std::fs::read_to_string(path).map_err(|error| format!("Could not read JSON: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("JSON is invalid: {error}"))
}

fn write_json_file<T>(path: &Path, value: &T) -> io::Result<()>
where
    T: serde::Serialize,
{
    let content = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    std::fs::write(path, content)
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

fn ensure_executable(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = std::fs::metadata(path)?;
        let mut permissions = metadata.permissions();
        let mode = permissions.mode();
        if mode & 0o111 == 0 {
            permissions.set_mode(mode | 0o755);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn test_paths(temp: &tempfile::TempDir) -> IntegrationPaths {
        IntegrationPaths {
            home: temp.path().join("home"),
            current_exe: temp.path().join("DailyOS.app/Contents/MacOS/dailyos"),
            current_dir: temp.path().to_path_buf(),
            run_launcher_check: false,
        }
    }

    fn make_executable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(path, permissions).expect("chmod");
        }
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        let mut file = File::create(path).expect("create file");
        file.write_all(bytes).expect("write file");
        make_executable(path);
    }

    fn write_provenance(
        dir: &Path,
        server_filename: &str,
        server_sha: &str,
        launcher_filename: &str,
        launcher_sha: &str,
        stub: bool,
    ) {
        std::fs::create_dir_all(dir).expect("create provenance dir");
        let provenance = McpBundleProvenance {
            schema_version: MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION,
            guard_epoch: MCP_RUNTIME_GUARD_EPOCH.to_string(),
            target_triple: "test-target".to_string(),
            app_build_sha: env!("BUILD_GIT_SHA").to_string(),
            generated_at: "2026-06-02T00:00:00Z".to_string(),
            stub,
            sidecars: vec![
                McpSidecarProvenance {
                    name: MCP_SERVER_NAME.to_string(),
                    filename: server_filename.to_string(),
                    build_sha: env!("BUILD_GIT_SHA").to_string(),
                    sha256: server_sha.to_string(),
                    stub,
                },
                McpSidecarProvenance {
                    name: MCP_LAUNCHER_NAME.to_string(),
                    filename: launcher_filename.to_string(),
                    build_sha: env!("BUILD_GIT_SHA").to_string(),
                    sha256: launcher_sha.to_string(),
                    stub,
                },
            ],
        };
        write_json_file(
            &dir.join("dailyos-mcp-bundle-test-target.provenance.json"),
            &provenance,
        )
        .expect("write provenance");
    }

    fn setup_dev_runtime(temp: &tempfile::TempDir, stub: bool) -> IntegrationPaths {
        let paths = test_paths(temp);
        let binary_dir = temp.path().join("src-tauri").join("binaries");
        let server = binary_dir.join("dailyos-mcp-test-target");
        let launcher = binary_dir.join("dailyos-mcp-launcher-test-target");
        write_file(&server, b"server");
        write_file(&launcher, b"launcher");
        write_provenance(
            &binary_dir,
            "dailyos-mcp-test-target",
            &sha256_file(&server).expect("server hash"),
            "dailyos-mcp-launcher-test-target",
            &sha256_file(&launcher).expect("launcher hash"),
            stub,
        );
        paths
    }

    fn setup_packaged_runtime(temp: &tempfile::TempDir) -> IntegrationPaths {
        let paths = test_paths(temp);
        let macos_dir = temp.path().join("DailyOS.app/Contents/MacOS");
        let resource_binary_dir = temp.path().join("DailyOS.app/Contents/Resources/binaries");
        let server = macos_dir.join(MCP_SERVER_NAME);
        let launcher = macos_dir.join(MCP_LAUNCHER_NAME);
        write_file(&server, b"packaged server");
        write_file(&launcher, b"packaged launcher");
        write_provenance(
            &resource_binary_dir,
            "dailyos-mcp-test-target",
            &sha256_file(&server).expect("server hash"),
            "dailyos-mcp-launcher-test-target",
            &sha256_file(&launcher).expect("launcher hash"),
            false,
        );
        paths
    }

    #[test]
    fn status_marks_legacy_target_command_unsafe() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = test_paths(&temp);
        let config_path = paths.claude_config_path();
        std::fs::create_dir_all(config_path.parent().expect("parent")).expect("create parent");
        write_json_file(
            &config_path,
            &serde_json::json!({
                "mcpServers": {
                    "dailyos": {
                        "command": temp.path().join("src-tauri/target/debug/dailyos-mcp"),
                        "args": [],
                        "env": {}
                    }
                }
            }),
        )
        .expect("write config");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status.message.contains("Unsafe legacy"));
    }

    #[test]
    fn configure_rewrites_to_managed_launcher_and_manifest() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);

        let result = configure_claude_desktop_with_paths(&paths).expect("configure");

        assert!(result.success);
        assert!(paths.managed_launcher_path().exists());
        assert!(paths.managed_manifest_path().exists());
        let config: Value = read_json_file(&paths.claude_config_path()).expect("config");
        let server = &config["mcpServers"]["dailyos"];
        let expected_launcher = paths.managed_launcher_path().to_string_lossy().to_string();
        assert_eq!(server["command"].as_str(), Some(expected_launcher.as_str()));
        assert_eq!(manifest_arg(server), Some(paths.managed_manifest_path()));
    }

    #[test]
    fn configure_resolves_packaged_resources_binaries_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_packaged_runtime(&temp);

        configure_claude_desktop_with_paths(&paths).expect("configure");

        let manifest: McpLauncherManifest =
            read_json_file(&paths.managed_manifest_path()).expect("manifest");
        assert_eq!(manifest.source_kind, McpRuntimeSourceKind::AppBundle);
        assert_eq!(manifest.final_server_db_mode, "live");
        assert!(manifest.bundle_provenance_path.ends_with(
            "Contents/Resources/binaries/dailyos-mcp-bundle-test-target.provenance.json"
        ));
    }

    #[test]
    fn configure_rejects_stub_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, true);

        let error = configure_claude_desktop_with_paths(&paths).expect_err("stub rejected");

        assert!(error.contains("provenance_is_stub"));
    }

    #[test]
    fn status_accepts_valid_managed_launcher_manifest() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(status.success, "{}", status.message);
        assert_eq!(status.message, "Connected");
    }

    #[test]
    fn status_rejects_manifest_not_backed_by_bundled_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let mut manifest: McpLauncherManifest =
            read_json_file(&paths.managed_manifest_path()).expect("manifest");
        manifest.expected_sidecar_sha256 = "0".repeat(64);
        write_json_file(&paths.managed_manifest_path(), &manifest).expect("write manifest");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status.message.contains("sidecar provenance mismatch"));
    }
}
