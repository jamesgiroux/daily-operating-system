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

// ─── Managed Workspace Files (I275) ─────────────────────────────────────────
//
// CLAUDE.md and .claude/settings.json are written to the workspace root so that
// Claude Code / Cowork automatically understands the workspace structure.
// Files are overwritten when the app version changes (sentinel check).

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// CLAUDE.md template written to workspace root.
/// First line is a version sentinel so we can detect staleness.
const CLAUDE_MD_TEMPLATE: &str = r#"<!-- dailyos:{{VERSION}} — managed by DailyOS, do not edit -->
# DailyOS Workspace

This is a DailyOS operational intelligence workspace. It contains structured intelligence about meetings, accounts, projects, people, and actions maintained by the DailyOS app.

## How to Use This Workspace

You have access to rich operational intelligence. Use it to help with meeting prep, account reviews, action completion, reports, and strategic thinking.

### Quick Start

- **Today's briefing:** Read `_today/data/briefing.json` for your daily narrative
- **Today's meetings:** Read `_today/data/schedule.json` for timeline and context
- **Account intelligence:** Read `Accounts/<name>/intelligence.json` for AI assessment
- **Open actions:** Read `_today/data/actions.json` for prioritized tasks

### Workspace Structure

```
_today/data/           → Today's briefing, schedule, actions, emails
_archive/YYYY-MM-DD/   → Historical briefings
Accounts/<name>/       → Account intelligence (dashboard.json, intelligence.json, dashboard.md)
Projects/<name>/       → Project intelligence (same structure)
People/<slug>.md       → Stakeholder profiles
_inbox/                → Incoming files for processing
```

### Intelligence Files

Each entity (account or project) has up to three files:

- **intelligence.json** — AI-synthesized assessment: executive summary, risks, wins, stakeholder insights, meeting readiness
- **dashboard.json** — Mechanical data: lifecycle, health, team, domains, metadata
- **dashboard.md** — Rich human-readable artifact combining both

### Schedule & Actions

- **schedule.json** — Array of today's meetings with `id`, `title`, `start_time`, `end_time`, `meeting_type`, `attendees`, `account_id`, `prep_status`
- **actions.json** — Prioritized open actions with `id`, `title`, `priority`, `status`, `due_date`, `account_id`, `project_id`, `context`
- **briefing.json** — Narrative daily briefing with `focus`, `sections[]`, AI-written synthesis

### Principles

- Intelligence files are current — trust them as the source of truth
- JSON files are for structured queries, markdown files are for narrative context
- Lead with conclusions, not data — the intelligence already synthesizes meaning
- Actions have entity context — follow `entity_id` links for full background

### Writing Deliverables

Place deliverables in entity subdirectories so DailyOS detects and indexes them:

- `Accounts/<name>/Documents/` — reports, analyses, deliverables
- `Accounts/<name>/Meeting-Notes/` — meeting summaries and outcomes
- `Accounts/<name>/Call-Transcripts/` — transcript files with YAML frontmatter
- `Projects/<name>/Documents/` — same structure for projects
"#;

/// Write managed workspace files (CLAUDE.md + .claude/settings.json).
///
/// Overwrites when the app version changes. Skips if already current.
/// Called from `initialize_workspace()` and on app startup.
pub fn write_managed_workspace_files(workspace: &Path) -> Result<(), String> {
    write_workspace_claude_md(workspace)?;
    write_workspace_claude_settings(workspace)?;
    Ok(())
}

/// Write CLAUDE.md with version sentinel. Skips if already current version.
fn write_workspace_claude_md(workspace: &Path) -> Result<(), String> {
    let claude_md_path = workspace.join("CLAUDE.md");
    let sentinel = format!("<!-- dailyos:{} ", APP_VERSION);

    // Check existing file for version match
    if claude_md_path.exists() {
        if let Ok(first_line) = read_first_line(&claude_md_path) {
            if first_line.contains(&sentinel) {
                return Ok(()); // Already current
            }
        }
    }

    let content = CLAUDE_MD_TEMPLATE.replace("{{VERSION}}", APP_VERSION);
    atomic_write_str(&claude_md_path, &content)
        .map_err(|e| format!("Failed to write CLAUDE.md: {}", e))?;

    log::info!(
        "Wrote workspace CLAUDE.md (v{}): {}",
        APP_VERSION,
        claude_md_path.display()
    );
    Ok(())
}

/// Write .claude/settings.json with version sentinel. Skips if already current.
fn write_workspace_claude_settings(workspace: &Path) -> Result<(), String> {
    let claude_dir = workspace.join(".claude");
    let settings_path = claude_dir.join("settings.json");

    // Check existing file for version match
    if settings_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&settings_path) {
            let version_field = format!("\"_version\": \"{}\"", APP_VERSION);
            if content.contains(&version_field) {
                return Ok(()); // Already current
            }
        }
    }

    // Ensure .claude/ directory exists
    if !claude_dir.exists() {
        std::fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("Failed to create .claude/: {}", e))?;
    }

    let settings = format!(
        r#"{{
  "_version": "{}",
  "_managedBy": "DailyOS",
  "permissions": {{
    "allow": [
      "Read",
      "Write",
      "Edit",
      "Glob",
      "Grep"
    ]
  }}
}}"#,
        APP_VERSION
    );

    atomic_write_str(&settings_path, &settings)
        .map_err(|e| format!("Failed to write .claude/settings.json: {}", e))?;

    log::info!(
        "Wrote workspace .claude/settings.json (v{}): {}",
        APP_VERSION,
        settings_path.display()
    );
    Ok(())
}

/// Read the first line of a file (for sentinel checking).
fn read_first_line(path: &Path) -> Result<String, std::io::Error> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line)
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

/// Validate an identifier passed across IPC boundaries.
///
/// Allows alphanumeric, hyphens, underscores, spaces, and dots — covers both
/// UUID-style IDs and AI-generated action IDs (e.g. "ai-2026-02-05 meeting_name-0").
/// Rejects path traversal sequences and control characters.
pub fn validate_id_slug(value: &str, field: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field} cannot be empty"));
    }
    if trimmed.len() > 200 {
        return Err(format!("{field} is too long (max 200 chars)"));
    }
    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(format!("{field} contains path traversal characters"));
    }
    if trimmed.chars().any(|c| c.is_control()) {
        return Err(format!("{field} contains control characters"));
    }
    Ok(trimmed.to_string())
}

/// Validate and normalize a user-facing string from IPC.
pub fn validate_bounded_string(
    value: &str,
    field: &str,
    min_len: usize,
    max_len: usize,
) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.len() < min_len {
        return Err(format!("{field} is too short (min {min_len} chars)"));
    }
    if trimmed.len() > max_len {
        return Err(format!("{field} is too long (max {max_len} chars)"));
    }
    Ok(trimmed.to_string())
}

pub fn validate_enum_string<'a>(
    value: &'a str,
    field: &str,
    allowed: &[&str],
) -> Result<&'a str, String> {
    if allowed.contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "Invalid {field}: {value}. Allowed values: {}",
            allowed.join(", ")
        ))
    }
}

pub fn validate_yyyy_mm_dd(value: &str, field: &str) -> Result<String, String> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| value.to_string())
        .map_err(|_| format!("Invalid {field}: expected YYYY-MM-DD"))
}

/// Validates email format with basic RFC 5322 compliance.
/// Returns trimmed lowercase email on success.
/// Accepts synthetic @unknown.local emails as valid.
pub fn validate_email(email: &str) -> Result<String, String> {
    let trimmed = email.trim().to_lowercase();

    let parts: Vec<&str> = trimmed.split('@').collect();
    if parts.len() != 2 {
        return Err("Email must contain exactly one @".to_string());
    }

    let (local, domain) = (parts[0], parts[1]);

    if local.is_empty() {
        return Err("Email local part cannot be empty".to_string());
    }
    if domain.is_empty() {
        return Err("Email domain cannot be empty".to_string());
    }

    // Domain must have at least one dot or be known synthetic
    if !domain.contains('.') && domain != "unknown.local" {
        return Err("Email domain must contain a dot".to_string());
    }

    // Reject invalid local part patterns
    if local.contains(' ') || local.contains("..") {
        return Err("Email local part contains invalid characters".to_string());
    }

    Ok(trimmed)
}

/// Normalize domain list: trim, lowercase, dedup, remove empty.
pub fn normalize_domains(domains: &[String]) -> Vec<String> {
    let mut out: Vec<String> = domains
        .iter()
        .map(|d| d.trim().to_lowercase())
        .filter(|d| !d.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
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
        .split(['.', '_', '-', '+'])
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
        assert_eq!(
            slugify("Weekly Sync — Team Alpha"),
            "weekly-sync-team-alpha"
        );
    }

    #[test]
    fn test_slugify_single_word() {
        assert_eq!(slugify("simple"), "simple");
    }

    // Person helper tests (I51)

    #[test]
    fn test_person_id_from_email() {
        assert_eq!(
            person_id_from_email("sarah.chen@acme.com"),
            "sarah-chen-acme-com"
        );
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
        assert_eq!(
            classify_relationship("me@myco.com", Some("myco.com")),
            "internal"
        );
        assert_eq!(
            classify_relationship("them@other.com", Some("myco.com")),
            "external"
        );
        assert_eq!(classify_relationship("anyone@any.com", None), "unknown");
        assert_eq!(classify_relationship("anyone@any.com", Some("")), "unknown");
    }

    #[test]
    fn test_classify_relationship_multi() {
        let domains = vec!["myco.com".to_string(), "subsidiary.com".to_string()];
        assert_eq!(
            classify_relationship_multi("me@myco.com", &domains),
            "internal"
        );
        assert_eq!(
            classify_relationship_multi("you@subsidiary.com", &domains),
            "internal"
        );
        assert_eq!(
            classify_relationship_multi("them@other.com", &domains),
            "external"
        );
        assert_eq!(
            classify_relationship_multi("anyone@any.com", &[]),
            "unknown"
        );
        assert_eq!(
            classify_relationship_multi("no-at-sign", &domains),
            "unknown"
        );
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

    // Email validation tests

    #[test]
    fn test_validate_email_valid() {
        assert_eq!(
            validate_email("test@example.com").unwrap(),
            "test@example.com"
        );
        assert_eq!(
            validate_email("  User@EXAMPLE.COM  ").unwrap(),
            "user@example.com"
        );
        assert_eq!(
            validate_email("person.abc@unknown.local").unwrap(),
            "person.abc@unknown.local"
        );
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(validate_email("no-at-sign").is_err());
        assert!(validate_email("@@double").is_err());
        assert!(validate_email("x@").is_err());
        assert!(validate_email("@missing").is_err());
        assert!(validate_email("user@nodot").is_err());
        assert!(validate_email("user @example.com").is_err());
        assert!(validate_email("us..er@example.com").is_err());
    }

    // Domain normalization tests

    #[test]
    fn test_normalize_domains() {
        let input = vec![
            "  Example.COM  ".to_string(),
            "".to_string(),
            "test.io".to_string(),
        ];
        assert_eq!(normalize_domains(&input), vec!["example.com", "test.io"]);
    }

    #[test]
    fn test_normalize_domains_dedup() {
        let input = vec![
            "acme.com".to_string(),
            "ACME.COM".to_string(),
            "other.io".to_string(),
        ];
        assert_eq!(normalize_domains(&input), vec!["acme.com", "other.io"]);
    }

    // Managed workspace files tests (I275)

    #[test]
    fn test_managed_files_fresh_write() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_managed_workspace_files(dir.path()).unwrap();

        // CLAUDE.md exists with sentinel
        let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.starts_with(&format!("<!-- dailyos:{}", APP_VERSION)));
        assert!(claude_md.contains("DailyOS Workspace"));
        assert!(claude_md.contains("briefing.json"));

        // .claude/settings.json exists with version
        let settings =
            std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        assert!(settings.contains(&format!("\"_version\": \"{}\"", APP_VERSION)));
        assert!(settings.contains("\"_managedBy\": \"DailyOS\""));
        assert!(settings.contains("\"Read\""));
    }

    #[test]
    fn test_managed_files_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_managed_workspace_files(dir.path()).unwrap();

        let md1 = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        let settings1 =
            std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();

        // Call again — should be a no-op
        write_managed_workspace_files(dir.path()).unwrap();

        let md2 = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        let settings2 =
            std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();

        assert_eq!(md1, md2);
        assert_eq!(settings1, settings2);
    }

    #[test]
    fn test_managed_files_version_bump_overwrites() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Write an old-version sentinel
        let old_content = "<!-- dailyos:0.0.0 — managed by DailyOS, do not edit -->\n# Old";
        std::fs::write(dir.path().join("CLAUDE.md"), old_content).unwrap();

        // Also write old settings
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(
            claude_dir.join("settings.json"),
            r#"{"_version": "0.0.0"}"#,
        )
        .unwrap();

        write_managed_workspace_files(dir.path()).unwrap();

        // Should be overwritten with current version
        let claude_md = std::fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
        assert!(claude_md.contains(&format!("dailyos:{}", APP_VERSION)));
        assert!(!claude_md.contains("0.0.0"));

        let settings =
            std::fs::read_to_string(dir.path().join(".claude/settings.json")).unwrap();
        assert!(settings.contains(&format!("\"_version\": \"{}\"", APP_VERSION)));
    }

    #[test]
    fn test_managed_files_creates_claude_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(!dir.path().join(".claude").exists());

        write_managed_workspace_files(dir.path()).unwrap();

        assert!(dir.path().join(".claude").is_dir());
        assert!(dir.path().join(".claude/settings.json").exists());
    }
}
