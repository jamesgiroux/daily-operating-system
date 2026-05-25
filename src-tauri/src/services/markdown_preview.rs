//! Markdown preview read service for workspace sources.
//!
//! The service resolves opaque workspace source handles to already-ingested
//! workspace files, re-validates the file through `WorkspaceSourceRegistry`,
//! renders Markdown to HTML, and strips active/raw HTML surfaces before the
//! result leaves the runtime boundary.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use abilities_runtime::abilities::markdown_preview::contracts::{
    MarkdownPreviewOutput, MarkdownPreviewReadRequest,
};
use abilities_runtime::services::context::MarkdownPreviewReadError;
use pulldown_cmark::{html, CowStr, Event, Options, Parser, Tag, TagEnd};
use rusqlite::{params, Connection, OptionalExtension};

use crate::services::workspace_ingestion::registry::WorkspaceSourceRegistry;

const SCHEMA_VERSION: u32 = 1;
const MAX_PREVIEW_BYTES: u64 = 1_048_576;
const SANITIZER_VERSION: &str = "markdown-preview-v1";

#[derive(Debug, Clone)]
struct ResolvedPreviewSource {
    canonical_path: PathBuf,
    source_asof: String,
    lifecycle_state: String,
    content_type: Option<String>,
}

pub fn read_markdown_preview(
    conn: &Connection,
    workspace_root: &Path,
    request: MarkdownPreviewReadRequest,
) -> Result<MarkdownPreviewOutput, MarkdownPreviewReadError> {
    let input = request.input;
    if input.schema_version != SCHEMA_VERSION {
        return Err(MarkdownPreviewReadError::InvalidSourceHandle(
            "unsupported schema version".to_string(),
        ));
    }
    validate_source_handle(&input.source_handle)?;

    let source = resolve_source(conn, &input.source_handle)?;
    ensure_markdown_source(&source)?;
    let (file, _identity) = WorkspaceSourceRegistry::open_validated(
        workspace_root,
        &source.canonical_path,
    )
    .map_err(|_| {
        MarkdownPreviewReadError::SourceUnavailable("workspace source unavailable".to_string())
    })?;
    let markdown = read_preview_text(file)?;
    let (preview_html, blocked_asset_count) = render_markdown(&markdown);

    Ok(MarkdownPreviewOutput {
        schema_version: SCHEMA_VERSION,
        preview_html,
        source_asof: source.source_asof,
        lifecycle_state: source.lifecycle_state,
        trust_band_summary: "needs_verification".to_string(),
        source_label: "Workspace source".to_string(),
        blocked_asset_count,
        asset_resolver_available: false,
        sanitizer_version: SANITIZER_VERSION.to_string(),
    })
}

fn validate_source_handle(source_handle: &str) -> Result<(), MarkdownPreviewReadError> {
    let valid = !source_handle.is_empty()
        && source_handle.len() <= 160
        && source_handle
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-'));
    if valid {
        Ok(())
    } else {
        Err(MarkdownPreviewReadError::InvalidSourceHandle(
            "source handle must be an opaque ASCII token".to_string(),
        ))
    }
}

fn resolve_source(
    conn: &Connection,
    source_handle: &str,
) -> Result<ResolvedPreviewSource, MarkdownPreviewReadError> {
    if let Some(source) = resolve_placement_source(conn, source_handle)? {
        return Ok(source);
    }
    if let Some(source) = resolve_run_source(conn, source_handle)? {
        return Ok(source);
    }
    Err(MarkdownPreviewReadError::SourceNotFound)
}

fn resolve_placement_source(
    conn: &Connection,
    source_handle: &str,
) -> Result<Option<ResolvedPreviewSource>, MarkdownPreviewReadError> {
    conn.query_row(
        "SELECT l.canonical_path,
                COALESCE(p.source_asof, l.source_asof),
                COALESCE(p.lifecycle_state, l.lifecycle_state),
                p.content_type
           FROM workspace_placement_idempotency p
           JOIN workspace_file_lifecycle l ON l.file_id = p.file_id
          WHERE p.source_handle = ?1
            AND p.status = 'succeeded'
          LIMIT 1",
        params![source_handle],
        |row| {
            Ok(ResolvedPreviewSource {
                canonical_path: PathBuf::from(row.get::<_, String>(0)?),
                source_asof: row.get(1)?,
                lifecycle_state: row.get(2)?,
                content_type: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|error| MarkdownPreviewReadError::SourceUnavailable(error.to_string()))
}

fn resolve_run_source(
    conn: &Connection,
    source_handle: &str,
) -> Result<Option<ResolvedPreviewSource>, MarkdownPreviewReadError> {
    conn.query_row(
        "SELECT l.canonical_path,
                l.source_asof,
                l.lifecycle_state
           FROM document_ingestion_runs r
           JOIN workspace_file_lifecycle l ON l.file_id = r.file_id
          WHERE r.run_id = ?1
            AND r.status = 'success'
          ORDER BY r.started_at DESC, r.id DESC
          LIMIT 1",
        params![source_handle],
        |row| {
            Ok(ResolvedPreviewSource {
                canonical_path: PathBuf::from(row.get::<_, String>(0)?),
                source_asof: row.get(1)?,
                lifecycle_state: row.get(2)?,
                content_type: None,
            })
        },
    )
    .optional()
    .map_err(|error| MarkdownPreviewReadError::SourceUnavailable(error.to_string()))
}

fn ensure_markdown_source(source: &ResolvedPreviewSource) -> Result<(), MarkdownPreviewReadError> {
    if matches!(
        source.content_type.as_deref(),
        Some("text/markdown") | Some("text/x-markdown")
    ) {
        return Ok(());
    }
    let extension = source
        .canonical_path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    if matches!(extension.as_deref(), Some("md") | Some("markdown")) {
        return Ok(());
    }
    Err(MarkdownPreviewReadError::UnsupportedSource(
        "source is not a markdown document".to_string(),
    ))
}

fn read_preview_text(mut file: File) -> Result<String, MarkdownPreviewReadError> {
    let mut bytes = Vec::new();
    let read = file
        .by_ref()
        .take(MAX_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| MarkdownPreviewReadError::SourceUnavailable(error.to_string()))?;
    if read as u64 > MAX_PREVIEW_BYTES || bytes.len() as u64 > MAX_PREVIEW_BYTES {
        return Err(MarkdownPreviewReadError::UnsupportedSource(
            "source exceeds markdown preview size limit".to_string(),
        ));
    }
    String::from_utf8(bytes).map_err(|_| {
        MarkdownPreviewReadError::UnsupportedSource("source is not valid UTF-8".to_string())
    })
}

fn render_markdown(markdown: &str) -> (String, u32) {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(markdown, options);
    let mut filtered = Vec::new();
    let mut blocked_asset_count = 0_u32;
    let mut blocked_image_depth = 0_u32;
    let mut blocked_link_depth = 0_u32;
    let mut blocked_html_tag: Option<&'static str> = None;

    for event in parser {
        if let Some(tag) = blocked_html_tag {
            if let Event::Html(value) | Event::InlineHtml(value) = &event {
                if closes_raw_html_tag(value.as_ref(), tag) {
                    blocked_html_tag = None;
                }
            }
            continue;
        }

        if blocked_image_depth > 0 {
            match event {
                Event::Start(Tag::Image { .. }) => {
                    blocked_image_depth = blocked_image_depth.saturating_add(1);
                }
                Event::End(TagEnd::Image) => {
                    blocked_image_depth = blocked_image_depth.saturating_sub(1);
                }
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                if is_allowed_href(dest_url.as_ref()) {
                    filtered.push(Event::Start(Tag::Link {
                        link_type,
                        dest_url,
                        title,
                        id,
                    }));
                } else {
                    blocked_link_depth = blocked_link_depth.saturating_add(1);
                }
            }
            Event::End(TagEnd::Link) if blocked_link_depth > 0 => {
                blocked_link_depth = blocked_link_depth.saturating_sub(1);
            }
            Event::Html(value) | Event::InlineHtml(value) => {
                if let Some(tag) = opens_dangerous_raw_html_tag(value.as_ref()) {
                    if !closes_raw_html_tag(value.as_ref(), tag) {
                        blocked_html_tag = Some(tag);
                    }
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                blocked_image_depth = 1;
                blocked_asset_count = blocked_asset_count.saturating_add(1);
                filtered.push(Event::Text(CowStr::Borrowed(blocked_asset_label(
                    dest_url.as_ref(),
                ))));
            }
            Event::End(TagEnd::Image) => {}
            other => filtered.push(other),
        }
    }

    let mut output = String::new();
    html::push_html(&mut output, filtered.into_iter());
    (output, blocked_asset_count)
}

fn is_allowed_href(href: &str) -> bool {
    if href.trim() != href || href.is_empty() || href.starts_with("//") {
        return false;
    }
    if href.chars().any(char::is_control) {
        return false;
    }
    match url::Url::parse(href) {
        Ok(parsed) => matches!(parsed.scheme(), "http" | "https" | "mailto"),
        Err(_) => false,
    }
}

fn opens_dangerous_raw_html_tag(html: &str) -> Option<&'static str> {
    let lower = html.to_ascii_lowercase();
    [
        "script", "style", "iframe", "object", "embed", "svg", "math",
    ]
    .into_iter()
    .find(|tag| lower.contains(&format!("<{tag}")))
}

fn closes_raw_html_tag(html: &str, tag: &str) -> bool {
    html.to_ascii_lowercase().contains(&format!("</{tag}>"))
}

fn blocked_asset_label(url: &str) -> &'static str {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:") {
        "[remote asset blocked]"
    } else {
        "[local asset blocked]"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_preview_db() -> (tempfile::TempDir, Connection, PathBuf) {
        let root = tempfile::tempdir().expect("tempdir");
        let path = root.path().join("source.md");
        std::fs::write(
            &path,
            "# Preview\n\nBody<script>alert(1)</script>\n\n![Track](https://example.invalid/pixel.png)",
        )
        .expect("write markdown");
        let canonical_path = path.canonicalize().expect("canonical path");
        let conn = Connection::open_in_memory().expect("conn");
        conn.execute_batch(
            "CREATE TABLE workspace_file_lifecycle (
                file_id TEXT PRIMARY KEY,
                canonical_path TEXT NOT NULL,
                source_asof TEXT NOT NULL,
                lifecycle_state TEXT NOT NULL
             );
             CREATE TABLE workspace_placement_idempotency (
                source_handle TEXT,
                status TEXT NOT NULL,
                file_id TEXT NOT NULL,
                source_asof TEXT,
                lifecycle_state TEXT,
                content_type TEXT
             );
             CREATE TABLE document_ingestion_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                status TEXT NOT NULL,
                started_at TEXT NOT NULL
             );",
        )
        .expect("schema");
        crate::services::workspace_ingestion::graph::insert_markdown_preview_lifecycle_fixture_for_tests(
            &conn,
            canonical_path.to_string_lossy().as_ref(),
        );
        (root, conn, canonical_path)
    }

    #[test]
    fn placement_source_renders_markdown_and_blocks_unsafe_html_and_assets() {
        let (root, conn, _path) = setup_preview_db();
        conn.execute(
            "INSERT INTO workspace_placement_idempotency
                (source_handle, status, file_id, source_asof, lifecycle_state, content_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "source_alpha",
                "succeeded",
                "file-alpha",
                "2026-05-24T10:00:00.000Z",
                "ingested",
                "text/markdown"
            ],
        )
        .expect("placement");

        let output = read_markdown_preview(
            &conn,
            root.path(),
            MarkdownPreviewReadRequest {
                input: crate::abilities::markdown_preview::contracts::MarkdownPreviewInput {
                    schema_version: 1,
                    source_handle: "source_alpha".to_string(),
                },
            },
        )
        .expect("preview");

        assert!(output.preview_html.contains("<h1>Preview</h1>"));
        assert!(output.preview_html.contains("[remote asset blocked]"));
        assert!(!output.preview_html.contains("<script"));
        assert!(!output.preview_html.contains("alert(1)"));
        assert!(!output.preview_html.contains("https://example.invalid"));
        assert_eq!(output.blocked_asset_count, 1);
        assert_eq!(output.asset_resolver_available, false);
        let serialized = serde_json::to_string(&output).expect("serialize");
        assert!(!serialized.contains("source_alpha"));
        assert!(!serialized.contains("file-alpha"));
        assert!(!serialized.contains(root.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn ingestion_run_handle_resolves_markdown_source_without_placement_row() {
        let (root, conn, _path) = setup_preview_db();
        crate::services::workspace_ingestion::graph::insert_markdown_preview_run_fixture_for_tests(
            &conn,
            "11111111-2222-4333-8444-555555555555",
        );

        let output = read_markdown_preview(
            &conn,
            root.path(),
            MarkdownPreviewReadRequest {
                input: crate::abilities::markdown_preview::contracts::MarkdownPreviewInput {
                    schema_version: 1,
                    source_handle: "11111111-2222-4333-8444-555555555555".to_string(),
                },
            },
        )
        .expect("preview");

        assert!(output.preview_html.contains("<h1>Preview</h1>"));
        assert_eq!(output.source_label, "Workspace source");
    }

    #[test]
    fn unsafe_markdown_link_schemes_are_removed_before_output() {
        let (root, conn, path) = setup_preview_db();
        std::fs::write(
            &path,
            "[JS](javascript:alert%281%29) [Data](data:text/html;base64,AAAA) [File](file:///tmp/source.md) [Blob](blob:https://example.invalid/id) [VB](vbscript:msgbox%281%29) [Proto](//example.invalid/path) [Good](https://example.invalid/ok) [Mail](mailto:user@example.com)",
        )
        .expect("write unsafe markdown");
        conn.execute(
            "INSERT INTO workspace_placement_idempotency
                (source_handle, status, file_id, source_asof, lifecycle_state, content_type)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                "source_links",
                "succeeded",
                "file-alpha",
                "2026-05-24T10:00:00.000Z",
                "ingested",
                "text/markdown"
            ],
        )
        .expect("placement");

        let output = read_markdown_preview(
            &conn,
            root.path(),
            MarkdownPreviewReadRequest {
                input: crate::abilities::markdown_preview::contracts::MarkdownPreviewInput {
                    schema_version: 1,
                    source_handle: "source_links".to_string(),
                },
            },
        )
        .expect("preview");

        for forbidden in [
            "javascript:",
            "data:",
            "file:",
            "blob:",
            "vbscript:",
            "href=\"//example.invalid",
        ] {
            assert!(
                !output.preview_html.contains(forbidden),
                "preview leaked unsafe href scheme {forbidden}: {}",
                output.preview_html
            );
        }
        assert!(output.preview_html.contains("JS"));
        assert!(output
            .preview_html
            .contains("href=\"https://example.invalid/ok\""));
        assert!(output
            .preview_html
            .contains("href=\"mailto:user@example.com\""));
        assert!(!is_allowed_href("https://control.invalid/\u{0001}"));
    }

    #[test]
    fn path_like_source_handle_is_rejected_before_db_lookup() {
        let (_root, conn, _path) = setup_preview_db();
        let error = read_markdown_preview(
            &conn,
            Path::new("/tmp"),
            MarkdownPreviewReadRequest {
                input: crate::abilities::markdown_preview::contracts::MarkdownPreviewInput {
                    schema_version: 1,
                    source_handle: "../secret.md".to_string(),
                },
            },
        )
        .expect_err("path-like handle rejected");

        assert!(matches!(
            error,
            MarkdownPreviewReadError::InvalidSourceHandle(_)
        ));
    }
}
