use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tempfile::NamedTempFile;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum InstallIdError {
    #[error("could not determine the application support directory")]
    AppSupportDirUnavailable,
    #[error("failed to create anonymous install ID directory `{path}`: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read anonymous install ID `{path}`: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("anonymous install ID at `{path}` is not a UUID: {source}")]
    Parse { path: PathBuf, source: uuid::Error },
    #[error("failed to stage anonymous install ID in `{path}`: {source}")]
    Stage {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write anonymous install ID `{path}`: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to restrict anonymous install ID permissions `{path}`: {source}")]
    Permissions {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to atomically persist anonymous install ID `{path}`: {source}")]
    Persist {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnonInstallId(Uuid);

impl AnonInstallId {
    pub fn generate_on_opt_in() -> Result<Self, InstallIdError> {
        let app_support_dir =
            default_app_support_dir().ok_or(InstallIdError::AppSupportDirUnavailable)?;
        Self::generate_on_opt_in_in(&app_support_dir)
    }

    pub fn generate_on_opt_in_in(app_support_dir: &Path) -> Result<Self, InstallIdError> {
        let path = anon_id_path(app_support_dir);
        if path.exists() {
            return Self::load_from_path(&path);
        }

        let parent = path
            .parent()
            .expect("anon_id path is always nested under app support");
        fs::create_dir_all(parent).map_err(|source| InstallIdError::CreateDir {
            path: parent.to_path_buf(),
            source,
        })?;

        let id = Self(Uuid::new_v4());
        let mut tmp = NamedTempFile::new_in(parent).map_err(|source| InstallIdError::Stage {
            path: parent.to_path_buf(),
            source,
        })?;
        tmp.write_all(id.to_string().as_bytes())
            .and_then(|()| tmp.write_all(b"\n"))
            .and_then(|()| tmp.flush())
            .map_err(|source| InstallIdError::Write {
                path: path.clone(),
                source,
            })?;
        set_file_mode_0600(tmp.path()).map_err(|source| InstallIdError::Permissions {
            path: tmp.path().to_path_buf(),
            source,
        })?;
        tmp.persist(&path).map_err(|err| InstallIdError::Persist {
            path: path.clone(),
            source: err.error,
        })?;
        set_file_mode_0600(&path).map_err(|source| InstallIdError::Permissions {
            path: path.clone(),
            source,
        })?;
        set_backup_exclusion(&path);

        Ok(id)
    }

    pub fn load_existing() -> Result<Option<Self>, InstallIdError> {
        let Some(app_support_dir) = default_app_support_dir() else {
            return Ok(None);
        };
        Self::load_existing_in(&app_support_dir)
    }

    pub fn load_existing_in(app_support_dir: &Path) -> Result<Option<Self>, InstallIdError> {
        let path = anon_id_path(app_support_dir);
        if !path.exists() {
            return Ok(None);
        }
        Self::load_from_path(&path).map(Some)
    }

    pub fn as_uuid(self) -> Uuid {
        self.0
    }

    fn load_from_path(path: &Path) -> Result<Self, InstallIdError> {
        let raw = fs::read_to_string(path).map_err(|source| InstallIdError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Uuid::parse_str(raw.trim())
            .map(Self)
            .map_err(|source| InstallIdError::Parse {
                path: path.to_path_buf(),
                source,
            })
    }
}

impl fmt::Display for AnonInstallId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Serialize for AnonInstallId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

pub fn default_app_support_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("dailyos"))
}

pub fn anon_id_path(app_support_dir: &Path) -> PathBuf {
    app_support_dir.join("anon_id")
}

#[cfg(unix)]
fn set_file_mode_0600(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_file_mode_0600(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(target_os = "macos")]
fn set_backup_exclusion(path: &Path) {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let Ok(path) = CString::new(path.as_os_str().as_bytes()) else {
        return;
    };
    let Ok(name) = CString::new("com.apple.metadata:com_apple_backup_excludeItem") else {
        return;
    };
    let value = b"com.apple.backupd";
    unsafe {
        libc::setxattr(
            path.as_ptr(),
            name.as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            0,
            0,
        );
    }
}

#[cfg(not(target_os = "macos"))]
fn set_backup_exclusion(_path: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anon_id_is_created_only_when_generate_on_opt_in_is_called() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = anon_id_path(dir.path());

        assert!(!path.exists());

        let id = AnonInstallId::generate_on_opt_in_in(dir.path()).expect("generate ID");
        assert!(path.exists());
        assert_eq!(
            AnonInstallId::load_existing_in(dir.path()).expect("load ID"),
            Some(id)
        );
    }

    #[test]
    #[cfg(unix)]
    fn anon_id_file_uses_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = anon_id_path(dir.path());
        AnonInstallId::generate_on_opt_in_in(dir.path()).expect("generate ID");

        let mode = fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
