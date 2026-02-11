use std::path::{Path, PathBuf};

// ─── Entity Directory Template ──────────────────────────────────────────────
//
// Core subdirectories created inside every entity (account or project).
// These are app-managed — the user can add more, but these always exist.

/// App-managed subdirectory names inside entity directories.
/// Used by `is_bu_directory` to distinguish these from BU child accounts.
pub const MANAGED_ENTITY_DIRS: &[&str] = &["Call-Transcripts", "Meeting-Notes", "Documents"];

/// Bootstrap the standard directory template inside an entity directory.
///
/// Creates `Call-Transcripts/`, `Meeting-Notes/`, `Documents/` with README
/// files that help external tools (Claude Desktop, CLI tools) understand
/// the structure. Idempotent — skips existing directories, never overwrites.
pub fn bootstrap_entity_directory(
    entity_dir: &Path,
    entity_name: &str,
    entity_type: &str, // "account" or "project"
) -> Result<(), String> {
    // Root README
    let root_readme = entity_dir.join("README.md");
    if !root_readme.exists() {
        let content = format!(
            r#"# {name}

This directory is managed by [DailyOS](https://dailyos.dev). It contains operational intelligence for the {etype} "{name}".

## Structure

- `dashboard.json` — Structured data (factual fields, metrics). Machine-readable.
- `dashboard.md` — Generated overview. Human and AI readable. Do not edit directly.
- `intelligence.json` — AI-synthesized intelligence. Auto-updated when content changes.
- `Call-Transcripts/` — Meeting call transcripts with YAML frontmatter.
- `Meeting-Notes/` — Meeting summaries, notes, and outcomes.
- `Documents/` — General documents related to this {etype}.

## For AI Tools

Read `dashboard.md` for a comprehensive overview of this {etype}. For structured data, read `dashboard.json` and `intelligence.json`. All markdown files in this directory tree are indexed for intelligence enrichment — adding files here improves the AI's understanding of this {etype}.
"#,
            name = entity_name,
            etype = entity_type,
        );
        std::fs::write(&root_readme, content)
            .map_err(|e| format!("Failed to write README: {}", e))?;
    }

    // Subdirectories with READMEs
    let subdirs: &[(&str, &str)] = &[
        (
            "Call-Transcripts",
            "Meeting call transcripts. Files include YAML frontmatter with meeting metadata (ID, title, account, date, type). New transcripts placed here are automatically indexed for intelligence enrichment.",
        ),
        (
            "Meeting-Notes",
            "Meeting summaries, notes, and outcomes. Captures from post-meeting prompts and manually added notes. Indexed for intelligence enrichment.",
        ),
        (
            "Documents",
            "General documents related to this entity. Inbox-processed files, reports, and reference material. Any file added here is automatically indexed for intelligence enrichment.",
        ),
    ];

    for (dir_name, description) in subdirs {
        let dir_path = entity_dir.join(dir_name);
        if !dir_path.exists() {
            std::fs::create_dir_all(&dir_path)
                .map_err(|e| format!("Failed to create {}: {}", dir_name, e))?;
        }
        let readme_path = dir_path.join("README.md");
        if !readme_path.exists() {
            let content = format!(
                "# {dir}\n\n{desc}\n",
                dir = dir_name.replace('-', " "),
                desc = description,
            );
            std::fs::write(&readme_path, content)
                .map_err(|e| format!("Failed to write {}/README.md: {}", dir_name, e))?;
        }
    }

    Ok(())
}

/// Validates a filename resolves within the inbox directory.
/// Returns the resolved path or an error if traversal is detected.
pub fn validate_inbox_path(workspace: &Path, filename: &str) -> Result<PathBuf, String> {
    // Reject any path component traversal before joining
    if filename.contains("..") || filename.starts_with('/') || filename.starts_with('\\') {
        return Err("Invalid filename: path traversal detected".into());
    }
    let inbox_dir = workspace.join("_inbox");
    let file_path = inbox_dir.join(filename);
    if !file_path.starts_with(&inbox_dir) {
        return Err("Invalid filename: path traversal detected".into());
    }
    Ok(file_path)
}

/// Validates an entity name contains no path separators or traversal sequences.
pub fn validate_entity_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Name cannot be empty".into());
    }
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("Name contains invalid characters".into());
    }
    if name.contains(':')
        || name.contains('*')
        || name.contains('?')
        || name.contains('"')
        || name.contains('<')
        || name.contains('>')
        || name.contains('|')
    {
        return Err("Name contains filesystem-unsafe characters".into());
    }
    Ok(name)
}

/// Writes content to a file atomically: write to .tmp, then rename.
/// Rename is atomic on the same filesystem (POSIX guarantee).
pub fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    std::fs::write(&tmp_path, content)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Atomic write with string content (convenience).
pub fn atomic_write_str(path: &Path, content: &str) -> std::io::Result<()> {
    atomic_write(path, content.as_bytes())
}

/// Derive a person ID from an email address.
///
/// Example: "sarah.chen@acme.com" → "sarah-chen-acme-com"
pub fn person_id_from_email(email: &str) -> String {
    slugify(&email.to_lowercase())
}

/// Derive a display name from an email address (best-effort).
///
/// Example: "sarah.chen@acme.com" → "Sarah Chen"
pub fn name_from_email(email: &str) -> String {
    let local = email.split('@').next().unwrap_or(email);
    local
        .split(|c: char| c == '.' || c == '_' || c == '-' || c == '+')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Derive an organization name from an email domain (best-effort).
///
/// Example: "sarah.chen@acme.com" → "Acme"
pub fn org_from_email(email: &str) -> String {
    let domain = email.split('@').nth(1).unwrap_or("");
    let org_part = domain.split('.').next().unwrap_or(domain);
    let mut chars = org_part.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// Classify a person as internal/external based on the user's domain.
///
/// Returns "internal" if the email domain matches `user_domain`,
/// "external" if it doesn't, or "unknown" if no `user_domain` is set.
pub fn classify_relationship(email: &str, user_domain: Option<&str>) -> String {
    match user_domain {
        Some(domain) if !domain.is_empty() => {
            classify_relationship_multi(email, &[domain.to_string()])
        }
        _ => "unknown".to_string(),
    }
}

/// Classify a person as internal/external based on multiple user domains (I171).
///
/// Returns "internal" if the email domain matches ANY of the user's domains,
/// "external" if it matches none, or "unknown" if the list is empty.
pub fn classify_relationship_multi(email: &str, user_domains: &[String]) -> String {
    if user_domains.is_empty() {
        return "unknown".to_string();
    }
    let email_domain = email.split('@').nth(1).unwrap_or("");
    if email_domain.is_empty() {
        return "unknown".to_string();
    }
    for domain in user_domains {
        if !domain.is_empty() && email_domain.eq_ignore_ascii_case(domain) {
            return "internal".to_string();
        }
    }
    "external".to_string()
}

/// Convert a display name to a URL-safe kebab-case slug.
///
/// Example: "Acme Corp" → "acme-corp"
pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Sanitize a string for use as a filesystem directory or file name (I70).
///
/// Strips `:*?"<>|`, replaces `/\` with `-`, trims leading/trailing dots and spaces,
/// and falls back to "unnamed" if the result is empty.
pub fn sanitize_for_filesystem(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .filter(|c| !matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|'))
        .map(|c| if c == '/' || c == '\\' { '-' } else { c })
        .collect();
    let trimmed = sanitized.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Acme Corp"), "acme-corp");
    }

    #[test]
    fn test_slugify_multi_word() {
        assert_eq!(slugify("Q2 Platform Migration"), "q2-platform-migration");
    }

    #[test]
    fn test_slugify_preserves_hyphens() {
        assert_eq!(slugify("Bring-a-Trailer"), "bring-a-trailer");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("Weekly Sync — Team Alpha"), "weekly-sync-team-alpha");
    }

    #[test]
    fn test_slugify_single_word() {
        assert_eq!(slugify("simple"), "simple");
    }

    // Person helper tests (I51)

    #[test]
    fn test_person_id_from_email() {
        assert_eq!(person_id_from_email("sarah.chen@acme.com"), "sarah-chen-acme-com");
        assert_eq!(person_id_from_email("JOE@BIGCORP.IO"), "joe-bigcorp-io");
    }

    #[test]
    fn test_name_from_email() {
        assert_eq!(name_from_email("sarah.chen@acme.com"), "Sarah Chen");
        assert_eq!(name_from_email("joe_smith@bigcorp.io"), "Joe Smith");
        assert_eq!(name_from_email("alice@example.com"), "Alice");
    }

    #[test]
    fn test_org_from_email() {
        assert_eq!(org_from_email("sarah@acme.com"), "Acme");
        assert_eq!(org_from_email("joe@bigcorp.io"), "Bigcorp");
    }

    #[test]
    fn test_classify_relationship() {
        assert_eq!(classify_relationship("me@myco.com", Some("myco.com")), "internal");
        assert_eq!(classify_relationship("them@other.com", Some("myco.com")), "external");
        assert_eq!(classify_relationship("anyone@any.com", None), "unknown");
        assert_eq!(classify_relationship("anyone@any.com", Some("")), "unknown");
    }

    #[test]
    fn test_classify_relationship_multi() {
        let domains = vec!["myco.com".to_string(), "subsidiary.com".to_string()];
        assert_eq!(classify_relationship_multi("me@myco.com", &domains), "internal");
        assert_eq!(classify_relationship_multi("you@subsidiary.com", &domains), "internal");
        assert_eq!(classify_relationship_multi("them@other.com", &domains), "external");
        assert_eq!(classify_relationship_multi("anyone@any.com", &[]), "unknown");
        assert_eq!(classify_relationship_multi("no-at-sign", &domains), "unknown");
    }

    // Path traversal guard tests (I60)

    #[test]
    fn test_validate_inbox_path_valid() {
        let workspace = Path::new("/workspace");
        assert!(validate_inbox_path(workspace, "report.pdf").is_ok());
        assert!(validate_inbox_path(workspace, "subdir/file.md").is_ok());
    }

    #[test]
    fn test_validate_inbox_path_traversal() {
        let workspace = Path::new("/workspace");
        assert!(validate_inbox_path(workspace, "../../etc/passwd").is_err());
        assert!(validate_inbox_path(workspace, "../../../bar").is_err());
        assert!(validate_inbox_path(workspace, "foo/../../outside").is_err());
    }

    // Entity name validation tests (I60)

    #[test]
    fn test_validate_entity_name_valid() {
        assert_eq!(validate_entity_name("Acme Corp"), Ok("Acme Corp"));
        assert_eq!(validate_entity_name("  Beta Inc  "), Ok("Beta Inc"));
    }

    #[test]
    fn test_validate_entity_name_empty() {
        assert!(validate_entity_name("").is_err());
        assert!(validate_entity_name("   ").is_err());
    }

    #[test]
    fn test_validate_entity_name_traversal() {
        assert!(validate_entity_name("../etc").is_err());
        assert!(validate_entity_name("foo/bar").is_err());
        assert!(validate_entity_name("foo\\bar").is_err());
    }

    #[test]
    fn test_validate_entity_name_unsafe_chars() {
        assert!(validate_entity_name("foo:bar").is_err());
        assert!(validate_entity_name("foo*bar").is_err());
        assert!(validate_entity_name("foo?bar").is_err());
        assert!(validate_entity_name("foo\"bar").is_err());
        assert!(validate_entity_name("foo<bar").is_err());
        assert!(validate_entity_name("foo>bar").is_err());
        assert!(validate_entity_name("foo|bar").is_err());
    }

    // Filesystem sanitization tests (I70)

    #[test]
    fn test_sanitize_for_filesystem_basic() {
        assert_eq!(sanitize_for_filesystem("Acme Corp"), "Acme Corp");
    }

    #[test]
    fn test_sanitize_for_filesystem_strips_unsafe() {
        assert_eq!(sanitize_for_filesystem("foo:bar*baz"), "foobarbaz");
        assert_eq!(sanitize_for_filesystem("what?\"yes\""), "whatyes");
        assert_eq!(sanitize_for_filesystem("<tag>|pipe"), "tagpipe");
    }

    #[test]
    fn test_sanitize_for_filesystem_replaces_slashes() {
        assert_eq!(sanitize_for_filesystem("foo/bar\\baz"), "foo-bar-baz");
    }

    #[test]
    fn test_sanitize_for_filesystem_trims_dots_spaces() {
        assert_eq!(sanitize_for_filesystem("  ..hidden..  "), "hidden");
    }

    #[test]
    fn test_sanitize_for_filesystem_fallback() {
        assert_eq!(sanitize_for_filesystem(""), "unnamed");
        assert_eq!(sanitize_for_filesystem("..."), "unnamed");
        assert_eq!(sanitize_for_filesystem(":*?"), "unnamed");
    }

    // Atomic write tests (I64)

    #[test]
    fn test_atomic_write_basic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.txt");
        atomic_write_str(&path, "hello world").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello world");
        // tmp file should be cleaned up
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn test_atomic_write_overwrite() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.txt");
        atomic_write_str(&path, "first").unwrap();
        atomic_write_str(&path, "second").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "second");
    }

    // Entity directory template tests

    #[test]
    fn test_bootstrap_entity_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entity_dir = dir.path().join("Acme");
        std::fs::create_dir_all(&entity_dir).unwrap();

        bootstrap_entity_directory(&entity_dir, "Acme", "account").unwrap();

        // Verify subdirectories exist
        assert!(entity_dir.join("Call-Transcripts").is_dir());
        assert!(entity_dir.join("Meeting-Notes").is_dir());
        assert!(entity_dir.join("Documents").is_dir());

        // Verify READMEs exist
        assert!(entity_dir.join("README.md").exists());
        assert!(entity_dir.join("Call-Transcripts/README.md").exists());
        assert!(entity_dir.join("Meeting-Notes/README.md").exists());
        assert!(entity_dir.join("Documents/README.md").exists());

        // Verify root README has entity name and type
        let readme = std::fs::read_to_string(entity_dir.join("README.md")).unwrap();
        assert!(readme.contains("Acme"));
        assert!(readme.contains("account"));
    }

    #[test]
    fn test_bootstrap_entity_directory_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entity_dir = dir.path().join("Beta");
        std::fs::create_dir_all(&entity_dir).unwrap();

        // Write a custom README first
        std::fs::write(entity_dir.join("README.md"), "# Custom").unwrap();

        bootstrap_entity_directory(&entity_dir, "Beta", "project").unwrap();

        // Custom README should NOT be overwritten
        let readme = std::fs::read_to_string(entity_dir.join("README.md")).unwrap();
        assert_eq!(readme, "# Custom");

        // But subdirectories should still be created
        assert!(entity_dir.join("Call-Transcripts").is_dir());
    }

    #[test]
    fn test_managed_entity_dirs_constant() {
        assert_eq!(MANAGED_ENTITY_DIRS.len(), 3);
        assert!(MANAGED_ENTITY_DIRS.contains(&"Call-Transcripts"));
        assert!(MANAGED_ENTITY_DIRS.contains(&"Meeting-Notes"));
        assert!(MANAGED_ENTITY_DIRS.contains(&"Documents"));
    }
}
