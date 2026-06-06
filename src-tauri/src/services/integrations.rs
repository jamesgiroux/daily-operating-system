// Integrations service - business logic for Claude Desktop MCP configuration.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(all(target_os = "macos", unix))]
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::mcp_launcher_contract::{
    McpBundleProvenance, McpLauncherManifest, McpRuntimeSourceKind, McpSidecarProvenance,
};
use crate::mcp_runtime_guard_constants::{
    DAILYOS_APP_BUNDLE_IDENTIFIER, MCP_BUNDLE_PROVENANCE_SCHEMA_VERSION,
    MCP_LAUNCHER_MANIFEST_SCHEMA_VERSION, MCP_LAUNCHER_NAME, MCP_NO_ENV_DEFAULT_DB_MODE,
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
const CONFIGURE_LOCK_FILENAME: &str = ".configure.lock";
const MANIFEST_FILENAME: &str = "dailyos-mcp-manifest.json";
const RUNTIME_DIR_PREFIX: &str = "runtime-";
const MCP_CONFIG_KEY: &str = "dailyos";
const CONFIGURE_LOCK_TIMEOUT: Duration = Duration::from_secs(15);
const CONFIGURE_LOCK_RETRY_INTERVAL: Duration = Duration::from_millis(25);
const RUNTIME_GENERATION_CREATE_ATTEMPTS: usize = 100;
const LAUNCHER_CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const LAUNCHER_CHECK_OUTPUT_LIMIT: u64 = 64 * 1024;
#[cfg(target_os = "macos")]
const APP_BUNDLE_SIGNATURE_TIMEOUT: Duration = Duration::from_secs(5);
static UNIQUE_PATH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    launcher_check_timeout: Duration,
    verify_app_bundle_signature: bool,
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
            launcher_check_timeout: LAUNCHER_CHECK_TIMEOUT,
            verify_app_bundle_signature: true,
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

    fn configure_lock_path(&self) -> PathBuf {
        self.app_mcp_dir().join(CONFIGURE_LOCK_FILENAME)
    }
}

#[derive(Debug, Clone)]
struct ResolvedMcpRuntime {
    source_kind: McpRuntimeSourceKind,
    provenance_path: PathBuf,
    launcher_source_path: PathBuf,
    sidecar_path: PathBuf,
    expected_launcher_sha256: String,
    expected_sidecar_sha256: String,
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

    if !server_env_is_empty(server) {
        return result(
            false,
            "Unsafe Claude Desktop config: DailyOS MCP env overrides are not allowed",
            Some(config_path),
            Some(command),
        );
    }

    let manifest_path =
        match manifest_arg(server) {
            Some(path) => path,
            None => return result(
                false,
                "Unsafe Claude Desktop config: missing or unexpected launcher manifest arguments",
                Some(config_path),
                Some(command),
            ),
        };

    if let Err(message) = validate_managed_runtime_config(paths, &command, &manifest_path) {
        return result(false, &message, Some(config_path), Some(command));
    }

    match validate_existing_launcher(paths, &command, &manifest_path) {
        Ok(()) => result(true, "Connected", Some(config_path), Some(command)),
        Err(message) => result(false, &message, Some(config_path), Some(command)),
    }
}

fn validate_managed_runtime_config(
    paths: &IntegrationPaths,
    launcher_path: &Path,
    manifest_path: &Path,
) -> Result<(), String> {
    if launcher_path.file_name().and_then(|name| name.to_str()) != Some(MCP_LAUNCHER_NAME) {
        return Err(
            "Unsafe Claude Desktop config: command is not the DailyOS managed launcher".to_string(),
        );
    }
    if manifest_path.file_name().and_then(|name| name.to_str()) != Some(MANIFEST_FILENAME) {
        return Err("Unsafe Claude Desktop config: manifest is not DailyOS managed".to_string());
    }

    let launcher_dir = launcher_path.parent().ok_or_else(|| {
        "Unsafe Claude Desktop config: command is not the DailyOS managed launcher".to_string()
    })?;
    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        "Unsafe Claude Desktop config: manifest is not DailyOS managed".to_string()
    })?;
    if !paths_equal(launcher_dir, manifest_dir) {
        return Err(
            "Unsafe Claude Desktop config: launcher and manifest generations differ".to_string(),
        );
    }
    if !is_dailyos_managed_runtime_dir(paths, launcher_dir) {
        return Err("Unsafe Claude Desktop config: runtime is not DailyOS managed".to_string());
    }
    validate_private_managed_runtime_dirs(paths, launcher_dir)?;
    Ok(())
}

fn is_dailyos_managed_runtime_dir(paths: &IntegrationPaths, runtime_dir: &Path) -> bool {
    if paths_equal(runtime_dir, &paths.app_mcp_dir()) {
        return true;
    }
    let Some(parent) = runtime_dir.parent() else {
        return false;
    };
    let Some(name) = runtime_dir.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    name.starts_with(RUNTIME_DIR_PREFIX) && paths_equal(parent, &paths.app_mcp_dir())
}

/// Configure Claude Desktop to use the guarded DailyOS MCP launcher.
pub fn configure_claude_desktop(
    ctx: &crate::services::context::ServiceContext<'_>,
) -> Result<ClaudeDesktopConfigResult, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let paths = IntegrationPaths::from_environment()?;
    configure_claude_desktop_with_paths(&paths)
}

/// Refresh an existing DailyOS-managed Claude Desktop MCP registration.
///
/// This is intentionally narrower than the user-initiated configure command:
/// startup may repair a prior DailyOS-managed launcher after an app update, but
/// it must not add DailyOS to Claude Desktop or rewrite user-owned entries.
pub fn refresh_existing_claude_desktop_configuration(
    ctx: &crate::services::context::ServiceContext<'_>,
) -> Result<Option<ClaudeDesktopConfigResult>, String> {
    ctx.check_mutation_allowed().map_err(|e| e.to_string())?;
    let paths = IntegrationPaths::from_environment()?;
    refresh_existing_claude_desktop_configuration_with_paths(&paths)
}

fn refresh_existing_claude_desktop_configuration_with_paths(
    paths: &IntegrationPaths,
) -> Result<Option<ClaudeDesktopConfigResult>, String> {
    let config_path = paths.claude_config_path();
    let config = read_claude_config_or_empty_if_missing(&config_path)
        .map_err(|error| format!("Failed to read Claude Desktop config: {error}"))?;
    if existing_dailyos_managed_config(paths, &config)?.is_none() {
        return Ok(None);
    }

    configure_claude_desktop_with_paths_internal(paths, ConfigureMode::RefreshExistingManaged)
}

struct ExistingManagedConfig {
    command: PathBuf,
    manifest_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigureMode {
    UserInitiated,
    RefreshExistingManaged,
}

fn existing_dailyos_managed_config(
    paths: &IntegrationPaths,
    config: &Value,
) -> Result<Option<ExistingManagedConfig>, String> {
    let Some(servers) = config.get("mcpServers") else {
        return Ok(None);
    };
    let servers = servers.as_object().ok_or_else(|| {
        "Failed to read Claude Desktop config: mcpServers is not an object".to_string()
    })?;
    let Some(server) = servers.get(MCP_CONFIG_KEY) else {
        return Ok(None);
    };
    if !server_env_is_empty(server) {
        return Ok(None);
    }
    let Some(command) = server
        .get("command")
        .and_then(Value::as_str)
        .map(PathBuf::from)
    else {
        return Ok(None);
    };
    let Some(manifest_path) = manifest_arg(server) else {
        return Ok(None);
    };

    if !is_managed_runtime_config_shape(paths, &command, &manifest_path) {
        return Ok(None);
    }
    validate_managed_runtime_config(paths, &command, &manifest_path)?;
    Ok(Some(ExistingManagedConfig {
        command,
        manifest_path,
    }))
}

fn is_managed_runtime_config_shape(
    paths: &IntegrationPaths,
    launcher_path: &Path,
    manifest_path: &Path,
) -> bool {
    if launcher_path.file_name().and_then(|name| name.to_str()) != Some(MCP_LAUNCHER_NAME) {
        return false;
    }
    if manifest_path.file_name().and_then(|name| name.to_str()) != Some(MANIFEST_FILENAME) {
        return false;
    }
    let Some(launcher_dir) = launcher_path.parent() else {
        return false;
    };
    let Some(manifest_dir) = manifest_path.parent() else {
        return false;
    };
    paths_equal(launcher_dir, manifest_dir) && is_dailyos_managed_runtime_dir(paths, launcher_dir)
}

fn configure_claude_desktop_with_paths(
    paths: &IntegrationPaths,
) -> Result<ClaudeDesktopConfigResult, String> {
    configure_claude_desktop_with_paths_internal(paths, ConfigureMode::UserInitiated)?
        .ok_or_else(|| "Failed to configure Claude Desktop: no configuration written".to_string())
}

fn configure_claude_desktop_with_paths_internal(
    paths: &IntegrationPaths,
    mode: ConfigureMode,
) -> Result<Option<ClaudeDesktopConfigResult>, String> {
    create_dailyos_mcp_dir_synced(paths)
        .map_err(|error| format!("Failed to create MCP runtime directory: {error}"))?;
    let _runtime_guard = acquire_configure_file_lock(paths)?;
    let runtime = resolve_mcp_runtime(paths)?;
    let config_path = paths.claude_config_path();
    let config = read_claude_config_or_empty_if_missing(&config_path)
        .map_err(|error| format!("Failed to read Claude Desktop config: {error}"))?;
    if config
        .get("mcpServers")
        .is_some_and(|servers| !servers.is_object())
    {
        return Err(
            "Failed to read Claude Desktop config: mcpServers is not an object".to_string(),
        );
    }
    if mode == ConfigureMode::RefreshExistingManaged {
        let Some(existing_config) = existing_dailyos_managed_config(paths, &config)? else {
            return Ok(None);
        };
        if validate_existing_launcher(
            paths,
            &existing_config.command,
            &existing_config.manifest_path,
        )
        .is_ok()
        {
            return Ok(None);
        }
        configure_claude_desktop_with_resolved_runtime(
            paths,
            runtime,
            config_path,
            mode,
            Some(existing_config),
        )
    } else {
        configure_claude_desktop_with_resolved_runtime(paths, runtime, config_path, mode, None)
    }
}

fn configure_claude_desktop_with_resolved_runtime(
    paths: &IntegrationPaths,
    runtime: ResolvedMcpRuntime,
    config_path: PathBuf,
    mode: ConfigureMode,
    expected_refresh_config: Option<ExistingManagedConfig>,
) -> Result<Option<ClaudeDesktopConfigResult>, String> {
    let runtime_dir = create_runtime_generation_dir(paths)?;
    let configure_result = (|| -> Result<ClaudeDesktopConfigResult, (String, bool)> {
        let managed_launcher = runtime_dir.join(MCP_LAUNCHER_NAME);
        let manifest_path = runtime_dir.join(MANIFEST_FILENAME);

        std::fs::copy(&runtime.launcher_source_path, &managed_launcher)
            .map_err(|error| (format!("Failed to install MCP launcher: {error}"), true))?;
        ensure_executable(&managed_launcher).map_err(|error| {
            (
                format!("Failed to make MCP launcher executable: {error}"),
                true,
            )
        })?;
        sync_file_and_parent(&managed_launcher).map_err(|error| {
            (
                format!("Failed to sync MCP launcher generation: {error}"),
                true,
            )
        })?;

        let runtime = install_runtime_generation_assets(runtime, &runtime_dir)
            .map_err(|error| (error, true))?;
        let final_manifest_value = build_manifest(&runtime, &managed_launcher);
        write_json_file(&manifest_path, &final_manifest_value).map_err(|error| {
            (
                format!("Failed to write MCP launcher manifest: {error}"),
                true,
            )
        })?;
        validate_existing_launcher(paths, &managed_launcher, &manifest_path)
            .map_err(|error| (error, true))?;

        run_before_config_publish_hook(&config_path);
        let mut publish_config =
            read_claude_config_or_empty_if_missing(&config_path).map_err(|error| {
                (
                    format!("Failed to read Claude Desktop config: {error}"),
                    true,
                )
            })?;
        if publish_config
            .get("mcpServers")
            .is_some_and(|servers| !servers.is_object())
        {
            return Err((
                "Failed to read Claude Desktop config: mcpServers is not an object".to_string(),
                true,
            ));
        }
        if mode == ConfigureMode::RefreshExistingManaged {
            let expected_config = expected_refresh_config.as_ref().ok_or_else(|| {
                (
                    "Claude Desktop config changed during startup refresh".to_string(),
                    true,
                )
            })?;
            let current_config = existing_dailyos_managed_config(paths, &publish_config)
                .map_err(|error| (error, true))?
                .ok_or_else(|| {
                    (
                        "Claude Desktop config changed during startup refresh".to_string(),
                        true,
                    )
                })?;
            if !managed_configs_match(expected_config, &current_config) {
                return Err((
                    "Claude Desktop config changed during startup refresh".to_string(),
                    true,
                ));
            }
        }
        if !publish_config
            .get("mcpServers")
            .is_some_and(Value::is_object)
        {
            publish_config["mcpServers"] = serde_json::json!({});
        }
        publish_config["mcpServers"][MCP_CONFIG_KEY] = serde_json::json!({
            "command": managed_launcher.to_string_lossy(),
            "args": ["--manifest", manifest_path.to_string_lossy()],
            "env": {}
        });

        if let Some(parent) = config_path.parent() {
            create_dir_all_synced(parent)
                .map_err(|error| (format!("Failed to create config directory: {error}"), true))?;
        }
        write_json_file_with_publish_state(&config_path, &publish_config).map_err(|failure| {
            (
                format!("Failed to write Claude Desktop config: {}", failure.error),
                !failure.published,
            )
        })?;

        Ok(result(
            true,
            "Claude Desktop configured. Restart Claude Desktop to connect.",
            Some(config_path),
            Some(managed_launcher),
        ))
    })();

    match configure_result {
        Ok(result) => Ok(Some(result)),
        Err((error, cleanup_runtime)) => {
            if cleanup_runtime {
                cleanup_runtime_generation(&runtime_dir);
            }
            Err(error)
        }
    }
}

fn managed_configs_match(left: &ExistingManagedConfig, right: &ExistingManagedConfig) -> bool {
    paths_equal(&left.command, &right.command)
        && paths_equal(&left.manifest_path, &right.manifest_path)
}

#[cfg(test)]
struct BeforeConfigPublishHook {
    config_path: PathBuf,
    hook: Box<dyn FnOnce() + Send + 'static>,
}

#[cfg(test)]
static BEFORE_CONFIG_PUBLISH_HOOK: std::sync::Mutex<Option<BeforeConfigPublishHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
fn set_before_config_publish_hook(config_path: PathBuf, hook: Box<dyn FnOnce() + Send + 'static>) {
    let mut pending = BEFORE_CONFIG_PUBLISH_HOOK
        .lock()
        .expect("before config publish hook lock");
    assert!(
        pending.is_none(),
        "before config publish hook should not leak between tests"
    );
    *pending = Some(BeforeConfigPublishHook { config_path, hook });
}

#[cfg(test)]
fn run_before_config_publish_hook(config_path: &Path) {
    let pending = {
        let mut pending = BEFORE_CONFIG_PUBLISH_HOOK
            .lock()
            .expect("before config publish hook lock");
        if pending
            .as_ref()
            .is_some_and(|hook| paths_equal(&hook.config_path, config_path))
        {
            pending.take()
        } else {
            None
        }
    };
    if let Some(pending) = pending {
        (pending.hook)();
    }
}

#[cfg(not(test))]
fn run_before_config_publish_hook(_config_path: &Path) {}

fn create_runtime_generation_dir(paths: &IntegrationPaths) -> Result<PathBuf, String> {
    for _ in 0..RUNTIME_GENERATION_CREATE_ATTEMPTS {
        let runtime_dir = runtime_generation_dir(paths);
        match create_private_directory(&runtime_dir) {
            Ok(()) => {
                sync_directory(&paths.app_mcp_dir())
                    .map_err(|error| format!("Failed to sync MCP runtime directory: {error}"))?;
                return Ok(runtime_dir);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Failed to create MCP runtime generation: {error}")),
        }
    }
    Err("Failed to create unique MCP runtime generation".to_string())
}

fn install_runtime_generation_assets(
    mut runtime: ResolvedMcpRuntime,
    runtime_dir: &Path,
) -> Result<ResolvedMcpRuntime, String> {
    if runtime.source_kind == McpRuntimeSourceKind::RepoBinaries {
        let sidecar_path = runtime_dir.join(&runtime.sidecar.filename);
        std::fs::copy(&runtime.sidecar_path, &sidecar_path)
            .map_err(|error| format!("Failed to install MCP sidecar: {error}"))?;
        ensure_executable(&sidecar_path)
            .map_err(|error| format!("Failed to make MCP sidecar executable: {error}"))?;
        sync_file_and_parent(&sidecar_path)
            .map_err(|error| format!("Failed to sync MCP sidecar generation: {error}"))?;

        runtime.sidecar_path = sidecar_path;
    }
    Ok(runtime)
}

fn runtime_generation_dir(paths: &IntegrationPaths) -> PathBuf {
    let sequence = UNIQUE_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    paths.app_mcp_dir().join(format!(
        "{RUNTIME_DIR_PREFIX}{}-{timestamp_nanos}-{sequence}",
        std::process::id(),
    ))
}

fn unique_sibling_path(path: &Path, label: &str, kind: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(label);
    let sequence = UNIQUE_PATH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    path.with_file_name(format!(
        "{file_name}.{label}.{kind}-{}-{sequence}",
        std::process::id()
    ))
}

fn remove_file_if_exists(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn cleanup_runtime_generation(runtime_dir: &Path) {
    if let Err(_error) = std::fs::remove_dir_all(runtime_dir) {}
}

struct ConfigureFileLock {
    file: File,
}

fn acquire_configure_file_lock(paths: &IntegrationPaths) -> Result<ConfigureFileLock, String> {
    acquire_configure_file_lock_with_timeout(paths, CONFIGURE_LOCK_TIMEOUT)
}

#[cfg(test)]
fn try_acquire_configure_file_lock(paths: &IntegrationPaths) -> Result<ConfigureFileLock, String> {
    acquire_configure_file_lock_with_timeout(paths, Duration::ZERO)
}

fn acquire_configure_file_lock_with_timeout(
    paths: &IntegrationPaths,
    timeout: Duration,
) -> Result<ConfigureFileLock, String> {
    let lock_path = paths.configure_lock_path();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|error| format!("Failed to open MCP configuration lock: {error}"))?;
    let deadline = Instant::now() + timeout;
    loop {
        match try_lock_file_exclusive(&file) {
            Ok(()) => break,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    return Err(format!(
                        "Timed out waiting for MCP configuration transaction lock {}",
                        lock_path.display()
                    ));
                }
                thread::sleep(CONFIGURE_LOCK_RETRY_INTERVAL);
            }
            Err(error) => {
                return Err(format!(
                    "Failed to lock MCP configuration transaction {}: {error}",
                    lock_path.display()
                ));
            }
        }
    }
    Ok(ConfigureFileLock { file })
}

#[cfg(unix)]
fn try_lock_file_exclusive(file: &File) -> io::Result<()> {
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn try_lock_file_exclusive(_file: &File) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
impl Drop for ConfigureFileLock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.file.as_raw_fd(), libc::LOCK_UN);
        }
    }
}

#[cfg(not(unix))]
impl Drop for ConfigureFileLock {
    fn drop(&mut self) {}
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
    verify_app_bundle_signature: bool,
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
                        verify_app_bundle_signature: paths.verify_app_bundle_signature,
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
                    verify_app_bundle_signature: false,
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

    let launcher_source_path = candidate.binary_dir.join(runtime_filename(
        &launcher,
        candidate.packaged_filenames,
        &provenance.target_triple,
    )?);
    let sidecar_path = candidate.binary_dir.join(runtime_filename(
        &sidecar,
        candidate.packaged_filenames,
        &provenance.target_triple,
    )?);

    let (expected_launcher_sha256, expected_sidecar_sha256) = match candidate.source_kind {
        McpRuntimeSourceKind::RepoBinaries => {
            verify_runtime_file_repairing(
                &launcher_source_path,
                &launcher.sha256,
                MCP_LAUNCHER_NAME,
            )?;
            verify_runtime_file_repairing(&sidecar_path, &sidecar.sha256, MCP_SERVER_NAME)?;
            (launcher.sha256.clone(), sidecar.sha256.clone())
        }
        McpRuntimeSourceKind::AppBundle => {
            if candidate.verify_app_bundle_signature {
                verify_app_bundle_signature_for_macos_dir(&candidate.binary_dir)?;
            }
            (
                verified_runtime_file_hash(&launcher_source_path, MCP_LAUNCHER_NAME)?,
                verified_runtime_file_hash(&sidecar_path, MCP_SERVER_NAME)?,
            )
        }
    };

    Ok(ResolvedMcpRuntime {
        source_kind: candidate.source_kind,
        provenance_path: candidate.provenance_path,
        launcher_source_path,
        sidecar_path,
        expected_launcher_sha256,
        expected_sidecar_sha256,
        launcher,
        sidecar,
    })
}

fn runtime_filename(
    sidecar: &McpSidecarProvenance,
    packaged: bool,
    target_triple: &str,
) -> Result<String, String> {
    if packaged {
        return Ok(sidecar.name.clone());
    }

    let expected = format!("{}-{target_triple}", sidecar.name);
    if sidecar.filename != expected {
        return Err(format!("{}_filename_mismatch", sidecar.name));
    }
    let mut components = Path::new(&sidecar.filename).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(sidecar.filename.clone()),
        _ => Err(format!("{}_filename_invalid", sidecar.name)),
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
    if provenance.target_triple != env!("BUILD_TARGET_TRIPLE") {
        return Err("provenance_target_triple_mismatch".to_string());
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
    let actual = verified_runtime_file_hash(path, label)?;
    if actual != expected_sha256 {
        return Err(format!("{label}_hash_mismatch"));
    }
    Ok(())
}

fn verify_runtime_file_repairing(
    path: &Path,
    expected_sha256: &str,
    label: &str,
) -> Result<(), String> {
    ensure_executable(path).map_err(|error| format!("{label}_not_executable: {error}"))?;
    verify_runtime_file(path, expected_sha256, label)
}

fn verified_runtime_file_hash(path: &Path, label: &str) -> Result<String, String> {
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
    if !metadata_is_executable(&metadata) {
        return Err(format!("{label}_not_executable"));
    }
    sha256_file(path).map_err(|error| format!("{label}_hash_failed: {error}"))
}

#[cfg(unix)]
fn metadata_is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn metadata_is_executable(_metadata: &std::fs::Metadata) -> bool {
    true
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
        expected_launcher_sha256: runtime.expected_launcher_sha256.clone(),
        expected_sidecar_sha256: runtime.expected_sidecar_sha256.clone(),
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
    if manifest.final_server_db_mode != expected_db_mode_for_source_kind(&manifest.source_kind) {
        return Err("Unsafe Claude Desktop config: launcher manifest DB mode mismatch".to_string());
    }
    if !paths_equal(&manifest.launcher_path, launcher_path) {
        return Err("Unsafe Claude Desktop config: launcher manifest path mismatch".to_string());
    }
    let provenance = verify_manifest_provenance(&manifest)?;
    if manifest.source_kind == McpRuntimeSourceKind::AppBundle && paths.verify_app_bundle_signature
    {
        verify_app_bundle_signature_for_runtime_path(&manifest.sidecar_path)?;
    }
    let expected_launcher_sha256 = expected_managed_launcher_sha256(&manifest, &provenance)?;
    verify_runtime_file(launcher_path, &expected_launcher_sha256, MCP_LAUNCHER_NAME)?;
    verify_runtime_file(
        &manifest.sidecar_path,
        &manifest.expected_sidecar_sha256,
        MCP_SERVER_NAME,
    )?;

    if paths.run_launcher_check {
        run_launcher_check(launcher_path, manifest_path, paths.launcher_check_timeout)?;
    }

    Ok(())
}

struct VerifiedManifestProvenance {
    launcher: McpSidecarProvenance,
}

fn expected_managed_launcher_sha256(
    manifest: &McpLauncherManifest,
    provenance: &VerifiedManifestProvenance,
) -> Result<String, String> {
    match manifest.source_kind {
        McpRuntimeSourceKind::RepoBinaries => Ok(provenance.launcher.sha256.clone()),
        McpRuntimeSourceKind::AppBundle => {
            let launcher_source_path = manifest
                .sidecar_path
                .parent()
                .ok_or_else(|| {
                    "Unsafe Claude Desktop config: app bundle launcher source missing".to_string()
                })?
                .join(MCP_LAUNCHER_NAME);
            verified_runtime_file_hash(&launcher_source_path, MCP_LAUNCHER_NAME)
                .map_err(|error| format!("Unsafe Claude Desktop config: {error}"))
        }
    }
}

fn verify_manifest_provenance(
    manifest: &McpLauncherManifest,
) -> Result<VerifiedManifestProvenance, String> {
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

    if manifest.launcher_build_sha != launcher.build_sha {
        return Err("Unsafe Claude Desktop config: launcher provenance mismatch".to_string());
    }
    if manifest.sidecar_build_sha != sidecar.build_sha {
        return Err("Unsafe Claude Desktop config: sidecar provenance mismatch".to_string());
    }
    if manifest.source_kind == McpRuntimeSourceKind::RepoBinaries {
        if manifest.expected_launcher_sha256 != launcher.sha256 {
            return Err("Unsafe Claude Desktop config: launcher provenance mismatch".to_string());
        }
        if manifest.expected_sidecar_sha256 != sidecar.sha256 {
            return Err("Unsafe Claude Desktop config: sidecar provenance mismatch".to_string());
        }
    }
    match manifest.source_kind {
        McpRuntimeSourceKind::AppBundle => verify_manifest_app_bundle_roots(manifest)?,
        McpRuntimeSourceKind::RepoBinaries => verify_manifest_repo_binaries_siblings(manifest)?,
    }
    Ok(VerifiedManifestProvenance {
        launcher: launcher.clone(),
    })
}

fn verify_manifest_repo_binaries_siblings(manifest: &McpLauncherManifest) -> Result<(), String> {
    let launcher_path = manifest
        .launcher_path
        .canonicalize()
        .map_err(|error| format!("Unsafe Claude Desktop config: launcher path invalid: {error}"))?;
    let sidecar_path = manifest
        .sidecar_path
        .canonicalize()
        .map_err(|error| format!("Unsafe Claude Desktop config: sidecar path invalid: {error}"))?;
    let launcher_dir = launcher_path
        .parent()
        .ok_or_else(|| "Unsafe Claude Desktop config: launcher directory missing".to_string())?;
    if sidecar_path.parent() != Some(launcher_dir) {
        return Err(
            "Unsafe Claude Desktop config: repo-binaries sidecar is not in managed runtime"
                .to_string(),
        );
    }
    Ok(())
}

fn verify_manifest_app_bundle_roots(manifest: &McpLauncherManifest) -> Result<(), String> {
    let sidecar_path = manifest
        .sidecar_path
        .canonicalize()
        .map_err(|error| format!("Unsafe Claude Desktop config: sidecar path invalid: {error}"))?;
    let provenance_path = manifest
        .bundle_provenance_path
        .canonicalize()
        .map_err(|error| {
            format!("Unsafe Claude Desktop config: provenance path invalid: {error}")
        })?;
    let sidecar_root = app_bundle_root_for_macos_runtime_path(&sidecar_path)
        .ok_or_else(|| "Unsafe Claude Desktop config: sidecar is not in app bundle".to_string())?;
    let provenance_root = app_bundle_root_for_resource_path(&provenance_path).ok_or_else(|| {
        "Unsafe Claude Desktop config: provenance is not in app bundle".to_string()
    })?;
    if sidecar_root != provenance_root {
        return Err("Unsafe Claude Desktop config: app bundle root mismatch".to_string());
    }
    Ok(())
}

fn verify_app_bundle_signature_for_runtime_path(path: &Path) -> Result<(), String> {
    let macos_dir = path
        .parent()
        .ok_or_else(|| "app_bundle_signature_macos_dir_missing".to_string())?;
    verify_app_bundle_signature_for_macos_dir(macos_dir)
}

fn verify_app_bundle_signature_for_macos_dir(macos_dir: &Path) -> Result<(), String> {
    let app_bundle = app_bundle_root_for_macos_dir(macos_dir)
        .ok_or_else(|| "app_bundle_signature_root_missing".to_string())?;
    verify_app_bundle_signature(&app_bundle)
}

fn app_bundle_root_for_macos_dir(macos_dir: &Path) -> Option<PathBuf> {
    if macos_dir.file_name().and_then(|name| name.to_str()) != Some("MacOS") {
        return None;
    }
    let contents = macos_dir.parent()?;
    if contents.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return None;
    }
    contents.parent().map(Path::to_path_buf)
}

fn app_bundle_root_for_macos_runtime_path(path: &Path) -> Option<PathBuf> {
    app_bundle_root_for_macos_dir(path.parent()?)
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

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
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
    wait_child_with_timeout(
        &mut child,
        APP_BUNDLE_SIGNATURE_TIMEOUT,
        "app_bundle_signature_timeout",
    )
}

#[cfg(target_os = "macos")]
fn verify_app_bundle_signature_identity(
    app_bundle: &Path,
    expected_team_id: &str,
) -> Result<(), String> {
    let details = codesign_details(app_bundle)?;
    verify_codesign_identity_with_expected_team_id(&details, expected_team_id)
}

#[cfg(target_os = "macos")]
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
    let status = wait_child_with_timeout(
        &mut child,
        APP_BUNDLE_SIGNATURE_TIMEOUT,
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

#[cfg(target_os = "macos")]
fn verify_codesign_identity(details: &str) -> Result<(), String> {
    verify_codesign_identity_with_expected_team_id(details, expected_apple_team_id()?)
}

#[cfg(any(target_os = "macos", test))]
fn app_bundle_trusted_anchor_requirement(expected_team_id: &str) -> Result<String, String> {
    validate_apple_team_id(expected_team_id)?;
    Ok(format!(
        "anchor apple generic and identifier \"{DAILYOS_APP_BUNDLE_IDENTIFIER}\" and certificate leaf[subject.OU] = \"{expected_team_id}\""
    ))
}

#[cfg(any(target_os = "macos", test))]
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

#[cfg(any(target_os = "macos", test))]
fn verify_codesign_identity_with_expected_team_id(
    details: &str,
    expected_team_id: &str,
) -> Result<(), String> {
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

#[cfg(any(target_os = "macos", test))]
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

#[cfg(target_os = "macos")]
fn reject_unsafe_app_bundle_path(app_bundle: &Path) -> Result<(), String> {
    if path_contains_control_character(app_bundle) {
        return Err("app_bundle_signature_path_unsafe".to_string());
    }
    Ok(())
}

#[cfg(all(target_os = "macos", unix))]
fn path_contains_control_character(path: &Path) -> bool {
    path.as_os_str()
        .as_bytes()
        .iter()
        .any(|byte| byte.is_ascii_control())
}

#[cfg(all(target_os = "macos", not(unix)))]
fn path_contains_control_character(path: &Path) -> bool {
    path.as_os_str()
        .to_string_lossy()
        .chars()
        .any(char::is_control)
}

#[cfg(target_os = "macos")]
fn expected_apple_team_id() -> Result<&'static str, String> {
    let team_id = option_env!("DAILYOS_APPLE_TEAM_ID")
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "app_bundle_signature_team_id_missing".to_string())?;
    validate_apple_team_id(team_id)?;
    Ok(team_id)
}

#[cfg(not(target_os = "macos"))]
fn verify_app_bundle_signature(_app_bundle: &Path) -> Result<(), String> {
    Err("app_bundle_signature_verification_unavailable".to_string())
}

fn run_launcher_check(
    launcher_path: &Path,
    manifest_path: &Path,
    timeout: Duration,
) -> Result<(), String> {
    let mut command = Command::new(launcher_path);
    command
        .arg("--manifest")
        .arg(manifest_path)
        .arg("--check")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_own_process_group(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| format!("launcher_check_spawn_failed: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "launcher_check_stdout_unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "launcher_check_stderr_unavailable".to_string())?;
    let stdout_reader = read_limited(stdout, LAUNCHER_CHECK_OUTPUT_LIMIT);
    let stderr_reader = read_limited(stderr, LAUNCHER_CHECK_OUTPUT_LIMIT);
    let deadline = Instant::now() + timeout;
    let status = wait_child_with_deadline(&mut child, deadline, "launcher_check_timeout")?;
    let (stdout, stdout_truncated) = match recv_reader_until(stdout_reader, "stdout", deadline) {
        Ok(output) => output,
        Err(error) => {
            terminate_child_process_group(&mut child);
            return Err(error);
        }
    };
    let (stderr, stderr_truncated) = match recv_reader_until(stderr_reader, "stderr", deadline) {
        Ok(output) => output,
        Err(error) => {
            terminate_child_process_group(&mut child);
            return Err(error);
        }
    };
    if stdout_truncated {
        return Err("launcher_check_stdout_too_large".to_string());
    }
    if stderr_truncated {
        return Err("launcher_check_stderr_too_large".to_string());
    }
    if status.success() {
        validate_launcher_check_payload(&stdout, manifest_path)
    } else {
        let detail = launcher_check_stderr_detail(&stderr);
        if detail.is_empty() {
            Err(format!("launcher_check_failed: {status}"))
        } else {
            Err(format!("launcher_check_failed: {status}: {detail}"))
        }
    }
}

fn validate_launcher_check_payload(stdout: &[u8], manifest_path: &Path) -> Result<(), String> {
    let manifest: McpLauncherManifest = read_json_file(manifest_path)
        .map_err(|error| format!("launcher_check_manifest_invalid: {error}"))?;
    let payload: Value = serde_json::from_slice(stdout)
        .map_err(|error| format!("launcher_check_invalid_json: {error}"))?;
    if payload.get("status").and_then(Value::as_str) != Some("ok") {
        return Err("launcher_check_status_not_ok".to_string());
    }
    if payload.get("guardEpoch").and_then(Value::as_str) != Some(MCP_RUNTIME_GUARD_EPOCH) {
        return Err("launcher_check_guard_epoch_mismatch".to_string());
    }
    let db_mode = payload
        .get("finalServerDbMode")
        .and_then(Value::as_str)
        .ok_or_else(|| "launcher_check_db_mode_missing".to_string())?;
    if db_mode != manifest.final_server_db_mode {
        return Err("launcher_check_db_mode_mismatch".to_string());
    }
    Ok(())
}

fn wait_child_with_timeout(
    child: &mut Child,
    timeout: Duration,
    timeout_error: &str,
) -> Result<ExitStatus, String> {
    wait_child_with_deadline(child, Instant::now() + timeout, timeout_error)
}

fn wait_child_with_deadline(
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
            Err(error) => return Err(format!("launcher_check_wait_failed: {error}")),
        }
    }
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

fn recv_reader_until(
    reader: Receiver<io::Result<(Vec<u8>, bool)>>,
    label: &str,
    deadline: Instant,
) -> Result<(Vec<u8>, bool), String> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .ok_or_else(|| "launcher_check_timeout".to_string())?;
    match reader.recv_timeout(remaining) {
        Ok(result) => {
            result.map_err(|error| format!("launcher_check_{label}_read_failed: {error}"))
        }
        Err(RecvTimeoutError::Timeout) => Err("launcher_check_timeout".to_string()),
        Err(RecvTimeoutError::Disconnected) => {
            Err(format!("launcher_check_{label}_reader_disconnected"))
        }
    }
}

fn launcher_check_stderr_detail(stderr: &[u8]) -> String {
    const DETAIL_LIMIT: usize = 512;
    let detail = String::from_utf8_lossy(stderr)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" | ");
    if detail.chars().count() > DETAIL_LIMIT {
        detail.chars().take(DETAIL_LIMIT).collect()
    } else {
        detail
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

fn expected_db_mode_for_source_kind(source_kind: &McpRuntimeSourceKind) -> &'static str {
    match source_kind {
        McpRuntimeSourceKind::AppBundle => "live",
        McpRuntimeSourceKind::RepoBinaries => MCP_NO_ENV_DEFAULT_DB_MODE,
    }
}

fn manifest_arg(server: &Value) -> Option<PathBuf> {
    let args = server.get("args")?.as_array()?;
    if args.len() != 2 || args.first().and_then(Value::as_str) != Some("--manifest") {
        return None;
    }
    args.get(1).and_then(Value::as_str).map(PathBuf::from)
}

fn server_env_is_empty(server: &Value) -> bool {
    match server.get("env") {
        None => true,
        Some(env) => env.as_object().is_some_and(serde_json::Map::is_empty),
    }
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

fn read_claude_config_or_empty_if_missing(path: &Path) -> Result<Value, String> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let value: Value = serde_json::from_str(&content)
                .map_err(|error| format!("JSON is invalid: {error}"))?;
            if !value.is_object() {
                return Err("JSON top-level value is not an object".to_string());
            }
            Ok(value)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(error) => Err(format!("Could not read JSON: {error}")),
    }
}

fn write_json_file<T>(path: &Path, value: &T) -> io::Result<()>
where
    T: serde::Serialize,
{
    write_json_file_with_publish_state(path, value).map_err(|failure| failure.error)
}

struct JsonWriteFailure {
    error: io::Error,
    published: bool,
}

fn write_json_file_with_publish_state<T>(path: &Path, value: &T) -> Result<(), JsonWriteFailure>
where
    T: serde::Serialize,
{
    let content = serde_json::to_string_pretty(value).map_err(|error| JsonWriteFailure {
        error: io::Error::new(io::ErrorKind::InvalidData, error),
        published: false,
    })?;
    let temp_path = unique_sibling_path(path, "json", "tmp");
    let mut published = false;
    let write_result = (|| -> io::Result<()> {
        let mut file = create_json_replacement_temp_file(path, &temp_path)?;
        file.write_all(content.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp_path, path)?;
        published = true;
        maybe_fail_json_parent_sync_after_rename(path)?;
        if let Some(parent) = path.parent() {
            sync_directory(parent)?;
        }
        Ok(())
    })();
    if let Err(error) = write_result {
        if let Err(_error) = remove_file_if_exists(&temp_path) {}
        return Err(JsonWriteFailure { error, published });
    }
    Ok(())
}

#[cfg(test)]
type JsonParentSyncFailureTarget = Option<PathBuf>;

#[cfg(test)]
static JSON_PARENT_SYNC_FAILURE_TARGET: std::sync::Mutex<JsonParentSyncFailureTarget> =
    std::sync::Mutex::new(None);

#[cfg(test)]
fn fail_next_json_parent_sync_after_rename_for_path(path: &Path) {
    *JSON_PARENT_SYNC_FAILURE_TARGET
        .lock()
        .expect("json failure target lock") = Some(path.to_path_buf());
}

#[cfg(test)]
fn maybe_fail_json_parent_sync_after_rename(path: &Path) -> io::Result<()> {
    let mut target = JSON_PARENT_SYNC_FAILURE_TARGET
        .lock()
        .expect("json failure target lock");
    if target
        .as_ref()
        .is_some_and(|target_path| paths_equal(target_path, path))
    {
        *target = None;
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "injected parent sync failure after rename",
        ));
    }
    Ok(())
}

#[cfg(not(test))]
fn maybe_fail_json_parent_sync_after_rename(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn create_json_replacement_temp_file(path: &Path, temp_path: &Path) -> io::Result<File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    let mode = match std::fs::metadata(path) {
        Ok(metadata) => metadata.permissions().mode() & 0o777,
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0o600,
        Err(error) => return Err(error),
    };
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(temp_path)
}

#[cfg(not(unix))]
fn create_json_replacement_temp_file(_path: &Path, temp_path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
}

fn create_dailyos_mcp_dir_synced(paths: &IntegrationPaths) -> io::Result<()> {
    std::fs::create_dir_all(&paths.home)?;
    create_private_directory(&paths.home.join(".dailyos"))?; // dailyos-path-allowed: Claude Desktop MCP launcher config is shared app-managed runtime state, not DB-mode-scoped data.
    create_private_directory(&paths.app_mcp_dir())?;
    sync_directory_tree(&paths.app_mcp_dir())
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    match std::fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => {
            let mut permissions = std::fs::metadata(path)?.permissions();
            permissions.set_mode(0o700);
            std::fs::set_permissions(path, permissions)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            validate_private_directory(path)
        }
        Err(error) => Err(error),
    }
}

#[cfg(not(unix))]
fn create_private_directory(path: &Path) -> io::Result<()> {
    match std::fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn validate_private_managed_runtime_dirs(
    paths: &IntegrationPaths,
    runtime_dir: &Path,
) -> Result<(), String> {
    for dir in [
        paths.home.join(".dailyos"), // dailyos-path-allowed: validates shared app-managed MCP runtime parent permissions before trusting Claude config.
        paths.app_mcp_dir(),
        runtime_dir.to_path_buf(),
    ] {
        validate_private_directory(&dir)
            .map_err(|error| format!("Unsafe Claude Desktop config: managed runtime directory permissions are unsafe: {error}"))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn validate_private_managed_runtime_dirs(
    _paths: &IntegrationPaths,
    _runtime_dir: &Path,
) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn validate_private_directory(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = std::fs::metadata(path)?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{} is not a directory", path.display()),
        ));
    }
    let mode = metadata.permissions().mode();
    if mode & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} has mode {:o}", path.display(), mode & 0o777),
        ));
    }
    Ok(())
}

fn create_dir_all_synced(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)?;
    sync_directory_tree(path)
}

fn sync_file_and_parent(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()?;
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

fn sync_directory_tree(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        for ancestor in path.ancestors() {
            sync_directory(ancestor)?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
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
    use std::sync::{Arc, Barrier};

    fn test_paths(temp: &tempfile::TempDir) -> IntegrationPaths {
        IntegrationPaths {
            home: temp.path().join("home"),
            current_exe: temp.path().join("DailyOS.app/Contents/MacOS/dailyos"),
            current_dir: temp.path().to_path_buf(),
            run_launcher_check: false,
            launcher_check_timeout: LAUNCHER_CHECK_TIMEOUT,
            verify_app_bundle_signature: false,
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

    #[cfg(unix)]
    fn clear_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(permissions.mode() & !0o111);
        std::fs::set_permissions(path, permissions).expect("chmod");
    }

    #[cfg(unix)]
    fn path_is_executable(path: &Path) -> bool {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o111
            != 0
    }

    #[cfg(unix)]
    fn file_mode(path: &Path) -> u32 {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777
    }

    fn write_file(path: &Path, bytes: &[u8]) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        let mut file = File::create(path).expect("create file");
        file.write_all(bytes).expect("write file");
        make_executable(path);
    }

    fn test_server_filename() -> String {
        format!("{}-{}", MCP_SERVER_NAME, env!("BUILD_TARGET_TRIPLE"))
    }

    fn test_launcher_filename() -> String {
        format!("{}-{}", MCP_LAUNCHER_NAME, env!("BUILD_TARGET_TRIPLE"))
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
            target_triple: env!("BUILD_TARGET_TRIPLE").to_string(),
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
        let server = binary_dir.join(test_server_filename());
        let launcher = binary_dir.join(test_launcher_filename());
        write_file(&server, b"server");
        write_file(&launcher, b"#!/bin/sh\nexit 0\n");
        write_provenance(
            &binary_dir,
            &test_server_filename(),
            &sha256_file(&server).expect("server hash"),
            &test_launcher_filename(),
            &sha256_file(&launcher).expect("launcher hash"),
            stub,
        );
        paths
    }

    fn write_dailyos_config(paths: &IntegrationPaths, command: &Path, args: Vec<Value>) {
        let config_path = paths.claude_config_path();
        std::fs::create_dir_all(config_path.parent().expect("parent")).expect("create parent");
        write_json_file(
            &config_path,
            &serde_json::json!({
                "mcpServers": {
                    "dailyos": {
                        "command": command,
                        "args": args,
                        "env": {}
                    }
                }
            }),
        )
        .expect("write config");
    }

    fn active_dailyos_server(paths: &IntegrationPaths) -> Value {
        let config: Value = read_json_file(&paths.claude_config_path()).expect("config");
        config["mcpServers"]["dailyos"].clone()
    }

    fn active_launcher_path(paths: &IntegrationPaths) -> PathBuf {
        active_dailyos_server(paths)["command"]
            .as_str()
            .map(PathBuf::from)
            .expect("active launcher")
    }

    fn active_manifest_path(paths: &IntegrationPaths) -> PathBuf {
        manifest_arg(&active_dailyos_server(paths)).expect("active manifest")
    }

    fn active_manifest(paths: &IntegrationPaths) -> McpLauncherManifest {
        read_json_file(&active_manifest_path(paths)).expect("manifest")
    }

    fn active_sidecar_path(paths: &IntegrationPaths) -> PathBuf {
        active_manifest(paths).sidecar_path
    }

    fn runtime_generation_count(paths: &IntegrationPaths) -> usize {
        match std::fs::read_dir(paths.app_mcp_dir()) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.starts_with(RUNTIME_DIR_PREFIX))
                })
                .count(),
            Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
            Err(error) => panic!("read runtime dir: {error}"),
        }
    }

    fn runtime_binary_dir(paths: &IntegrationPaths) -> PathBuf {
        paths.current_dir.join("src-tauri").join("binaries")
    }

    fn runtime_server_path(paths: &IntegrationPaths) -> PathBuf {
        runtime_binary_dir(paths).join(test_server_filename())
    }

    fn runtime_launcher_path(paths: &IntegrationPaths) -> PathBuf {
        runtime_binary_dir(paths).join(test_launcher_filename())
    }

    fn refresh_runtime_provenance(paths: &IntegrationPaths) {
        let binary_dir = runtime_binary_dir(paths);
        let server = runtime_server_path(paths);
        let launcher = runtime_launcher_path(paths);
        write_provenance(
            &binary_dir,
            &test_server_filename(),
            &sha256_file(&server).expect("server hash"),
            &test_launcher_filename(),
            &sha256_file(&launcher).expect("launcher hash"),
            false,
        );
    }

    fn rewrite_manifest<F>(paths: &IntegrationPaths, mutate: F)
    where
        F: FnOnce(&mut McpLauncherManifest),
    {
        let manifest_path = active_manifest_path(paths);
        let mut manifest: McpLauncherManifest = read_json_file(&manifest_path).expect("manifest");
        mutate(&mut manifest);
        write_json_file(&manifest_path, &manifest).expect("write manifest");
    }

    fn rewrite_test_provenance_file<F>(path: &Path, mutate: F)
    where
        F: FnOnce(&mut McpBundleProvenance),
    {
        let mut provenance: McpBundleProvenance = read_json_file(path).expect("provenance");
        mutate(&mut provenance);
        write_json_file(path, &provenance).expect("write provenance");
    }

    #[cfg(unix)]
    #[test]
    fn write_json_preserves_existing_file_mode_and_defaults_new_files_private() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("claude_desktop_config.json");
        std::fs::write(
            &config_path,
            r#"{"mcpServers":{"other":{"env":{"TOKEN":"secret"}}}}"#,
        )
        .expect("write config");
        let mut permissions = std::fs::metadata(&config_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(&config_path, permissions).expect("chmod");

        write_json_file(
            &config_path,
            &serde_json::json!({
                "mcpServers": {
                    "other": {"env": {"TOKEN": "secret"}},
                    "dailyos": {}
                }
            }),
        )
        .expect("rewrite config");

        assert_eq!(file_mode(&config_path), 0o600);

        let new_path = temp.path().join("new-config.json");
        write_json_file(&new_path, &serde_json::json!({"mcpServers": {}}))
            .expect("write new config");

        assert_eq!(file_mode(&new_path), 0o600);
    }

    fn setup_packaged_runtime(temp: &tempfile::TempDir) -> IntegrationPaths {
        let paths = test_paths(temp);
        let macos_dir = temp.path().join("DailyOS.app/Contents/MacOS");
        let resource_binary_dir = temp.path().join("DailyOS.app/Contents/Resources/binaries");
        let server = macos_dir.join(MCP_SERVER_NAME);
        let launcher = macos_dir.join(MCP_LAUNCHER_NAME);
        write_file(&server, b"unsigned packaged server");
        write_file(&launcher, b"unsigned packaged launcher");
        write_provenance(
            &resource_binary_dir,
            "dailyos-mcp-test-target",
            &sha256_file(&server).expect("server hash"),
            "dailyos-mcp-launcher-test-target",
            &sha256_file(&launcher).expect("launcher hash"),
            false,
        );
        write_file(&server, b"signed packaged server");
        write_file(&launcher, b"signed packaged launcher");
        paths
    }

    #[test]
    fn status_marks_legacy_target_command_unsafe() {
        for raw_command in [
            "src-tauri/target/debug/dailyos-mcp",
            "src-tauri/target/release/dailyos-mcp",
            ".cargo/bin/dailyos-mcp",
        ] {
            let temp = tempfile::tempdir().expect("tempdir");
            let paths = test_paths(&temp);
            write_dailyos_config(&paths, &temp.path().join(raw_command), vec![]);

            let status = get_claude_desktop_status_with_paths(&paths);

            assert!(!status.success, "{raw_command}");
            assert!(status.message.contains("Unsafe legacy"), "{raw_command}");
        }
    }

    #[test]
    fn codesign_identity_rejects_duplicate_identifier_fields() {
        let details = format!(
            "Executable=/tmp/Fake\nIdentifier={DAILYOS_APP_BUNDLE_IDENTIFIER}\nTeamIdentifier=DAILYOSTEAM\nIdentifier=com.example.fake\n"
        );

        let error = verify_codesign_identity_with_expected_team_id(&details, "DAILYOSTEAM")
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
    fn configure_rewrites_to_managed_launcher_and_manifest() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        write_dailyos_config(
            &paths,
            &temp.path().join("src-tauri/target/debug/dailyos-mcp"),
            vec![],
        );

        let result = configure_claude_desktop_with_paths(&paths).expect("configure");

        assert!(result.success);
        let server = active_dailyos_server(&paths);
        let launcher = active_launcher_path(&paths);
        let manifest = active_manifest_path(&paths);
        assert!(launcher.exists());
        assert!(manifest.exists());
        assert_eq!(
            launcher.file_name().and_then(|name| name.to_str()),
            Some(MCP_LAUNCHER_NAME)
        );
        assert_eq!(
            manifest.file_name().and_then(|name| name.to_str()),
            Some(MANIFEST_FILENAME)
        );
        assert!(paths_equal(
            launcher.parent().expect("launcher parent"),
            manifest.parent().expect("manifest parent")
        ));
        assert!(is_dailyos_managed_runtime_dir(
            &paths,
            launcher.parent().expect("launcher parent")
        ));
        assert_eq!(manifest_arg(&server), Some(manifest));
    }

    #[test]
    fn configure_stages_repo_sidecar_inside_managed_generation_and_anchors_source_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);

        configure_claude_desktop_with_paths(&paths).expect("configure");

        let launcher = active_launcher_path(&paths);
        let manifest = active_manifest(&paths);
        let runtime_dir = launcher.parent().expect("runtime dir");
        assert_eq!(manifest.source_kind, McpRuntimeSourceKind::RepoBinaries);
        assert!(paths_equal(
            manifest.sidecar_path.parent().expect("sidecar parent"),
            runtime_dir
        ));
        assert!(paths_equal(
            manifest
                .bundle_provenance_path
                .parent()
                .expect("provenance parent"),
            &runtime_binary_dir(&paths)
        ));
        assert!(manifest.sidecar_path.is_file());
        assert!(manifest.bundle_provenance_path.is_file());
    }

    #[cfg(unix)]
    #[test]
    fn status_rejects_group_or_world_writable_managed_runtime_dir() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let runtime_dir = active_launcher_path(&paths)
            .parent()
            .expect("runtime dir")
            .to_path_buf();
        let mut permissions = std::fs::metadata(&runtime_dir)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o777);
        std::fs::set_permissions(&runtime_dir, permissions).expect("chmod");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status
            .message
            .contains("managed runtime directory permissions are unsafe"));
    }

    #[test]
    fn configure_rewrites_each_legacy_raw_command_class() {
        for raw_command in [
            "src-tauri/target/debug/dailyos-mcp",
            "src-tauri/target/release/dailyos-mcp",
            ".cargo/bin/dailyos-mcp",
        ] {
            let temp = tempfile::tempdir().expect("tempdir");
            let paths = setup_dev_runtime(&temp, false);
            write_dailyos_config(&paths, &temp.path().join(raw_command), vec![]);

            configure_claude_desktop_with_paths(&paths).expect("configure");

            let server = active_dailyos_server(&paths);
            let launcher = active_launcher_path(&paths);
            let manifest = active_manifest_path(&paths);
            assert_eq!(
                server["command"].as_str(),
                Some(launcher.to_string_lossy().as_ref()),
                "raw command {raw_command} should be rewritten"
            );
            assert_eq!(manifest_arg(&server), Some(manifest));
        }
    }

    #[test]
    fn startup_refresh_skips_missing_or_unmanaged_config() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);

        let missing = refresh_existing_claude_desktop_configuration_with_paths(&paths)
            .expect("missing config should not fail");

        assert!(missing.is_none());
        assert!(!paths.claude_config_path().exists());

        let unmanaged_command = temp.path().join("outside").join(MCP_LAUNCHER_NAME);
        write_dailyos_config(
            &paths,
            &unmanaged_command,
            vec![
                Value::String("--manifest".to_string()),
                Value::String(
                    temp.path()
                        .join("outside")
                        .join(MANIFEST_FILENAME)
                        .to_string_lossy()
                        .to_string(),
                ),
            ],
        );
        let original_config =
            std::fs::read_to_string(paths.claude_config_path()).expect("read config");

        let unmanaged = refresh_existing_claude_desktop_configuration_with_paths(&paths)
            .expect("unmanaged config should not fail");

        assert!(unmanaged.is_none());
        assert_eq!(
            std::fs::read_to_string(paths.claude_config_path()).expect("read config"),
            original_config
        );
    }

    #[test]
    fn startup_refresh_noops_when_existing_managed_config_is_current() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let original_config =
            std::fs::read_to_string(paths.claude_config_path()).expect("read config");
        let original_generation_count = runtime_generation_count(&paths);

        let refreshed = refresh_existing_claude_desktop_configuration_with_paths(&paths)
            .expect("startup refresh should not fail");

        assert!(refreshed.is_none());
        assert_eq!(runtime_generation_count(&paths), original_generation_count);
        assert_eq!(
            std::fs::read_to_string(paths.claude_config_path()).expect("read config"),
            original_config
        );
    }

    #[test]
    fn startup_refresh_preserves_managed_config_with_env_overrides() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let config_path = paths.claude_config_path();
        let mut config: Value = read_json_file(&config_path).expect("config");
        config["mcpServers"]["dailyos"]["env"] = serde_json::json!({
            "DAILYOS_MCP_LEGACY_V1": "1"
        });
        write_json_file(&config_path, &config).expect("write config");
        let original_config =
            std::fs::read_to_string(paths.claude_config_path()).expect("read config");
        let original_generation_count = runtime_generation_count(&paths);

        let refreshed = refresh_existing_claude_desktop_configuration_with_paths(&paths)
            .expect("startup refresh should not fail");

        assert!(refreshed.is_none());
        assert_eq!(runtime_generation_count(&paths), original_generation_count);
        assert_eq!(
            std::fs::read_to_string(paths.claude_config_path()).expect("read config"),
            original_config
        );
    }

    #[test]
    fn startup_refresh_rechecks_managed_shape_under_configure_lock() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        write_dailyos_config(
            &paths,
            &temp.path().join("outside").join(MCP_LAUNCHER_NAME),
            vec![
                Value::String("--manifest".to_string()),
                Value::String(
                    temp.path()
                        .join("outside")
                        .join(MANIFEST_FILENAME)
                        .to_string_lossy()
                        .to_string(),
                ),
            ],
        );
        let original_config =
            std::fs::read_to_string(paths.claude_config_path()).expect("read config");
        let original_generation_count = runtime_generation_count(&paths);

        let refreshed = configure_claude_desktop_with_paths_internal(
            &paths,
            ConfigureMode::RefreshExistingManaged,
        )
        .expect("refresh guard should not fail");

        assert!(refreshed.is_none());
        assert_eq!(runtime_generation_count(&paths), original_generation_count);
        assert_eq!(
            std::fs::read_to_string(paths.claude_config_path()).expect("read config"),
            original_config
        );
    }

    #[test]
    fn startup_refresh_updates_existing_managed_config_after_app_runtime_changes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_packaged_runtime(&temp);
        configure_claude_desktop_with_paths(&paths).expect("initial configure");
        let previous_launcher_path = active_launcher_path(&paths);
        let previous_manifest_path = active_manifest_path(&paths);
        let previous_manifest = active_manifest(&paths);

        let macos_dir = temp.path().join("DailyOS.app/Contents/MacOS");
        let resource_binary_dir = temp.path().join("DailyOS.app/Contents/Resources/binaries");
        let packaged_server = macos_dir.join(MCP_SERVER_NAME);
        let packaged_launcher = macos_dir.join(MCP_LAUNCHER_NAME);
        write_file(&packaged_server, b"signed packaged server generation two");
        write_file(
            &packaged_launcher,
            b"signed packaged launcher generation two",
        );
        write_provenance(
            &resource_binary_dir,
            "dailyos-mcp-test-target",
            &sha256_file(&packaged_server).expect("server hash"),
            "dailyos-mcp-launcher-test-target",
            &sha256_file(&packaged_launcher).expect("launcher hash"),
            false,
        );

        let result = refresh_existing_claude_desktop_configuration_with_paths(&paths)
            .expect("startup refresh")
            .expect("managed config should refresh");

        assert!(result.success);
        let current_launcher_path = active_launcher_path(&paths);
        let current_manifest_path = active_manifest_path(&paths);
        let current_manifest = active_manifest(&paths);
        assert_ne!(current_launcher_path, previous_launcher_path);
        assert_ne!(current_manifest_path, previous_manifest_path);
        assert_eq!(
            current_manifest.source_kind,
            McpRuntimeSourceKind::AppBundle
        );
        assert_eq!(
            std::fs::read(&current_launcher_path).expect("current launcher"),
            b"signed packaged launcher generation two"
        );
        assert_eq!(
            current_manifest.expected_sidecar_sha256,
            sha256_file(&current_manifest.sidecar_path).expect("current sidecar hash")
        );
        assert_ne!(
            current_manifest.expected_sidecar_sha256,
            previous_manifest.expected_sidecar_sha256
        );

        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(status.success, "{}", status.message);
    }

    #[cfg(unix)]
    #[test]
    fn failed_config_write_cleans_staged_runtime_generation() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        let config_dir = paths.claude_config_path().parent().unwrap().to_path_buf();
        std::fs::create_dir_all(&config_dir).expect("create config dir");
        let mut permissions = std::fs::metadata(&config_dir)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o500);
        std::fs::set_permissions(&config_dir, permissions).expect("chmod readonly");

        let error =
            configure_claude_desktop_with_paths(&paths).expect_err("config write should fail");

        let mut permissions = std::fs::metadata(&config_dir)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o700);
        std::fs::set_permissions(&config_dir, permissions).expect("restore chmod");

        assert!(
            error.contains("Failed to write Claude Desktop config"),
            "{error}"
        );
        assert_eq!(runtime_generation_count(&paths), 0);
    }

    #[test]
    fn config_publish_parent_sync_failure_keeps_published_runtime() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        fail_next_json_parent_sync_after_rename_for_path(&paths.claude_config_path());

        let error = configure_claude_desktop_with_paths(&paths)
            .expect_err("injected parent sync failure should surface");

        assert!(
            error.contains("Failed to write Claude Desktop config"),
            "{error}"
        );
        assert_eq!(runtime_generation_count(&paths), 1);
        assert!(active_launcher_path(&paths).is_file());
        assert!(active_manifest_path(&paths).is_file());
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(status.success, "{}", status.message);
    }

    #[test]
    fn startup_refresh_does_not_overwrite_config_changed_before_publish() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let original_generation_count = runtime_generation_count(&paths);

        write_file(
            &runtime_launcher_path(&paths),
            b"#!/bin/sh\nexit 0\n# generation two\n",
        );
        refresh_runtime_provenance(&paths);

        let hook_paths = paths.clone();
        let outside_dir = temp.path().join("outside-after-recheck");
        set_before_config_publish_hook(
            paths.claude_config_path(),
            Box::new(move || {
                write_dailyos_config(
                    &hook_paths,
                    &outside_dir.join(MCP_LAUNCHER_NAME),
                    vec![
                        Value::String("--manifest".to_string()),
                        Value::String(
                            outside_dir
                                .join(MANIFEST_FILENAME)
                                .to_string_lossy()
                                .to_string(),
                        ),
                    ],
                );
            }),
        );

        let error = refresh_existing_claude_desktop_configuration_with_paths(&paths)
            .expect_err("concurrent config edit should abort startup refresh");

        assert!(
            error.contains("Claude Desktop config changed during startup refresh"),
            "{error}"
        );
        assert_eq!(runtime_generation_count(&paths), original_generation_count);
        let server = active_dailyos_server(&paths);
        assert!(server["command"]
            .as_str()
            .is_some_and(|command| command.contains("outside-after-recheck")));
    }

    #[test]
    fn configure_rejects_malformed_existing_config_without_overwriting() {
        for bad_config in ["{not-json", "[]", "{\"mcpServers\":[] }"] {
            let temp = tempfile::tempdir().expect("tempdir");
            let paths = setup_dev_runtime(&temp, false);
            let config_path = paths.claude_config_path();
            std::fs::create_dir_all(config_path.parent().expect("config parent"))
                .expect("create config parent");
            std::fs::write(&config_path, bad_config).expect("write malformed config");

            let error = configure_claude_desktop_with_paths(&paths)
                .expect_err("malformed config should fail closed");

            assert!(error.contains("Failed to read Claude Desktop config"));
            assert_eq!(
                std::fs::read_to_string(&config_path).expect("read config"),
                bad_config
            );
            let runtime_generation_exists = std::fs::read_dir(paths.app_mcp_dir())
                .expect("read mcp dir")
                .filter_map(Result::ok)
                .any(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.starts_with(RUNTIME_DIR_PREFIX))
                });
            assert!(
                !runtime_generation_exists,
                "malformed config must fail before staging a runtime generation"
            );
        }
    }

    #[test]
    fn concurrent_configure_calls_leave_valid_managed_runtime_pair() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        let barrier = Arc::new(Barrier::new(3));

        let first_paths = paths.clone();
        let first_barrier = Arc::clone(&barrier);
        let first = std::thread::spawn(move || {
            first_barrier.wait();
            configure_claude_desktop_with_paths(&first_paths)
        });

        let second_paths = paths.clone();
        let second_barrier = Arc::clone(&barrier);
        let second = std::thread::spawn(move || {
            second_barrier.wait();
            configure_claude_desktop_with_paths(&second_paths)
        });

        barrier.wait();
        first
            .join()
            .expect("first thread")
            .expect("first configure");
        second
            .join()
            .expect("second thread")
            .expect("second configure");

        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(status.success, "{}", status.message);
        assert!(active_launcher_path(&paths).is_file());
        assert!(active_manifest_path(&paths).is_file());
    }

    #[test]
    fn configure_file_lock_blocks_second_holder() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = test_paths(&temp);
        std::fs::create_dir_all(paths.app_mcp_dir()).expect("create mcp dir");

        let first = acquire_configure_file_lock(&paths).expect("first lock");
        let second = try_acquire_configure_file_lock(&paths);
        assert!(second.is_err(), "second lock should be blocked");
        let error = second.err().expect("second error");
        assert!(
            error.contains("Timed out waiting for MCP configuration transaction lock"),
            "{error}"
        );

        drop(first);
        let _third = try_acquire_configure_file_lock(&paths).expect("lock released");
    }

    #[test]
    fn failed_reconfigure_preserves_existing_managed_runtime_pair() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("initial configure");
        let existing_launcher_path = active_launcher_path(&paths);
        let existing_manifest_path = active_manifest_path(&paths);
        let existing_config: Value = read_json_file(&paths.claude_config_path()).expect("config");
        let existing_launcher = std::fs::read(&existing_launcher_path).expect("read launcher");
        let existing_manifest =
            std::fs::read_to_string(&existing_manifest_path).expect("read manifest");

        write_file(&runtime_launcher_path(&paths), b"#!/bin/sh\nexit 42\n");
        refresh_runtime_provenance(&paths);
        let mut strict_paths = paths.clone();
        strict_paths.run_launcher_check = true;

        let error =
            configure_claude_desktop_with_paths(&strict_paths).expect_err("launcher check fails");

        assert!(error.contains("launcher_check_failed"));
        let config: Value = read_json_file(&paths.claude_config_path()).expect("config");
        assert_eq!(config, existing_config);
        assert_eq!(
            std::fs::read(existing_launcher_path).expect("read launcher"),
            existing_launcher
        );
        assert_eq!(
            std::fs::read_to_string(existing_manifest_path).expect("read manifest"),
            existing_manifest
        );
    }

    #[test]
    fn successful_reconfigure_activates_new_generation_without_overwriting_previous() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("initial configure");
        let previous_launcher_path = active_launcher_path(&paths);
        let previous_manifest_path = active_manifest_path(&paths);
        let previous_launcher =
            std::fs::read(&previous_launcher_path).expect("read previous launcher");
        let previous_manifest =
            std::fs::read_to_string(&previous_manifest_path).expect("read previous manifest");

        write_file(
            &runtime_launcher_path(&paths),
            b"#!/bin/sh\nexit 0\n# generation two\n",
        );
        refresh_runtime_provenance(&paths);
        configure_claude_desktop_with_paths(&paths).expect("reconfigure");

        let current_launcher_path = active_launcher_path(&paths);
        let current_manifest_path = active_manifest_path(&paths);
        assert_ne!(current_launcher_path, previous_launcher_path);
        assert_ne!(current_manifest_path, previous_manifest_path);
        assert_eq!(
            std::fs::read(&previous_launcher_path).expect("read previous launcher"),
            previous_launcher
        );
        assert_eq!(
            std::fs::read_to_string(&previous_manifest_path).expect("read previous manifest"),
            previous_manifest
        );

        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(status.success, "{}", status.message);
    }

    #[test]
    fn configure_resolves_packaged_resources_binaries_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_packaged_runtime(&temp);

        configure_claude_desktop_with_paths(&paths).expect("configure");

        let manifest = active_manifest(&paths);
        assert_eq!(manifest.source_kind, McpRuntimeSourceKind::AppBundle);
        assert_eq!(manifest.final_server_db_mode, "live");
        assert!(manifest.bundle_provenance_path.ends_with(
            "Contents/Resources/binaries/dailyos-mcp-bundle-test-target.provenance.json"
        ));
        let provenance: McpBundleProvenance =
            read_json_file(&manifest.bundle_provenance_path).expect("provenance");
        let provenance_sidecar_sha = provenance
            .sidecar(MCP_SERVER_NAME)
            .expect("sidecar provenance")
            .sha256
            .clone();
        assert_ne!(manifest.expected_sidecar_sha256, provenance_sidecar_sha);
        assert_eq!(
            manifest.expected_sidecar_sha256,
            sha256_file(&manifest.sidecar_path).expect("signed sidecar hash")
        );

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(status.success, "{}", status.message);
    }

    #[test]
    fn configure_rejects_unsigned_packaged_runtime_when_signature_required() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mut paths = setup_packaged_runtime(&temp);
        paths.verify_app_bundle_signature = true;

        let error = configure_claude_desktop_with_paths(&paths)
            .expect_err("unsigned app bundle should not be trusted");

        assert!(
            error.contains("app_bundle_signature_"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn configure_rejects_stub_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, true);

        let error = configure_claude_desktop_with_paths(&paths).expect_err("stub rejected");

        assert!(error.contains("provenance_is_stub"));
    }

    #[test]
    fn configure_rejects_wrong_target_provenance_before_selecting_runtime() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        let provenance_path =
            runtime_binary_dir(&paths).join("dailyos-mcp-bundle-test-target.provenance.json");
        rewrite_test_provenance_file(&provenance_path, |provenance| {
            provenance.target_triple = "wrong-target".to_string();
        });

        let error = configure_claude_desktop_with_paths(&paths).expect_err("target rejected");

        assert!(error.contains("provenance_target_triple_mismatch"));
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
    fn status_rejects_duplicate_manifest_args() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let config_path = paths.claude_config_path();
        let mut config: Value = read_json_file(&config_path).expect("config");
        config["mcpServers"]["dailyos"]["args"] = serde_json::json!([
            "--manifest",
            active_manifest_path(&paths),
            "--manifest",
            temp.path().join("alt-manifest.json")
        ]);
        write_json_file(&config_path, &config).expect("write config");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status
            .message
            .contains("unexpected launcher manifest arguments"));
    }

    #[test]
    fn status_rejects_dailyos_env_overrides() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let config_path = paths.claude_config_path();
        let mut config: Value = read_json_file(&config_path).expect("config");
        config["mcpServers"]["dailyos"]["env"] = serde_json::json!({
            "DAILYOS_MCP_LEGACY_V1": "1"
        });
        write_json_file(&config_path, &config).expect("write config");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status.message.contains("env overrides are not allowed"));
    }

    #[test]
    fn status_rejects_manifest_not_backed_by_bundled_provenance() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        let manifest_path = active_manifest_path(&paths);
        let mut manifest: McpLauncherManifest = read_json_file(&manifest_path).expect("manifest");
        manifest.expected_sidecar_sha256 = "0".repeat(64);
        write_json_file(&manifest_path, &manifest).expect("write manifest");

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status.message.contains("sidecar provenance mismatch"));
    }

    #[test]
    fn status_rejects_repo_binaries_live_db_mode_manifest() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");
        rewrite_manifest(&paths, |manifest| {
            manifest.final_server_db_mode = "live".to_string();
        });

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status.message.contains("manifest DB mode mismatch"));
    }

    #[test]
    fn status_rejects_missing_or_stale_managed_runtime() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let launcher = active_launcher_path(&paths);
        std::fs::remove_file(&launcher).expect("remove launcher");
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(!status.success);
        assert!(status.message.contains("launcher path invalid"));

        write_file(&launcher, b"#!/bin/sh\nexit 0\n");
        let launcher_hash = sha256_file(&launcher).expect("launcher hash");
        rewrite_manifest(&paths, |manifest| {
            manifest.expected_launcher_sha256 = launcher_hash;
            manifest.guard_epoch = "stale".to_string();
        });
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(!status.success);
        assert!(status.message.contains("stale launcher manifest"));
    }

    #[cfg(unix)]
    #[test]
    fn status_rejects_non_executable_runtime_without_chmod() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let managed_launcher = active_launcher_path(&paths);
        clear_executable(&managed_launcher);
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(!status.success);
        assert!(status
            .message
            .contains("dailyos-mcp-launcher_not_executable"));
        assert!(!path_is_executable(&managed_launcher));

        make_executable(&managed_launcher);
        let sidecar = active_sidecar_path(&paths);
        clear_executable(&sidecar);
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(!status.success);
        assert!(status.message.contains("dailyos-mcp_not_executable"));
        assert!(!path_is_executable(&sidecar));
    }

    #[test]
    fn status_rejects_hash_missing_sidecar_and_launcher_check_failures() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        write_file(
            &active_launcher_path(&paths),
            b"#!/bin/sh\nexit 0\n# changed\n",
        );
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(!status.success);
        assert!(status
            .message
            .contains("dailyos-mcp-launcher_hash_mismatch"));

        let launcher = active_launcher_path(&paths);
        write_file(&launcher, b"#!/bin/sh\nexit 42\n");
        let launcher_hash = sha256_file(&launcher).expect("launcher hash");
        let manifest = active_manifest(&paths);
        let server = manifest.sidecar_path;
        write_provenance(
            manifest
                .bundle_provenance_path
                .parent()
                .expect("provenance parent"),
            &test_server_filename(),
            &sha256_file(&server).expect("server hash"),
            &test_launcher_filename(),
            &launcher_hash,
            false,
        );
        rewrite_manifest(&paths, |manifest| {
            manifest.expected_launcher_sha256 = launcher_hash;
        });
        let mut strict_paths = paths.clone();
        strict_paths.run_launcher_check = true;
        let status = get_claude_desktop_status_with_paths(&strict_paths);
        assert!(!status.success);
        assert!(status.message.contains("launcher_check_failed"));

        rewrite_manifest(&paths, |manifest| {
            let _ = std::fs::remove_file(&manifest.sidecar_path);
        });
        let status = get_claude_desktop_status_with_paths(&paths);
        assert!(!status.success);
        assert!(status.message.contains("sidecar path invalid"));
    }

    #[test]
    fn status_rejects_manifest_that_blesses_tampered_managed_launcher() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_packaged_runtime(&temp);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let launcher = active_launcher_path(&paths);
        write_file(
            &launcher,
            b"#!/bin/sh\nexit 0\n# tampered managed launcher\n",
        );
        let launcher_hash = sha256_file(&launcher).expect("launcher hash");
        rewrite_manifest(&paths, |manifest| {
            manifest.expected_launcher_sha256 = launcher_hash;
        });

        let status = get_claude_desktop_status_with_paths(&paths);

        assert!(!status.success);
        assert!(status
            .message
            .contains("dailyos-mcp-launcher_hash_mismatch"));
    }

    #[test]
    fn status_includes_launcher_check_refusal_details() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let launcher = active_launcher_path(&paths);
        write_file(
            &launcher,
            b"#!/bin/sh\necho 'dailyos-mcp-launcher refused to start: manifest_guard_epoch_mismatch' >&2\nexit 1\n",
        );
        let launcher_hash = sha256_file(&launcher).expect("launcher hash");
        let manifest = active_manifest(&paths);
        let server = manifest.sidecar_path;
        write_provenance(
            manifest
                .bundle_provenance_path
                .parent()
                .expect("provenance parent"),
            &test_server_filename(),
            &sha256_file(&server).expect("server hash"),
            &test_launcher_filename(),
            &launcher_hash,
            false,
        );
        rewrite_manifest(&paths, |manifest| {
            manifest.expected_launcher_sha256 = launcher_hash;
        });
        let mut strict_paths = paths.clone();
        strict_paths.run_launcher_check = true;

        let status = get_claude_desktop_status_with_paths(&strict_paths);

        assert!(!status.success);
        assert!(status.message.contains("launcher_check_failed"));
        assert!(status.message.contains("manifest_guard_epoch_mismatch"));
    }

    #[test]
    fn status_rejects_exit_zero_launcher_check_without_valid_payload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let launcher = active_launcher_path(&paths);
        write_file(&launcher, b"#!/bin/sh\nexit 0\n");
        let launcher_hash = sha256_file(&launcher).expect("launcher hash");
        let manifest = active_manifest(&paths);
        let server = manifest.sidecar_path;
        write_provenance(
            manifest
                .bundle_provenance_path
                .parent()
                .expect("provenance parent"),
            &test_server_filename(),
            &sha256_file(&server).expect("server hash"),
            &test_launcher_filename(),
            &launcher_hash,
            false,
        );
        rewrite_manifest(&paths, |manifest| {
            manifest.expected_launcher_sha256 = launcher_hash;
        });
        let mut strict_paths = paths.clone();
        strict_paths.run_launcher_check = true;

        let status = get_claude_desktop_status_with_paths(&strict_paths);

        assert!(!status.success);
        assert!(status.message.contains("launcher_check_invalid_json"));
    }

    #[test]
    fn launcher_check_stderr_detail_truncates_on_char_boundary() {
        let diagnostic = "é".repeat(600);

        let detail = launcher_check_stderr_detail(diagnostic.as_bytes());

        assert_eq!(detail.chars().count(), 512);
        assert!(detail.is_char_boundary(detail.len()));
    }

    #[test]
    fn status_rejects_hanging_launcher_check_with_timeout() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = setup_dev_runtime(&temp, false);
        configure_claude_desktop_with_paths(&paths).expect("configure");

        let launcher = active_launcher_path(&paths);
        write_file(&launcher, b"#!/bin/sh\nsleep 5\n");
        let launcher_hash = sha256_file(&launcher).expect("launcher hash");
        let manifest = active_manifest(&paths);
        let server = manifest.sidecar_path;
        write_provenance(
            manifest
                .bundle_provenance_path
                .parent()
                .expect("provenance parent"),
            &test_server_filename(),
            &sha256_file(&server).expect("server hash"),
            &test_launcher_filename(),
            &launcher_hash,
            false,
        );
        rewrite_manifest(&paths, |manifest| {
            manifest.expected_launcher_sha256 = launcher_hash;
        });
        let mut strict_paths = paths.clone();
        strict_paths.run_launcher_check = true;
        strict_paths.launcher_check_timeout = Duration::from_millis(100);

        let status = get_claude_desktop_status_with_paths(&strict_paths);

        assert!(!status.success);
        assert!(status.message.contains("launcher_check_timeout"));
    }
}
