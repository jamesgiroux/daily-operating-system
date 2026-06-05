use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::db::DbMode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicaRefreshReport {
    pub source_db_path: PathBuf,
    pub replica_db_path: PathBuf,
    pub source_workspace_path: PathBuf,
    pub replica_workspace_path: PathBuf,
    pub files_copied: usize,
    pub directories_created: usize,
}

pub fn refresh_replica_from_live() -> Result<ReplicaRefreshReport, String> {
    refresh_replica_from_live_with_path_loader(ReplicaRefreshPaths::from_runtime)
}

fn refresh_replica_from_live_with_path_loader(
    load_paths: impl FnOnce() -> Result<ReplicaRefreshPaths, String>,
) -> Result<ReplicaRefreshReport, String> {
    if crate::db::db_mode() != DbMode::Live || !crate::db::live_db_mode_explicitly_requested() {
        return Err(
            "doctor replica-refresh requires explicit Live DB mode for its read-only production source; rerun with --live or DAILYOS_DB_MODE=live"
                .to_string(),
        );
    }

    let paths = load_paths()?;
    refresh_replica_from_live_paths(paths)
}

#[derive(Debug, Clone)]
struct ReplicaRefreshPaths {
    source_db_path: PathBuf,
    replica_db_path: PathBuf,
    source_workspace_path: PathBuf,
    replica_workspace_path: PathBuf,
    replica_config_path: PathBuf,
    live_config: crate::types::Config,
}

impl ReplicaRefreshPaths {
    fn from_runtime() -> Result<Self, String> {
        let dailyos_dir = crate::db::dailyos_data_dir().map_err(|e| e.to_string())?;
        let source_db_path = dailyos_dir.join("dailyos.db");
        let replica_db_path = dailyos_dir.join("dailyos-replica.db");
        let live_config = read_live_config()?;
        let configured_workspace = Some(live_config.workspace_path.as_str());
        let source_workspace_path =
            crate::state::workspace_path_for_mode(DbMode::Live, configured_workspace)?;
        let replica_workspace_path = crate::state::workspace_path_for_mode(DbMode::Replica, None)?;
        let replica_config_path = crate::state::replica_config_path()?;

        Ok(Self {
            source_db_path,
            replica_db_path,
            source_workspace_path,
            replica_workspace_path,
            replica_config_path,
            live_config,
        })
    }
}

fn read_live_config() -> Result<crate::types::Config, String> {
    let live_config_path = crate::state::live_config_path()?;
    let content = fs::read_to_string(&live_config_path)
        .map_err(|e| format!("Failed to read live config: {e}"))?;
    let mut config: crate::types::Config =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse live config: {e}"))?;
    config.normalize();
    Ok(config)
}

fn refresh_replica_from_live_paths(
    paths: ReplicaRefreshPaths,
) -> Result<ReplicaRefreshReport, String> {
    let staged_paths = StagedReplicaPaths::from_paths(&paths);
    reject_workspace_refresh_path_overlap(
        &paths.source_workspace_path,
        &paths.replica_workspace_path,
        &staged_paths.workspace_path,
    )?;
    ensure_source_ready(&paths.source_db_path)?;
    let stage_result = stage_replica_refresh(&paths, &staged_paths);
    let (files_copied, directories_created) = match stage_result {
        Ok(counts) => counts,
        Err(error) => {
            cleanup_staged_replica(&staged_paths);
            return Err(error);
        }
    };

    if let Err(error) = activate_staged_replica(&paths, &staged_paths) {
        cleanup_staged_replica(&staged_paths);
        return Err(error);
    }

    Ok(ReplicaRefreshReport {
        source_db_path: paths.source_db_path,
        replica_db_path: paths.replica_db_path,
        source_workspace_path: paths.source_workspace_path,
        replica_workspace_path: paths.replica_workspace_path,
        files_copied,
        directories_created,
    })
}

#[derive(Debug, Clone)]
struct StagedReplicaPaths {
    db_path: PathBuf,
    workspace_path: PathBuf,
    config_path: PathBuf,
}

impl StagedReplicaPaths {
    fn from_paths(paths: &ReplicaRefreshPaths) -> Self {
        Self {
            db_path: crate::db_backup::database_refresh_temp_path(&paths.replica_db_path),
            workspace_path: workspace_temp_path(&paths.replica_workspace_path),
            config_path: refresh_temp_path(&paths.replica_config_path),
        }
    }
}

fn stage_replica_refresh(
    paths: &ReplicaRefreshPaths,
    staged_paths: &StagedReplicaPaths,
) -> Result<(usize, usize), String> {
    clone_database(
        &paths.source_db_path,
        &paths.replica_db_path,
        &staged_paths.db_path,
    )?;
    let counts = stage_workspace(&paths.source_workspace_path, &staged_paths.workspace_path)?;
    write_replica_config(paths, &staged_paths.config_path)?;
    Ok(counts)
}

fn activate_staged_replica(
    paths: &ReplicaRefreshPaths,
    staged_paths: &StagedReplicaPaths,
) -> Result<(), String> {
    let workspace_activation = activate_staged_path(
        &staged_paths.workspace_path,
        &paths.replica_workspace_path,
        StagedPathKind::Directory,
    )?;

    let config_activation = match activate_staged_path(
        &staged_paths.config_path,
        &paths.replica_config_path,
        StagedPathKind::File,
    ) {
        Ok(activation) => activation,
        Err(error) => {
            workspace_activation.rollback();
            return Err(error);
        }
    };

    let db_activation = match crate::db_backup::activate_staged_database_clone(
        &staged_paths.db_path,
        &paths.replica_db_path,
    ) {
        Ok(activation) => activation,
        Err(error) => {
            config_activation.rollback();
            workspace_activation.rollback();
            return Err(error);
        }
    };

    db_activation.commit();
    config_activation.commit();
    workspace_activation.commit();
    Ok(())
}

fn cleanup_staged_replica(staged_paths: &StagedReplicaPaths) {
    cleanup_path(
        &staged_paths.db_path,
        StagedPathKind::File,
        "staged replica DB",
    );
    cleanup_path(
        &staged_paths.workspace_path,
        StagedPathKind::Directory,
        "staged replica workspace",
    );
    cleanup_path(
        &staged_paths.config_path,
        StagedPathKind::File,
        "staged replica config",
    );
}

fn ensure_source_ready(source_db_path: &Path) -> Result<(), String> {
    if !source_db_path.exists() {
        return Err(format!(
            "Production DB source does not exist: {}",
            source_db_path.display()
        ));
    }

    let wal = wal_path(source_db_path);
    if wal.exists() {
        let wal_len = fs::metadata(&wal)
            .map_err(|e| format!("Failed to inspect production WAL {}: {e}", wal.display()))?
            .len();
        if wal_len > 0 {
            return Err(format!(
                "Production WAL has live frames ({} bytes at {}). Close DailyOS or checkpoint before replica-refresh.",
                wal_len,
                wal.display()
            ));
        }
    }

    Ok(())
}

fn clone_database(
    source_db_path: &Path,
    replica_db_path: &Path,
    staged_db_path: &Path,
) -> Result<(), String> {
    crate::db_backup::clone_database_to_staged_path(source_db_path, replica_db_path, staged_db_path)
}

fn refresh_workspace(source: &Path, destination: &Path) -> Result<(usize, usize), String> {
    let staged_destination = workspace_temp_path(destination);
    reject_workspace_refresh_path_overlap(source, destination, &staged_destination)?;
    let counts = match stage_workspace(source, &staged_destination) {
        Ok(counts) => counts,
        Err(error) => {
            cleanup_path(
                &staged_destination,
                StagedPathKind::Directory,
                "staged replica workspace",
            );
            return Err(error);
        }
    };
    match activate_staged_path(&staged_destination, destination, StagedPathKind::Directory) {
        Ok(activation) => {
            activation.commit();
            Ok(counts)
        }
        Err(error) => {
            cleanup_path(
                &staged_destination,
                StagedPathKind::Directory,
                "staged replica workspace",
            );
            Err(error)
        }
    }
}

fn stage_workspace(source: &Path, staged_destination: &Path) -> Result<(usize, usize), String> {
    if !source.exists() {
        return Err(format!(
            "Live workspace source does not exist: {}",
            source.display()
        ));
    }
    let source_metadata = fs::symlink_metadata(source)
        .map_err(|e| format!("Failed to inspect live workspace source: {e}"))?;
    if source_metadata.file_type().is_symlink() {
        return Err(format!(
            "Refusing to clone symlink live workspace root: {}",
            source.display()
        ));
    }
    if !source_metadata.is_dir() {
        return Err(format!(
            "Live workspace source is not a directory: {}",
            source.display()
        ));
    }
    if staged_destination.exists() {
        fs::remove_dir_all(staged_destination).map_err(|e| {
            format!(
                "Failed to remove existing replica workspace temp directory {}: {e}",
                staged_destination.display()
            )
        })?;
    }
    fs::create_dir_all(staged_destination).map_err(|e| {
        format!(
            "Failed to create replica workspace temp directory {}: {e}",
            staged_destination.display()
        )
    })?;

    match copy_workspace_tree(source, staged_destination) {
        Ok(counts) => Ok(counts),
        Err(error) => {
            cleanup_path(
                staged_destination,
                StagedPathKind::Directory,
                "staged replica workspace",
            );
            Err(error)
        }
    }
}

fn workspace_temp_path(destination: &Path) -> PathBuf {
    refresh_temp_path(destination)
}

fn refresh_temp_path(destination: &Path) -> PathBuf {
    destination.with_file_name(format!(
        "{}.refresh.tmp",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dailyos-replica-workspace")
    ))
}

fn refresh_previous_path(destination: &Path) -> PathBuf {
    destination.with_file_name(format!(
        "{}.refresh.previous",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dailyos-replica-workspace")
    ))
}

fn reject_workspace_refresh_path_overlap(
    source: &Path,
    destination: &Path,
    staged_destination: &Path,
) -> Result<(), String> {
    let source_canonical = source
        .canonicalize()
        .map_err(|e| format!("Failed to resolve live workspace source: {e}"))?;

    for (label, path) in [
        ("replica workspace destination", destination),
        ("staged replica workspace", staged_destination),
        (
            "previous replica workspace backup",
            &refresh_previous_path(destination),
        ),
    ] {
        let managed_candidate = canonical_candidate(path)?;
        if managed_candidate == source_canonical
            || managed_candidate.starts_with(&source_canonical)
            || source_canonical.starts_with(&managed_candidate)
        {
            return Err(format!(
                "Live workspace source must not overlap {label}: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn canonical_candidate(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve {}: {e}", path.display()));
    }
    let parent = path
        .parent()
        .ok_or_else(|| format!("Path has no parent: {}", path.display()))?;
    if parent.exists() {
        let mut candidate = parent
            .canonicalize()
            .map_err(|e| format!("Failed to resolve {}: {e}", parent.display()))?;
        if let Some(name) = path.file_name() {
            candidate.push(name);
        }
        return Ok(candidate);
    }
    Ok(path.to_path_buf())
}

fn copy_workspace_tree(source: &Path, destination: &Path) -> Result<(usize, usize), String> {
    let mut files_copied = 0_usize;
    let mut directories_created = 1_usize;
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|e| format!("Failed to walk live workspace: {e}"))?;
        let path = entry.path();
        let relative = path.strip_prefix(source).map_err(|e| {
            format!(
                "Failed to compute workspace relative path for {}: {e}",
                path.display()
            )
        })?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        let file_type = entry.file_type();
        if file_type.is_symlink() {
            return Err(format!(
                "Refusing to clone symlink in live workspace: {}",
                path.display()
            ));
        }
        if file_type.is_dir() {
            fs::create_dir_all(&target)
                .map_err(|e| format!("Failed to create workspace directory: {e}"))?;
            directories_created += 1;
            continue;
        }
        if file_type.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create workspace parent: {e}"))?;
            }
            fs::copy(path, &target).map_err(|e| {
                format!(
                    "Failed to copy workspace file {} -> {}: {e}",
                    path.display(),
                    target.display()
                )
            })?;
            files_copied += 1;
        }
    }

    Ok((files_copied, directories_created))
}

fn write_replica_config(
    paths: &ReplicaRefreshPaths,
    staged_config_path: &Path,
) -> Result<(), String> {
    let config = replica_config_from_live(paths.live_config.clone(), &paths.replica_workspace_path);
    let serialized =
        serde_json::to_string_pretty(&config).map_err(|e| format!("Serialize config: {e}"))?;
    crate::util::atomic_write_str(staged_config_path, &serialized)
        .map_err(|e| format!("Failed to write replica config: {e}"))?;
    harden_replica_config_permissions(staged_config_path)
}

fn harden_replica_config_permissions(config_path: &Path) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        set_owner_only_directory_permissions(parent)?;
    }
    set_owner_only_file_permissions(config_path)
}

#[cfg(unix)]
fn set_owner_only_directory_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| {
        format!(
            "Failed to set owner-only permissions on replica config directory {}: {e}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_owner_only_directory_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn set_owner_only_file_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| {
        format!(
            "Failed to set owner-only permissions on replica config {}: {e}",
            path.display()
        )
    })
}

#[cfg(not(unix))]
fn set_owner_only_file_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn replica_config_from_live(
    mut config: crate::types::Config,
    replica_workspace_path: &Path,
) -> crate::types::Config {
    config.workspace_path = replica_workspace_path.to_string_lossy().to_string();
    config.google = crate::types::GoogleConfig::default();
    config.drive = crate::types::DriveConfig::default();
    config.linear = crate::linear::LinearConfig::default();
    config.clay = crate::clay::ClayConfig::default();
    config.quill = crate::quill::QuillConfig::default();
    config.granola = crate::granola::GranolaConfig::default();
    config.gravatar = crate::gravatar::GravatarConfig::default();
    config
}

#[derive(Debug, Clone, Copy)]
enum StagedPathKind {
    File,
    Directory,
}

struct ActivatedPath {
    destination: PathBuf,
    backup: Option<PathBuf>,
    kind: StagedPathKind,
}

impl ActivatedPath {
    fn rollback(self) {
        cleanup_path(&self.destination, self.kind, "activated replica path");
        if let Some(backup) = self.backup {
            if backup.exists() {
                if let Err(error) = fs::rename(&backup, &self.destination) {
                    log::warn!(
                        "replica-refresh rollback failed restoring {} from {}: {error}",
                        self.destination.display(),
                        backup.display()
                    );
                }
            }
        }
    }

    fn commit(self) {
        if let Some(backup) = self.backup {
            cleanup_path(&backup, self.kind, "previous replica path");
        }
    }
}

fn activate_staged_path(
    staged: &Path,
    destination: &Path,
    kind: StagedPathKind,
) -> Result<ActivatedPath, String> {
    if staged == destination {
        return Err("Staged path must differ from destination path".to_string());
    }
    if !staged.exists() {
        return Err(format!("Staged path does not exist: {}", staged.display()));
    }

    let backup = refresh_previous_path(destination);
    cleanup_path(&backup, kind, "stale previous replica path");
    let backup = if destination.exists() {
        fs::rename(destination, &backup).map_err(|e| {
            format!(
                "Failed to move existing replica path {} aside to {}: {e}",
                destination.display(),
                backup.display()
            )
        })?;
        Some(backup)
    } else {
        None
    };

    match fs::rename(staged, destination) {
        Ok(()) => Ok(ActivatedPath {
            destination: destination.to_path_buf(),
            backup,
            kind,
        }),
        Err(error) => {
            if let Some(backup) = backup.as_ref() {
                if backup.exists() {
                    if let Err(rollback_error) = fs::rename(backup, destination) {
                        log::warn!(
                            "replica-refresh activation rollback failed restoring {} from {}: {rollback_error}",
                            destination.display(),
                            backup.display()
                        );
                    }
                }
            }
            Err(format!(
                "Failed to activate replica path {} from {}: {error}",
                destination.display(),
                staged.display()
            ))
        }
    }
}

fn cleanup_path(path: &Path, kind: StagedPathKind, label: &str) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            log::warn!(
                "{label} cleanup could not inspect {}: {error}",
                path.display()
            );
            return;
        }
    };
    let result = match kind {
        StagedPathKind::File => fs::remove_file(path),
        StagedPathKind::Directory if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)
        }
        StagedPathKind::Directory => fs::remove_file(path),
    };
    if let Err(error) = result {
        log::warn!("{label} cleanup failed for {}: {error}", path.display());
    }
}

fn wal_path(db_path: &Path) -> PathBuf {
    db_path.with_file_name(format!(
        "{}-wal",
        db_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("dailyos.db")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct ResetDbMode;

    impl Drop for ResetDbMode {
        fn drop(&mut self) {
            crate::db::set_db_mode(DbMode::Live);
        }
    }

    struct EnvGuard {
        name: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn remove(name: &'static str) -> Self {
            let previous = std::env::var(name).ok();
            std::env::remove_var(name);
            Self { name, previous }
        }

        fn set(name: &'static str, value: &str) -> Self {
            let previous = std::env::var(name).ok();
            std::env::set_var(name, value);
            Self { name, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.as_deref() {
                Some(value) => std::env::set_var(self.name, value),
                None => std::env::remove_var(self.name),
            }
        }
    }

    fn test_config(workspace_path: &Path) -> crate::types::Config {
        let mut config: crate::types::Config = serde_json::from_value(serde_json::json!({
            "workspacePath": workspace_path.to_string_lossy(),
        }))
        .expect("test config");
        config.normalize();
        config
    }

    fn write_marker_db(path: &Path, label: &str) {
        let db = crate::db::ActionDb::open_at(
            path.to_path_buf(),
            Arc::new(crate::db::LocalKeychain::new()),
        )
        .expect("open marker db");
        db.conn_ref()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS replica_refresh_marker (label TEXT);
                 DELETE FROM replica_refresh_marker;",
            )
            .expect("marker schema");
        db.conn_ref()
            .execute(
                "INSERT INTO replica_refresh_marker (label) VALUES (?1)",
                [label],
            )
            .expect("marker insert");
        db.conn_ref()
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .expect("checkpoint marker db");
    }

    fn read_marker_db(path: &Path) -> String {
        let db =
            crate::db::ActionDb::open_readonly_at(path, Arc::new(crate::db::LocalKeychain::new()))
                .expect("open cloned marker db");
        db.conn_ref()
            .query_row("SELECT label FROM replica_refresh_marker", [], |row| {
                row.get(0)
            })
            .expect("read marker")
    }

    #[test]
    fn refresh_replica_from_live_requires_explicit_live_mode() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        let _env = EnvGuard::remove("DAILYOS_DB_MODE");
        crate::db::set_db_mode(DbMode::Live);

        let error =
            refresh_replica_from_live().expect_err("implicit release Live mode must be refused");
        assert!(error.contains("requires explicit Live DB mode"));
    }

    #[test]
    fn refresh_replica_from_live_accepts_explicit_live_mode_before_loading_paths() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        let _env = EnvGuard::set("DAILYOS_DB_MODE", "live");
        crate::db::set_db_mode(DbMode::Live);

        let error = refresh_replica_from_live_with_path_loader(|| {
            Err("explicit live accepted; path loader reached".to_string())
        })
        .expect_err("path loader should provide the terminal test error");
        assert_eq!(error, "explicit live accepted; path loader reached");
    }

    #[test]
    fn ensure_source_ready_rejects_live_wal_frames() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dailyos.db");
        fs::write(&db_path, b"not a real db").expect("db");
        fs::write(wal_path(&db_path), b"live frames").expect("wal");

        let error = ensure_source_ready(&db_path).expect_err("live WAL frames must fail");
        assert!(error.contains("Production WAL has live frames"));
    }

    #[test]
    fn refresh_workspace_copies_files_and_rejects_symlinks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source = dir.path().join("live");
        let destination = dir.path().join("replica");
        fs::create_dir_all(source.join("Accounts/Example")).expect("source dirs");
        fs::write(source.join("Accounts/Example/dashboard.json"), "{}").expect("file");

        let (files, dirs) = refresh_workspace(&source, &destination).expect("refresh workspace");
        assert_eq!(files, 1);
        assert!(dirs >= 2);
        assert!(destination.join("Accounts/Example/dashboard.json").exists());

        #[cfg(unix)]
        {
            let symlink_source = dir.path().join("with-symlink");
            let symlink_destination = dir.path().join("replica-symlink");
            fs::create_dir_all(&symlink_source).expect("symlink source");
            fs::create_dir_all(&symlink_destination).expect("existing symlink destination");
            fs::write(symlink_destination.join("previous.txt"), "keep").expect("previous copy");
            std::os::unix::fs::symlink("/tmp", symlink_source.join("outside")).expect("symlink");
            let error =
                refresh_workspace(&symlink_source, &symlink_destination).expect_err("symlink");
            assert!(error.contains("Refusing to clone symlink"));
            assert!(
                symlink_destination.join("previous.txt").exists(),
                "failed refresh should preserve existing replica workspace"
            );

            let real_source_root = dir.path().join("real-root");
            let symlink_root = dir.path().join("symlink-root");
            let symlink_root_destination = dir.path().join("replica-symlink-root");
            fs::create_dir_all(&real_source_root).expect("real source root");
            fs::write(real_source_root.join("dashboard.json"), "{}").expect("real source file");
            fs::create_dir_all(&symlink_root_destination)
                .expect("existing symlink root destination");
            fs::write(symlink_root_destination.join("previous.txt"), "keep")
                .expect("previous symlink root copy");
            std::os::unix::fs::symlink(&real_source_root, &symlink_root)
                .expect("workspace root symlink");
            let error = refresh_workspace(&symlink_root, &symlink_root_destination)
                .expect_err("root symlink");
            assert!(error.contains("Refusing to clone symlink live workspace root"));
            assert!(
                symlink_root_destination.join("previous.txt").exists(),
                "root symlink failure should preserve existing replica workspace"
            );
        }
    }

    #[test]
    fn refresh_workspace_rejects_live_source_overlap_with_managed_replica_paths() {
        let dir = tempfile::tempdir().expect("tempdir");

        let destination = dir.path().join("replica-workspace");
        let source_inside_destination = destination.join("live-source");
        fs::create_dir_all(&source_inside_destination).expect("nested source");
        fs::write(destination.join("previous.txt"), "keep").expect("destination marker");
        fs::write(source_inside_destination.join("source.txt"), "keep").expect("source marker");

        let error = refresh_workspace(&source_inside_destination, &destination)
            .expect_err("source inside destination must be rejected");
        assert!(error.contains("must not overlap replica workspace destination"));
        assert!(
            source_inside_destination.join("source.txt").exists(),
            "overlap rejection must not clean the live source"
        );
        assert!(
            destination.join("previous.txt").exists(),
            "overlap rejection must not clean the existing replica workspace"
        );

        let staged_destination = dir.path().join("replica-workspace-staged");
        let staged_source = workspace_temp_path(&staged_destination);
        fs::create_dir_all(&staged_source).expect("staged source");
        fs::write(staged_source.join("source.txt"), "keep").expect("staged source marker");
        let error = refresh_workspace(&staged_source, &staged_destination)
            .expect_err("source equal to staged path must be rejected");
        assert!(error.contains("must not overlap staged replica workspace"));
        assert!(
            staged_source.join("source.txt").exists(),
            "staged-overlap rejection must happen before generic staged cleanup"
        );

        let previous_destination = dir.path().join("replica-workspace-previous");
        let previous_source = refresh_previous_path(&previous_destination);
        fs::create_dir_all(&previous_source).expect("previous source");
        fs::write(previous_source.join("source.txt"), "keep").expect("previous source marker");
        let error = refresh_workspace(&previous_source, &previous_destination)
            .expect_err("source equal to previous path must be rejected");
        assert!(error.contains("must not overlap previous replica workspace backup"));
        assert!(
            previous_source.join("source.txt").exists(),
            "previous-overlap rejection must happen before stale backup cleanup"
        );
    }

    #[cfg(unix)]
    #[test]
    fn refresh_failure_after_db_stage_preserves_existing_replica_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source_db_path = dir.path().join("dailyos.db");
        let replica_db_path = dir.path().join("dailyos-replica.db");
        write_marker_db(&source_db_path, "new-live");
        write_marker_db(&replica_db_path, "old-replica");

        let source_workspace_path = dir.path().join("live-workspace");
        let replica_workspace_path = dir.path().join("replica-workspace");
        fs::create_dir_all(&source_workspace_path).expect("source workspace");
        fs::create_dir_all(&replica_workspace_path).expect("replica workspace");
        fs::write(replica_workspace_path.join("previous.txt"), "keep")
            .expect("previous replica workspace");
        std::os::unix::fs::symlink("/tmp", source_workspace_path.join("outside"))
            .expect("nested symlink");

        let replica_config_path = dir.path().join("config-replica.json");
        let paths = ReplicaRefreshPaths {
            source_db_path: source_db_path.clone(),
            replica_db_path: replica_db_path.clone(),
            source_workspace_path,
            replica_workspace_path: replica_workspace_path.clone(),
            replica_config_path,
            live_config: test_config(dir.path()),
        };

        let error = refresh_replica_from_live_paths(paths).expect_err("workspace symlink failure");
        assert!(error.contains("Refusing to clone symlink"));
        assert_eq!(read_marker_db(&replica_db_path), "old-replica");
        assert!(
            replica_workspace_path.join("previous.txt").exists(),
            "failed refresh must preserve previous replica workspace"
        );
        assert!(
            !crate::db_backup::database_refresh_temp_path(&replica_db_path).exists(),
            "failed refresh should clean staged DB"
        );
    }

    #[test]
    fn activation_db_failure_rolls_back_workspace_and_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let replica_db_path = dir.path().join("dailyos-replica.db");
        write_marker_db(&replica_db_path, "old-replica");

        let replica_workspace_path = dir.path().join("replica-workspace");
        fs::create_dir_all(&replica_workspace_path).expect("old replica workspace");
        fs::write(replica_workspace_path.join("previous.txt"), "keep")
            .expect("old workspace marker");

        let replica_config_path = dir.path().join("config-replica.json");
        fs::write(&replica_config_path, "old config").expect("old config");

        let paths = ReplicaRefreshPaths {
            source_db_path: dir.path().join("dailyos.db"),
            replica_db_path: replica_db_path.clone(),
            source_workspace_path: dir.path().join("live-workspace"),
            replica_workspace_path: replica_workspace_path.clone(),
            replica_config_path: replica_config_path.clone(),
            live_config: test_config(dir.path()),
        };
        let staged_paths = StagedReplicaPaths::from_paths(&paths);
        fs::create_dir_all(&staged_paths.workspace_path).expect("staged workspace");
        fs::write(staged_paths.workspace_path.join("new.txt"), "new")
            .expect("staged workspace marker");
        fs::write(&staged_paths.config_path, "new config").expect("staged config");
        assert!(
            !staged_paths.db_path.exists(),
            "missing staged DB should force activation failure after workspace/config activation"
        );

        let error = activate_staged_replica(&paths, &staged_paths)
            .expect_err("missing staged DB should fail activation");
        assert!(error.contains("Database clone staged file does not exist"));

        assert_eq!(read_marker_db(&replica_db_path), "old-replica");
        assert!(
            replica_workspace_path.join("previous.txt").exists(),
            "old workspace must be restored after DB activation failure"
        );
        assert!(
            !replica_workspace_path.join("new.txt").exists(),
            "staged workspace must not remain active after rollback"
        );
        assert_eq!(
            fs::read_to_string(&replica_config_path).expect("restored config"),
            "old config"
        );
    }

    #[test]
    fn refresh_replica_from_live_paths_clones_db_workspace_and_redacted_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let source_db_path = dir.path().join("dailyos.db");
        let replica_db_path = dir.path().join("dailyos-replica.db");
        write_marker_db(&source_db_path, "new-live");

        let source_workspace_path = dir.path().join("live-workspace");
        let replica_workspace_path = dir.path().join("replica-workspace");
        fs::create_dir_all(source_workspace_path.join("Accounts/Example")).expect("source dirs");
        fs::write(
            source_workspace_path.join("Accounts/Example/dashboard.json"),
            "{}",
        )
        .expect("source workspace file");

        let live_config: crate::types::Config = serde_json::from_value(serde_json::json!({
            "workspacePath": source_workspace_path.to_string_lossy(),
            "google": {
                "enabled": true,
                "tokenPath": "/live/token.json"
            },
            "linear": {
                "enabled": true,
                "apiKey": "lin-secret"
            }
        }))
        .expect("live config");
        let replica_config_path = dir.path().join("config-replica.json");
        let paths = ReplicaRefreshPaths {
            source_db_path: source_db_path.clone(),
            replica_db_path: replica_db_path.clone(),
            source_workspace_path: source_workspace_path.clone(),
            replica_workspace_path: replica_workspace_path.clone(),
            replica_config_path: replica_config_path.clone(),
            live_config,
        };

        let report = refresh_replica_from_live_paths(paths).expect("refresh replica");

        assert_eq!(report.source_db_path, source_db_path);
        assert_eq!(report.replica_db_path, replica_db_path);
        assert_eq!(report.source_workspace_path, source_workspace_path);
        assert_eq!(report.replica_workspace_path, replica_workspace_path);
        assert_eq!(report.files_copied, 1);
        assert!(report.directories_created >= 2);
        assert_eq!(read_marker_db(&report.replica_db_path), "new-live");
        assert!(
            report
                .replica_workspace_path
                .join("Accounts/Example/dashboard.json")
                .exists(),
            "workspace file should be copied into the replica workspace"
        );

        let replica_config: crate::types::Config = serde_json::from_str(
            &fs::read_to_string(&replica_config_path).expect("replica config"),
        )
        .expect("parse replica config");
        assert_eq!(
            replica_config.workspace_path,
            report.replica_workspace_path.to_string_lossy()
        );
        assert!(!replica_config.google.enabled);
        assert!(replica_config.google.token_path.is_none());
        assert!(!replica_config.linear.enabled);
        assert!(replica_config.linear.api_key.is_none());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let parent = replica_config_path.parent().expect("replica config parent");
            assert_eq!(
                fs::metadata(parent)
                    .expect("replica config parent mode")
                    .permissions()
                    .mode()
                    & 0o777,
                0o700,
                "replica config parent should be owner-only for the doctor path"
            );
            assert_eq!(
                fs::metadata(&replica_config_path)
                    .expect("replica config mode")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600,
                "replica config should be owner-only after activation"
            );
        }
        assert!(
            !StagedReplicaPaths::from_paths(&ReplicaRefreshPaths {
                source_db_path: report.source_db_path,
                replica_db_path: report.replica_db_path,
                source_workspace_path: report.source_workspace_path,
                replica_workspace_path: report.replica_workspace_path,
                replica_config_path,
                live_config: test_config(dir.path()),
            })
            .db_path
            .exists(),
            "staged DB should be cleaned after a successful refresh"
        );
    }

    #[test]
    fn replica_config_redacts_live_connector_credentials() {
        let live_config: crate::types::Config = serde_json::from_value(serde_json::json!({
            "workspacePath": "/live",
            "google": {
                "enabled": true,
                "tokenPath": "/live/token.json"
            },
            "drive": {
                "enabled": true
            },
            "linear": {
                "enabled": true,
                "apiKey": "lin-secret"
            },
            "clay": {
                "enabled": true,
                "apiKey": "clay-secret",
                "smitheryNamespace": "namespace",
                "smitheryConnectionId": "connection"
            },
            "quill": {
                "enabled": true,
                "bridgePath": "/live/quill-bridge.js"
            },
            "granola": {
                "enabled": true,
                "cachePath": "/live/granola-cache.json"
            },
            "gravatar": {
                "enabled": true,
                "apiKey": "legacy-gravatar-secret"
            }
        }))
        .expect("live config");
        let replica = replica_config_from_live(live_config, Path::new("/replica-workspace"));

        assert_eq!(replica.workspace_path, "/replica-workspace");
        assert!(!replica.google.enabled);
        assert!(replica.google.token_path.is_none());
        assert!(!replica.drive.enabled);
        assert!(!replica.linear.enabled);
        assert!(replica.linear.api_key.is_none());
        assert!(!replica.clay.enabled);
        assert!(replica.clay.api_key.is_none());
        assert!(replica.clay.smithery_namespace.is_none());
        assert!(replica.clay.smithery_connection_id.is_none());
        assert!(!replica.quill.enabled);
        assert!(!replica.granola.enabled);
        assert!(replica.granola.cache_path.is_empty());
        assert!(!replica.gravatar.enabled);
        assert!(replica.gravatar.api_key.is_none());
    }

    #[test]
    fn clone_database_rejects_non_live_mode_prod_source() {
        let _lock = crate::db::DB_MODE_TEST_LOCK.lock().expect("db mode lock");
        let _reset = ResetDbMode;
        crate::db::set_db_mode(DbMode::Replica);
        let dailyos_dir = crate::db::dailyos_data_dir().expect("data dir");
        let source = dailyos_dir.join("dailyos.db");
        fs::create_dir_all(source.parent().expect("parent")).expect("parent");
        fs::write(&source, b"placeholder").expect("source");
        let replica = dailyos_dir.join("dailyos-replica.db");

        let staged = crate::db_backup::database_refresh_temp_path(&replica);
        let error = clone_database(&source, &replica, &staged).expect_err("non-Live source denied");
        assert!(error.contains("Refused to open production database"));
    }
}
