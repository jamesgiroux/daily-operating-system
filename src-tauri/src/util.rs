use std::path::{Path, PathBuf};

use tauri::path::BaseDirectory;
use tauri::{AppHandle, Manager};

/// Resolve the path to a bundled Python script.
///
/// Priority chain (I59):
/// 1. Dev mode: `CARGO_MANIFEST_DIR/../scripts/{name}` (works in tests + `pnpm tauri dev`)
/// 2. Production: Tauri resource bundle (`$RESOURCE/scripts/{name}`)
/// 3. Fallback: workspace `_tools/{name}` (CLI-era compatibility, ADR-0025)
pub fn resolve_script_path(app_handle: &AppHandle, workspace: &Path, script_name: &str) -> PathBuf {
    // 1. Dev mode — compile-time constant, always works in tests and tauri dev
    if cfg!(debug_assertions) {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or(Path::new("."));
        let dev_script = repo_root.join("scripts").join(script_name);
        if dev_script.exists() {
            return dev_script;
        }
    }

    // 2. Production — Tauri-bundled resource
    if let Ok(resource_path) = app_handle
        .path()
        .resolve(format!("scripts/{}", script_name), BaseDirectory::Resource)
    {
        if resource_path.exists() {
            return resource_path;
        }
    }

    // 3. Fallback — workspace _tools/ (CLI-era scripts)
    let workspace_script = workspace.join("_tools").join(script_name);
    if workspace_script.exists() {
        return workspace_script;
    }

    // Not found — return a descriptive path for the error message.
    // In dev mode: repo path. In production: resource path.
    if cfg!(debug_assertions) {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or(Path::new("."));
        repo_root.join("scripts").join(script_name)
    } else {
        // Best-effort: return the resource path even if resolve failed
        PathBuf::from(format!("scripts/{}", script_name))
    }
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
            let email_domain = email.split('@').nth(1).unwrap_or("");
            if email_domain.eq_ignore_ascii_case(domain) {
                "internal".to_string()
            } else {
                "external".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
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
}
