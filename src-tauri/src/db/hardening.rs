//! Filesystem hardening for the DailyOS data directory.
//!
//! Sets restrictive permissions and excludes sensitive data from Time Machine.
//! All operations are best-effort — log warnings on failure, never crash.
//! Runs once per process lifetime (guarded by `OnceLock`).

use std::path::Path;
use std::sync::OnceLock;

/// Guard ensuring hardening runs at most once per process.
static HARDENED: OnceLock<()> = OnceLock::new();

/// Apply filesystem hardening to the DailyOS data directory.
/// Only runs once per process — subsequent calls are no-ops.
pub fn harden_data_directory(dailyos_dir: &Path) {
    HARDENED.get_or_init(|| {
        set_directory_permissions(dailyos_dir);
        set_file_permissions(&dailyos_dir.join("dailyos.db"));
        set_file_permissions(&dailyos_dir.join("dailyos-dev.db"));
        exclude_from_time_machine(dailyos_dir);
    });
}

/// Set 0o700 on a directory (owner rwx only).
fn set_directory_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.is_dir() {
            if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)) {
                log::warn!(
                    "Failed to set directory permissions on {}: {e}",
                    path.display()
                );
            }
        }
    }
}

/// Set 0o600 on a file (owner rw only).
pub fn set_file_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if path.is_file() {
            if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
                log::warn!("Failed to set file permissions on {}: {e}", path.display());
            }
        }
    }
}

/// Exclude the directory from Time Machine backups via `tmutil addexclusion`.
/// Best-effort — logs warning on failure.
fn exclude_from_time_machine(path: &Path) {
    match std::process::Command::new("tmutil")
        .args(["addexclusion", "-p", &path.to_string_lossy()])
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log::warn!("tmutil addexclusion failed: {stderr}");
            } else {
                log::info!("Time Machine exclusion set for {}", path.display());
            }
        }
        Err(e) => {
            log::warn!("Failed to run tmutil: {e}");
        }
    }
}
