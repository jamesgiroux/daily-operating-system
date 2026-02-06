//! Archive workflow implementation
//!
//! Pure Rust file operations (no AI needed):
//! - Move _today/*.md files to archive/YYYY-MM-DD/
//! - Preserve week-*.md files (weekly view)
//! - Silent operation (no notifications)

use std::path::Path;

use chrono::Local;
use tokio::fs;

/// Result of running the archive workflow
#[derive(Debug, Clone)]
pub struct ArchiveResult {
    /// Number of files moved to archive
    pub files_archived: usize,
    /// Path to the archive directory (empty if nothing archived)
    pub archive_path: String,
}

/// Run the archive workflow (pure Rust, no AI)
///
/// Moves _today/*.md files to archive/YYYY-MM-DD/, except for:
/// - week-*.md files (preserved for weekly view)
/// - Directories
/// - Non-markdown files
///
/// Returns success even if _today/ is empty or doesn't exist.
pub async fn run_archive(workspace: &Path) -> Result<ArchiveResult, String> {
    let today_dir = workspace.join("_today");

    // Graceful: if _today/ doesn't exist, succeed silently
    if !today_dir.exists() {
        log::info!("Archive: _today/ doesn't exist, nothing to archive");
        return Ok(ArchiveResult {
            files_archived: 0,
            archive_path: String::new(),
        });
    }

    // Create archive/YYYY-MM-DD/
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let archive_dir = today_dir.join("archive").join(&date_str);

    // Collect files to archive first (before creating archive dir)
    let files_to_archive = collect_archivable_files(&today_dir).await?;

    // If nothing to archive, return early without creating the directory
    if files_to_archive.is_empty() {
        log::info!("Archive: no files to archive in _today/");
        return Ok(ArchiveResult {
            files_archived: 0,
            archive_path: String::new(),
        });
    }

    // Create archive directory
    fs::create_dir_all(&archive_dir)
        .await
        .map_err(|e| format!("Failed to create archive dir {}: {}", archive_dir.display(), e))?;

    // Move files
    let mut archived = 0;
    for file_path in files_to_archive {
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let dest = archive_dir.join(file_name);

        match fs::rename(&file_path, &dest).await {
            Ok(_) => {
                log::debug!("Archived: {} -> {}", file_path.display(), dest.display());
                archived += 1;
            }
            Err(e) => {
                // Log error but continue with other files
                log::error!("Failed to move {}: {}", file_path.display(), e);
            }
        }
    }

    // Clean ephemeral data directory (JSON is regenerated each briefing)
    let data_cleaned = clean_data_directory(&today_dir).await;
    if data_cleaned > 0 {
        log::info!("Archive: cleaned {} data files", data_cleaned);
    }

    log::info!(
        "Archive complete: {} files moved to {}",
        archived,
        archive_dir.display()
    );

    Ok(ArchiveResult {
        files_archived: archived,
        archive_path: archive_dir.to_string_lossy().to_string(),
    })
}

/// Collect files that should be archived from _today/
///
/// Includes: *.md files (except week-*.md)
/// Excludes: directories, non-md files, week-*.md files
async fn collect_archivable_files(
    today_dir: &Path,
) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files = Vec::new();

    let mut entries = fs::read_dir(today_dir)
        .await
        .map_err(|e| format!("Failed to read _today/: {}", e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read directory entry: {}", e))?
    {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            // Include only .md files
            if !name.ends_with(".md") {
                continue;
            }

            // Skip week-*.md files (preserved for weekly view)
            if name.starts_with("week-") {
                log::debug!("Preserving weekly file: {}", name);
                continue;
            }

            files.push(path);
        }
    }

    Ok(files)
}

/// Clean ephemeral data directory after archiving.
///
/// Removes all JSON files in `_today/data/` and the `preps/` subdirectory.
/// These are generated output — the next briefing writes fresh JSON.
async fn clean_data_directory(today_dir: &Path) -> usize {
    let data_dir = today_dir.join("data");
    if !data_dir.exists() {
        return 0;
    }

    let mut cleaned = 0;

    // Remove all JSON files in data/
    if let Ok(mut entries) = fs::read_dir(&data_dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
                if fs::remove_file(&path).await.is_ok() {
                    cleaned += 1;
                }
            }
        }
    }

    // Remove preps/ directory
    let preps_dir = data_dir.join("preps");
    if preps_dir.exists() {
        let _ = fs::remove_dir_all(&preps_dir).await;
        cleaned += 1;
    }

    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_archive_nonexistent_today() {
        let temp = TempDir::new().unwrap();
        // Don't create _today/ directory

        let result = run_archive(temp.path()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.files_archived, 0);
        assert!(result.archive_path.is_empty());
    }

    #[tokio::test]
    async fn test_archive_empty_today() {
        let temp = TempDir::new().unwrap();
        let today_dir = temp.path().join("_today");
        fs::create_dir(&today_dir).await.unwrap();

        let result = run_archive(temp.path()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.files_archived, 0);
        assert!(result.archive_path.is_empty());
    }

    #[tokio::test]
    async fn test_archive_moves_md_files() {
        let temp = TempDir::new().unwrap();
        let today_dir = temp.path().join("_today");
        fs::create_dir(&today_dir).await.unwrap();

        // Create test files
        fs::write(today_dir.join("overview.md"), "# Overview").await.unwrap();
        fs::write(today_dir.join("actions.md"), "# Actions").await.unwrap();

        let result = run_archive(temp.path()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.files_archived, 2);

        // Files should be moved
        assert!(!today_dir.join("overview.md").exists());
        assert!(!today_dir.join("actions.md").exists());

        // Archive should exist with files
        assert!(result.archive_path.contains("archive"));
    }

    #[tokio::test]
    async fn test_archive_preserves_week_files() {
        let temp = TempDir::new().unwrap();
        let today_dir = temp.path().join("_today");
        fs::create_dir(&today_dir).await.unwrap();

        // Create test files
        fs::write(today_dir.join("overview.md"), "# Overview").await.unwrap();
        fs::write(today_dir.join("week-overview.md"), "# Week").await.unwrap();
        fs::write(today_dir.join("week-actions.md"), "# Week Actions").await.unwrap();

        let result = run_archive(temp.path()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.files_archived, 1); // Only overview.md

        // week-* files should remain
        assert!(today_dir.join("week-overview.md").exists());
        assert!(today_dir.join("week-actions.md").exists());

        // overview.md should be moved
        assert!(!today_dir.join("overview.md").exists());
    }

    #[tokio::test]
    async fn test_archive_ignores_non_md_files() {
        let temp = TempDir::new().unwrap();
        let today_dir = temp.path().join("_today");
        fs::create_dir(&today_dir).await.unwrap();

        // Create test files
        fs::write(today_dir.join("overview.md"), "# Overview").await.unwrap();
        fs::write(today_dir.join("notes.txt"), "Some notes").await.unwrap();
        fs::write(today_dir.join("data.json"), "{}").await.unwrap();

        let result = run_archive(temp.path()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.files_archived, 1); // Only overview.md

        // Non-md files should remain
        assert!(today_dir.join("notes.txt").exists());
        assert!(today_dir.join("data.json").exists());
    }

    #[tokio::test]
    async fn test_archive_ignores_subdirectories() {
        let temp = TempDir::new().unwrap();
        let today_dir = temp.path().join("_today");
        fs::create_dir(&today_dir).await.unwrap();

        // Create subdirectory and files
        fs::write(today_dir.join("overview.md"), "# Overview").await.unwrap();
        let subdir = today_dir.join("meetings");
        fs::create_dir(&subdir).await.unwrap();
        fs::write(subdir.join("prep.md"), "# Prep").await.unwrap();

        let result = run_archive(temp.path()).await;

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.files_archived, 1); // Only overview.md

        // Subdirectory should remain
        assert!(subdir.exists());
        assert!(subdir.join("prep.md").exists());
    }

    #[tokio::test]
    async fn test_archive_cleans_data_directory() {
        let temp = TempDir::new().unwrap();
        let today_dir = temp.path().join("_today");
        let data_dir = today_dir.join("data");
        let preps_dir = data_dir.join("preps");

        fs::create_dir_all(&preps_dir).await.unwrap();

        // Create markdown file to trigger archiving
        fs::write(today_dir.join("overview.md"), "# Overview").await.unwrap();

        // Create data files that should be cleaned
        fs::write(data_dir.join("schedule.json"), "{}").await.unwrap();
        fs::write(data_dir.join("actions.json"), "{}").await.unwrap();
        fs::write(data_dir.join("manifest.json"), "{}").await.unwrap();
        fs::write(preps_dir.join("0900-customer-acme.json"), "{}").await.unwrap();

        let result = run_archive(temp.path()).await;
        assert!(result.is_ok());

        // Data JSON files should be removed
        assert!(!data_dir.join("schedule.json").exists());
        assert!(!data_dir.join("actions.json").exists());
        assert!(!data_dir.join("manifest.json").exists());

        // Preps directory should be removed
        assert!(!preps_dir.exists());

        // data/ directory itself should still exist (only contents cleaned)
        assert!(data_dir.exists());
    }
}
